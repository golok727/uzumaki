use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::future::poll_fn;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use anyhow::{Context as _, Result};
use deno_core::futures::task::{ArcWake, waker};
use deno_core::{PollEventLoopOptions, v8};
use deno_runtime::worker::MainWorker;
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::window::{WindowAttributes, WindowButtons, WindowId, WindowLevel};
use winit::{application::ApplicationHandler, event::WindowEvent};

use crate::clipboard;
use crate::cursor;
use crate::element::UzNodeId;
use crate::event_dispatch;
use crate::gpu::GpuContext;
use crate::runtime::worker::{WorkerBuildOptions, create_worker};
use crate::terminal_colors;
use crate::ui::UIState;
use crate::window;

#[derive(Clone, Debug)]
pub struct AppConfig {
    /// Entry module to execute (resolved absolute path).
    pub entry: PathBuf,
    /// Root directory used for module/node resolution.
    pub app_root: PathBuf,
    /// Extra runtime args exposed via `Deno.args`.
    pub args: Vec<String>,
    /// Root directory for `Uz.path.resource(rel)`.
    /// In dev: project dir (where `uzumaki.config.json` sits, or the entry's parent).
    /// In standalone: `<exe_dir>/resources` (or wherever the host stages bundled files).
    pub resource_root: PathBuf,
    /// App identifier (e.g. `com.uzumaki.playground`). Used as the per-app
    /// folder name under the platform cache/data/config dirs.
    pub identifier: String,
    /// `import_source` injected by the automatic JSX transform. Defaults to
    /// `uzumaki-react`; configurable via `jsxImportSource` in
    /// `uzumaki.config.json` for users running their own renderer.
    pub jsx_import_source: Option<String>,
}

/// Estimated bytes a single retained Rust DOM node holds. Reported to V8 via
/// `adjust_amount_of_external_allocated_memory` so cppgc schedules collections
/// based on the real memory footprint, not just the size of the JS wrapper.
pub const NODE_EXTERNAL_BYTES: i64 = 1024;

pub struct WindowEntry {
    pub dom: UIState,
    pub handle: Option<window::Window>,
    pub rem_base: f32,
    pub cursor_blink_generation: u64,
    pub transparent: bool,
    pub window_level: WindowLevel,
    pub content_protected: bool,
    pub enabled_buttons: WindowButtons,
}

impl WindowEntry {
    /*
     * Inner size in logicl pixels
     */
    pub fn inner_size(&self) -> Option<(u32, u32)> {
        self.handle.as_ref().map(|handle| {
            let scale_factor = handle.winit_window.scale_factor();
            let size: winit::dpi::LogicalSize<u32> =
                handle.winit_window.inner_size().to_logical(scale_factor);

            (size.width, size.height)
        })
    }

    pub fn scale_factor(&self) -> Option<f32> {
        self.handle
            .as_ref()
            .map(|handle| handle.winit_window.scale_factor() as f32)
    }

    pub fn apply_cached_window_state(&self, attributes: WindowAttributes) -> WindowAttributes {
        attributes
            .with_transparent(self.transparent)
            .with_window_level(self.window_level)
            .with_content_protected(self.content_protected)
            .with_enabled_buttons(self.enabled_buttons)
    }
}

pub(crate) type WindowEntryId = u32;

pub struct AppState {
    pub windows: HashMap<WindowEntryId, WindowEntry>,
    pub winit_id_to_entry_id: HashMap<WindowId, WindowEntryId>,
    pub mouse_buttons: u8, // todo move to UIState ?
    pub modifiers: u32,    // same
    pub clipboard: RefCell<clipboard::SystemClipboard>,
    pub gpu: GpuContext,
    pub image_cache: HashMap<String, crate::element::ImageData>,
    /// Slab cleanup deferred from cppgc finalizers. Drained incrementally on
    /// each event-loop tick so a GC pause that frees thousands of CoreNodes
    /// never has to walk the slab synchronously.
    pub pending_destroy: VecDeque<(WindowEntryId, UzNodeId)>,
    /// Net change in external bytes attributable to retained DOM nodes since
    /// the last flush. Pushed to V8 with
    /// `Isolate::adjust_amount_of_external_allocated_memory` so cppgc sees the
    /// real cost of CoreNodes, not just the wrapper struct.
    pub external_memory_delta: i64,
}

