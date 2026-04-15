pub mod runtime;

pub mod clipboard;
pub mod element;
pub mod elements;
pub mod event_dispatch;
pub mod geometry;
pub mod gpu;
pub mod input;
pub mod interactivity;
pub mod selection;
pub mod style;
pub mod text;
pub mod text_buffer;
pub mod text_model;
pub mod window;

use runtime::module_loader::{UzCjsCodeAnalyzer, UzRequireLoader};
use runtime::resolver::UzCjsTracker;
use runtime::sys::UzSys;

use anyhow::Result;
use deno_core::*;
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
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use winit::event_loop::EventLoopProxy;
use winit::{application::ApplicationHandler, event::WindowEvent, window::WindowId};

use crate::element::{ElementTree, NodeId};
use crate::gpu::GpuContext;
use crate::prop_keys::PropKey;
use crate::selection::{DomSelection, SelectionRange};
use crate::style::*;

mod prop_keys {
    include!(concat!(env!("OUT_DIR"), "/prop_keys.rs"));
}

pub use deno_core;
pub use deno_runtime;
pub use rustls;

pub static TS_VERSION: &str = "5.9.2";

#[cfg(feature = "snapshot")]
pub fn create_snapshot(
    snapshot_path: std::path::PathBuf,
    snapshot_options: deno_runtime::ops::bootstrap::SnapshotOptions,
) {
    deno_runtime::snapshot::create_runtime_snapshot(
        snapshot_path,
        snapshot_options,
        vec![uzumaki::init()],
    );
}

pub struct WindowEntry {
    pub dom: ElementTree,
    pub handle: Option<window::Window>,
    pub rem_base: f32,
}

type WindowEntryId = u32;

pub struct AppState {
    pub gpu: GpuContext,
    pub windows: HashMap<WindowEntryId, WindowEntry>,
    pub winit_id_to_entry_id: HashMap<WindowId, WindowEntryId>,
    pub mouse_buttons: u8,
    pub modifiers: u32,
    pub clipboard: RefCell<clipboard::SystemClipboard>,
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

type SharedAppState = Rc<RefCell<AppState>>;

fn with_state<R>(state: &SharedAppState, f: impl FnOnce(&mut AppState) -> R) -> R {
    f(&mut state.borrow_mut())
}

#[derive(Debug, Clone)]
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct CreateWindowOptions {
    width: u32,
    height: u32,
    title: String,
}

#[op2]
#[serde]
pub fn op_create_window(
    state: &mut OpState,
    #[serde] options: CreateWindowOptions,
) -> Result<WindowEntryId, deno_error::JsErrorBox> {
    static NEXT_WINDOW_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
    let id = NEXT_WINDOW_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let mut dom = ElementTree::new();
        let root = dom.create_view(Style {
            display: Display::Flex,
            size: Size {
                width: Length::Percent(1.0),
                height: Length::Percent(1.0),
            },
            ..Default::default()
        });
        dom.set_root(root);

        s.windows.insert(
            id,
            WindowEntry {
                dom,
                handle: None,
                rem_base: 16.0,
            },
        );
    });

    let proxy = state.borrow::<EventLoopProxy<UserEvent>>();
    proxy
        .send_event(UserEvent::CreateWindow {
            id,
            width: options.width,
            height: options.height,
            title: options.title,
        })
        .map_err(|_| {
            deno_error::JsErrorBox::new(
                "UzumakiInternalError",
                "cannot create window after application free",
            )
        })?;

    Ok(id)
}

#[op2(fast)]
pub fn op_request_quit(state: &mut OpState) -> Result<(), deno_error::JsErrorBox> {
    let proxy = state.borrow::<EventLoopProxy<UserEvent>>();
    proxy
        .send_event(UserEvent::Quit)
        .map_err(|_| deno_error::JsErrorBox::new("UzumakiInternalError", "error quitting"))?;
    Ok(())
}

