use deno_core::*;

use crate::app::{SharedAppState, with_state};
use crate::cursor;
use crate::element::{self, ElementTree, NodeId};
use crate::prop_keys::PropKey;
use crate::style::*;

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