impl AppState {
    pub fn winit_window_id_to_entry_id(&self, window_id: &WindowId) -> Option<WindowEntryId> {
        self.winit_id_to_entry_id.get(window_id).cloned()
    }

    pub fn paint_window(&mut self, id: &WindowEntryId) {
        if let Some(window) = self.windows.get_mut(id)
            && let Some(handle) = &mut window.handle
        {
            handle.paint_and_present(&mut window.dom);
        }
    }

    pub fn on_redraw_requested(&mut self, wid: &WindowEntryId) {
        if let Some(entry) = self.windows.get_mut(wid) {
            let WindowEntry { handle, dom, .. } = entry;
            if let Some(handle) = handle {
                event_dispatch::handle_redraw(dom, handle);
                // handle.winit_window.request_redraw();
            }
        }
    }
    /// Process up to `budget` deferred node destroys. Adaptive: large
    /// backlogs drain faster while small backlogs trickle through cheaply.
    pub fn drain_pending_destroy(&mut self) {
        let len = self.pending_destroy.len();
        if len == 0 {
            return;
        }
        let budget = (len / 8).clamp(64, 2048).min(len);
        for _ in 0..budget {
            let Some((window_id, node_id)) = self.pending_destroy.pop_front() else {
                break;
            };
            if let Some(entry) = self.windows.get_mut(&window_id) {
                entry.dom.destroy_node(node_id);
            }
        }
    }

    pub fn on_resize(&mut self, id: &WindowEntryId, width: u32, height: u32) -> bool {
        if let Some(window) = self.windows.get_mut(id)
            && let Some(handle) = &mut window.handle
            && handle.on_resize(width, height)
        {
            handle.winit_window.request_redraw();
            return true;
        }
        false
    }
}

// Safety: We only access AppState from the main thread
unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

pub(crate) type SharedAppState = Rc<RefCell<AppState>>;

pub(crate) fn with_state<R>(state: &SharedAppState, f: impl FnOnce(&mut AppState) -> R) -> R {
    f(&mut state.borrow_mut())
}

pub(crate) fn with_state_ref<R>(state: &SharedAppState, f: impl FnOnce(&AppState) -> R) -> R {
    f(&state.borrow())
}

#[derive(Debug, Clone)]
pub(crate) enum UserEvent {
    CreateWindow {
        id: u32,
        options: crate::ops::window::WindowOptions,
    },
    CloseWindow {
        id: u32,
    },
    RequestRedraw {
        id: u32,
    },
    CursorBlink {
        id: u32,
        generation: u64,
    },
    WakeJs,
    Quit,
}

struct JsWakeHandle {
    proxy: winit::event_loop::EventLoopProxy<UserEvent>,
    queued: AtomicBool,
}

impl JsWakeHandle {
    fn wake(&self) {
        if !self.queued.swap(true, Ordering::SeqCst) {
            let _ = self.proxy.send_event(UserEvent::WakeJs);
        }
    }

    fn clear(&self) {
        self.queued.store(false, Ordering::SeqCst);
    }
}

impl ArcWake for JsWakeHandle {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        JsWakeHandle::wake(arc_self.as_ref());
    }
}

pub struct Application {
    // for now lets use this, we should write our own runtime in future :p
    worker: MainWorker,
    app_state: SharedAppState,
    main_file: PathBuf,
    app_root: PathBuf,
    event_loop: Option<winit::event_loop::EventLoop<UserEvent>>,
    module_loaded: bool,
    pub(crate) tokio_runtime: tokio::runtime::Runtime,
    global_app_event_dispatch_fn: v8::Global<v8::Function>,
    js_wake_handle: Arc<JsWakeHandle>,
}