#[op2(fast)]
pub fn op_request_redraw(
    state: &mut OpState,
    #[smi] window_id: u32,
) -> Result<(), deno_error::JsErrorBox> {
    let proxy = state.borrow::<EventLoopProxy<UserEvent>>();
    proxy
        .send_event(UserEvent::RequestRedraw { id: window_id })
        .map_err(|_| {
            deno_error::JsErrorBox::new("UzumakiInternalError", "error requesting redraw")
        })?;
    Ok(())
}

#[op2(fast)]
pub fn op_get_root_node_id(state: &mut OpState, #[smi] window_id: u32) -> u32 {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get(&window_id).expect("window not found");
        entry.dom.root.expect("no root node") as u32
    })
}

#[op2(fast)]
pub fn op_create_element(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[string] element_type: String,
) -> u32 {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        let id = if element_type == "input" {
            entry.dom.create_input(Style::default())
        } else {
            entry.dom.create_view(Style::default())
        };
        id as u32
    })
}

#[op2(fast)]
pub fn op_create_text_node(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[string] text: String,
) -> u32 {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        entry.dom.create_text(text, Style::default()) as u32
    })
}

#[op2(fast)]
pub fn op_append_child(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] parent_id: u32,
    #[smi] child_id: u32,
) {
    let pid = parent_id as NodeId;
    let cid = child_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        entry.dom.append_child(pid, cid);
    });
}

#[op2(fast)]
pub fn op_insert_before(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] parent_id: u32,
    #[smi] child_id: u32,
    #[smi] before_id: u32,
) {
    let pid = parent_id as NodeId;
    let cid = child_id as NodeId;
    let bid = before_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        entry.dom.insert_before(pid, cid, bid);
    });
}

#[op2(fast)]
pub fn op_remove_child(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] parent_id: u32,
    #[smi] child_id: u32,
) {
    let pid = parent_id as NodeId;
    let cid = child_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        entry.dom.remove_child(pid, cid);
    });
}

#[op2(fast)]
pub fn op_set_text(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[string] text: String,
) {
    let nid = node_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        entry.dom.set_text_content(nid, text);
    });
}

#[op2(fast)]
pub fn op_reset_dom(state: &mut OpState, #[smi] window_id: u32) {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        if let Some(entry) = s.windows.get_mut(&window_id) {
            let root = entry.dom.root.expect("no root node");
            entry.dom.clear_children(root);
        }
    });
}

#[op2(fast)]
pub fn op_set_length_prop(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[smi] prop: u32,
    value: f64,
    #[smi] unit: u32,
) {
    let nid = node_id as NodeId;
    let Ok(prop) = PropKey::try_from(prop) else {
        return;
    };
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        let length = match unit {
            0 => Length::Px(value as f32),
            1 => Length::Percent(value as f32),
            2 => Length::Px(value as f32 * entry.rem_base),
            _ => Length::Auto,
        };
        {
            let style = &mut entry.dom.nodes[nid].style;
            match prop {
                PropKey::W => style.size.width = length,
                PropKey::H => style.size.height = length,
                PropKey::MinW => style.min_size.width = length,
                PropKey::MinH => style.min_size.height = length,
                _ => return,
            }
        }
        sync_taffy(&mut entry.dom, nid);
    });
}

#[op2(fast)]
pub fn op_set_color_prop(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[smi] prop: u32,
    #[smi] r: u32,
    #[smi] g: u32,
    #[smi] b: u32,
    #[smi] a: u32,
) {
    let nid = node_id as NodeId;
    let Ok(prop) = PropKey::try_from(prop) else {
        return;
    };
    let color = Color::rgba(r as u8, g as u8, b as u8, a as u8);
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");

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
            let style = &mut entry.dom.nodes[nid].style;
            match prop {
                PropKey::Bg => style.background = Some(color),
                PropKey::Color => style.text.color = color,
                PropKey::BorderColor => style.border_color = Some(color),
                _ => return,
            }
        }
        sync_taffy(&mut entry.dom, nid);
    });
}

