use napi::bindgen_prelude::*;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;

use napi_derive::napi;
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

struct WindowEntry {
    dom: Dom,
    /// Present once the winit window has been created by the event loop
    handle: Option<Window>,
}

struct AppState {
    gpu: GpuContext,
    windows: HashMap<String, WindowEntry>,
    winit_id_to_label: HashMap<WindowId, String>,
    pending_events: Vec<AppEvent>,
}

thread_local! {
    static APP_STATE: RefCell<Option<AppState>> = RefCell::new(None);
    static LOOP_PROXY: RefCell<Option<EventLoopProxy<UserEvent>>> = RefCell::new(None);
}

fn with_state<R>(f: impl FnOnce(&mut AppState) -> R) -> R {
    APP_STATE.with(|s| {
        let mut borrow = s.borrow_mut();
        let state = borrow.as_mut().expect("Application not initialized");
        f(state)
    })
}

fn send_proxy_event(event: UserEvent) {
    LOOP_PROXY.with(|p| {
        if let Some(proxy) = &*p.borrow() {
            let _ = proxy.send_event(event);
        }
    });
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NodeEventData {
    window_label: String,
    node_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyEventData {
    window_label: String,
    key: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResizeEventData {
    window_label: String,
    width: u32,
    height: u32,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum AppEvent {
    Click(NodeEventData),
    MouseDown(NodeEventData),
    MouseUp(NodeEventData),
    KeyDown(KeyEventData),
    KeyUp(KeyEventData),
    Resize(ResizeEventData),
    HotReload,
}

#[napi]
pub fn poll_events() -> serde_json::Value {
    with_state(|state| {
        let events: Vec<AppEvent> = state.pending_events.drain(..).collect();
        serde_json::to_value(&events).unwrap_or(serde_json::Value::Array(vec![]))
    })
}

#[napi]
pub fn reset_dom(label: String) {
    with_state(|state| {
        if let Some(entry) = state.windows.get_mut(&label) {
            let root = entry.dom.root.expect("no root node");
            entry.dom.clear_children(root);
        }
    });
}

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
    with_state(|state| {
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

        state
            .windows
            .insert(options.label.clone(), WindowEntry { dom, handle: None });
    });

    send_proxy_event(UserEvent::CreateWindow {
        label: options.label,
        width: options.width,
        height: options.height,
        title: options.title,
    });
}

#[napi]
pub fn request_quit() {
    send_proxy_event(UserEvent::Quit);
}

#[napi]
pub fn request_redraw(label: String) {
    send_proxy_event(UserEvent::RequestRedraw { label });
}

#[napi]
pub fn get_root_node_id(label: String) -> String {
    with_state(|state| {
        let entry = state.windows.get(&label).expect("window not found");
        entry.dom.root.expect("no root node").to_string_id()
    })
}

#[napi]
pub fn create_element(label: String, element_type: String) -> String {
    let _ = element_type;
    with_state(|state| {
        let entry = state.windows.get_mut(&label).expect("window not found");
        entry.dom.create_view(Style::default()).to_string_id()
    })
}

#[napi]
pub fn create_text_node(label: String, text: String) -> String {
    with_state(|state| {
        let entry = state.windows.get_mut(&label).expect("window not found");
        entry.dom.create_text(text, Style::default()).to_string_id()
    })
}

#[napi]
pub fn append_child(label: String, parent_id: String, child_id: String) {
    with_state(|state| {
        let entry = state.windows.get_mut(&label).expect("window not found");
        entry.dom.append_child(
            NodeId::from_string_id(&parent_id),
            NodeId::from_string_id(&child_id),
        );
    })
}

#[napi]
pub fn insert_before(label: String, parent_id: String, child_id: String, before_id: String) {
    with_state(|state| {
        let entry = state.windows.get_mut(&label).expect("window not found");
        entry.dom.insert_before(
            NodeId::from_string_id(&parent_id),
            NodeId::from_string_id(&child_id),
            NodeId::from_string_id(&before_id),
        );
    })
}

#[napi]
pub fn remove_child(label: String, parent_id: String, child_id: String) {
    with_state(|state| {
        let entry = state.windows.get_mut(&label).expect("window not found");
        entry.dom.remove_child(
            NodeId::from_string_id(&parent_id),
            NodeId::from_string_id(&child_id),
        );
    })
}

#[napi]
pub fn set_text(label: String, node_id: String, text: String) {
    with_state(|state| {
        let entry = state.windows.get_mut(&label).expect("window not found");
        entry
            .dom
            .set_text_content(NodeId::from_string_id(&node_id), text);
    })
}

#[napi]
pub fn set_property(label: String, node_id: String, prop: String, value: String) {
    with_state(|state| {
        let entry = state.windows.get_mut(&label).expect("window not found");
        let nid = NodeId::from_string_id(&node_id);
        apply_property(&mut entry.dom, nid, &prop, &value);
    })
}

// ── Style helpers ────────────────────────────────────────────────────

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
    } else if s == "transparent" {
        Color::TRANSPARENT
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
    event_loop: Option<EventLoop<UserEvent>>,
}

#[napi]
impl Application {
    #[napi(constructor)]
    pub fn new() -> Self {
        let gpu = pollster::block_on(GpuContext::new()).expect("Failed to create GPU context");

        let event_loop = EventLoop::<UserEvent>::with_user_event()
            .build()
            .expect("Error creating event loop");

        LOOP_PROXY.with(|p| {
            *p.borrow_mut() = Some(event_loop.create_proxy());
        });

        APP_STATE.with(|s| {
            *s.borrow_mut() = Some(AppState {
                gpu,
                windows: HashMap::new(),
                winit_id_to_label: HashMap::new(),
                pending_events: Vec::new(),
            });
        });

        Self {
            on_init: None,
            event_loop: Some(event_loop),
        }
    }

    #[napi]
    pub fn on_init(&mut self, f: Function<'static, ()>) {
        self.on_init = Some(f);
    }

    #[napi]
    pub fn pump_app_events(&mut self) -> bool {
        use winit::platform::pump_events::EventLoopExtPumpEvents;
        use winit::platform::pump_events::PumpStatus;

        let mut event_loop = self.event_loop.take().expect("event loop not initialized");
        let status = event_loop.pump_app_events(Some(Duration::ZERO), self);
        self.event_loop = Some(event_loop);

        matches!(status, PumpStatus::Continue)
    }

    #[napi]
    pub fn destroy(&mut self) {
        self.event_loop.take();
        LOOP_PROXY.with(|p| {
            p.borrow_mut().take();
        });
        APP_STATE.with(|s| {
            s.borrow_mut().take();
        });
    }
}

impl ApplicationHandler<UserEvent> for Application {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        println!("Application resumed");
        if let Some(cb) = self.on_init.take() {
            let _ = cb.call(());
        }
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

                let is_visible = attributes.visible;

                println!("Creating window");
                let Ok(winit_window) = event_loop.create_window(attributes.with_visible(false))
                else {
                    println!("Failed to create window");
                    return;
                };

                let winit_window = std::sync::Arc::new(winit_window);
                let wid = winit_window.id();

                with_state(|state| {
                    assert!(
                        state.windows.contains_key(&label),
                        "Window entry '{}' must exist before creating handle",
                        label
                    );
                    match Window::new(&state.gpu, winit_window) {
                        Ok(mut window) => {
                            state.winit_id_to_label.insert(wid, label.clone());
                            let entry = state.windows.get_mut(&label).unwrap();

                            window.paint_and_present(
                                &state.gpu.device,
                                &state.gpu.queue,
                                &mut entry.dom,
                            );

                            window.winit_window.set_visible(is_visible);
                            entry.handle = Some(window);
                        }
                        Err(e) => println!("Error creating window : {:#?}", e),
                    }
                });
            }
            UserEvent::RequestRedraw { label } => {
                with_state(|state| {
                    if let Some(entry) = state.windows.get(&label) {
                        if let Some(ref handle) = entry.handle {
                            handle.winit_window.request_redraw();
                        }
                    }
                });
            }
            UserEvent::Quit => {
                with_state(|state| {
                    state.windows.clear();
                    state.winit_id_to_label.clear();
                });
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

        with_state(|state| {
            let Some(label) = state.winit_id_to_label.get(&window_id).cloned() else {
                return;
            };

            let mut needs_redraw = false;
            let mut js_node_events: Vec<(String, &str)> = Vec::new();

            match event {
                WindowEvent::Resized(size) => {
                    if let Some(entry) = state.windows.get_mut(&label) {
                        if let Some(ref mut handle) = entry.handle {
                            if handle.on_resize(&state.gpu.device, size.width, size.height) {
                                handle.winit_window.request_redraw();
                            }
                        }
                    }
                    state.pending_events.push(AppEvent::Resize(ResizeEventData {
                        window_label: label.clone(),
                        width: size.width,
                        height: size.height,
                    }));
                }
                WindowEvent::RedrawRequested => {
                    if let Some(entry) = state.windows.get_mut(&label) {
                        let WindowEntry { handle, dom } = entry;
                        if let Some(handle) = handle {
                            handle.paint_and_present(&state.gpu.device, &state.gpu.queue, dom);
                        }
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    if let Some(entry) = state.windows.get_mut(&label) {
                        let WindowEntry { handle, dom } = entry;
                        if let Some(handle) = handle {
                            let scale = handle.winit_window.scale_factor();
                            let logical_x = position.x / scale;
                            let logical_y = position.y / scale;
                            let old_top = dom.hit_state.top_hit;
                            dom.update_hit_test(logical_x, logical_y);
                            if old_top != dom.hit_state.top_hit {
                                needs_redraw = true;
                            }
                        }
                    }
                }
                WindowEvent::MouseInput {
                    state: btn_state,
                    button,
                    ..
                } => {
                    use winit::event::ElementState;

                    let mouse_button = match button {
                        winit::event::MouseButton::Left => crate::interactivity::MouseButton::Left,
                        winit::event::MouseButton::Right => {
                            crate::interactivity::MouseButton::Right
                        }
                        winit::event::MouseButton::Middle => {
                            crate::interactivity::MouseButton::Middle
                        }
                        _ => crate::interactivity::MouseButton::Left,
                    };

                    if let Some(entry) = state.windows.get_mut(&label) {
                        let dom = &mut entry.dom;
                        if let Some((mx, my)) = dom.hit_state.mouse_position {
                            match btn_state {
                                ElementState::Pressed => {
                                    let top = dom.hit_state.top_hit;
                                    dom.set_active(top);
                                    dom.dispatch_mouse_down(mx, my, mouse_button);
                                    // Collect mousedown JS events
                                    for hitbox in dom.hitbox_store.hitboxes().iter().rev() {
                                        if hitbox.bounds.contains(mx, my) {
                                            let node = &dom.nodes[hitbox.node_id];
                                            if node.interactivity.js_interactive {
                                                js_node_events.push((
                                                    hitbox.node_id.to_string_id(),
                                                    "mousedown",
                                                ));
                                            }
                                        }
                                    }
                                    needs_redraw = true;
                                }
                                ElementState::Released => {
                                    dom.dispatch_mouse_up(mx, my, mouse_button);
                                    // Collect mouseup JS events
                                    for hitbox in dom.hitbox_store.hitboxes().iter().rev() {
                                        if hitbox.bounds.contains(mx, my) {
                                            let node = &dom.nodes[hitbox.node_id];
                                            if node.interactivity.js_interactive {
                                                js_node_events.push((
                                                    hitbox.node_id.to_string_id(),
                                                    "mouseup",
                                                ));
                                            }
                                        }
                                    }
                                    if let Some(active) = dom.hit_state.active_hitbox {
                                        if dom.hit_state.is_hovered(active) {
                                            dom.dispatch_click(mx, my, mouse_button);
                                            for hitbox in dom.hitbox_store.hitboxes().iter().rev() {
                                                if hitbox.bounds.contains(mx, my) {
                                                    let node = &dom.nodes[hitbox.node_id];
                                                    if node.interactivity.js_interactive {
                                                        js_node_events.push((
                                                            hitbox.node_id.to_string_id(),
                                                            "click",
                                                        ));
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
                WindowEvent::KeyboardInput {
                    event: key_event, ..
                } => {
                    use winit::event::ElementState;
                    use winit::keyboard::{Key, NamedKey};

                    if key_event.state == ElementState::Pressed {
                        // F5 == hot reload
                        if key_event.logical_key == Key::Named(NamedKey::F5) {
                            state.pending_events.push(AppEvent::HotReload);
                        } else {
                            let key_str = match &key_event.logical_key {
                                Key::Character(c) => c.to_string(),
                                Key::Named(named) => format!("{:?}", named),
                                _ => return,
                            };
                            state.pending_events.push(AppEvent::KeyDown(KeyEventData {
                                window_label: label.clone(),
                                key: key_str,
                            }));
                        }
                    } else {
                        let key_str = match &key_event.logical_key {
                            Key::Character(c) => c.to_string(),
                            Key::Named(named) => format!("{:?}", named),
                            _ => return,
                        };
                        state.pending_events.push(AppEvent::KeyUp(KeyEventData {
                            window_label: label.clone(),
                            key: key_str,
                        }));
                    }
                }
                WindowEvent::CursorLeft { .. } => {
                    if let Some(entry) = state.windows.get_mut(&label) {
                        entry.dom.hit_state = Default::default();
                        needs_redraw = true;
                    }
                }
                WindowEvent::CloseRequested => {
                    println!("Close window event");
                    state.winit_id_to_label.remove(&window_id);
                    state.windows.remove(&label);
                    if state.windows.is_empty() {
                        event_loop.exit();
                    }
                }
                _ => {}
            }

            // Push JS node events
            for (node_id_str, event_kind) in js_node_events {
                let event = match event_kind {
                    "click" => AppEvent::Click(NodeEventData {
                        window_label: label.clone(),
                        node_id: node_id_str,
                    }),
                    "mousedown" => AppEvent::MouseDown(NodeEventData {
                        window_label: label.clone(),
                        node_id: node_id_str,
                    }),
                    "mouseup" => AppEvent::MouseUp(NodeEventData {
                        window_label: label.clone(),
                        node_id: node_id_str,
                    }),
                    _ => continue,
                };
                state.pending_events.push(event);
            }

            if needs_redraw {
                if let Some(entry) = state.windows.get(&label) {
                    if let Some(ref handle) = entry.handle {
                        handle.winit_window.request_redraw();
                    }
                }
            }
        });
    }
}