impl Application {
    pub fn new_with_root(
        startup_snapshot: Option<&'static [u8]>,
        config: AppConfig,
    ) -> Result<Self> {
        let tokio_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .expect("failed to create tokio runtime");

        let main_file: PathBuf = config.entry.clone();
        let app_root: PathBuf = config.app_root.clone();

        let mut worker = {
            let _guard = tokio_runtime.enter();
            create_worker(WorkerBuildOptions {
                entry: &main_file,
                app_root: &app_root,
                args: config.args.clone(),
                headless: false,
                jsx_import_source: config.jsx_import_source.clone(),
                extensions: vec![crate::uzumaki::init()],
                startup_snapshot,
            })?
        };

        let global_app_event_dispatch_fn = {
            let context = worker.js_runtime.main_context();
            deno_core::scope!(scope, &mut worker.js_runtime);
            let context_local = v8::Local::new(scope, context);
            let global_obj = context_local.global(scope);

            let key = v8::String::new_external_onebyte_static(scope, b"__uzumaki_on_app_event__")
                .ok_or_else(|| anyhow::anyhow!("failed to create v8 string"))?;

            let val = global_obj.get(scope, key.into()).ok_or_else(|| {
                anyhow::anyhow!("__uzumaki_on_app_event__ not found on globalThis")
            })?;

            let func = v8::Local::<v8::Function>::try_from(val)
                .map_err(|_| anyhow::anyhow!("__uzumaki_on_app_event__ is not a function"))?;

            v8::Global::new(scope, func)
        };

        let event_loop: winit::event_loop::EventLoop<UserEvent> =
            winit::event_loop::EventLoop::with_user_event().build()?;
        let event_loop_proxy = event_loop.create_proxy();
        let js_wake_handle = Arc::new(JsWakeHandle {
            proxy: event_loop_proxy.clone(),
            queued: AtomicBool::new(false),
        });

        // Create GPU context
        let gpu = pollster::block_on(GpuContext::new()).expect("Failed to create GPU context");

        let system_clipboard =
            clipboard::SystemClipboard::new().expect("failed to initialize system clipboard");

        let app_state = Rc::new(RefCell::new(AppState {
            gpu,
            windows: HashMap::new(),
            winit_id_to_entry_id: HashMap::new(),
            mouse_buttons: 0,
            modifiers: 0,
            clipboard: RefCell::new(system_clipboard),
            image_cache: HashMap::new(),
            pending_destroy: VecDeque::new(),
            external_memory_delta: 0,
        }));

        // Put shared state and event loop proxy into OpState
        {
            let op_state = worker.js_runtime.op_state();
            let mut borrow = op_state.borrow_mut();
            borrow.put(app_state.clone());
            borrow.put(event_loop_proxy);
            borrow.put(config);
        }

        Ok(Self {
            worker,
            app_state,
            main_file,
            app_root,
            event_loop: Some(event_loop),
            module_loaded: false,
            tokio_runtime,
            global_app_event_dispatch_fn,
            js_wake_handle,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let Some(mut event_loop) = self.event_loop.take() else {
            return Ok(());
        };

        loop {
            if let Err(err) = self.pump_js() {
                print_runtime_error(&err);
            }

            let status = event_loop.pump_app_events(Some(Duration::from_millis(16)), self);
            if let PumpStatus::Exit(_) = status {
                break;
            }
        }

        Ok(())
    }

    fn pump_js(&mut self) -> Result<()> {
        let wake_handle = self.js_wake_handle.clone();
        wake_handle.clear();

        let (delta, has_pending) = {
            let mut state = self.app_state.borrow_mut();
            let delta = std::mem::take(&mut state.external_memory_delta);
            (delta, !state.pending_destroy.is_empty())
        };
        if delta != 0 {
            self.worker
                .js_runtime
                .v8_isolate()
                .adjust_amount_of_external_allocated_memory(delta);
        }

        if has_pending {
            self.app_state.borrow_mut().drain_pending_destroy();
        }

        let rt = &self.tokio_runtime;
        let worker = &mut self.worker;
        rt.block_on(async {
            tokio::task::yield_now().await;
            poll_fn(|_| {
                let waker = waker(wake_handle.clone());
                let mut cx = Context::from_waker(&waker);
                match worker
                    .js_runtime
                    .poll_event_loop(&mut cx, PollEventLoopOptions::default())
                {
                    Poll::Ready(Ok(())) | Poll::Pending => {}
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(anyhow::Error::new(e))),
                }
                Poll::Ready(Ok(()))
            })
            .await?;
            Ok::<_, anyhow::Error>(())
        })?;

        if !self.app_state.borrow().pending_destroy.is_empty() {
            JsWakeHandle::wake(&self.js_wake_handle);
        }

        Ok(())
    }

    fn load_main_module(&mut self) -> Result<()> {
        let specifier = deno_core::resolve_path(
            self.main_file
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("entry path is not valid utf-8"))?,
            &self.app_root,
        )
        .context("failed to resolve main module path")?;

        let rt = &self.tokio_runtime;
        rt.block_on(async { self.worker.execute_main_module(&specifier).await })
            .with_context(|| format!("failed to execute main module {specifier}"))?;
        self.pump_js()?;
        Ok(())
    }