#[op2(fast)]
pub fn op_set_f32_prop(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[smi] prop: u32,
    value: f64,
) {
    let nid = node_id as NodeId;
    let Ok(prop) = PropKey::try_from(prop) else {
        return;
    };
    let v = value as f32;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");

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
            PropKey::Scrollable => {
                let node = &mut entry.dom.nodes[nid];
                if v > 0.5 {
                    node.style.overflow_y = Overflow::Scroll;
                    if node.scroll_state.is_none() {
                        node.scroll_state = Some(element::ScrollState::new());
                    }
                } else {
                    node.style.overflow_y = Overflow::Visible;
                    node.scroll_state = None;
                }
                sync_taffy(&mut entry.dom, nid);
                return;
            }
            PropKey::TextSelect => {
                entry.dom.nodes[nid].selectable = Some(v > 0.5);
                return;
            }
            _ => {}
        }

        {
            let style = &mut entry.dom.nodes[nid].style;
            match prop {
                PropKey::P => style.padding = Edges::all(v),
                PropKey::Px => {
                    style.padding.left = v;
                    style.padding.right = v;
                }
                PropKey::Py => {
                    style.padding.top = v;
                    style.padding.bottom = v;
                }
                PropKey::Pt => style.padding.top = v,
                PropKey::Pb => style.padding.bottom = v,
                PropKey::Pl => style.padding.left = v,
                PropKey::Pr => style.padding.right = v,
                PropKey::M => style.margin = Edges::all(v),
                PropKey::Mx => {
                    style.margin.left = v;
                    style.margin.right = v;
                }
                PropKey::My => {
                    style.margin.top = v;
                    style.margin.bottom = v;
                }
                PropKey::Mt => style.margin.top = v,
                PropKey::Mb => style.margin.bottom = v,
                PropKey::Ml => style.margin.left = v,
                PropKey::Mr => style.margin.right = v,
                PropKey::Flex => {
                    style.display = Display::Flex;
                    style.flex_grow = v;
                }
                PropKey::FlexGrow => style.flex_grow = v,
                PropKey::FlexShrink => style.flex_shrink = v,
                PropKey::Gap => {
                    style.gap = GapSize {
                        width: DefiniteLength::Px(v),
                        height: DefiniteLength::Px(v),
                    };
                }
                PropKey::FontSize => style.text.font_size = v,
                PropKey::FontWeight => {}
                PropKey::Rounded => style.corner_radii = Corners::uniform(v),
                PropKey::RoundedTL => style.corner_radii.top_left = v,
                PropKey::RoundedTR => style.corner_radii.top_right = v,
                PropKey::RoundedBR => style.corner_radii.bottom_right = v,
                PropKey::RoundedBL => style.corner_radii.bottom_left = v,
                PropKey::Border => style.border_widths = Edges::all(v),
                PropKey::BorderTop => style.border_widths.top = v,
                PropKey::BorderRight => style.border_widths.right = v,
                PropKey::BorderBottom => style.border_widths.bottom = v,
                PropKey::BorderLeft => style.border_widths.left = v,
                PropKey::Opacity => style.opacity = v,
                PropKey::Visible => {
                    style.visibility = if v > 0.5 {
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

#[op2(fast)]
pub fn op_set_enum_prop(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[smi] prop: u32,
    #[smi] value: i32,
) {
    let nid = node_id as NodeId;
    let Ok(prop) = PropKey::try_from(prop) else {
        return;
    };
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        {
            let style = &mut entry.dom.nodes[nid].style;
            match prop {
                PropKey::FlexDir => {
                    style.flex_direction = match value {
                        0 => FlexDirection::Row,
                        1 => FlexDirection::Column,
                        2 => FlexDirection::RowReverse,
                        3 => FlexDirection::ColumnReverse,
                        _ => FlexDirection::Row,
                    };
                }
                PropKey::Items => {
                    style.align_items = Some(match value {
                        0 => AlignItems::FlexStart,
                        1 => AlignItems::FlexEnd,
                        2 => AlignItems::Center,
                        3 => AlignItems::Stretch,
                        4 => AlignItems::Baseline,
                        _ => AlignItems::Stretch,
                    });
                }
                PropKey::Justify => {
                    style.justify_content = Some(match value {
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
                    style.display = match value {
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

// ── Input attribute ops ─────────────────────────────────────────────

#[op2(fast)]
pub fn op_set_input_value(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[string] value: String,
) {
    let nid = node_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        if let Some(node) = entry.dom.nodes.get_mut(nid)
            && let Some(is) = node.behavior.as_input_mut()
        {
            is.set_value(value);
        }
    });
}

#[op2]
#[string]
pub fn op_get_input_value(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
) -> String {
    let nid = node_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get(&window_id).expect("window not found");
        entry
            .dom
            .nodes
            .get(nid)
            .and_then(|node| node.behavior.as_input())
            .map(|is| is.model.text())
            .unwrap_or_default()
    })
}

#[op2(fast)]
pub fn op_set_input_placeholder(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[string] placeholder: String,
) {
    let nid = node_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        if let Some(node) = entry.dom.nodes.get_mut(nid)
            && let Some(is) = node.behavior.as_input_mut()
        {
            is.placeholder = placeholder;
        }
    });
}

#[op2(fast)]
pub fn op_set_input_disabled(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    disabled: bool,
) {
    let nid = node_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        if let Some(node) = entry.dom.nodes.get_mut(nid)
            && let Some(is) = node.behavior.as_input_mut()
        {
            is.disabled = disabled;
        }
    });
}

#[op2(fast)]
pub fn op_set_input_max_length(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[smi] max_length: i32,
) {
    let nid = node_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        if let Some(node) = entry.dom.nodes.get_mut(nid)
            && let Some(is) = node.behavior.as_input_mut()
        {
            is.model.max_length = if max_length > 0 {
                Some(max_length as usize)
            } else {
                None
            };
        }
    });
}

#[op2(fast)]
pub fn op_set_input_multiline(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    multiline: bool,
) {
    let nid = node_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        if let Some(node) = entry.dom.nodes.get_mut(nid)
            && let Some(is) = node.behavior.as_input_mut()
        {
            is.multiline = multiline;
        }
    });
}

