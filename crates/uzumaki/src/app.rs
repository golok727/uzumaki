use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use anyhow::Result;
use deno_core::futures::task::{ArcWake, waker};
use deno_core::{PollEventLoopOptions, v8};
use deno_resolver::npm::{
    ByonmNpmResolverCreateOptions, CreateInNpmPkgCheckerOptions, DenoInNpmPackageChecker,
    NpmResolver, NpmResolverCreateOptions,
};
use deno_runtime::BootstrapOptions;
use deno_runtime::deno_fs::FileSystem;
use deno_runtime::deno_node::NodeExtInitServices;
use deno_runtime::deno_permissions::PermissionsContainer;
use deno_runtime::deno_web::{BlobStore, InMemoryBroadcastChannel};
use deno_runtime::worker::{MainWorker, WorkerOptions, WorkerServiceOptions};
use node_resolver::analyze::{CjsModuleExportAnalyzer, NodeCodeTranslator, NodeCodeTranslatorMode};
use node_resolver::cache::NodeResolutionSys;
use winit::window::WindowId;
use winit::{application::ApplicationHandler, event::WindowEvent};

use crate::clipboard;
use crate::cursor;
use crate::event_dispatch;
use crate::gpu::GpuContext;
use crate::runtime::module_loader::{UzCjsCodeAnalyzer, UzRequireLoader};
use crate::runtime::resolver::UzCjsTracker;
use crate::runtime::sys::UzSys;
use crate::ui::UIState;
use crate::{runtime, window};

pub struct WindowEntry {
    pub dom: UIState,
    pub handle: Option<window::Window>,
    pub rem_base: f32,
    pub cursor_blink_generation: u64,
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
}

impl AppState {
    pub fn winit_window_id_to_entry_id(&self, window_id: &WindowId) -> Option<WindowEntryId> {
        self.winit_id_to_entry_id.get(window_id).cloned()
    }

    pub fn paint_window(&mut self, id: &WindowEntryId) {
        if let Some(window) = self.windows.get_mut(id)
            && let Some(handle) = &mut window.handle
        {
            handle.paint_and_present(&self.gpu.device, &self.gpu.queue, &mut window.dom);
        }
    }

