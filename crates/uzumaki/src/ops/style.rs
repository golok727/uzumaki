use deno_core::*;

use crate::app::{SharedAppState, with_state};
use crate::cursor;
use crate::element::{self, Node, UzNodeId};
use crate::prop_keys::PropKey;
use crate::style::*;
use crate::ui::UIState;

/// Outcome of applying a style prop to a node.
enum StyleEffect {
    /// Prop key didn't match — nothing changed.
    Ignored,
    /// Applied, but layout-independent (no taffy sync needed).
    Applied,
    /// Applied and layout changed — caller must sync taffy.
    AppliedNeedsSync,
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
    let nid = node_id as UzNodeId;
    let Ok(prop) = PropKey::try_from(prop) else {
        return;
    };
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return;
        };
        let length = match unit {
            0 => Length::Px(value as f32),
            1 => Length::Percent(value as f32),
            2 => Length::Px(value as f32 * entry.rem_base),
            _ => Length::Auto,
        };
        let Some(node) = entry.dom.nodes.get_mut(nid) else {
            return;
        };
        if set_length_style_prop(&mut node.style, prop, length) {
            sync_taffy(&mut entry.dom, nid);
        }
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
    let nid = node_id as UzNodeId;
    let Ok(prop) = PropKey::try_from(prop) else {
        return;
    };
    let color = Color::rgba(r as u8, g as u8, b as u8, a as u8);
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return;
        };
        let Some(node) = entry.dom.nodes.get_mut(nid) else {
            return;
        };
        if matches!(
            set_color_style_prop(node, prop, color),
            StyleEffect::AppliedNeedsSync
        ) {
            sync_taffy(&mut entry.dom, nid);
        }
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
    let nid = node_id as UzNodeId;
    let Ok(prop) = PropKey::try_from(prop) else {
        return;
    };
    let v = value as f32;
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return;
        };
        let Some(node) = entry.dom.nodes.get_mut(nid) else {
            return;
        };
        if matches!(
            set_f32_style_prop(node, prop, v),
            StyleEffect::AppliedNeedsSync
        ) {
            sync_taffy(&mut entry.dom, nid);
        }
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
    let nid = node_id as UzNodeId;
    let Ok(prop) = PropKey::try_from(prop) else {
        return;
    };
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return;
        };
        let Some(node) = entry.dom.nodes.get_mut(nid) else {
            return;
        };
        if set_enum_style_prop(&mut node.style, prop, value) {
            sync_taffy(&mut entry.dom, nid);
        }
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
    let nid = node_id as UzNodeId;
    let Ok(prop) = PropKey::try_from(prop) else {
        return;
    };
    let app_state = state.borrow::<SharedAppState>().clone();
    with_state(&app_state, |s| {
        let Some(entry) = s.windows.get_mut(&window_id) else {
            return;
        };
        let Some(node) = entry.dom.nodes.get_mut(nid) else {
            return;
        };

        #[allow(clippy::single_match)]
        match prop {
            PropKey::Cursor => {
                node.style.cursor = cursor::UzCursorIcon::parse(value);
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

// ── Helpers ─────────────────────────────────────────────────────────

fn set_length_style_prop(style: &mut UzStyle, prop: PropKey, length: Length) -> bool {
    match prop {
        PropKey::W => style.size.width = length,
        PropKey::H => style.size.height = length,
        PropKey::MinW => style.min_size.width = length,
        PropKey::MinH => style.min_size.height = length,
        _ => return false,
    }
    true
}

fn set_color_style_prop(node: &mut Node, prop: PropKey, color: Color) -> StyleEffect {
    match prop {
        PropKey::HoverBg | PropKey::HoverColor | PropKey::HoverBorderColor => {
            let r = node
                .interactivity
                .hover_style
                .get_or_insert_with(|| Box::new(UzStyleRefinement::default()));
            match prop {
                PropKey::HoverBg => r.background = Some(color),
                PropKey::HoverColor => r.text.color = Some(color),
                PropKey::HoverBorderColor => r.border_color = Some(color),
                _ => unreachable!(),
            }
            StyleEffect::Applied
        }
        PropKey::ActiveBg | PropKey::ActiveColor | PropKey::ActiveBorderColor => {
            let r = node
                .interactivity
                .active_style
                .get_or_insert_with(|| Box::new(UzStyleRefinement::default()));
            match prop {
                PropKey::ActiveBg => r.background = Some(color),
                PropKey::ActiveColor => r.text.color = Some(color),
                PropKey::ActiveBorderColor => r.border_color = Some(color),
                _ => unreachable!(),
            }
            StyleEffect::Applied
        }
        PropKey::Bg => {
            node.style.background = Some(color);
            StyleEffect::AppliedNeedsSync
        }
        PropKey::Color => {
            node.style.text.color = color;
            StyleEffect::AppliedNeedsSync
        }
        PropKey::BorderColor => {
            node.style.border_color = Some(color);
            StyleEffect::AppliedNeedsSync
        }
        _ => StyleEffect::Ignored,
    }
}

fn set_f32_style_prop(node: &mut Node, prop: PropKey, v: f32) -> StyleEffect {
    // Non-layout and non-style branches first.
    match prop {
        PropKey::HoverOpacity => {
            let r = node
                .interactivity
                .hover_style
                .get_or_insert_with(|| Box::new(UzStyleRefinement::default()));
            r.opacity = Some(v);
            return StyleEffect::Applied;
        }
        PropKey::ActiveOpacity => {
            let r = node
                .interactivity
                .active_style
                .get_or_insert_with(|| Box::new(UzStyleRefinement::default()));
            r.opacity = Some(v);
            return StyleEffect::Applied;
        }
        PropKey::Interactive => {
            node.interactivity.js_interactive = v > 0.5;
            return StyleEffect::Applied;
        }
        PropKey::Scrollable => {
            if v > 0.5 {
                node.style.overflow_y = Overflow::Scroll;
                if node.scroll_state.is_none() {
                    node.scroll_state = Some(element::ScrollState::new());
                }
            } else {
                node.style.overflow_y = Overflow::Visible;
                node.scroll_state = None;
            }
            return StyleEffect::AppliedNeedsSync;
        }
        PropKey::TextSelect => {
            node.set_text_selectable((v > 0.5).into());
            return StyleEffect::Applied;
        }
        _ => {}
    }

    let style = &mut node.style;
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
        PropKey::Visibility => {
            style.visibility = if v > 0.5 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
        PropKey::Top => style.inset.top = Length::Px(v),
        PropKey::Right => style.inset.right = Length::Px(v),
        PropKey::Bottom => style.inset.bottom = Length::Px(v),
        PropKey::Left => style.inset.left = Length::Px(v),
        _ => return StyleEffect::Ignored,
    }
    StyleEffect::AppliedNeedsSync
}

fn set_enum_style_prop(style: &mut UzStyle, prop: PropKey, value: i32) -> bool {
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
        PropKey::OverflowWrap => {
            style.text.overflow_wrap = match value {
                0 => OverflowWrap::Normal,
                1 => OverflowWrap::Anywhere,
                2 => OverflowWrap::BreakWord,
                _ => OverflowWrap::Normal,
            };
        }
        PropKey::WordBreak => {
            style.text.word_break = match value {
                0 => WordBreak::Normal,
                1 => WordBreak::BreakAll,
                2 => WordBreak::KeepAll,
                _ => WordBreak::Normal,
            };
        }
        PropKey::Position => {
            style.position = match value {
                0 => Position::Relative,
                1 => Position::Absolute,
                _ => Position::Relative,
            };
        }
        _ => return false,
    }
    true
}

fn sync_taffy(dom: &mut UIState, node_id: UzNodeId) {
    let Some(node) = dom.nodes.get(node_id) else {
        return;
    };
    let taffy_style = node.style.to_taffy();
    let tn = node.taffy_node;
    let text_style = node.style.text.clone();
    dom.taffy.set_style(tn, taffy_style).unwrap();
    if let Some(ctx) = dom.taffy.get_node_context_mut(tn) {
        ctx.text_style = text_style;
    }
}
