use napi::bindgen_prelude::*;
use std::{collections::HashMap, sync::Arc, sync::LazyLock};

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

use crate::element::{Dom, NodeId};
use crate::gpu::GpuContext;
use crate::style::*;

static LOOP_PROXY: Mutex<Option<EventLoopProxy<UserEvent>>> = Mutex::new(None);
static DOM_REGISTRY: LazyLock<Mutex<HashMap<String, Arc<Mutex<Dom>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

enum UserEvent {
    CreateWindow {
        label: String,
        width: u32,
        height: u32,
        title: String,
    },
    RequestRedraw {
        label: String,
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
    // Create DOM immediately so JS can call getRootNodeId right after
    let mut dom = Dom::new();
    let root = dom.create_view(Style {
        display: Display::Flex,
        size: Size {
            width: Length::Percent(1.0),
            height: Length::Percent(1.0),
        },
        ..Default::default()
    });
    dom.set_root(root);

    let dom_arc = Arc::new(Mutex::new(dom));
    DOM_REGISTRY.lock().insert(options.label.clone(), dom_arc);

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
pub fn request_redraw(label: String) {
    let proxy = LOOP_PROXY.lock();
    if let Some(proxy) = &*proxy {
        let _ = proxy.send_event(UserEvent::RequestRedraw { label });
    }
}

// ── DOM manipulation napi functions ──────────────────────────────────

fn with_dom<R>(label: &str, f: impl FnOnce(&mut Dom) -> R) -> R {
    let registry = DOM_REGISTRY.lock();
    let dom_arc = registry.get(label).expect("window not found");
    let mut dom = dom_arc.lock();
    f(&mut dom)
}

#[napi]
pub fn get_root_node_id(label: String) -> String {
    with_dom(&label, |dom| {
        dom.root.expect("no root node").to_string_id()
    })
}

#[napi]
pub fn create_element(label: String, element_type: String) -> String {
    let _ = element_type;
    with_dom(&label, |dom| {
        let node_id = dom.create_view(Style::default());
        node_id.to_string_id()
    })
}

#[napi]
pub fn create_text_node(label: String, text: String) -> String {
    with_dom(&label, |dom| {
        let node_id = dom.create_text(text, Style::default());
        node_id.to_string_id()
    })
}

#[napi]
pub fn append_child(label: String, parent_id: String, child_id: String) {
    with_dom(&label, |dom| {
        dom.append_child(
            NodeId::from_string_id(&parent_id),
            NodeId::from_string_id(&child_id),
        );
    })
}

#[napi]
pub fn insert_before(label: String, parent_id: String, child_id: String, before_id: String) {
    with_dom(&label, |dom| {
        dom.insert_before(
            NodeId::from_string_id(&parent_id),
            NodeId::from_string_id(&child_id),
            NodeId::from_string_id(&before_id),
        );
    })
}

#[napi]
pub fn remove_child(label: String, parent_id: String, child_id: String) {
    with_dom(&label, |dom| {
        dom.remove_child(
            NodeId::from_string_id(&parent_id),
            NodeId::from_string_id(&child_id),
        );
    })
}

#[napi]
pub fn set_text(label: String, node_id: String, text: String) {
    with_dom(&label, |dom| {
        dom.set_text_content(NodeId::from_string_id(&node_id), text);
    })
}

#[napi]
pub fn set_property(label: String, node_id: String, prop: String, value: String) {
    with_dom(&label, |dom| {
        let nid = NodeId::from_string_id(&node_id);
        apply_property(dom, nid, &prop, &value);
    })
}

// ── Property application ─────────────────────────────────────────────

fn apply_property(dom: &mut Dom, node_id: NodeId, prop: &str, value: &str) {
    if let Some(hover_prop) = prop.strip_prefix("hover:") {
        let node = &mut dom.nodes[node_id];
        let refinement = node
            .interactivity
            .hover_style
            .get_or_insert_with(|| Box::new(StyleRefinement::default()));
        apply_style_refinement(refinement, hover_prop, value);
        return;
    }

    if let Some(active_prop) = prop.strip_prefix("active:") {
        let node = &mut dom.nodes[node_id];
        let refinement = node
            .interactivity
            .active_style
            .get_or_insert_with(|| Box::new(StyleRefinement::default()));
        apply_style_refinement(refinement, active_prop, value);
        return;
    }

    match prop {
        "interactive" => {
            dom.nodes[node_id].interactivity.js_interactive = value == "true";
            return;
        }
        "visible" => {
            dom.nodes[node_id].style.visibility = if value == "true" {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            sync_taffy(dom, node_id);
            return;
        }
        _ => {}
    }

    apply_base_style(dom, node_id, prop, value);
}

fn apply_base_style(dom: &mut Dom, node_id: NodeId, prop: &str, value: &str) {
    let node = &mut dom.nodes[node_id];
    let s = &mut node.style;

    match prop {
        "h" => s.size.height = parse_length(value),
        "w" => s.size.width = parse_length(value),
        "p" => s.padding = Edges::all(parse_f32(value)),
        "px" => {
            let v = parse_f32(value);
            s.padding.left = v;
            s.padding.right = v;
        }
        "py" => {
            let v = parse_f32(value);
            s.padding.top = v;
            s.padding.bottom = v;
        }
        "pt" => s.padding.top = parse_f32(value),
        "pb" => s.padding.bottom = parse_f32(value),
        "pl" => s.padding.left = parse_f32(value),
        "pr" => s.padding.right = parse_f32(value),
        "m" => s.margin = Edges::all(parse_f32(value)),
        "mx" => {
            let v = parse_f32(value);
            s.margin.left = v;
            s.margin.right = v;
        }
        "my" => {
            let v = parse_f32(value);
            s.margin.top = v;
            s.margin.bottom = v;
        }
        "mt" => s.margin.top = parse_f32(value),
        "mb" => s.margin.bottom = parse_f32(value),
        "ml" => s.margin.left = parse_f32(value),
        "mr" => s.margin.right = parse_f32(value),
        "flex" => {
            s.display = Display::Flex;
            match value {
                "col" | "column" => s.flex_direction = FlexDirection::Column,
                "row" => s.flex_direction = FlexDirection::Row,
                _ => {
                    if let Ok(v) = value.parse::<f32>() {
                        s.flex_grow = v;
                    }
                }
            }
        }
        "flexDir" => s.flex_direction = parse_flex_direction(value),
        "flexGrow" => s.flex_grow = parse_f32(value),
        "flexShrink" => s.flex_shrink = parse_f32(value),
        "items" => s.align_items = Some(parse_align_items(value)),
        "justify" => s.justify_content = Some(parse_justify_content(value)),
        "gap" => {
            let v = parse_f32(value);
            s.gap = GapSize {
                width: DefiniteLength::Px(v),
                height: DefiniteLength::Px(v),
            };
        }
        "bg" => s.background = Some(parse_color(value)),
        "color" => s.text.color = parse_color(value),
        "fontSize" => {
            let fs = parse_f32(value);
            s.text.font_size = fs;
        }
        "fontWeight" => {}
        "rounded" => s.corner_radii = Corners::uniform(parse_f32(value)),
        "roundedTL" => s.corner_radii.top_left = parse_f32(value),
        "roundedTR" => s.corner_radii.top_right = parse_f32(value),
        "roundedBR" => s.corner_radii.bottom_right = parse_f32(value),
        "roundedBL" => s.corner_radii.bottom_left = parse_f32(value),
        "border" => s.border_widths = Edges::all(parse_f32(value)),
        "borderTop" => s.border_widths.top = parse_f32(value),
        "borderRight" => s.border_widths.right = parse_f32(value),
        "borderBottom" => s.border_widths.bottom = parse_f32(value),
        "borderLeft" => s.border_widths.left = parse_f32(value),
        "borderColor" => s.border_color = Some(parse_color(value)),
        "opacity" => s.opacity = parse_f32(value),
        "display" => {
            s.display = match value {
                "none" => Display::None,
                "flex" => Display::Flex,
                "block" => Display::Block,
                _ => Display::Flex,
            }
        }
        "cursor" => {}
        _ => return,
    }

    sync_taffy(dom, node_id);
}

fn sync_taffy(dom: &mut Dom, node_id: NodeId) {
    let node = &dom.nodes[node_id];
    let taffy_style = node.style.to_taffy();
    let tn = node.taffy_node;
    dom.taffy.set_style(tn, taffy_style).unwrap();

    // Also sync font_size in node context
    let font_size = node.style.text.font_size;
    if let Some(ctx) = dom.taffy.get_node_context_mut(tn) {
        ctx.font_size = font_size;
    }
}

fn apply_style_refinement(r: &mut StyleRefinement, prop: &str, value: &str) {
    match prop {
        "bg" => r.background = Some(parse_color(value)),
        "color" => r.text.color = Some(parse_color(value)),
        "opacity" => r.opacity = Some(parse_f32(value)),
        "borderColor" => r.border_color = Some(parse_color(value)),
        _ => {}
    }
}

// ── Value parsing helpers ────────────────────────────────────────────

fn parse_f32(s: &str) -> f32 {
    s.parse().unwrap_or(0.0)
}

fn parse_length(s: &str) -> Length {
    match s {
        "auto" => Length::Auto,
        "full" => Length::Percent(1.0),
        _ => {
            if let Some(pct) = s.strip_suffix('%') {
                Length::Percent(pct.parse::<f32>().unwrap_or(0.0) / 100.0)
            } else {
                Length::Px(parse_f32(s))
            }
        }
    }
}

fn parse_color(s: &str) -> Color {
    if let Some(hex) = s.strip_prefix('#') {
        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                Color::rgb(r, g, b)
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
                Color::rgba(r, g, b, a)
            }
            _ => Color::WHITE,
        }
    } else {
        Color::WHITE
    }
}

fn parse_flex_direction(s: &str) -> FlexDirection {
    match s {
        "row" => FlexDirection::Row,
        "col" | "column" => FlexDirection::Column,
        "row-reverse" => FlexDirection::RowReverse,
        "col-reverse" | "column-reverse" => FlexDirection::ColumnReverse,
        _ => FlexDirection::Row,
    }
}

fn parse_align_items(s: &str) -> AlignItems {
    match s {
        "start" | "flex-start" => AlignItems::FlexStart,
        "end" | "flex-end" => AlignItems::FlexEnd,
        "center" => AlignItems::Center,
        "stretch" => AlignItems::Stretch,
        "baseline" => AlignItems::Baseline,
        _ => AlignItems::Stretch,
    }
}

fn parse_justify_content(s: &str) -> JustifyContent {
    match s {
        "start" | "flex-start" => JustifyContent::FlexStart,
        "end" | "flex-end" => JustifyContent::FlexEnd,
        "center" => JustifyContent::Center,
        "between" | "space-between" => JustifyContent::SpaceBetween,
        "around" | "space-around" => JustifyContent::SpaceAround,
        "evenly" | "space-evenly" => JustifyContent::SpaceEvenly,
        _ => JustifyContent::FlexStart,
    }
}

// ── Application ──────────────────────────────────────────────────────

#[napi]
pub struct Application {
    on_init: Option<Function<'static, ()>>,
    on_window_event: Option<Function<'static, ()>>,
    on_dom_event: Option<Function<'static, (String, String, String)>>,
    gpu: GpuContext,
    windows: HashMap<WindowId, Window>,
    window_label_to_id: HashMap<String, WindowId>,
    window_id_to_label: HashMap<WindowId, String>,
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
            on_dom_event: None,
            windows: Default::default(),
            window_label_to_id: Default::default(),
            window_id_to_label: Default::default(),
        }
    }

    fn insert_window(&mut self, winit_window: Arc<winit::window::Window>, label: String) {
        assert!(
            !self.window_label_to_id.contains_key(&label),
            "Window with label '{}' already exists",
            label
        );

        let dom = DOM_REGISTRY
            .lock()
            .get(&label)
            .cloned()
            .unwrap_or_else(|| {
                // Fallback: create a demo tree if no pre-created DOM exists
                Arc::new(Mutex::new(crate::element::build_demo_tree()))
            });

        match Window::new(&self.gpu, winit_window, dom) {
            Ok(window) => {
                let wid = window.id();
                self.window_label_to_id.insert(label.clone(), wid);
                self.window_id_to_label.insert(wid, label);
                self.windows.insert(wid, window);
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
    pub fn on_dom_event(&mut self, f: Function<'static, (String, String, String)>) {
        self.on_dom_event = Some(f);
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
            UserEvent::RequestRedraw { label } => {
                if let Some(id) = self.window_label_to_id.get(&label) {
                    if let Some(window) = self.windows.get(id) {
                        window.winit_window.request_redraw();
                    }
                }
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
        let mut js_click_node_ids: Vec<String> = Vec::new();

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
                    let mut dom = window.dom.lock();
                    let old_top = dom.hit_state.top_hit;
                    dom.update_hit_test(position.x, position.y);
                    let new_top = dom.hit_state.top_hit;
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
                    let mut dom = window.dom.lock();
                    if let Some((mx, my)) = dom.hit_state.mouse_position {
                        match state {
                            ElementState::Pressed => {
                                let top = dom.hit_state.top_hit;
                                dom.set_active(top);
                                dom.dispatch_mouse_down(mx, my, mouse_button);
                                needs_redraw = true;
                            }
                            ElementState::Released => {
                                dom.dispatch_mouse_up(mx, my, mouse_button);
                                if let Some(active) = dom.hit_state.active_hitbox {
                                    if dom.hit_state.is_hovered(active) {
                                        dom.dispatch_click(mx, my, mouse_button);
                                        // Collect node IDs with js_interactive for JS dispatch
                                        for hitbox in dom.hitbox_store.hitboxes().iter().rev() {
                                            if hitbox.bounds.contains(mx, my) {
                                                let node = &dom.nodes[hitbox.node_id];
                                                if node.interactivity.js_interactive {
                                                    js_click_node_ids
                                                        .push(hitbox.node_id.to_string_id());
                                                }
                                            }
                                        }
                                    }
                                }
                                dom.set_active(None);
                                needs_redraw = true;
                            }
                        }
                    }
                }
            }
            WindowEvent::CursorLeft { .. } => {
                if let Some(window) = self.windows.get_mut(&window_id) {
                    let mut dom = window.dom.lock();
                    dom.hit_state = Default::default();
                    needs_redraw = true;
                }
            }
            WindowEvent::CloseRequested => {
                println!("Close this stupid app ");
                if let Some(label) = self.window_id_to_label.remove(&window_id) {
                    self.window_label_to_id.remove(&label);
                    DOM_REGISTRY.lock().remove(&label);
                }
                self.windows.remove(&window_id);
                if self.windows.is_empty() {
                    event_loop.exit();
                }
            }
            _ => {}
        }

        // Dispatch JS click events directly via callback
        if !js_click_node_ids.is_empty() {
            if let Some(label) = self.window_id_to_label.get(&window_id) {
                if let Some(cb) = &self.on_dom_event {
                    for node_id_str in js_click_node_ids {
                        let _ =
                            cb.call((label.clone(), node_id_str, "click".to_string()));
                    }
                }
            }
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
