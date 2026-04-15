pub mod runtime;

pub mod app;
pub mod clipboard;
pub mod cursor;
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

use deno_core::*;
use winit::event_loop::EventLoopProxy;

use crate::app::{SharedAppState, UserEvent, WindowEntry, WindowEntryId, with_state};
use crate::element::{ElementTree, NodeId};
use crate::prop_keys::PropKey;
use crate::selection::{DomSelection, SelectionRange};
use crate::style::*;

pub use crate::app::Application;

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

#[op2(fast)]
pub fn op_set_string_prop(
    state: &mut OpState,
    #[smi] window_id: u32,
    #[smi] node_id: u32,
    #[smi] prop: u32,
    #[string] value: &str,
) {
    let nid = node_id as NodeId;
    let Ok(prop) = PropKey::try_from(prop) else {
        return;
    };
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let entry = s.windows.get_mut(&window_id).expect("window not found");
        let Some(node) = entry.dom.nodes.get_mut(nid) else {
            return;
        };

        #[allow(clippy::single_match)]
        match prop {
            PropKey::Cursor => {
                node.style.cursor = cursor::CursorIcon::parse(value);
                if let Some(handle) = entry.handle.as_mut()
                    && let Some(top) = entry.dom.hit_state.top_node
                {
                    let icon = entry.dom.resolve_cursor(top);
                    handle.set_cursor(icon);
                }
            }
            _ => {}
        }
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
    op_set_string_prop,
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
