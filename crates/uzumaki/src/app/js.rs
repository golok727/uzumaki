use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use anyhow::Result;
use deno_core::v8;
use deno_runtime::worker::MainWorker;
use winit::event_loop::EventLoopProxy;
use winit::window::{Theme, WindowButtons, WindowLevel};

use crate::app::handle::{MainToJs, PendingDestroy, UserEvent, WindowEntryId};
use crate::app::{AppConfig, print_runtime_error};
use crate::cursor::UzCursorIcon;
use crate::element::ImageData;
use crate::event_dispatch;
use crate::ops::window::WindowOptions;
use crate::runtime::worker::{WorkerBuildOptions, create_worker};
use crate::ui::UIState;
use crate::window;

/// Estimated bytes a single retained Rust DOM node holds. Reported to V8 via
/// `adjust_amount_of_external_allocated_memory` so cppgc schedules collections
/// based on the real memory footprint, not just the size of the JS wrapper.
pub const NODE_EXTERNAL_BYTES: i64 = 1024;

/// JS-thread-only state. Holds every per-window DOM, the image cache, and the
/// cppgc-deferred destroy queue. Never accessed from the main winit thread.
pub struct JsState {
    pub proxy: EventLoopProxy<UserEvent>,
    pub windows: HashMap<WindowEntryId, JsWindow>,
    pub mouse_buttons: u8,
    pub modifiers: u32,
    pub image_cache: HashMap<String, ImageData>,
    pub pending_destroy: VecDeque<PendingDestroy>,
    /// Net change in external bytes attributable to retained DOM nodes since
    /// the last flush. Pushed to V8 with
    /// `Isolate::adjust_amount_of_external_allocated_memory`.
    pub external_memory_delta: i64,
}

impl JsState {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        Self {
            proxy,
            windows: HashMap::new(),
            mouse_buttons: 0,
            modifiers: 0,
            image_cache: HashMap::new(),
            pending_destroy: VecDeque::new(),
            external_memory_delta: 0,
        }
    }

    /// Drain a bounded batch of deferred node destroys. Adaptive: large
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
}

pub type SharedJsState = Rc<RefCell<JsState>>;

pub fn with_state<R>(state: &SharedJsState, f: impl FnOnce(&mut JsState) -> R) -> R {
    f(&mut state.borrow_mut())
}

pub fn with_state_ref<R>(state: &SharedJsState, f: impl FnOnce(&JsState) -> R) -> R {
    f(&state.borrow())
}

/// JS-thread per-window state. Holds the DOM, the text renderer (font/layout
/// contexts), and a mirror of every window attribute exposed to JS so getters
/// stay synchronous without round-tripping to main.
///
/// `window` is `None` between `op_create_window` and the matching
/// `MainToJs::WindowCreated` reply — i.e. before the winit window exists.
pub struct JsWindow {
    pub window: Option<window::Window>,
    pub dom: UIState,
    pub rem_base: f32,
    pub cursor_blink_generation: u64,
    pub state: WindowMirror,
}

/// Mirror of every window attribute that JS may read synchronously. Updates
/// happen in two places: (1) setters apply the change locally and forward via
/// `UserEvent`; (2) main-thread events (resize/focus/move) update the mirror
/// when forwarded as `WindowEvent`s.
pub struct WindowMirror {
    pub title: String,
    pub visible: bool,
    pub resizable: bool,
    pub decorations: bool,
    pub transparent: bool,
    pub maximized: bool,
    pub minimized: bool,
    pub fullscreen: bool,
    pub window_level: WindowLevel,
    pub content_protected: bool,
    pub enabled_buttons: WindowButtons,
    pub theme: Option<Theme>,
    pub focused: bool,
    pub outer_position: Option<winit::dpi::PhysicalPosition<i32>>,
    pub outer_size: Option<winit::dpi::PhysicalSize<u32>>,
}