#[op2(fast)]
pub fn op_set_input_secure(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    secure: bool,
) {
    let nid = node_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        if let Some(node) = entry.dom.nodes.get_mut(nid)
            && let Some(is) = node.behavior.as_input_mut()
        {
            is.secure = secure;
        }
    });
}

#[op2(fast)]
pub fn op_focus_input(state: &mut OpState, #[smi] window_id: u32, #[smi] node_id: u32) {
    let nid = node_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        entry.dom.set_selection(DomSelection {
            root: nid,
            range: SelectionRange::default(),
        });
    });
}

#[op2(fast)]
pub fn op_set_rem_base(state: &mut OpState, #[smi] window_id: u32, value: f64) {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        if let Some(entry) = s.windows.get_mut(&window_id) {
            entry.rem_base = value as f32;
        }
    });
}

#[op2]
pub fn op_get_window_width(state: &mut OpState, #[smi] window_id: u32) -> Option<u32> {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        s.windows.get(&window_id).and_then(|entry| {
            entry.handle.as_ref().map(|h| {
                let size = h.winit_window.inner_size();
                size.width
            })
        })
    })
}

#[op2]
pub fn op_get_window_height(state: &mut OpState, #[smi] window_id: u32) -> Option<u32> {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        s.windows.get(&window_id).and_then(|entry| {
            entry.handle.as_ref().map(|h| {
                let size = h.winit_window.inner_size();
                size.height
            })
        })
    })
}