    pub fn on_redraw_requested(&mut self, wid: &WindowEntryId) {
        if let Some(entry) = self.windows.get_mut(wid) {
            let WindowEntry { handle, dom, .. } = entry;
            if let Some(handle) = handle {
                event_dispatch::handle_redraw(dom, handle, &self.gpu.device, &self.gpu.queue);
                // handle.winit_window.request_redraw();
            }
        }
    }
    pub fn on_resize(&mut self, id: &WindowEntryId, width: u32, height: u32) -> bool {
        if let Some(window) = self.windows.get_mut(id)
            && let Some(handle) = &mut window.handle
            && handle.on_resize(&self.gpu.device, width, height)
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
        width: u32,
        height: u32,
        title: String,
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
    pub tokio_runtime: Option<tokio::runtime::Runtime>,
    global_app_event_dispatch_fn: Option<v8::Global<v8::Function>>,
    js_wake_handle: Arc<JsWakeHandle>,
}

impl Application {
    pub fn new_with_root(
        main_file: impl Into<PathBuf>,
        app_root: impl Into<PathBuf>,
        args: Vec<String>,
        startup_snapshot: Option<&'static [u8]>,
    ) -> Result<Self> {
        let main_file: PathBuf = main_file.into();
        let app_root: PathBuf = app_root.into();
        let sys = sys_traits::impls::RealSys;

        // --- BYONM node resolution ---
        let root_node_modules = app_root.join("node_modules");
        let pkg_json_resolver: node_resolver::PackageJsonResolverRc<UzSys> =
            Arc::new(node_resolver::PackageJsonResolver::new(sys.clone(), None));

        let in_npm_pkg_checker = DenoInNpmPackageChecker::new(CreateInNpmPkgCheckerOptions::Byonm);

        let npm_resolver = NpmResolver::<UzSys>::new(NpmResolverCreateOptions::Byonm(
            ByonmNpmResolverCreateOptions {
                root_node_modules_dir: Some(root_node_modules),
                search_stop_dir: None,
                sys: NodeResolutionSys::new(sys.clone(), None),
                pkg_json_resolver: pkg_json_resolver.clone(),
            },
        ));

        let cjs_tracker = Arc::new(UzCjsTracker::new(
            in_npm_pkg_checker.clone(),
            pkg_json_resolver.clone(),
            deno_resolver::cjs::IsCjsResolutionMode::ImplicitTypeCommonJs,
            vec![],
        ));

        let node_resolver = Arc::new(node_resolver::NodeResolver::new(
            in_npm_pkg_checker.clone(),
            node_resolver::DenoIsBuiltInNodeModuleChecker,
            npm_resolver.clone(),
            pkg_json_resolver.clone(),
            NodeResolutionSys::new(sys.clone(), None),
            node_resolver::NodeResolverOptions::default(),
        ));

        let cjs_code_analyzer = UzCjsCodeAnalyzer {
            cjs_tracker: cjs_tracker.clone(),
        };
        let cjs_module_export_analyzer = Arc::new(CjsModuleExportAnalyzer::new(
            cjs_code_analyzer,
            in_npm_pkg_checker.clone(),
            node_resolver.clone(),
            npm_resolver.clone(),
            pkg_json_resolver.clone(),
            sys.clone(),
        ));
        let node_code_translator = Arc::new(NodeCodeTranslator::new(
            cjs_module_export_analyzer,
            NodeCodeTranslatorMode::ModuleLoader,
        ));

        let fs: Arc<dyn FileSystem> = Arc::new(deno_runtime::deno_fs::RealFs);

        let descriptor_parser = Arc::new(
            deno_runtime::permissions::RuntimePermissionDescriptorParser::new(sys.clone()),
        );

        let main_module = deno_core::resolve_path(main_file.to_str().unwrap(), &app_root)?;

        let services = WorkerServiceOptions {
            blob_store: Arc::new(BlobStore::default()),
            broadcast_channel: InMemoryBroadcastChannel::default(),
            deno_rt_native_addon_loader: None,
            feature_checker: Arc::new(deno_runtime::FeatureChecker::default()),
            fs: fs.clone(),
            module_loader: Rc::new(runtime::ts::TypescriptModuleLoader {
                source_maps: runtime::ts::SourceMapStore::default(),
                node_resolver: node_resolver.clone(),
                cjs_tracker: cjs_tracker.clone(),
                node_code_translator,
            }),
            node_services: Some(NodeExtInitServices {
                node_require_loader: Rc::new(UzRequireLoader {
                    cjs_tracker: cjs_tracker.clone(),
                }),
                node_resolver,
                pkg_json_resolver,
                sys: sys.clone(),
            }),
            npm_process_state_provider: None,
            permissions: PermissionsContainer::allow_all(descriptor_parser),
            root_cert_store_provider: None,
            fetch_dns_resolver: Default::default(),
            shared_array_buffer_store: None,
            compiled_wasm_module_store: None,
            v8_code_cache: None,
            bundle_provider: None,
        };

        let options = WorkerOptions {
            extensions: vec![crate::uzumaki::init()],
            startup_snapshot,
            skip_op_registration: false,
            bootstrap: BootstrapOptions {
                args: args.clone(),
                mode: deno_runtime::WorkerExecutionMode::None,
                ..Default::default()
            },
            ..Default::default()
        };

        let worker = MainWorker::bootstrap_from_options(&main_module, services, options);

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
        }));

        // Put shared state and event loop proxy into OpState
        {
            let op_state = worker.js_runtime.op_state();
            let mut borrow = op_state.borrow_mut();
            borrow.put(app_state.clone());
            borrow.put(event_loop_proxy);
        }

        Ok(Self {
            worker,
            app_state,
            main_file,
            app_root,
            event_loop: Some(event_loop),
            module_loaded: false,
            tokio_runtime: None,
            global_app_event_dispatch_fn: None,
            js_wake_handle,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let Some(event_loop) = self.event_loop.take() else {
            return Ok(());
        };
        // event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(self)?;
        Ok(())
    }

    fn pump_js(&mut self) {
        let wake_handle = self.js_wake_handle.clone();
        wake_handle.clear();

        let rt = self.tokio_runtime.as_ref().unwrap();
        let _guard = rt.enter();
        let waker = waker(wake_handle);
        let mut cx = Context::from_waker(&waker);

        match self
            .worker
            .js_runtime
            .poll_event_loop(&mut cx, PollEventLoopOptions::default())
        {
            Poll::Ready(Ok(())) | Poll::Pending => {}
            Poll::Ready(Err(e)) => eprintln!("JS error: {e}"),
        }
    }

    fn load_main_module(&mut self) {
        let specifier =
            deno_core::resolve_path(self.main_file.to_str().unwrap(), &self.app_root).unwrap();

        let rt = self.tokio_runtime.as_ref().unwrap();
        rt.block_on(async {
            self.worker.execute_main_module(&specifier).await.unwrap();
        });
        self.pump_js();
    }

    fn ensure_dispatch_fn(&mut self) -> Result<()> {
        if self.global_app_event_dispatch_fn.is_some() {
            return Ok(());
        }

        let resolved = {
            let context = self.worker.js_runtime.main_context();
            deno_core::scope!(scope, &mut self.worker.js_runtime);
            let context_local = v8::Local::new(scope, context);
            let global_obj = context_local.global(scope);

            let key = v8::String::new_external_onebyte_static(scope, b"__uzumaki_on_app_event__")
                .ok_or_else(|| anyhow::anyhow!("failed to create v8 string"))?;

            let val = global_obj
                .get(scope, key.into())
                .ok_or_else(|| anyhow::anyhow!("__uzumaki_dispatch__ not found on globalThis"))?;

            let func = v8::Local::<v8::Function>::try_from(val)
                .map_err(|_| anyhow::anyhow!("__uzumaki_dispatch__ is not a function"))?;

            v8::Global::new(scope, func)
        };
        // scope dropped, safe to write to self
        self.global_app_event_dispatch_fn = Some(resolved);
        Ok(())
    }

    /// Dispatch an event to JS. Returns true if `preventDefault()` was called.
    fn dispatch_event_to_js(&mut self, event: &event_dispatch::AppEvent) -> bool {
        if let Err(e) = self.ensure_dispatch_fn() {
            eprintln!("[uzumaki] dispatch fn not available: {e}");
            return false;
        }

        let rt = self.tokio_runtime.as_ref().unwrap();
        // Deno's timer ops require an active Tokio runtime. App events are invoked
        // directly from winit callbacks, so we need to re-enter the runtime before
        // calling into JS event handlers.
        let _guard = rt.enter();

        // Clone the Global handle so we don't hold a borrow on self
        // while the scope borrows self.worker.js_runtime
        let dispatch_fn = self.global_app_event_dispatch_fn.clone().unwrap();

        let context = self.worker.js_runtime.main_context();
        deno_core::scope!(scope, &mut self.worker.js_runtime);
        v8::tc_scope!(scope, scope);

        let context_local = v8::Local::new(scope, context);
        let _global_obj = context_local.global(scope);

        let func = v8::Local::new(scope, &dispatch_fn);
        let undefined = v8::undefined(scope);

        let event_val = match deno_core::serde_v8::to_v8(scope, event) {
            Ok(val) => val,
            Err(e) => {
                eprintln!("[uzumaki] failed to serialize event: {e}");
                return false;
            }
        };

        let result = func.call(scope, undefined.into(), &[event_val]);

        if let Some(exception) = scope.exception() {
            let error = deno_core::error::JsError::from_v8_exception(scope, exception);
            eprintln!("[uzumaki] event handler error: {error}");
            return false;
        }

        // JS returns true if defaultPrevented
        result.map(|v| v.is_true()).unwrap_or(false)
    }

    fn spawn_cursor_blink_timer(&self, id: WindowEntryId, generation: u64, delay: Duration) {
        let proxy = self.js_wake_handle.proxy.clone();
        let handle = self.tokio_runtime.as_ref().unwrap().handle().clone();
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
                .and_then(|input| input.next_blink_toggle_in(entry.dom.window_focused));

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
            self.load_main_module();
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.pump_js();
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::CreateWindow {
                id,
                width,
                height,
                title,
            } => {
                let attributes = winit::window::WindowAttributes::default()
                    .with_title(title)
                    .with_inner_size(winit::dpi::Size::new(winit::dpi::LogicalSize::new(
                        width, height,
                    )))
                    .with_min_inner_size(winit::dpi::Size::new(winit::dpi::LogicalSize::new(
                        400, 300,
                    )));

                let is_visible = attributes.visible;

                let Ok(winit_window) = event_loop.create_window(attributes.with_visible(false))
                else {
                    eprintln!("Failed to create window");
                    return;
                };
                winit_window.set_ime_allowed(true);

                let winit_window = Arc::new(winit_window);
                let winit_id = winit_window.id();

                let mut state = self.app_state.borrow_mut();
                assert!(
                    state.windows.contains_key(&id),
                    "Window entry '{}' must exist before creating handle",
                    id
                );
                match window::Window::new(&state.gpu, winit_window) {
                    Ok(handle) => {
                        state.winit_id_to_entry_id.insert(winit_id, id);

                        let window = state.windows.get_mut(&id).unwrap();
                        handle.winit_window.set_visible(is_visible);
                        window.handle = Some(handle);
                        // handle.paint_and_present(
                        //     &state.gpu.device,
                        //     &state.gpu.queue,
                        //     &mut window.dom,
                        // );
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
                                    input.next_blink_toggle_in(entry.dom.window_focused)
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
                self.pump_js();
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
                        ElementState::Pressed => state.mouse_buttons |= button_bit,
                        ElementState::Released => state.mouse_buttons &= !button_bit,
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
                        // 2a. Check for clipboard shortcuts (Ctrl+C/X/V)
                        let clipboard_cmd = {
                            let state = self.app_state.borrow();

                            state.windows.get(&wid).and_then(|entry| {
                                let mut cb = state.clipboard.borrow_mut();
                                event_dispatch::build_clipboard_command(
                                    &entry.dom, &key_event, modifiers, &mut cb,
                                )
                            })
                        };

                        let clipboard_consumed = if let Some(cmd) = clipboard_cmd {
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
                                    if redraw {
                                        needs_redraw = true;
                                    }
                                    if checkbox_redraw {
                                        needs_redraw = true;
                                    }
                                    let mut all_events = events;
                                    all_events.extend(checkbox_events);
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
                                let value = is.text();
                                event_dispatch::update_ime_cursor_area(&mut entry.dom, handle);
                                needs_redraw = true;
                                Some(vec![event_dispatch::AppEvent::Input(
                                    event_dispatch::InputEventData {
                                        window_id: wid,
                                        node_id: fid,
                                        value,
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
                let scroll_delta_y = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y as f64 * 40.0,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y,
                };
                if let Some(entry) = state.windows.get_mut(&wid)
                    && let Some(handle) = entry.handle.as_mut()
                    && event_dispatch::handle_mouse_wheel(&mut entry.dom, handle, scroll_delta_y)
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