impl WindowMirror {
    pub fn from_options(options: &WindowOptions) -> Self {
        Self {
            title: options.title().to_string(),
            visible: options.visible(),
            resizable: options.resizable(),
            decorations: options.decorations(),
            transparent: options.transparent(),
            maximized: options.maximized(),
            minimized: options.minimized(),
            fullscreen: options.fullscreen(),
            window_level: options.window_level(),
            content_protected: options.content_protected(),
            enabled_buttons: options.enabled_buttons(),
            theme: options.theme_winit(),
            focused: false,
            outer_position: None,
            outer_size: None,
        }
    }
}

impl JsWindow {
    pub fn new(options: &WindowOptions) -> Self {
        let mut dom = UIState::new();
        let root = dom.create_view(crate::style::UzStyle::root());
        dom.set_root(root);
        Self {
            window: None,
            dom,
            rem_base: 16.0,
            cursor_blink_generation: 0,
            state: WindowMirror::from_options(options),
        }
    }

    /// Inner size in logical pixels.
    pub fn inner_size(&self) -> Option<(u32, u32)> {
        self.window.as_ref().map(|w| {
            let scale = w.scale_factor();
            let (pw, ph) = w.inner_size();
            (
                (pw as f64 / scale).round() as u32,
                (ph as f64 / scale).round() as u32,
            )
        })
    }

    pub fn scale_factor(&self) -> Option<f32> {
        self.window.as_ref().map(|w| w.scale_factor() as f32)
    }
}

/// Spawn the JS thread. Returns immediately; the thread owns the deno worker
/// and a current-thread tokio runtime for the lifetime of the application.
pub fn spawn_js_thread(
    snapshot: Option<&'static [u8]>,
    config: AppConfig,
    proxy: EventLoopProxy<UserEvent>,
    main_to_js: flume::Receiver<MainToJs>,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("uzumaki-js".into())
        .spawn(move || {
            if let Err(err) = run_js_thread(snapshot, config, proxy, main_to_js) {
                print_runtime_error(&err);
            }
        })
        .expect("failed to spawn uzumaki-js thread")
}

fn run_js_thread(
    snapshot: Option<&'static [u8]>,
    config: AppConfig,
    proxy: EventLoopProxy<UserEvent>,
    main_to_js: flume::Receiver<MainToJs>,
) -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("failed to create tokio runtime");

    let main_file = config.entry.clone();
    let app_root = config.app_root.clone();

    let mut worker = {
        let _guard = rt.enter();
        create_worker(WorkerBuildOptions {
            entry: &main_file,
            app_root: &app_root,
            args: config.args.clone(),
            headless: false,
            jsx_import_source: config.jsx_import_source.clone(),
            extensions: vec![crate::uzumaki::init()],
            startup_snapshot: snapshot,
        })?
    };

    let global_dispatch_fn = {
        let context = worker.js_runtime.main_context();
        deno_core::scope!(scope, &mut worker.js_runtime);
        let context_local = v8::Local::new(scope, context);
        let global_obj = context_local.global(scope);

        let key = v8::String::new_external_onebyte_static(scope, b"__uzumaki_on_app_event__")
            .ok_or_else(|| anyhow::anyhow!("failed to create v8 string"))?;
        let val = global_obj
            .get(scope, key.into())
            .ok_or_else(|| anyhow::anyhow!("__uzumaki_on_app_event__ not found on globalThis"))?;
        let func = v8::Local::<v8::Function>::try_from(val)
            .map_err(|_| anyhow::anyhow!("__uzumaki_on_app_event__ is not a function"))?;
        v8::Global::new(scope, func)
    };

    let state: SharedJsState = Rc::new(RefCell::new(JsState::new(proxy.clone())));
    {
        let op_state = worker.js_runtime.op_state();
        let mut borrow = op_state.borrow_mut();
        borrow.put(state.clone());
        borrow.put(proxy.clone());
        borrow.put(config);
    }

    rt.block_on(async move {
        // Execute the entry module once. After this returns, the deno event
        // loop will keep running on subsequent `run_event_loop` calls inside
        // the main select.
        let specifier = deno_core::resolve_path(
            main_file
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("entry path is not valid utf-8"))?,
            &app_root,
        )?;
        worker.execute_main_module(&specifier).await?;

        run_main_loop(&mut worker, &state, &global_dispatch_fn, &main_to_js).await
    })
}

