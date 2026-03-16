use napi::bindgen_prelude::*;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
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

static NEXT_WINDOW_ID: AtomicU32 = AtomicU32::new(1);

struct WindowEntry {
    dom: Dom,
    /// Present once the winit window has been created by the event loop
    handle: Option<Window>,
    /// Root font size for rem unit resolution (default 16.0)
    rem_base: f32,
}

struct AppState {
    gpu: GpuContext,
    windows: HashMap<u32, WindowEntry>,
    winit_id_to_id: HashMap<WindowId, u32>,
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
    window_id: u32,
    node_id: NodeId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyEventData {
    window_id: u32,
    key: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResizeEventData {
    window_id: u32,
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
pub fn reset_dom(window_id: u32) {
    with_state(|state| {
        if let Some(entry) = state.windows.get_mut(&window_id) {
            let root = entry.dom.root.expect("no root node");
            entry.dom.clear_children(root);
        }
    });
}

enum UserEvent {
    CreateWindow {
        id: u32,
        width: u32,
        height: u32,
        title: String,
    },
    RequestRedraw {
        id: u32,
    },
    Quit,
}

#[napi(object)]
pub struct WindowOptions {
    pub width: u32,
    pub height: u32,
    pub title: String,
}

#[napi]
pub fn create_window(options: WindowOptions) -> u32 {
    let id = NEXT_WINDOW_ID.fetch_add(1, Ordering::Relaxed);

    with_state(|state| {
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

        state.windows.insert(
            id,
            WindowEntry {
                dom,
                handle: None,
                rem_base: 16.0,
            },
        );
    });

    send_proxy_event(UserEvent::CreateWindow {
        id,
        width: options.width,
        height: options.height,
        title: options.title,
    });

    id
}

#[napi]
pub fn request_quit() {
    send_proxy_event(UserEvent::Quit);
}

#[napi]
pub fn request_redraw(window_id: u32) {
    send_proxy_event(UserEvent::RequestRedraw { id: window_id });
}

#[napi]
pub fn get_root_node_id(window_id: u32) -> serde_json::Value {
    with_state(|state| {
        let entry = state.windows.get(&window_id).expect("window not found");
        serde_json::to_value(entry.dom.root.expect("no root node")).unwrap()
    })
}

#[napi]
pub fn create_element(window_id: u32, element_type: String) -> serde_json::Value {
    let _ = element_type;
    with_state(|state| {
        let entry = state.windows.get_mut(&window_id).expect("window not found");
        serde_json::to_value(entry.dom.create_view(Style::default())).unwrap()
    })
}

#[napi]
pub fn create_text_node(window_id: u32, text: String) -> serde_json::Value {
    with_state(|state| {
        let entry = state.windows.get_mut(&window_id).expect("window not found");
        serde_json::to_value(entry.dom.create_text(text, Style::default())).unwrap()
    })
}

#[napi]
pub fn append_child(window_id: u32, parent_id: serde_json::Value, child_id: serde_json::Value) {
    let pid = serde_json::from_value::<NodeId>(parent_id).unwrap();
    let cid = serde_json::from_value::<NodeId>(child_id).unwrap();
    with_state(|state| {
        let entry = state.windows.get_mut(&window_id).expect("window not found");
        entry.dom.append_child(pid, cid);
    })
}

#[napi]
pub fn insert_before(
    window_id: u32,
    parent_id: serde_json::Value,
    child_id: serde_json::Value,
    before_id: serde_json::Value,
) {
    let pid = serde_json::from_value::<NodeId>(parent_id).unwrap();
    let cid = serde_json::from_value::<NodeId>(child_id).unwrap();
    let bid = serde_json::from_value::<NodeId>(before_id).unwrap();
    with_state(|state| {
        let entry = state.windows.get_mut(&window_id).expect("window not found");
        entry.dom.insert_before(pid, cid, bid);
    })
}

#[napi]
pub fn remove_child(window_id: u32, parent_id: serde_json::Value, child_id: serde_json::Value) {
    let pid = serde_json::from_value::<NodeId>(parent_id).unwrap();
    let cid = serde_json::from_value::<NodeId>(child_id).unwrap();
    with_state(|state| {
        let entry = state.windows.get_mut(&window_id).expect("window not found");
        entry.dom.remove_child(pid, cid);
    })
}

#[napi]
pub fn set_text(window_id: u32, node_id: serde_json::Value, text: String) {
    let nid = serde_json::from_value::<NodeId>(node_id).unwrap();
    with_state(|state| {
        let entry = state.windows.get_mut(&window_id).expect("window not found");
        entry.dom.set_text_content(nid, text);
    })
}

// ── Prop key enum ────────────────────────────────────────────────────

#[napi]
pub enum PropKey {
    W = 0,
    H = 1,
    P = 2,
    Px = 3,
    Py = 4,
    Pt = 5,
    Pb = 6,
    Pl = 7,
    Pr = 8,
    M = 9,
    Mx = 10,
    My = 11,
    Mt = 12,
    Mb = 13,
    Ml = 14,
    Mr = 15,
    Flex = 16,
    FlexDir = 17,
    FlexGrow = 18,
    FlexShrink = 19,
    Items = 20,
    Justify = 21,
    Gap = 22,
    Bg = 23,
    Color = 24,
    FontSize = 25,
    FontWeight = 26,
    Rounded = 27,
    RoundedTL = 28,
    RoundedTR = 29,
    RoundedBR = 30,
    RoundedBL = 31,
    Border = 32,
    BorderTop = 33,
    BorderRight = 34,
    BorderBottom = 35,
    BorderLeft = 36,
    BorderColor = 37,
    Opacity = 38,
    Display = 39,
    Cursor = 40,
    Interactive = 41,
    Visible = 42,
    HoverBg = 43,
    HoverColor = 44,
    HoverOpacity = 45,
    HoverBorderColor = 46,
    ActiveBg = 47,
    ActiveColor = 48,
    ActiveOpacity = 49,
    ActiveBorderColor = 50,
}

// ── Typed value structs ──────────────────────────────────────────────

#[napi(object)]
pub struct JsLength {
    pub value: f64,
    /// 0 = px, 1 = percent, 2 = rem, 3 = auto
    pub unit: u8,
}

#[napi(object)]
pub struct JsColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

// ── Enum value types ─────────────────────────────────────────────────

#[napi]
pub enum FlexDirectionValue {
    Row = 0,
    Column = 1,
    RowReverse = 2,
    ColumnReverse = 3,
}

#[napi]
pub enum AlignItemsValue {
    FlexStart = 0,
    FlexEnd = 1,
    Center = 2,
    Stretch = 3,
    Baseline = 4,
}

#[napi]
pub enum JustifyContentValue {
    FlexStart = 0,
    FlexEnd = 1,
    Center = 2,
    SpaceBetween = 3,
    SpaceAround = 4,
    SpaceEvenly = 5,
}

#[napi]
pub enum DisplayValue {
    None = 0,
    Flex = 1,
    Block = 2,
}

// ── Typed property setters ───────────────────────────────────────────

#[napi]
pub fn set_length_prop(window_id: u32, node_id: serde_json::Value, prop: PropKey, value: JsLength) {
    let nid = serde_json::from_value::<NodeId>(node_id).unwrap();
    with_state(|state| {
        let entry = state.windows.get_mut(&window_id).expect("window not found");
        let length = match value.unit {
            0 => Length::Px(value.value as f32),
            1 => Length::Percent(value.value as f32),
            2 => Length::Px(value.value as f32 * entry.rem_base),
            _ => Length::Auto,
        };
        {
            let s = &mut entry.dom.nodes[nid].style;
            match prop {
                PropKey::W => s.size.width = length,
                PropKey::H => s.size.height = length,
                _ => return,
            }
        }
        sync_taffy(&mut entry.dom, nid);
    });
}

#[napi]
pub fn set_color_prop(window_id: u32, node_id: serde_json::Value, prop: PropKey, value: JsColor) {
    let nid = serde_json::from_value::<NodeId>(node_id).unwrap();
    let color = Color::rgba(value.r, value.g, value.b, value.a);
    with_state(|state| {
        let entry = state.windows.get_mut(&window_id).expect("window not found");

        match prop {
            PropKey::HoverBg | PropKey::HoverColor | PropKey::HoverBorderColor => {
                let r = entry.dom.nodes[nid]
                    .interactivity
                    .hover_style
                    .get_or_insert_with(|| Box::new(StyleRefinement::default()));
                match prop {
                    PropKey::HoverBg => r.background = Some(color),
                    PropKey::HoverColor => r.text.color = Some(color),
                    PropKey::HoverBorderColor => r.border_color = Some(color),
                    _ => unreachable!(),
                }
                return;
            }
            PropKey::ActiveBg | PropKey::ActiveColor | PropKey::ActiveBorderColor => {
                let r = entry.dom.nodes[nid]
                    .interactivity
                    .active_style
                    .get_or_insert_with(|| Box::new(StyleRefinement::default()));
                match prop {
                    PropKey::ActiveBg => r.background = Some(color),
                    PropKey::ActiveColor => r.text.color = Some(color),
                    PropKey::ActiveBorderColor => r.border_color = Some(color),
                    _ => unreachable!(),
                }
                return;
            }
            _ => {}
        }

        {
            let s = &mut entry.dom.nodes[nid].style;
            match prop {
                PropKey::Bg => s.background = Some(color),
                PropKey::Color => s.text.color = color,
                PropKey::BorderColor => s.border_color = Some(color),
                _ => return,
            }
        }
        sync_taffy(&mut entry.dom, nid);
    });
}

#[napi]
pub fn set_f32_prop(window_id: u32, node_id: serde_json::Value, prop: PropKey, value: f64) {
    let nid = serde_json::from_value::<NodeId>(node_id).unwrap();
    let v = value as f32;
    with_state(|state| {
        let entry = state.windows.get_mut(&window_id).expect("window not found");

        // Props that don't need sync_taffy
        match prop {
            PropKey::HoverOpacity => {
                let r = entry.dom.nodes[nid]
                    .interactivity
                    .hover_style
                    .get_or_insert_with(|| Box::new(StyleRefinement::default()));
                r.opacity = Some(v);
                return;
            }
            PropKey::ActiveOpacity => {
                let r = entry.dom.nodes[nid]
                    .interactivity
                    .active_style
                    .get_or_insert_with(|| Box::new(StyleRefinement::default()));
                r.opacity = Some(v);
                return;
            }
            PropKey::Interactive => {
                entry.dom.nodes[nid].interactivity.js_interactive = v > 0.5;
                return;
            }
            _ => {}
        }

        {
            let s = &mut entry.dom.nodes[nid].style;
            match prop {
                PropKey::P => s.padding = Edges::all(v),
                PropKey::Px => {
                    s.padding.left = v;
                    s.padding.right = v;
                }
                PropKey::Py => {
                    s.padding.top = v;
                    s.padding.bottom = v;
                }
                PropKey::Pt => s.padding.top = v,
                PropKey::Pb => s.padding.bottom = v,
                PropKey::Pl => s.padding.left = v,
                PropKey::Pr => s.padding.right = v,
                PropKey::M => s.margin = Edges::all(v),
                PropKey::Mx => {
                    s.margin.left = v;
                    s.margin.right = v;
                }
                PropKey::My => {
                    s.margin.top = v;
                    s.margin.bottom = v;
                }
                PropKey::Mt => s.margin.top = v,
                PropKey::Mb => s.margin.bottom = v,
                PropKey::Ml => s.margin.left = v,
                PropKey::Mr => s.margin.right = v,
                PropKey::Flex => {
                    s.display = Display::Flex;
                    s.flex_grow = v;
                }
                PropKey::FlexGrow => s.flex_grow = v,
                PropKey::FlexShrink => s.flex_shrink = v,
                PropKey::Gap => {
                    s.gap = GapSize {
                        width: DefiniteLength::Px(v),
                        height: DefiniteLength::Px(v),
                    };
                }
                PropKey::FontSize => s.text.font_size = v,
                PropKey::FontWeight => {}
                PropKey::Rounded => s.corner_radii = Corners::uniform(v),
                PropKey::RoundedTL => s.corner_radii.top_left = v,
                PropKey::RoundedTR => s.corner_radii.top_right = v,
                PropKey::RoundedBR => s.corner_radii.bottom_right = v,
                PropKey::RoundedBL => s.corner_radii.bottom_left = v,
                PropKey::Border => s.border_widths = Edges::all(v),
                PropKey::BorderTop => s.border_widths.top = v,
                PropKey::BorderRight => s.border_widths.right = v,
                PropKey::BorderBottom => s.border_widths.bottom = v,
                PropKey::BorderLeft => s.border_widths.left = v,
                PropKey::Opacity => s.opacity = v,
                PropKey::Visible => {
                    s.visibility = if v > 0.5 {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    };
                }
                PropKey::Cursor => {}
                _ => return,
            }
        }
        sync_taffy(&mut entry.dom, nid);
    });
}

#[napi]
pub fn set_enum_prop(window_id: u32, node_id: serde_json::Value, prop: PropKey, value: i32) {
    let nid = serde_json::from_value::<NodeId>(node_id).unwrap();
    with_state(|state| {
        let entry = state.windows.get_mut(&window_id).expect("window not found");
        {
            let s = &mut entry.dom.nodes[nid].style;
            match prop {
                PropKey::FlexDir => {
                    s.flex_direction = match value {
                        0 => FlexDirection::Row,
                        1 => FlexDirection::Column,
                        2 => FlexDirection::RowReverse,
                        3 => FlexDirection::ColumnReverse,
                        _ => FlexDirection::Row,
                    };
                }
                PropKey::Items => {
                    s.align_items = Some(match value {
                        0 => AlignItems::FlexStart,
                        1 => AlignItems::FlexEnd,
                        2 => AlignItems::Center,
                        3 => AlignItems::Stretch,
                        4 => AlignItems::Baseline,
                        _ => AlignItems::Stretch,
                    });
                }
                PropKey::Justify => {
                    s.justify_content = Some(match value {
                        0 => JustifyContent::FlexStart,
                        1 => JustifyContent::FlexEnd,
                        2 => JustifyContent::Center,
                        3 => JustifyContent::SpaceBetween,
                        4 => JustifyContent::SpaceAround,
                        5 => JustifyContent::SpaceEvenly,
                        _ => JustifyContent::FlexStart,
                    });
                }
                PropKey::Display => {
                    s.display = match value {
                        0 => Display::None,
                        1 => Display::Flex,
                        2 => Display::Block,
                        _ => Display::Flex,
                    };
                }
                _ => return,
            }
        }
        sync_taffy(&mut entry.dom, nid);
    });
}

#[napi]
pub fn set_rem_base(window_id: u32, value: f64) {
    with_state(|state| {
        if let Some(entry) = state.windows.get_mut(&window_id) {
            entry.rem_base = value as f32;
        }
    });
}

// ── Style helpers ────────────────────────────────────────────────────

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
                winit_id_to_id: HashMap::new(),
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
                        state.windows.contains_key(&id),
                        "Window entry '{}' must exist before creating handle",
                        id
                    );
                    match Window::new(&state.gpu, winit_window) {
                        Ok(mut window) => {
                            state.winit_id_to_id.insert(wid, id);
                            let entry = state.windows.get_mut(&id).unwrap();

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
            UserEvent::RequestRedraw { id } => {
                with_state(|state| {
                    if let Some(entry) = state.windows.get(&id) {
                        if let Some(ref handle) = entry.handle {
                            handle.winit_window.request_redraw();
                        }
                    }
                });
            }
            UserEvent::Quit => {
                with_state(|state| {
                    state.windows.clear();
                    state.winit_id_to_id.clear();
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
            let Some(&wid) = state.winit_id_to_id.get(&window_id) else {
                return;
            };

            let mut needs_redraw = false;
            let mut js_node_events: Vec<(NodeId, &str)> = Vec::new();

            match event {
                WindowEvent::Resized(size) => {
                    if let Some(entry) = state.windows.get_mut(&wid) {
                        if let Some(ref mut handle) = entry.handle {
                            if handle.on_resize(&state.gpu.device, size.width, size.height) {
                                handle.winit_window.request_redraw();
                            }
                        }
                    }
                    state.pending_events.push(AppEvent::Resize(ResizeEventData {
                        window_id: wid,
                        width: size.width,
                        height: size.height,
                    }));
                }
                WindowEvent::RedrawRequested => {
                    if let Some(entry) = state.windows.get_mut(&wid) {
                        let WindowEntry { handle, dom, .. } = entry;
                        if let Some(handle) = handle {
                            handle.paint_and_present(&state.gpu.device, &state.gpu.queue, dom);
                        }
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    if let Some(entry) = state.windows.get_mut(&wid) {
                        let WindowEntry { handle, dom, .. } = entry;
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

                    if let Some(entry) = state.windows.get_mut(&wid) {
                        let dom = &mut entry.dom;
                        if let Some((mx, my)) = dom.hit_state.mouse_position {
                            match btn_state {
                                ElementState::Pressed => {
                                    let top = dom.hit_state.top_hit;
                                    dom.set_active(top);
                                    dom.dispatch_mouse_down(mx, my, mouse_button);
                                    for hitbox in dom.hitbox_store.hitboxes().iter().rev() {
                                        if hitbox.bounds.contains(mx, my) {
                                            let node = &dom.nodes[hitbox.node_id];
                                            if node.interactivity.js_interactive {
                                                js_node_events.push((hitbox.node_id, "mousedown"));
                                            }
                                        }
                                    }
                                    needs_redraw = true;
                                }
                                ElementState::Released => {
                                    dom.dispatch_mouse_up(mx, my, mouse_button);
                                    for hitbox in dom.hitbox_store.hitboxes().iter().rev() {
                                        if hitbox.bounds.contains(mx, my) {
                                            let node = &dom.nodes[hitbox.node_id];
                                            if node.interactivity.js_interactive {
                                                js_node_events.push((hitbox.node_id, "mouseup"));
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
                                                        js_node_events
                                                            .push((hitbox.node_id, "click"));
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
                        if key_event.logical_key == Key::Named(NamedKey::F5) {
                            state.pending_events.push(AppEvent::HotReload);
                        } else {
                            let key_str = match &key_event.logical_key {
                                Key::Character(c) => c.to_string(),
                                Key::Named(named) => format!("{:?}", named),
                                _ => return,
                            };
                            state.pending_events.push(AppEvent::KeyDown(KeyEventData {
                                window_id: wid,
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
                            window_id: wid,
                            key: key_str,
                        }));
                    }
                }
                WindowEvent::CursorLeft { .. } => {
                    if let Some(entry) = state.windows.get_mut(&wid) {
                        entry.dom.hit_state = Default::default();
                        needs_redraw = true;
                    }
                }
                WindowEvent::CloseRequested => {
                    println!("Close window event");
                    state.winit_id_to_id.remove(&window_id);
                    state.windows.remove(&wid);
                    if state.windows.is_empty() {
                        event_loop.exit();
                    }
                }
                _ => {}
            }

            // Push JS node events
            for (node_id, event_kind) in js_node_events {
                let event = match event_kind {
                    "click" => AppEvent::Click(NodeEventData {
                        window_id: wid,
                        node_id,
                    }),
                    "mousedown" => AppEvent::MouseDown(NodeEventData {
                        window_id: wid,
                        node_id,
                    }),
                    "mouseup" => AppEvent::MouseUp(NodeEventData {
                        window_id: wid,
                        node_id,
                    }),
                    _ => continue,
                };
                state.pending_events.push(event);
            }

            if needs_redraw {
                if let Some(entry) = state.windows.get(&wid) {
                    if let Some(ref handle) = entry.handle {
                        handle.winit_window.request_redraw();
                    }
                }
            }
        });
    }
}
