use napi::bindgen_prelude::*;
use std::{collections::HashMap, sync::Arc};

use napi_derive::napi;
use parking_lot::Mutex;
use winit::{
    application::ApplicationHandler,
    event_loop::{EventLoop, EventLoopProxy},
    platform::windows::WindowAttributesExtWindows,
    window::WindowId,
};

pub mod element;
pub mod geometry;
pub mod gpu;
pub mod window;
use window::Window;

use crate::element::{build_demo_tree, Dom};
use crate::gpu::GpuContext;

static LOOP_PROXY: Mutex<Option<EventLoopProxy<UserEvent>>> = Mutex::new(None);

enum UserEvent {
    CreateWindow {
        label: String, // should be unique
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
    dom: Dom,
    windows: HashMap<WindowId, Window>,
    window_label_to_id: HashMap<String, WindowId>, // for js lookup
}

#[napi]
impl Application {
    #[napi(constructor)]
    pub fn new() -> Self {
        let gpu = pollster::block_on(GpuContext::new()).expect("Failed to create GPU context");

        Self {
            gpu,
            dom: build_demo_tree(),
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

        match Window::new(&self.gpu, winit_window) {
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
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
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
                    window.paint_and_present(&self.gpu.device, &self.gpu.queue, &mut self.dom);
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
        if let Some(f) = &mut self.on_window_event {
            let _ = f.call(());
        }
    }
}