async fn run_main_loop(
    worker: &mut MainWorker,
    state: &SharedJsState,
    dispatch_fn: &v8::Global<v8::Function>,
    main_to_js: &flume::Receiver<MainToJs>,
) -> Result<()> {
    loop {
        tokio::select! {
            biased;
            msg = main_to_js.recv_async() => {
                let Ok(msg) = msg else { break };
                if !handle_message(msg, worker, state, dispatch_fn) {
                    break;
                }
            }
            res = worker.run_event_loop(false) => {
                if let Err(e) = res {
                    print_runtime_error(&anyhow::Error::new(e));
                }
                // JS event loop is fully drained — park until the next message
                // arrives. We don't re-enter `run_event_loop` because it would
                // return immediately and busy-loop.
                let Ok(msg) = main_to_js.recv_async().await else { break };
                if !handle_message(msg, worker, state, dispatch_fn) {
                    break;
                }
            }
        }

        flush_external_memory(worker, state);
        with_state(state, |s| s.drain_pending_destroy());
    }
    Ok(())
}

/// Returns `false` to break the main loop.
fn handle_message(
    msg: MainToJs,
    worker: &mut MainWorker,
    state: &SharedJsState,
    dispatch_fn: &v8::Global<v8::Function>,
) -> bool {
    match msg {
        MainToJs::WindowCreated { id, shared } => {
            let proxy = with_state_ref(state, |s| s.proxy.clone());
            let window = window::Window::new(shared, proxy);
            with_state(state, |s| {
                if let Some(entry) = s.windows.get_mut(&id) {
                    entry.window = Some(window);
                }
            });
            refresh_cursor_blink_timer(state, id);
            dispatch_event_to_js(
                worker,
                dispatch_fn,
                &event_dispatch::AppEvent::WindowLoad(event_dispatch::WindowLoadEventData {
                    window_id: id,
                }),
            );
        }
        MainToJs::BuildFrame { id } => {
            build_frame(state, id);
            let proxy = with_state_ref(state, |s| s.proxy.clone());
            let _ = proxy.send_event(UserEvent::FrameReady { id });
        }
        MainToJs::WindowEvent { id, event } => {
            handle_window_event(worker, state, dispatch_fn, id, event);
        }
        MainToJs::CursorBlink { id, generation } => {
            handle_cursor_blink(state, id, generation);
        }
        MainToJs::Resumed => {}
        MainToJs::Shutdown => return false,
    }
    true
}

/// Run layout + paint on the JS thread and park the resulting `Scene` in
/// `WindowShared::pending_frame` for main to present.
fn build_frame(state: &SharedJsState, id: WindowEntryId) {
    use crate::paint::render::Painter;

    with_state(state, |s| {
        let Some(entry) = s.windows.get_mut(&id) else {
            return;
        };
        let JsWindow { window, dom, .. } = entry;
        let Some(window) = window.as_mut() else {
            return;
        };
        let (pw, ph) = window.shared.load_inner_size();
        let scale = window.shared.load_scale_factor();
        if pw == 0 || ph == 0 {
            return;
        }

        let mut scene = vello::Scene::new();
        dom.compute_layout(pw as f32, ph as f32, &mut window.text_renderer, scale);
        Painter::new(dom, &mut window.text_renderer, scale).paint(&mut scene);
        dom.refresh_hit_test();

        *window.shared.pending_frame.lock().unwrap() = Some(scene);
    });
}