#[op2]
#[string]
pub fn op_get_window_title(state: &mut OpState, #[smi] window_id: u32) -> Option<String> {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        s.windows
            .get(&window_id)
            .and_then(|entry| entry.handle.as_ref().map(|h| h.winit_window.title()))
    })
}

#[op2]
#[serde]
pub fn op_get_ancestor_path(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
) -> Vec<u32> {
    let nid = node_id as NodeId;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get(&window_id).expect("window not found");
        let mut path = Vec::new();
        let mut current = Some(nid);
        while let Some(id) = current {
            path.push(id as u32);
            current = entry.dom.nodes.get(id).and_then(|n| n.parent);
        }
        path
    })
}

// ── Selection query ops ──────────────────────────────────────────────

#[op2]
#[serde]
pub fn op_get_selection(state: &mut OpState, #[smi] window_id: u32) -> serde_json::Value {
    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct SelectionState {
        root_node_id: u32,
        anchor_offset: usize,
        active_offset: usize,
        start: usize,
        end: usize,
        run_length: usize,
        is_collapsed: bool,
        text: String,
    }

    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get(&window_id).expect("window not found");
        let dom = &entry.dom;
        let Some(sel) = dom.selection() else {
            return serde_json::Value::Null;
        };
        let run_length = dom.selection_run_length().unwrap_or(0);
        let text = dom.selected_text();
        serde_json::to_value(SelectionState {
            root_node_id: sel.root as u32,
            anchor_offset: sel.anchor(),
            active_offset: sel.active(),
            start: sel.start(),
            end: sel.end(),
            run_length,
            is_collapsed: sel.is_collapsed(),
            text,
        })
        .unwrap()
    })
}

#[op2]
#[string]
pub fn op_get_selected_text(state: &mut OpState, #[smi] window_id: u32) -> String {
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get(&window_id).expect("window not found");
        entry.dom.selected_text()
    })
}

#[op2]
#[string]
pub fn op_read_clipboard_text(state: &mut OpState) -> Option<String> {
    let app_state = state.borrow::<SharedAppState>().clone();
    let s = app_state.borrow();
    match s.clipboard.borrow_mut().read_text() {
        Ok(text) => text,
        Err(e) => {
            eprintln!("[uzumaki] clipboard read error: {e}");
            None
        }
    }
}

#[op2(fast)]
pub fn op_write_clipboard_text(state: &mut OpState, #[string] text: String) -> bool {
    let app_state = state.borrow::<SharedAppState>().clone();
    let s = app_state.borrow();
    match s.clipboard.borrow_mut().write_text(&text) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("[uzumaki] clipboard write error: {e}");
            false
        }
    }
}

fn sync_taffy(dom: &mut ElementTree, node_id: NodeId) {
    let node = &dom.nodes[node_id];
    let taffy_style = node.style.to_taffy();
    let tn = node.taffy_node;
    dom.taffy.set_style(tn, taffy_style).unwrap();

    let font_size = node.style.text.font_size;
    if let Some(ctx) = dom.taffy.get_node_context_mut(tn) {
        ctx.font_size = font_size;
    }
}

extension!(
  uzumaki,
  ops = [
    op_create_window,
    op_request_quit,
    op_request_redraw,
    op_get_root_node_id,
    op_create_element,
    op_create_text_node,
    op_append_child,
    op_insert_before,
    op_remove_child,
    op_set_text,
    op_reset_dom,
    op_set_length_prop,
    op_set_color_prop,
    op_set_f32_prop,
    op_set_enum_prop,
    op_set_input_value,
    op_get_input_value,
    op_set_input_placeholder,
    op_set_input_disabled,
    op_set_input_max_length,
    op_set_input_multiline,
    op_set_input_secure,
    op_focus_input,
    op_set_rem_base,
    op_get_window_width,
    op_get_window_height,
    op_get_window_title,
    op_get_ancestor_path,
    op_get_selection,
    op_get_selected_text,
    op_read_clipboard_text,
    op_write_clipboard_text,
  ],
  esm_entry_point = "ext:uzumaki/runtime.js",
  esm = [ dir "core", "runtime.js" ],
);

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
}

