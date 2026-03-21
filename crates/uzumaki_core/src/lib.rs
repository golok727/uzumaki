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
pub mod input;
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
    /// Bitmask of currently pressed mouse buttons (1=left, 2=right, 4=middle)
    mouse_buttons: u8,
    /// Bitmask of keyboard modifiers (1=ctrl, 2=alt, 4=shift, 8=meta)
    modifiers: u32,
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
struct MouseEventData {
    window_id: u32,
    node_id: NodeId,
    x: f32,
    y: f32,
    screen_x: f32,
    screen_y: f32,
    button: u8,
    buttons: u8,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyEventData {
    window_id: u32,
    key: String,
    code: String,
    key_code: u32,
    modifiers: u32,
    repeat: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResizeEventData {
    window_id: u32,
    width: u32,
    height: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InputEventData {
    window_id: u32,
    node_id: NodeId,
    value: String,
    input_type: String,
    data: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FocusEventData {
    window_id: u32,
    node_id: NodeId,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum AppEvent {
    Click(MouseEventData),
    MouseDown(MouseEventData),
    MouseUp(MouseEventData),
    KeyDown(KeyEventData),
    KeyUp(KeyEventData),
    Resize(ResizeEventData),
    Input(InputEventData),
    Focus(FocusEventData),
    Blur(FocusEventData),
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
    with_state(|state| {
        let entry = state.windows.get_mut(&window_id).expect("window not found");
        if element_type == "input" {
            serde_json::to_value(entry.dom.create_input(Style::default())).unwrap()
        } else {
            serde_json::to_value(entry.dom.create_view(Style::default())).unwrap()
        }
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

// ── Input attribute setters ───────────────────────────────────────────

#[napi]
pub fn set_input_value(window_id: u32, node_id: serde_json::Value, value: String) {
    let nid = serde_json::from_value::<NodeId>(node_id).unwrap();
    with_state(|state| {
        let entry = state.windows.get_mut(&window_id).expect("window not found");
        if let Some(node) = entry.dom.nodes.get_mut(nid) {
            if let Some(is) = &mut node.input_state {
                is.set_value(value);
            }
        }
    });
}

#[napi]
pub fn set_input_placeholder(window_id: u32, node_id: serde_json::Value, placeholder: String) {
    let nid = serde_json::from_value::<NodeId>(node_id).unwrap();
    with_state(|state| {
        let entry = state.windows.get_mut(&window_id).expect("window not found");
        if let Some(node) = entry.dom.nodes.get_mut(nid) {
            if let Some(is) = &mut node.input_state {
                is.placeholder = placeholder;
            }
        }
    });
}

#[napi]
pub fn set_input_disabled(window_id: u32, node_id: serde_json::Value, disabled: bool) {
    let nid = serde_json::from_value::<NodeId>(node_id).unwrap();
    with_state(|state| {
        let entry = state.windows.get_mut(&window_id).expect("window not found");
        if let Some(node) = entry.dom.nodes.get_mut(nid) {
            if let Some(is) = &mut node.input_state {
                is.disabled = disabled;
            }
        }
    });
}

#[napi]
pub fn set_input_max_length(window_id: u32, node_id: serde_json::Value, max_length: i32) {
    let nid = serde_json::from_value::<NodeId>(node_id).unwrap();
    with_state(|state| {
        let entry = state.windows.get_mut(&window_id).expect("window not found");
        if let Some(node) = entry.dom.nodes.get_mut(nid) {
            if let Some(is) = &mut node.input_state {
                is.max_length = if max_length > 0 {
                    Some(max_length as usize)
                } else {
                    None
                };
            }
        }
    });
}

#[napi]
pub fn set_input_multiline(window_id: u32, node_id: serde_json::Value, multiline: bool) {
    let nid = serde_json::from_value::<NodeId>(node_id).unwrap();
    with_state(|state| {
        let entry = state.windows.get_mut(&window_id).expect("window not found");
        if let Some(node) = entry.dom.nodes.get_mut(nid) {
            if let Some(is) = &mut node.input_state {
                is.multiline = multiline;
            }
        }
    });
}

#[napi]
pub fn set_input_secure(window_id: u32, node_id: serde_json::Value, secure: bool) {
    let nid = serde_json::from_value::<NodeId>(node_id).unwrap();
    with_state(|state| {
        let entry = state.windows.get_mut(&window_id).expect("window not found");
        if let Some(node) = entry.dom.nodes.get_mut(nid) {
            if let Some(is) = &mut node.input_state {
                is.secure = secure;
            }
        }
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
                mouse_buttons: 0,
                modifiers: 0,
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
                            // Update scroll offset for focused input before paint
                            if let Some(focused_id) = dom.focused_node {
                                let scroll_info = dom.nodes.get(focused_id).and_then(|node| {
                                    node.input_state.as_ref().map(|is| {
                                        let display_text = is.display_text();
                                        let font_size = node.style.text.font_size;
                                        let padding = node.style.padding.left;
                                        let input_padding = if padding > 0.0 { padding } else { 8.0 };
                                        let pt = node.style.padding.top;
                                        let top_pad = if pt > 0.0 { pt } else { 4.0 };
                                        let cursor_pos = is.selection.active;
                                        let taffy_node = node.taffy_node;
                                        (display_text, font_size, input_padding, top_pad, cursor_pos, taffy_node)
                                    })
                                });
                                if let Some((display_text, font_size, input_padding, top_pad, cursor_pos, taffy_node)) = scroll_info {
                                    let is_multiline = dom.nodes.get(focused_id)
                                        .and_then(|n| n.input_state.as_ref())
                                        .map(|is| is.multiline)
                                        .unwrap_or(false);

                                    let (input_width, input_height) = dom.taffy.layout(taffy_node)
                                        .map(|l| (l.size.width as f32 - input_padding * 2.0, l.size.height as f32))
                                        .unwrap_or((200.0, 100.0));

                                    if is_multiline {
                                        let positions = handle.text_renderer.grapheme_positions_2d(
                                            &display_text,
                                            font_size,
                                            Some(input_width),
                                        );
                                        let cursor_y = if cursor_pos < positions.len() {
                                            positions[cursor_pos].y
                                        } else {
                                            positions.last().map(|p| p.y).unwrap_or(0.0)
                                        };
                                        let line_height = (font_size * 1.2).round();
                                        if let Some(node) = dom.nodes.get_mut(focused_id) {
                                            if let Some(is) = &mut node.input_state {
                                                is.update_scroll_y(cursor_y, line_height, input_height - top_pad * 2.0);
                                            }
                                        }
                                    } else {
                                        let positions = handle.text_renderer.grapheme_x_positions(
                                            &display_text,
                                            font_size,
                                        );
                                        let cursor_x = if cursor_pos < positions.len() {
                                            positions[cursor_pos]
                                        } else {
                                            positions.last().copied().unwrap_or(0.0)
                                        };
                                        if let Some(node) = dom.nodes.get_mut(focused_id) {
                                            if let Some(is) = &mut node.input_state {
                                                is.update_scroll(cursor_x, input_width);
                                            }
                                        }
                                    }
                                }
                            }

                            handle.paint_and_present(&state.gpu.device, &state.gpu.queue, dom);

                            // Keep redrawing for cursor blink when input is focused
                            if dom.focused_node.is_some() && dom.window_focused {
                                handle.winit_window.request_redraw();
                            }
                        }
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let mouse_buttons = state.mouse_buttons;
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

                            // Input drag selection
                            if mouse_buttons & 1 != 0 {
                                if let Some(drag_nid) = dom.dragging_input {
                                    let cursor_info = {
                                        if let Some(node) = dom.nodes.get(drag_nid) {
                                            let is = node.input_state.as_ref();
                                            is.map(|is| {
                                                let display_text = is.display_text();
                                                let font_size = node.style.text.font_size;
                                                let scroll_offset = is.scroll_offset;
                                                let scroll_offset_y = is.scroll_offset_y;
                                                let is_multiline = is.multiline;
                                                let padding = node.style.padding.left as f64;
                                                let input_padding = if padding > 0.0 { padding } else { 8.0 };
                                                let pad_top = node.style.padding.top;
                                                let top_pad = if pad_top > 0.0 { pad_top } else { 4.0 };
                                                let hitbox_bounds = node.interactivity.hitbox_id
                                                    .and_then(|hid| dom.hitbox_store.get(hid))
                                                    .map(|hb| hb.bounds);
                                                let taffy_node = node.taffy_node;
                                                (display_text, font_size, scroll_offset, scroll_offset_y, is_multiline, input_padding, top_pad, hitbox_bounds, taffy_node)
                                            })
                                        } else {
                                            None
                                        }
                                    };

                                    if let Some((display_text, font_size, scroll_offset, scroll_offset_y, is_multiline, input_padding, top_pad, Some(hb), taffy_node)) = cursor_info {
                                        let grapheme_idx = if !display_text.is_empty() {
                                            if is_multiline {
                                                let wrap_width = dom.taffy.layout(taffy_node)
                                                    .map(|l| l.size.width as f32 - input_padding as f32 * 2.0)
                                                    .unwrap_or(200.0);
                                                let relative_x = (logical_x - hb.x - input_padding) as f32;
                                                let relative_y = (logical_y - hb.y) as f32 + scroll_offset_y - top_pad;
                                                handle.text_renderer.hit_to_grapheme_2d(&display_text, font_size, Some(wrap_width), relative_x, relative_y)
                                            } else {
                                                let relative_x = (logical_x - hb.x - input_padding) as f32 + scroll_offset;
                                                handle.text_renderer.hit_to_grapheme(&display_text, font_size, relative_x)
                                            }
                                        } else {
                                            0
                                        };

                                        if let Some(node) = dom.nodes.get_mut(drag_nid) {
                                            if let Some(is) = &mut node.input_state {
                                                is.selection.active = grapheme_idx;
                                                is.reset_blink();
                                            }
                                        }
                                        needs_redraw = true;
                                    }
                                }
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

                    // button number: 0=left, 1=middle, 2=right (browser spec)
                    let button_num: u8 = match button {
                        winit::event::MouseButton::Left => 0,
                        winit::event::MouseButton::Middle => 1,
                        winit::event::MouseButton::Right => 2,
                        _ => 0,
                    };
                    // buttons bitmask: 1=left, 2=right, 4=middle (browser spec)
                    let button_bit: u8 = match button_num {
                        0 => 1,
                        1 => 4,
                        2 => 2,
                        _ => 0,
                    };

                    // Update button state before building the event
                    match btn_state {
                        ElementState::Pressed => state.mouse_buttons |= button_bit,
                        ElementState::Released => state.mouse_buttons &= !button_bit,
                    }
                    let buttons = state.mouse_buttons;

                    // Collect event data while windows is borrowed
                    let mut mouse_events: Vec<AppEvent> = Vec::new();

                    if let Some(entry) = state.windows.get_mut(&wid) {
                        let dom = &mut entry.dom;
                        if let Some((mx, my)) = dom.hit_state.mouse_position {
                            let x = mx as f32;
                            let y = my as f32;

                            // Resolve topmost hit → NodeId for JS event target
                            let js_target = dom.hit_state.top_hit
                                .and_then(|hid| dom.hitbox_store.get(hid))
                                .map(|hb| hb.node_id);

                            match btn_state {
                                ElementState::Pressed => {
                                    let top = dom.hit_state.top_hit;
                                    dom.set_active(top);
                                    dom.dispatch_mouse_down(mx, my, mouse_button);
                                    if let Some(target) = js_target {
                                        mouse_events.push(AppEvent::MouseDown(MouseEventData {
                                            window_id: wid,
                                            node_id: target,
                                            x, y,
                                            screen_x: x, screen_y: y,
                                            button: button_num,
                                            buttons,
                                        }));
                                    }

                                    // Input focus handling (left button)
                                    if mouse_button == crate::interactivity::MouseButton::Left {
                                        let clicked_is_input = js_target
                                            .and_then(|nid| dom.nodes.get(nid))
                                            .map(|n| matches!(n.kind, crate::element::ElementKind::Input))
                                            .unwrap_or(false);

                                        let old_focus = dom.focused_node;

                                        if clicked_is_input {
                                            let nid = js_target.unwrap();

                                            // Double-click detection
                                            let now = std::time::Instant::now();
                                            let is_double_click = dom.last_click_node == Some(nid)
                                                && dom.last_click_time.map_or(false, |t| now.duration_since(t).as_millis() < 400);
                                            dom.last_click_time = Some(now);
                                            dom.last_click_node = Some(nid);

                                            // Focus if not already focused
                                            if old_focus != Some(nid) {
                                                if let Some(old_id) = old_focus {
                                                    if let Some(old_node) = dom.nodes.get_mut(old_id) {
                                                        if let Some(is) = &mut old_node.input_state {
                                                            is.focused = false;
                                                        }
                                                    }
                                                    mouse_events.push(AppEvent::Blur(FocusEventData {
                                                        window_id: wid,
                                                        node_id: old_id,
                                                    }));
                                                }
                                                dom.focused_node = Some(nid);
                                                if let Some(node) = dom.nodes.get_mut(nid) {
                                                    if let Some(is) = &mut node.input_state {
                                                        is.focused = true;
                                                        is.reset_blink();
                                                    }
                                                }
                                                mouse_events.push(AppEvent::Focus(FocusEventData {
                                                    window_id: wid,
                                                    node_id: nid,
                                                }));
                                            }

                                            // Place cursor at click position
                                            let cursor_info = {
                                                let node = &dom.nodes[nid];
                                                let is = node.input_state.as_ref().unwrap();
                                                let display_text = is.display_text();
                                                let font_size = node.style.text.font_size;
                                                let scroll_offset = is.scroll_offset;
                                                let scroll_offset_y = is.scroll_offset_y;
                                                let is_multiline = is.multiline;
                                                let padding = node.style.padding.left as f64;
                                                let input_padding = if padding > 0.0 { padding } else { 8.0 };
                                                let pad_top = node.style.padding.top;
                                                let top_pad = if pad_top > 0.0 { pad_top } else { 4.0 };
                                                let hitbox_bounds = node.interactivity.hitbox_id
                                                    .and_then(|hid| dom.hitbox_store.get(hid))
                                                    .map(|hb| hb.bounds);
                                                let taffy_node = node.taffy_node;
                                                (display_text, font_size, scroll_offset, scroll_offset_y, is_multiline, input_padding, top_pad, hitbox_bounds, taffy_node)
                                            };
                                            let (display_text, font_size, scroll_offset, scroll_offset_y, is_multiline, input_padding, top_pad, hitbox_bounds, taffy_node) = cursor_info;

                                            if let Some(hb) = hitbox_bounds {
                                                let text_renderer = entry.handle.as_mut().map(|h| &mut h.text_renderer);
                                                if let Some(tr) = text_renderer {
                                                    let grapheme_idx = if !display_text.is_empty() {
                                                        if is_multiline {
                                                            let wrap_width = dom.taffy.layout(taffy_node)
                                                                .map(|l| l.size.width as f32 - input_padding as f32 * 2.0)
                                                                .unwrap_or(200.0);
                                                            let relative_x = (mx - hb.x - input_padding) as f32;
                                                            let relative_y = (my - hb.y) as f32 + scroll_offset_y - top_pad;
                                                            tr.hit_to_grapheme_2d(&display_text, font_size, Some(wrap_width), relative_x, relative_y)
                                                        } else {
                                                            let relative_x = (mx - hb.x - input_padding) as f32 + scroll_offset;
                                                            tr.hit_to_grapheme(&display_text, font_size, relative_x)
                                                        }
                                                    } else {
                                                        0
                                                    };

                                                    if let Some(node) = dom.nodes.get_mut(nid) {
                                                        if let Some(is) = &mut node.input_state {
                                                            if is_double_click {
                                                                let (ws, we) = is.word_at(grapheme_idx);
                                                                is.selection.anchor = ws;
                                                                is.selection.active = we;
                                                            } else {
                                                                is.selection.set_cursor(grapheme_idx);
                                                            }
                                                            is.reset_blink();
                                                        }
                                                    }
                                                }
                                            }

                                            dom.dragging_input = Some(nid);
                                        } else {
                                            // Clicked non-input: blur focused
                                            if let Some(old_id) = old_focus {
                                                if let Some(old_node) = dom.nodes.get_mut(old_id) {
                                                    if let Some(is) = &mut old_node.input_state {
                                                        is.focused = false;
                                                    }
                                                }
                                                dom.focused_node = None;
                                                mouse_events.push(AppEvent::Blur(FocusEventData {
                                                    window_id: wid,
                                                    node_id: old_id,
                                                }));
                                            }
                                        }
                                    }

                                    needs_redraw = true;
                                }
                                ElementState::Released => {
                                    dom.dispatch_mouse_up(mx, my, mouse_button);
                                    if let Some(target) = js_target {
                                        mouse_events.push(AppEvent::MouseUp(MouseEventData {
                                            window_id: wid,
                                            node_id: target,
                                            x, y,
                                            screen_x: x, screen_y: y,
                                            button: button_num,
                                            buttons,
                                        }));
                                    }
                                    // Click fires if released on the same element that was pressed
                                    if let Some(active) = dom.hit_state.active_hitbox {
                                        if dom.hit_state.is_hovered(active) {
                                            dom.dispatch_click(mx, my, mouse_button);
                                            if let Some(target) = js_target {
                                                mouse_events.push(AppEvent::Click(MouseEventData {
                                                    window_id: wid,
                                                    node_id: target,
                                                    x, y,
                                                    screen_x: x, screen_y: y,
                                                    button: button_num,
                                                    buttons,
                                                }));
                                            }
                                        }
                                    }
                                    dom.set_active(None);
                                    dom.dragging_input = None;
                                    needs_redraw = true;
                                }
                            }
                        }
                    }

                    state.pending_events.extend(mouse_events);
                }
                WindowEvent::KeyboardInput {
                    event: key_event, ..
                } => {
                    use winit::event::ElementState;
                    use winit::keyboard::{Key, NamedKey, PhysicalKey};

                    // F5 → hot reload (keep existing behavior)
                    if key_event.state == ElementState::Pressed
                        && key_event.logical_key == Key::Named(NamedKey::F5)
                    {
                        state.pending_events.push(AppEvent::HotReload);
                        return;
                    }

                    // Route keyboard to focused input if present
                    let mut handled_by_input = false;
                    if key_event.state == ElementState::Pressed {
                        let modifiers = state.modifiers;
                        if let Some(entry) = state.windows.get_mut(&wid) {
                            if let Some(focused_id) = entry.dom.focused_node {
                                if let Some(node) = entry.dom.nodes.get_mut(focused_id) {
                                    if let Some(input_state) = &mut node.input_state {
                                        use crate::input::KeyResult;
                                        let result = input_state.handle_key(
                                            &key_event.logical_key,
                                            modifiers,
                                        );
                                        match result {
                                            KeyResult::Edit(edit) => {
                                                let value = input_state.text.clone();
                                                state.pending_events.push(AppEvent::Input(
                                                    InputEventData {
                                                        window_id: wid,
                                                        node_id: focused_id,
                                                        value,
                                                        input_type: edit.input_type.to_string(),
                                                        data: edit.data,
                                                    },
                                                ));
                                                needs_redraw = true;
                                                handled_by_input = true;
                                            }
                                            KeyResult::Blur => {
                                                input_state.focused = false;
                                                entry.dom.focused_node = None;
                                                state.pending_events.push(AppEvent::Blur(
                                                    FocusEventData {
                                                        window_id: wid,
                                                        node_id: focused_id,
                                                    },
                                                ));
                                                needs_redraw = true;
                                                handled_by_input = true;
                                            }
                                            KeyResult::Handled => {
                                                needs_redraw = true;
                                                handled_by_input = true;
                                            }
                                            KeyResult::VerticalNav { direction, extend } => {
                                                // Gather info needed for vertical navigation
                                                let display_text = input_state.display_text();
                                                let font_size = node.style.text.font_size;
                                                let cursor_pos = input_state.selection.active;
                                                let saved_sticky_x = input_state.sticky_x;
                                                let padding = node.style.padding.left;
                                                let input_padding = if padding > 0.0 { padding } else { 8.0 };
                                                let taffy_node = node.taffy_node;

                                                let wrap_width = entry.dom.taffy.layout(taffy_node)
                                                    .map(|l| l.size.width as f32 - input_padding * 2.0)
                                                    .unwrap_or(200.0);

                                                if let Some(handle) = &mut entry.handle {
                                                    let positions = handle.text_renderer.grapheme_positions_2d(
                                                        &display_text,
                                                        font_size,
                                                        Some(wrap_width),
                                                    );

                                                    let line_height = (font_size * 1.2).round();
                                                    let cur_pos = if cursor_pos < positions.len() {
                                                        positions[cursor_pos]
                                                    } else {
                                                        *positions.last().unwrap()
                                                    };

                                                    // Collect unique visual line y-values
                                                    let mut line_ys: Vec<f32> = Vec::new();
                                                    for pos in &positions {
                                                        if line_ys.last().map_or(true, |&last| (pos.y - last).abs() > 1.0) {
                                                            line_ys.push(pos.y);
                                                        }
                                                    }

                                                    // Find which visual line the cursor is on
                                                    let cur_line_idx = line_ys.iter()
                                                        .rposition(|&ly| cur_pos.y >= ly - 0.5)
                                                        .unwrap_or(0);

                                                    // Compute target line index
                                                    let target_line_idx = if direction < 0 {
                                                        if cur_line_idx == 0 { None } else { Some(cur_line_idx - 1) }
                                                    } else {
                                                        if cur_line_idx >= line_ys.len() - 1 { None } else { Some(cur_line_idx + 1) }
                                                    };

                                                    let target_idx = if let Some(tl_idx) = target_line_idx {
                                                        // Navigate to adjacent line: find closest x position
                                                        let target_x = saved_sticky_x.unwrap_or(cur_pos.x);
                                                        let target_line_y = line_ys[tl_idx];
                                                        let mut best_idx = 0;
                                                        let mut best_dist = f32::MAX;
                                                        for (i, pos) in positions.iter().enumerate() {
                                                            if (pos.y - target_line_y).abs() < line_height * 0.5 {
                                                                let dist = (pos.x - target_x).abs();
                                                                if dist < best_dist {
                                                                    best_dist = dist;
                                                                    best_idx = i;
                                                                }
                                                            }
                                                        }
                                                        // Preserve sticky_x
                                                        let sticky = saved_sticky_x.unwrap_or(cur_pos.x);
                                                        if let Some(node) = entry.dom.nodes.get_mut(focused_id) {
                                                            if let Some(is) = &mut node.input_state {
                                                                is.move_to(best_idx, extend);
                                                                is.sticky_x = Some(sticky);
                                                            }
                                                        }
                                                        best_idx
                                                    } else {
                                                        // Already on first/last line: go to start/end like browsers
                                                        let snap_idx = if direction < 0 { 0 } else { positions.len() - 1 };
                                                        if let Some(node) = entry.dom.nodes.get_mut(focused_id) {
                                                            if let Some(is) = &mut node.input_state {
                                                                is.move_to(snap_idx, extend);
                                                                // Don't preserve sticky_x when snapping to start/end
                                                            }
                                                        }
                                                        snap_idx
                                                    };

                                                    // Update scroll
                                                    let new_cursor_y = if target_idx < positions.len() {
                                                        positions[target_idx].y
                                                    } else {
                                                        positions.last().map(|p| p.y).unwrap_or(0.0)
                                                    };
                                                    let input_height = entry.dom.taffy.layout(taffy_node)
                                                        .map(|l| l.size.height as f32)
                                                        .unwrap_or(100.0);
                                                    let top_pad = {
                                                        let pt = entry.dom.nodes.get(focused_id)
                                                            .map(|n| n.style.padding.top)
                                                            .unwrap_or(0.0);
                                                        if pt > 0.0 { pt } else { 4.0 }
                                                    };
                                                    if let Some(node) = entry.dom.nodes.get_mut(focused_id) {
                                                        if let Some(is) = &mut node.input_state {
                                                            is.update_scroll_y(new_cursor_y, line_height, input_height - top_pad * 2.0);
                                                        }
                                                    }
                                                }
                                                needs_redraw = true;
                                                handled_by_input = true;
                                            }
                                            KeyResult::Ignored => {}
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if handled_by_input {
                        return;
                    }

                    let key_str = match &key_event.logical_key {
                        Key::Character(c) => c.to_string(),
                        Key::Named(named) => format!("{:?}", named),
                        _ => return,
                    };

                    let code_str = match key_event.physical_key {
                        PhysicalKey::Code(kc) => format!("{:?}", kc),
                        _ => String::new(),
                    };

                    let data = KeyEventData {
                        window_id: wid,
                        key: key_str,
                        code: code_str,
                        key_code: 0, // legacy field, not mapped
                        modifiers: state.modifiers,
                        repeat: key_event.repeat,
                    };

                    match key_event.state {
                        ElementState::Pressed => {
                            state.pending_events.push(AppEvent::KeyDown(data));
                        }
                        ElementState::Released => {
                            state.pending_events.push(AppEvent::KeyUp(data));
                        }
                    }
                }
                WindowEvent::ModifiersChanged(mods) => {
                    let m = mods.state();
                    let mut bits: u32 = 0;
                    if m.control_key() { bits |= 1; }
                    if m.alt_key() { bits |= 2; }
                    if m.shift_key() { bits |= 4; }
                    if m.super_key() { bits |= 8; }
                    state.modifiers = bits;
                }
                WindowEvent::Focused(focused) => {
                    if let Some(entry) = state.windows.get_mut(&wid) {
                        entry.dom.window_focused = focused;
                        if focused {
                            // Reset blink on refocus
                            if let Some(nid) = entry.dom.focused_node {
                                if let Some(node) = entry.dom.nodes.get_mut(nid) {
                                    if let Some(is) = &mut node.input_state {
                                        is.reset_blink();
                                    }
                                }
                            }
                        }
                        needs_redraw = true;
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