fn flush_external_memory(worker: &mut MainWorker, state: &SharedJsState) {
    let delta = with_state(state, |s| std::mem::take(&mut s.external_memory_delta));
    if delta != 0 {
        worker
            .js_runtime
            .v8_isolate()
            .adjust_amount_of_external_allocated_memory(delta);
    }
}

/// Returns true if `preventDefault()` was called on the dispatched event.
pub fn dispatch_event_to_js(
    worker: &mut MainWorker,
    dispatch_fn: &v8::Global<v8::Function>,
    event: &event_dispatch::AppEvent,
) -> bool {
    let context = worker.js_runtime.main_context();
    deno_core::scope!(scope, &mut worker.js_runtime);
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
                crate::terminal_colors::red_bold("Error")
            );
            return false;
        }
    };

    let result = func.call(scope, undefined.into(), &[event_val]);

    if let Some(exception) = scope.exception() {
        let error = deno_core::error::JsError::from_v8_exception(scope, exception);
        eprintln!("{} {error}", crate::terminal_colors::red_bold("Error"));
        return false;
    }

    result.map(|v| v.is_true()).unwrap_or(false)
}

fn handle_window_event(
    worker: &mut MainWorker,
    state: &SharedJsState,
    dispatch_fn: &v8::Global<v8::Function>,
    wid: WindowEntryId,
    event: winit::event::WindowEvent,
) {
    use winit::event::{Ime, WindowEvent};

    let mut needs_redraw = false;
    let mut refresh_blink_timer = false;

    match event {
        WindowEvent::Resized(_) => {
            // Main has already updated `shared.inner_size` and reconfigured the
            // GPU surface; we just need to notify JS using the new logical size.
            let logical = with_state_ref(state, |s| {
                s.windows
                    .get(&wid)
                    .and_then(|e| e.window.as_ref().map(|w| w.inner_size()))
            });
            if let Some((w, h)) = logical
                && w > 0
                && h > 0
            {
                dispatch_event_to_js(
                    worker,
                    dispatch_fn,
                    &event_dispatch::AppEvent::Resize(event_dispatch::ResizeEventData {
                        window_id: wid,
                        width: w,
                        height: h,
                    }),
                );
            }
        }
        WindowEvent::ScaleFactorChanged { .. } => {
            // Main has already updated `shared.scale_factor`; no JS-side mirror
            // to keep in sync since `Window::scale_factor()` reads the atomic.
        }
        WindowEvent::CursorMoved { position, .. } => {
            with_state(state, |s| {
                let mouse_buttons = s.mouse_buttons;
                if let Some(entry) = s.windows.get_mut(&wid) {
                    let JsWindow { window, dom, .. } = entry;
                    if let Some(window) = window
                        && event_dispatch::handle_cursor_moved(dom, window, position, mouse_buttons)
                    {
                        needs_redraw = true;
                    }
                }
            });
        }
        WindowEvent::MouseInput {
            state: btn_state,
            button,
            ..
        } => {
            refresh_blink_timer = true;
            let events = with_state(state, |s| {
                use winit::event::{ElementState, MouseButton};

                let button_bit: u8 = match button {
                    MouseButton::Left => 1,
                    MouseButton::Right => 2,
                    MouseButton::Middle => 4,
                    _ => 0,
                };

                match btn_state {
                    ElementState::Pressed => s.mouse_buttons |= button_bit,
                    ElementState::Released => s.mouse_buttons &= !button_bit,
                }
                let mouse_buttons = s.mouse_buttons;

                s.windows.get_mut(&wid).and_then(|entry| {
                    let JsWindow { window, dom, .. } = entry;
                    let window = window.as_mut()?;
                    let (redraw, mouse_events) = event_dispatch::handle_mouse_input(
                        dom,
                        window,
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
            });

            if let Some(events) = events {
                for event in events {
                    dispatch_event_to_js(worker, dispatch_fn, &event);
                }
            }
        }
        WindowEvent::KeyboardInput {
            event: key_event, ..
        } => {
            let modifiers = with_state_ref(state, |s| s.modifiers);

            let raw_event = with_state_ref(state, |s| {
                s.windows.get(&wid).and_then(|entry| {
                    event_dispatch::build_key_event(&entry.dom, wid, &key_event, modifiers)
                })
            });

            let prevented = if let Some(ref evt) = raw_event {
                dispatch_event_to_js(worker, dispatch_fn, evt)
            } else {
                false
            };

            if !prevented {
                if let Some(event_dispatch::AppEvent::HotReload) = raw_event {
                    // todo hotreload
                } else {
                    let tab_outcome = with_state(state, |s| {
                        s.windows.get_mut(&wid).map(|entry| {
                            event_dispatch::handle_tab_focus(
                                &mut entry.dom,
                                wid,
                                &key_event,
                                modifiers,
                            )
                        })
                    });
                    let tab_consumed = if let Some(outcome) = tab_outcome {
                        if outcome.needs_redraw {
                            needs_redraw = true;
                        }
                        for event in &outcome.events {
                            dispatch_event_to_js(worker, dispatch_fn, event);
                        }
                        outcome.consumed
                    } else {
                        false
                    };

                    let clipboard_cmd = with_state(state, |s| {
                        let bridge = crate::clipboard::ClipboardBridge::new(&s.proxy);
                        s.windows.get(&wid).and_then(|entry| {
                            event_dispatch::build_clipboard_command(
                                &entry.dom, &key_event, modifiers, &bridge,
                            )
                        })
                    });

                    let clipboard_consumed = if tab_consumed {
                        true
                    } else if let Some(cmd) = clipboard_cmd {
                        let clipboard_event = event_dispatch::clipboard_command_to_event(&cmd, wid);
                        let clipboard_prevented =
                            dispatch_event_to_js(worker, dispatch_fn, &clipboard_event);

                        if !clipboard_prevented {
                            let (redraw, follow_up_events) = with_state(state, |s| {
                                let bridge = crate::clipboard::ClipboardBridge::new(&s.proxy);
                                if let Some(entry) = s.windows.get_mut(&wid) {
                                    let tr = entry.window.as_mut().map(|w| &mut w.text_renderer);
                                    if let Some(text_renderer) = tr {
                                        event_dispatch::apply_clipboard_command(
                                            cmd,
                                            &mut entry.dom,
                                            wid,
                                            &bridge,
                                            text_renderer,
                                        )
                                    } else {
                                        (false, Vec::new())
                                    }
                                } else {
                                    (false, Vec::new())
                                }
                            });
                            if redraw {
                                needs_redraw = true;
                            }
                            for event in follow_up_events {
                                dispatch_event_to_js(worker, dispatch_fn, &event);
                            }
                            if needs_redraw {
                                with_state(state, |s| {
                                    if let Some(entry) = s.windows.get_mut(&wid)
                                        && let Some(window) = entry.window.as_mut()
                                    {
                                        event_dispatch::scroll_input_to_cursor(
                                            &mut entry.dom,
                                            window,
                                        );
                                    }
                                });
                            }
                        }
                        true
                    } else {
                        false
                    };

                    if !clipboard_consumed {
                        let input_events = with_state(state, |s| {
                            s.windows.get_mut(&wid).map(|entry| {
                                let window = entry.window.as_mut().unwrap();
                                let (redraw, events) = event_dispatch::handle_key_for_input(
                                    &mut entry.dom,
                                    window,
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
                                if redraw || checkbox_redraw || button_redraw {
                                    needs_redraw = true;
                                }
                                let mut all = events;
                                all.extend(checkbox_events);
                                all.extend(button_events);
                                all
                            })
                        });

                        if let Some(events) = input_events {
                            for event in events {
                                dispatch_event_to_js(worker, dispatch_fn, &event);
                            }
                        }

                        with_state(state, |s| {
                            if let Some(entry) = s.windows.get_mut(&wid)
                                && entry.dom.focused_node.is_none()
                                && event_dispatch::handle_key_for_view_selection(
                                    &mut entry.dom,
                                    &key_event,
                                    modifiers,
                                )
                            {
                                needs_redraw = true;
                            }
                        });
                    }
                }
            }
            refresh_blink_timer = true;
        }
        WindowEvent::ModifiersChanged(mods) => {
            with_state(state, |s| {
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
                s.modifiers = bits;
            });
        }
        WindowEvent::Focused(focused) => {
            with_state(state, |s| {
                if let Some(entry) = s.windows.get_mut(&wid) {
                    entry.state.focused = focused;
                    entry.dom.window_focused = focused;
                    if focused
                        && let Some(nid) = entry.dom.focused_node
                        && let Some(node) = entry.dom.nodes.get_mut(nid)
                        && let Some(is) = node.data.as_text_input_mut()
                    {
                        is.reset_blink();
                    }
                    if focused && let Some(window) = entry.window.as_mut() {
                        event_dispatch::update_ime_cursor_area(&mut entry.dom, window);
                    }
                    needs_redraw = true;
                }
            });
            refresh_blink_timer = true;
        }
        WindowEvent::Moved(pos) => {
            with_state(state, |s| {
                if let Some(entry) = s.windows.get_mut(&wid) {
                    entry.state.outer_position = Some(pos);
                }
            });
        }
        WindowEvent::ThemeChanged(theme) => {
            with_state(state, |s| {
                if let Some(entry) = s.windows.get_mut(&wid) {
                    entry.state.theme = Some(theme);
                }
            });
        }
        WindowEvent::Ime(ime) => match ime {
            Ime::Commit(text) => {
                let input_events = with_state(state, |s| {
                    s.windows.get_mut(&wid).and_then(|entry| {
                        let window = entry.window.as_mut()?;
                        let fid = entry.dom.focused_node?;

                        if let Some(meta) = event_dispatch::input_layout_meta(&entry.dom, fid)
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
                        let _edit = is.commit_ime_text(&text, &mut window.text_renderer)?;
                        event_dispatch::update_ime_cursor_area(&mut entry.dom, window);
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
                });
                if let Some(events) = input_events {
                    for event in events {
                        dispatch_event_to_js(worker, dispatch_fn, &event);
                    }
                }
                refresh_blink_timer = true;
            }
            Ime::Preedit(text, cursor) => {
                with_state(state, |s| {
                    if let Some(entry) = s.windows.get_mut(&wid)
                        && let Some(fid) = entry.dom.focused_node
                        && let Some(node) = entry.dom.nodes.get_mut(fid)
                        && let Some(is) = node.as_text_input_mut()
                    {
                        is.set_preedit(text.clone(), cursor);
                        if let Some(window) = entry.window.as_mut() {
                            event_dispatch::update_ime_cursor_area(&mut entry.dom, window);
                        }
                        needs_redraw = true;
                    }
                });
                refresh_blink_timer = true;
            }
            Ime::Enabled => {}
            Ime::Disabled => {
                with_state(state, |s| {
                    if let Some(entry) = s.windows.get_mut(&wid)
                        && let Some(fid) = entry.dom.focused_node
                        && let Some(node) = entry.dom.nodes.get_mut(fid)
                        && let Some(is) = node.as_text_input_mut()
                    {
                        is.clear_preedit();
                        if let Some(window) = entry.window.as_mut() {
                            event_dispatch::update_ime_cursor_area(&mut entry.dom, window);
                        }
                        needs_redraw = true;
                    }
                });
                refresh_blink_timer = true;
            }
        },
        WindowEvent::CursorLeft { .. } => {
            with_state(state, |s| {
                if let Some(entry) = s.windows.get_mut(&wid) {
                    entry.dom.hit_state = Default::default();
                    if let Some(window) = entry.window.as_mut() {
                        window.set_cursor(UzCursorIcon::Default);
                    }
                    needs_redraw = true;
                }
            });
        }
        WindowEvent::MouseWheel { delta, .. } => {
            with_state(state, |s| {
                let (mut dx, mut dy) = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        ((x as f64) * 40.0, (y as f64) * 40.0)
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => (pos.x, pos.y),
                };
                if s.modifiers & 4 != 0 && dx == 0.0 {
                    dx = dy;
                    dy = 0.0;
                }
                if let Some(entry) = s.windows.get_mut(&wid)
                    && let Some(window) = entry.window.as_mut()
                    && event_dispatch::handle_mouse_wheel(&mut entry.dom, window, dx, dy)
                {
                    needs_redraw = true;
                }
            });
        }
        WindowEvent::CloseRequested => {
            dispatch_event_to_js(
                worker,
                dispatch_fn,
                &event_dispatch::AppEvent::WindowClose(event_dispatch::WindowLoadEventData {
                    window_id: wid,
                }),
            );
            let proxy = with_state(state, |s| {
                s.windows.remove(&wid);
                s.proxy.clone()
            });
            let _ = proxy.send_event(UserEvent::CloseWindow { id: wid });
            return;
        }
        _ => {}
    }

    if needs_redraw {
        with_state_ref(state, |s| {
            if let Some(entry) = s.windows.get(&wid)
                && let Some(window) = entry.window.as_ref()
            {
                window.request_redraw();
            }
        });
    }

    if refresh_blink_timer {
        refresh_cursor_blink_timer(state, wid);
    }
}

fn refresh_cursor_blink_timer(state: &SharedJsState, id: WindowEntryId) {
    let next_timer = with_state(state, |s| {
        let entry = s.windows.get_mut(&id)?;
        entry.cursor_blink_generation = entry.cursor_blink_generation.wrapping_add(1);
        let generation = entry.cursor_blink_generation;
        let focused = entry.dom.window_focused;
        let next_delay = entry
            .dom
            .focused_node
            .and_then(|focused_id| entry.dom.nodes.get(focused_id))
            .and_then(|node| node.as_text_input())
            .and_then(|input| input.next_blink_toggle_in(true, focused));
        next_delay.map(|delay| (generation, delay))
    });

    if let Some((generation, delay)) = next_timer {
        let proxy = with_state_ref(state, |s| s.proxy.clone());
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            let _ = proxy.send_event(UserEvent::CursorBlink { id, generation });
        });
    }
}

/// Handle a CursorBlink user-event delivered back into the JS thread (forwarded
/// by main via `MainToJs` is unnecessary — the timer fires here directly and
/// re-enters this function via the same MainToJs path? No: we handle it via
/// `UserEvent` on the main thread side, which then forwards a synthetic
/// message. See `app::Application::user_event` for the round-trip.)
pub fn handle_cursor_blink(state: &SharedJsState, id: WindowEntryId, generation: u64) {
    let should_redraw = with_state_ref(state, |s| {
        s.windows
            .get(&id)
            .filter(|entry| entry.cursor_blink_generation == generation)
            .and_then(|entry| {
                entry
                    .dom
                    .focused_node
                    .and_then(|focused_id| entry.dom.nodes.get(focused_id))
                    .and_then(|node| node.as_text_input())
                    .and_then(|input| input.next_blink_toggle_in(true, entry.dom.window_focused))
                    .map(|_| ())
            })
            .is_some()
    });

    if should_redraw {
        with_state_ref(state, |s| {
            if let Some(entry) = s.windows.get(&id)
                && let Some(window) = entry.window.as_ref()
            {
                window.request_redraw();
            }
        });
        refresh_cursor_blink_timer(state, id);
    }
}