    /// Dispatch an event to JS. Returns true if `preventDefault()` was called.
    fn dispatch_event_to_js(&mut self, event: &event_dispatch::AppEvent) -> bool {
        let rt = &self.tokio_runtime;
        // Deno's timer ops require an active Tokio runtime. App events are invoked
        // directly from winit callbacks, so we need to re-enter the runtime before
        // calling into JS event handlers.
        let _guard = rt.enter();

        // while the scope borrows self.worker.js_runtime
        let dispatch_fn = &self.global_app_event_dispatch_fn;

        let context = self.worker.js_runtime.main_context();
        deno_core::scope!(scope, &mut self.worker.js_runtime);
        v8::tc_scope!(scope, scope);

        let context_local = v8::Local::new(scope, context);
        let _global_obj = context_local.global(scope);

        let func = v8::Local::new(scope, dispatch_fn);
        let undefined = v8::undefined(scope);

        let event_val = match deno_core::serde_v8::to_v8(scope, event) {
            Ok(val) => val,
            Err(e) => {
                eprintln!(
                    "{} failed to serialize event: {e}",
                    terminal_colors::red_bold("Error")
                );
                return false;
            }
        };

        let result = func.call(scope, undefined.into(), &[event_val]);

        if let Some(exception) = scope.exception() {
            let error = deno_core::error::JsError::from_v8_exception(scope, exception);
            eprintln!("{} {error}", terminal_colors::red_bold("Error"));
            return false;
        }

        // JS returns true if defaultPrevented
        result.map(|v| v.is_true()).unwrap_or(false)
    }

    fn spawn_cursor_blink_timer(&self, id: WindowEntryId, generation: u64, delay: Duration) {
        let proxy = self.js_wake_handle.proxy.clone();
        let handle = self.tokio_runtime.handle().clone();
        handle.spawn(async move {
            tokio::time::sleep(delay).await;
            let _ = proxy.send_event(UserEvent::CursorBlink { id, generation });
        });
    }

    fn refresh_cursor_blink_timer(&mut self, id: WindowEntryId) {
        let next_timer = {
            let mut state = self.app_state.borrow_mut();
            let Some(entry) = state.windows.get_mut(&id) else {
                return;
            };

            entry.cursor_blink_generation = entry.cursor_blink_generation.wrapping_add(1);
            let generation = entry.cursor_blink_generation;
            let next_delay = entry
                .dom
                .focused_node
                .and_then(|focused_id| entry.dom.nodes.get(focused_id))
                .and_then(|node| node.as_text_input())
                .and_then(|input| input.next_blink_toggle_in(true, entry.dom.window_focused));

            next_delay.map(|delay| (generation, delay))
        };

        if let Some((generation, delay)) = next_timer {
            self.spawn_cursor_blink_timer(id, generation, delay);
        }
    }

    fn close_window(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        wid: WindowEntryId,
    ) {
        self.dispatch_event_to_js(&event_dispatch::AppEvent::WindowClose(
            event_dispatch::WindowLoadEventData { window_id: wid },
        ));

        let mut state = self.app_state.borrow_mut();
        let winit_id = state
            .windows
            .get(&wid)
            .and_then(|entry| entry.handle.as_ref().map(|handle| handle.winit_window.id()));

        if let Some(winit_id) = winit_id {
            state.winit_id_to_entry_id.remove(&winit_id);
        }

        state.windows.remove(&wid);
        if state.windows.is_empty() {
            event_loop.exit();
        }
    }
}

