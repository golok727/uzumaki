use napi::bindgen_prelude::*;
use std::{collections::HashMap, sync::Arc};

use napi_derive::napi;
use parking_lot::Mutex;
use winit::{
    application::ApplicationHandler,
    event_loop::{EventLoop, EventLoopProxy},
    window::WindowId,
};

pub mod element;
pub mod geometry;
pub mod gpu;
pub mod interactivity;
pub mod style;
pub mod text;
pub mod window;
use window::Window;

use crate::element::build_demo_tree;
use crate::gpu::GpuContext;

static LOOP_PROXY: Mutex<Option<EventLoopProxy<UserEvent>>> = Mutex::new(None);

enum UserEvent {
    CreateWindow {
        label: String,
        width: u32,
        height: u32,
        title: String,
    },
    Quit,
}

#[napi(object)]
pub struct WindowOptions {
    pub label: String,
    pub width: u32,
    pub height: u32,
    pub title: String,
}

#[napi]
pub fn create_window(options: WindowOptions) {
    let proxy = LOOP_PROXY.lock();
    if let Some(proxy) = &*proxy {
        let _ = proxy.send_event(UserEvent::CreateWindow {
            label: options.label,
            width: options.width,
            height: options.height,
            title: options.title,
        });
    }
}

#[napi]
pub fn request_quit() {
    let proxy = LOOP_PROXY.lock();
    if let Some(proxy) = &*proxy {
        let _ = proxy.send_event(UserEvent::Quit);
    }
}

#[napi]
pub struct Application {
    on_init: Option<Function<'static, ()>>,
    on_window_event: Option<Function<'static, ()>>,
    gpu: GpuContext,
    windows: HashMap<WindowId, Window>,
    window_label_to_id: HashMap<String, WindowId>,
}

#[napi]
impl Application {
    #[napi(constructor)]
    pub fn new() -> Self {
        let gpu = pollster::block_on(GpuContext::new()).expect("Failed to create GPU context");

        Self {
            gpu,
            on_init: None,
            on_window_event: None,
            windows: Default::default(),
            window_label_to_id: Default::default(),
        }
    }

    fn insert_window(&mut self, winit_window: Arc<winit::window::Window>, label: String) {
        assert!(
            !self.window_label_to_id.contains_key(&label),
            "Window with label '{}' already exists",
            label
        );

        // Each window gets its own DOM
        let dom = build_demo_tree();

        match Window::new(&self.gpu, winit_window, dom) {
            Ok(window) => {
                self.window_label_to_id.insert(label, window.id());
                self.windows.insert(window.id(), window);
            }
            Err(e) => {
                println!("Error creating window : {:#?}", e)
            }
        }
    }

    #[napi]
    pub fn on_init(&mut self, f: Function<'static, ()>) {
        self.on_init = Some(f);
    }

    #[napi]
    pub fn on_window_event(&mut self, f: Function<'static, ()>) {
        self.on_window_event = Some(f);
    }

    #[napi]
    pub fn run(&mut self) {
        let event_loop = EventLoop::<UserEvent>::with_user_event()
            .build()
            .expect("Error creating event loop");

        {
            let mut lock = LOOP_PROXY.lock();
            *lock = Some(event_loop.create_proxy());
        }

        ctrlc::set_handler(|| {
            println!("SIGINT received, exiting...");
            request_quit();
        })
        .expect("error setting quit handler");

        println!("Starting event loop ");
        event_loop.run_app(self).expect("Error running event loop ");

        {
            let mut lock = LOOP_PROXY.lock();
            lock.take();
        }
    }
}

impl ApplicationHandler<UserEvent> for Application {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        println!("Application init");
        if let Some(cb) = self.on_init.take() {
            let _ = cb.call(());
        }
        println!("Application initialized");
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::CreateWindow {
                label,
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

                println!("Creating window");
                let Ok(winit_window) = event_loop.create_window(attributes) else {
                    println!("Failed to create window");
                    return;
                };

                let window = Arc::new(winit_window);
                self.insert_window(window, label);
            }
            UserEvent::Quit => {
                self.windows.clear();
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        use winit::event::WindowEvent;

        let mut needs_redraw = false;

        match event {
            WindowEvent::Resized(size) => {
                if let Some(window) = self.windows.get_mut(&window_id) {
                    if window.on_resize(&self.gpu.device, size.width, size.height) {
                        window.winit_window.request_redraw();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(window) = self.windows.get_mut(&window_id) {
                    window.paint_and_present(&self.gpu.device, &self.gpu.queue);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(window) = self.windows.get_mut(&window_id) {
                    let old_top = window.dom.hit_state.top_hit;
                    window.dom.update_hit_test(position.x, position.y);
                    let new_top = window.dom.hit_state.top_hit;
                    if old_top != new_top {
                        needs_redraw = true;
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                use winit::event::ElementState;

                let mouse_button = match button {
                    winit::event::MouseButton::Left => crate::interactivity::MouseButton::Left,
                    winit::event::MouseButton::Right => crate::interactivity::MouseButton::Right,
                    winit::event::MouseButton::Middle => crate::interactivity::MouseButton::Middle,
                    _ => crate::interactivity::MouseButton::Left,
                };

                if let Some(window) = self.windows.get_mut(&window_id) {
                    if let Some((mx, my)) = window.dom.hit_state.mouse_position {
                        match state {
                            ElementState::Pressed => {
                                let top = window.dom.hit_state.top_hit;
                                window.dom.set_active(top);
                                window.dom.dispatch_mouse_down(mx, my, mouse_button);
                                needs_redraw = true;
                            }
                            ElementState::Released => {
                                window.dom.dispatch_mouse_up(mx, my, mouse_button);
                                if let Some(active) = window.dom.hit_state.active_hitbox {
                                    if window.dom.hit_state.is_hovered(active) {
                                        window.dom.dispatch_click(mx, my, mouse_button);
                                    }
                                }
                                window.dom.set_active(None);
                                needs_redraw = true;
                            }
                        }
                    }
                }
            }
            WindowEvent::CursorLeft { .. } => {
                if let Some(window) = self.windows.get_mut(&window_id) {
                    window.dom.hit_state = Default::default();
                    needs_redraw = true;
                }
            }
            WindowEvent::CloseRequested => {
                println!("Close this stupid app ");
                self.windows.remove(&window_id);
                if self.windows.is_empty() {
                    event_loop.exit();
                }
            }
            _ => {}
        }

        if needs_redraw {
            if let Some(window) = self.windows.get(&window_id) {
                window.winit_window.request_redraw();
            }
        }

        if let Some(f) = &mut self.on_window_event {
            let _ = f.call(());
        }
    }
}