impl Application {
    pub fn new_with_root(
        main_file: impl Into<PathBuf>,
        app_root: impl Into<PathBuf>,
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
            extensions: vec![uzumaki::init()],
            startup_snapshot,
            skip_op_registration: false,
            bootstrap: BootstrapOptions {
                args: vec![],
                mode: deno_runtime::WorkerExecutionMode::None,
                ..Default::default()
            },
            ..Default::default()
        };

        let worker = MainWorker::bootstrap_from_options(&main_module, services, options);

        let event_loop: winit::event_loop::EventLoop<UserEvent> =
            winit::event_loop::EventLoop::with_user_event().build()?;

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
        }));

        // Put shared state and event loop proxy into OpState
        {
            let op_state = worker.js_runtime.op_state();
            let mut borrow = op_state.borrow_mut();
            borrow.put(app_state.clone());
            borrow.put(event_loop.create_proxy());
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

    fn tick_js(&mut self) {
        let rt = self.tokio_runtime.as_ref().unwrap();
        rt.block_on(async {
            tokio::select! {
                biased;
                result = self.worker.run_event_loop(false) => {
                    if let Err(e) = result {
                        eprintln!("JS error: {e}");
                    }
                }
                _ = tokio::task::yield_now() => {}
            }
        });
    }

    fn load_main_module(&mut self) {
        let specifier =
            deno_core::resolve_path(self.main_file.to_str().unwrap(), &self.app_root).unwrap();

        let rt = self.tokio_runtime.as_ref().unwrap();
        rt.block_on(async {
            self.worker.execute_main_module(&specifier).await.unwrap();
        });
        self.tick_js();
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
}

impl ApplicationHandler<UserEvent> for Application {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if !self.module_loaded {
            self.module_loaded = true;
            self.load_main_module();
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.tick_js();
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

                // Emit window load event after handle is ready
                self.dispatch_event_to_js(&event_dispatch::AppEvent::WindowLoad(
                    event_dispatch::WindowLoadEventData { window_id: id },
                ));
            }
            UserEvent::RequestRedraw { id } => {
                let state = self.app_state.borrow();
                if let Some(entry) = state.windows.get(&id)
                    && let Some(ref handle) = entry.handle
                {
                    handle.winit_window.request_redraw();
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
                        // HotReload is already dispatched, nothing more to do
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
                                        event_dispatch::apply_clipboard_command(
                                            cmd,
                                            &mut entry.dom,
                                            wid,
                                            &mut cb,
                                        )
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
                                    if redraw {
                                        needs_redraw = true;
                                    }
                                    events
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
                        && let Some(is) = node.behavior.as_input_mut()
                    {
                        is.reset_blink();
                    }
                    needs_redraw = true;
                }
            }
            WindowEvent::Ime(_ime) => {
                // todo (aadi): do this next
                // println!("IME EVENT: {:#?}", ime);
            }
            WindowEvent::CursorLeft { .. } => {
                let mut state = self.app_state.borrow_mut();
                if let Some(entry) = state.windows.get_mut(&wid) {
                    entry.dom.hit_state = Default::default();
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
                    && event_dispatch::handle_mouse_wheel(&mut entry.dom, scroll_delta_y)
                {
                    needs_redraw = true;
                }
            }
            WindowEvent::CloseRequested => {
                let mut state = self.app_state.borrow_mut();
                state.winit_id_to_entry_id.remove(&window_id);
                state.windows.remove(&wid);
                if state.windows.is_empty() {
                    event_loop.exit();
                    return;
                }
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
    }
}