impl ApplicationHandler<UserEvent> for Application {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if !self.module_loaded {
            self.module_loaded = true;
            if let Err(err) = self.load_main_module() {
                print_runtime_error(&err);
                std::process::exit(1);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::CreateWindow { id, options } => {
                let Some(attributes) =
                    self.app_state.borrow().windows.get(&id).map(|entry| {
                        entry.apply_cached_window_state(options.to_window_attributes())
                    })
                else {
                    eprintln!("Window entry '{id}' missing before creating handle");
                    return;
                };
                let is_visible = attributes.visible;
                let transparent = attributes.transparent;

                let Ok(winit_window) = event_loop.create_window(attributes.with_visible(false))
                else {
                    eprintln!("Failed to create window");
                    return;
                };
                winit_window.set_ime_allowed(true);

                let winit_window = Arc::new(winit_window);
                let winit_id = winit_window.id();

                let mut state = self.app_state.borrow_mut();
                match window::Window::new(&state.gpu, winit_window, transparent) {
                    Ok(handle) => {
                        let created = if let Some(window) = state.windows.get_mut(&id) {
                            options.apply_post_create_state(&handle.winit_window);
                            handle.winit_window.set_visible(is_visible);
                            window.handle = Some(handle);
                            true
                        } else {
                            eprintln!("Window entry '{id}' missing while creating handle");
                            false
                        };

                        if created {
                            state.winit_id_to_entry_id.insert(winit_id, id);
                        }
                    }
                    Err(e) => eprintln!("Error creating window: {:#?}", e),
                }
                state.paint_window(&id);
                drop(state);
                self.refresh_cursor_blink_timer(id);

                // Emit window load event after handle is ready
                self.dispatch_event_to_js(&event_dispatch::AppEvent::WindowLoad(
                    event_dispatch::WindowLoadEventData { window_id: id },
                ));
            }
            UserEvent::CloseWindow { id } => {
                self.close_window(event_loop, id);
            }
            UserEvent::RequestRedraw { id } => {
                let state = self.app_state.borrow();
                if let Some(entry) = state.windows.get(&id)
                    && let Some(ref handle) = entry.handle
                {
                    handle.winit_window.request_redraw();
                }
            }
            UserEvent::CursorBlink { id, generation } => {
                let should_redraw = {
                    let state = self.app_state.borrow();
                    state
                        .windows
                        .get(&id)
                        .filter(|entry| entry.cursor_blink_generation == generation)
                        .and_then(|entry| {
                            entry
                                .dom
                                .focused_node
                                .and_then(|focused_id| entry.dom.nodes.get(focused_id))
                                .and_then(|node| node.as_text_input())
                                .and_then(|input| {
                                    input.next_blink_toggle_in(true, entry.dom.window_focused)
                                })
                                .map(|_| ())
                        })
                        .is_some()
                };

                if should_redraw {
                    let state = self.app_state.borrow();
                    if let Some(entry) = state.windows.get(&id)
                        && let Some(ref handle) = entry.handle
                    {
                        handle.winit_window.request_redraw();
                    }
                    drop(state);
                    self.refresh_cursor_blink_timer(id);
                }
            }
            UserEvent::WakeJs => {
                self.js_wake_handle.clear();
                if let Err(err) = self.pump_js() {
                    print_runtime_error(&err);
                }
            }
            UserEvent::Quit => {
                let mut state = self.app_state.borrow_mut();
                state.windows.clear();
                state.winit_id_to_entry_id.clear();
                drop(state);
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(wid) = self
            .app_state
            .borrow()
            .winit_window_id_to_entry_id(&window_id)
        else {
            return;
        };

        let mut needs_redraw = false;
        let mut refresh_blink_timer = false;

        match event {
            WindowEvent::Resized(size) => {
                let needs_resize = {
                    let mut state = self.app_state.borrow_mut();
                    state.on_resize(&wid, size.width, size.height)
                };
                needs_resize.then(|| {
                    self.dispatch_event_to_js(&event_dispatch::AppEvent::Resize(
                        event_dispatch::ResizeEventData {
                            window_id: wid,
                            width: size.width,
                            height: size.height,
                        },
                    ));
                });
            }
            WindowEvent::RedrawRequested => {
                let mut state = self.app_state.borrow_mut();
                state.on_redraw_requested(&wid);
            }
            WindowEvent::CursorMoved { position, .. } => {
                let mut state = self.app_state.borrow_mut();
                let mouse_buttons = state.mouse_buttons;
                if let Some(entry) = state.windows.get_mut(&wid) {
                    let WindowEntry { handle, dom, .. } = entry;
                    if let Some(handle) = handle
                        && event_dispatch::handle_cursor_moved(dom, handle, position, mouse_buttons)
                    {
                        needs_redraw = true;
                    }
                }
            }
            WindowEvent::MouseInput {
                state: btn_state,
                button,
                ..
            } => {
                refresh_blink_timer = true;
                let events = {
                    let mut state = self.app_state.borrow_mut();
                    use winit::event::{ElementState, MouseButton};

                    // 1. Determine which bit to toggle
                    let button_bit: u8 = match button {
                        MouseButton::Left => 1,
                        MouseButton::Right => 2,
                        MouseButton::Middle => 4,
                        _ => 0,
                    };

                    // 2. Update bitmask state
                    match btn_state {
                        ElementState::Pressed => {
                            state.mouse_buttons |= button_bit;
                        }
                        ElementState::Released => {
                            state.mouse_buttons &= !button_bit;
                        }
                    }

                    let mouse_buttons = state.mouse_buttons;

                    // 3. Flattened dispatch logic using 'and_then' or guard patterns
                    state.windows.get_mut(&wid).and_then(|entry| {
                        let WindowEntry { handle, dom, .. } = entry;
                        let handle = handle.as_mut()?; // Returns None early if handle is None

                        let (redraw, mouse_events) = event_dispatch::handle_mouse_input(
                            dom,
                            handle,
                            wid,
                            btn_state,
                            button,
                            mouse_buttons,
                        );

                        if redraw {
                            needs_redraw = true;
                        }

                        Some(mouse_events)
                    })
                };

                if let Some(events) = events {
                    for event in events {
                        self.dispatch_event_to_js(&event);
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                let modifiers = self.app_state.borrow().modifiers;

                // 1. Build and dispatch the raw KeyDown/KeyUp event first
                let raw_event = {
                    let state = self.app_state.borrow();
                    state.windows.get(&wid).and_then(|entry| {
                        event_dispatch::build_key_event(&entry.dom, wid, &key_event, modifiers)
                    })
                };

                let prevented = if let Some(ref evt) = raw_event {
                    self.dispatch_event_to_js(evt)
                } else {
                    false
                };

                // 2. If not prevented, handle clipboard shortcuts, then input-level processing
                if !prevented {
                    if let Some(event_dispatch::AppEvent::HotReload) = raw_event {
                        // todo hotreload :3
                    } else {
                        // 2a. Tab: switch focus to next focusable element
                        let tab_outcome = {
                            let mut state = self.app_state.borrow_mut();
                            state.windows.get_mut(&wid).map(|entry| {
                                event_dispatch::handle_tab_focus(
                                    &mut entry.dom,
                                    wid,
                                    &key_event,
                                    modifiers,
                                )
                            })
                        };
                        let tab_consumed = if let Some(outcome) = tab_outcome {
                            if outcome.needs_redraw {
                                needs_redraw = true;
                            }
                            for event in &outcome.events {
                                self.dispatch_event_to_js(event);
                            }
                            outcome.consumed
                        } else {
                            false
                        };

                        // 2b. Check for clipboard shortcuts (Ctrl+C/X/V)
                        let clipboard_cmd = {
                            let state = self.app_state.borrow();

                            state.windows.get(&wid).and_then(|entry| {
                                let mut cb = state.clipboard.borrow_mut();
                                event_dispatch::build_clipboard_command(
                                    &entry.dom, &key_event, modifiers, &mut cb,
                                )
                            })
                        };

                        let clipboard_consumed = if tab_consumed {
                            true
                        } else if let Some(cmd) = clipboard_cmd {
                            // Dispatch clipboard event to JS
                            let clipboard_event =
                                event_dispatch::clipboard_command_to_event(&cmd, wid);
                            let clipboard_prevented = self.dispatch_event_to_js(&clipboard_event);

                            if !clipboard_prevented {
                                // Apply default clipboard action
                                let (redraw, follow_up_events) = {
                                    let mut state = self.app_state.borrow_mut();
                                    let AppState {
                                        ref mut windows,
                                        ref clipboard,
                                        ..
                                    } = *state;
                                    if let Some(entry) = windows.get_mut(&wid) {
                                        let mut cb = clipboard.borrow_mut();
                                        let tr =
                                            entry.handle.as_mut().map(|h| &mut h.text_renderer);
                                        if let Some(text_renderer) = tr {
                                            event_dispatch::apply_clipboard_command(
                                                cmd,
                                                &mut entry.dom,
                                                wid,
                                                &mut cb,
                                                text_renderer,
                                            )
                                        } else {
                                            (false, Vec::new())
                                        }
                                    } else {
                                        (false, Vec::new())
                                    }
                                };
                                if redraw {
                                    needs_redraw = true;
                                }
                                for event in follow_up_events {
                                    self.dispatch_event_to_js(&event);
                                }
                                // Scroll input to cursor after clipboard mutation
                                if needs_redraw {
                                    let mut state = self.app_state.borrow_mut();
                                    if let Some(entry) = state.windows.get_mut(&wid)
                                        && let Some(handle) = entry.handle.as_mut()
                                    {
                                        event_dispatch::scroll_input_to_cursor(
                                            &mut entry.dom,
                                            handle,
                                        );
                                    }
                                }
                            }
                            true // clipboard shortcut was consumed
                        } else {
                            false
                        };

                        // 2b. If no clipboard shortcut, handle normal input processing
                        if !clipboard_consumed {
                            let input_events = {
                                let mut state = self.app_state.borrow_mut();
                                state.windows.get_mut(&wid).map(|entry| {
                                    let handle = entry.handle.as_mut().unwrap();
                                    let (redraw, events) = event_dispatch::handle_key_for_input(
                                        &mut entry.dom,
                                        handle,
                                        wid,
                                        &key_event,
                                        modifiers,
                                    );
                                    let (checkbox_redraw, checkbox_events) =
                                        event_dispatch::handle_key_for_checkbox(
                                            &mut entry.dom,
                                            wid,
                                            &key_event,
                                        );
                                    let (button_redraw, button_events) =
                                        event_dispatch::handle_key_for_button(
                                            &mut entry.dom,
                                            wid,
                                            &key_event,
                                        );
                                    if redraw {
                                        needs_redraw = true;
                                    }
                                    if checkbox_redraw {
                                        needs_redraw = true;
                                    }
                                    if button_redraw {
                                        needs_redraw = true;
                                    }
                                    let mut all_events = events;
                                    all_events.extend(checkbox_events);
                                    all_events.extend(button_events);
                                    all_events
                                })
                            };

                            if let Some(events) = input_events {
                                for event in events {
                                    self.dispatch_event_to_js(&event);
                                }
                            }

                            // Handle view text selection shortcuts (only when no input is focused)
                            {
                                let mut state = self.app_state.borrow_mut();
                                if let Some(entry) = state.windows.get_mut(&wid)
                                    && entry.dom.focused_node.is_none()
                                    && event_dispatch::handle_key_for_view_selection(
                                        &mut entry.dom,
                                        &key_event,
                                        modifiers,
                                    )
                                {
                                    needs_redraw = true;
                                }
                            }
                        }
                    }
                }
                refresh_blink_timer = true;
            }
            WindowEvent::ModifiersChanged(mods) => {
                let mut state = self.app_state.borrow_mut();

                let m = mods.state();
                let mut bits: u32 = 0;
                if m.control_key() {
                    bits |= 1;
                }
                if m.alt_key() {
                    bits |= 2;
                }
                if m.shift_key() {
                    bits |= 4;
                }
                if m.super_key() {
                    bits |= 8;
                }
                state.modifiers = bits;
            }
            WindowEvent::Focused(focused) => {
                let mut state = self.app_state.borrow_mut();
                if let Some(entry) = state.windows.get_mut(&wid) {
                    entry.dom.window_focused = focused;
                    if focused
                        && let Some(nid) = entry.dom.focused_node
                        && let Some(node) = entry.dom.nodes.get_mut(nid)
                        && let Some(is) = node.data.as_text_input_mut()
                    {
                        is.reset_blink();
                    }
                    if focused && let Some(handle) = entry.handle.as_mut() {
                        event_dispatch::update_ime_cursor_area(&mut entry.dom, handle);
                    }
                    needs_redraw = true;
                }
                refresh_blink_timer = true;
            }
            WindowEvent::Ime(ime) => {
                use winit::event::Ime;
                match ime {
                    Ime::Commit(text) => {
                        let input_events = {
                            let mut state = self.app_state.borrow_mut();
                            state.windows.get_mut(&wid).and_then(|entry| {
                                let handle = entry.handle.as_mut()?;
                                let fid = entry.dom.focused_node?;

                                // Apply styles/width before IME commit
                                if let Some(meta) =
                                    event_dispatch::input_layout_meta(&entry.dom, fid)
                                    && let Some(node) = entry.dom.nodes.get_mut(fid)
                                    && let Some(is) = node.as_text_input_mut()
                                {
                                    crate::text::apply_text_style_to_editor(
                                        &mut is.editor,
                                        &meta.text_style,
                                    );
                                    is.editor.set_width(if meta.multiline {
                                        Some(meta.input_width)
                                    } else {
                                        None
                                    });
                                }

                                let node = entry.dom.nodes.get_mut(fid)?;
                                let is = node.as_text_input_mut()?;
                                let _edit = is.commit_ime_text(&text, &mut handle.text_renderer)?;
                                event_dispatch::update_ime_cursor_area(&mut entry.dom, handle);
                                needs_redraw = true;
                                Some(vec![event_dispatch::AppEvent::Input(
                                    event_dispatch::InputEventData {
                                        window_id: wid,
                                        node_id: fid,
                                        input_type: "insertCompositionText".to_string(),
                                        data: Some(text.clone()),
                                    },
                                )])
                            })
                        };
                        if let Some(events) = input_events {
                            for event in events {
                                self.dispatch_event_to_js(&event);
                            }
                        }
                        refresh_blink_timer = true;
                    }
                    Ime::Preedit(text, cursor) => {
                        let mut state = self.app_state.borrow_mut();
                        if let Some(entry) = state.windows.get_mut(&wid)
                            && let Some(fid) = entry.dom.focused_node
                            && let Some(node) = entry.dom.nodes.get_mut(fid)
                            && let Some(is) = node.as_text_input_mut()
                        {
                            is.set_preedit(text.clone(), cursor);
                            if let Some(handle) = entry.handle.as_mut() {
                                event_dispatch::update_ime_cursor_area(&mut entry.dom, handle);
                            }
                            needs_redraw = true;
                        }
                        refresh_blink_timer = true;
                    }
                    Ime::Enabled => {}
                    Ime::Disabled => {
                        let mut state = self.app_state.borrow_mut();
                        if let Some(entry) = state.windows.get_mut(&wid)
                            && let Some(fid) = entry.dom.focused_node
                            && let Some(node) = entry.dom.nodes.get_mut(fid)
                            && let Some(is) = node.as_text_input_mut()
                        {
                            is.clear_preedit();
                            if let Some(handle) = entry.handle.as_mut() {
                                event_dispatch::update_ime_cursor_area(&mut entry.dom, handle);
                            }
                            needs_redraw = true;
                        }
                        refresh_blink_timer = true;
                    }
                }
            }
            WindowEvent::CursorLeft { .. } => {
                let mut state = self.app_state.borrow_mut();
                if let Some(entry) = state.windows.get_mut(&wid) {
                    entry.dom.hit_state = Default::default();
                    if let Some(handle) = entry.handle.as_mut() {
                        handle.set_cursor(cursor::UzCursorIcon::Default);
                    }
                    needs_redraw = true;
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let mut state = self.app_state.borrow_mut();
                let (mut dx, mut dy) = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        ((x as f64) * 40.0, (y as f64) * 40.0)
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => (pos.x, pos.y),
                };
                // Shift+wheel translates a vertical scroll into a horizontal
                // one — the standard convention for mice without a tilt wheel.
                if state.modifiers & 4 != 0 && dx == 0.0 {
                    dx = dy;
                    dy = 0.0;
                }
                if let Some(entry) = state.windows.get_mut(&wid)
                    && let Some(handle) = entry.handle.as_mut()
                    && event_dispatch::handle_mouse_wheel(&mut entry.dom, handle, dx, dy)
                {
                    needs_redraw = true;
                }
            }
            WindowEvent::CloseRequested => {
                self.close_window(event_loop, wid);
                return;
            }
            _ => {}
        }

        if needs_redraw {
            let state = self.app_state.borrow();
            if let Some(entry) = state.windows.get(&wid)
                && let Some(ref handle) = entry.handle
            {
                handle.winit_window.request_redraw();
            }
        }

        if refresh_blink_timer {
            self.refresh_cursor_blink_timer(wid);
        }
    }
}

fn print_runtime_error(err: &anyhow::Error) {
    eprintln!("{} {:#}", terminal_colors::red_bold("Error"), err);
}
