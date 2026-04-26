use serde_json::{Value, json};

use crate::app::WindowEntry;
use crate::cursor;
use crate::element::{self, Node, UzNodeId};
use crate::prop_keys::PropKey;
use crate::style::*;
use crate::ui::UIState;

/// Outcome of applying a style prop to a node.
pub(super) enum StyleEffect {
    /// Prop key didn't match - nothing changed.
    Ignored,
    /// Applied, but layout-independent (no taffy sync needed).
    Applied,
    /// Applied and layout changed - caller must sync taffy.
    AppliedNeedsSync,
}
pub(super) fn set_str_attribute(
    node: &mut Node,
    name: &str,
    value: &str,
    rem_base: f32,
) -> StyleEffect {
    match name {
        "value" => {
            if let Some(input) = node.as_text_input_mut() {
                input.set_value(value);
                return StyleEffect::Applied;
            }
        }
        "placeholder" => {
            if let Some(input) = node.as_text_input_mut() {
                input.placeholder = value.to_string();
                return StyleEffect::Applied;
            }
        }
        "disabled" | "multiline" | "secure" | "checked" | "scrollable" | "selectable" => {
            return set_bool_attribute(node, name, parse_bool(value));
        }
        "maxLength" => {
            if let Some(input) = node.as_text_input_mut() {
                input.max_length = parse_max_length(value.parse::<f32>().unwrap_or(-1.0));
                return StyleEffect::Applied;
            }
        }
        _ => {}
    }

    let Ok(prop) = name.parse::<PropKey>() else {
        return StyleEffect::Ignored;
    };

    match prop {
        PropKey::W
        | PropKey::H
        | PropKey::MinW
        | PropKey::MinH
        | PropKey::Top
        | PropKey::Right
        | PropKey::Bottom
        | PropKey::Left => {
            if let Some(length) = parse_length(value, rem_base) {
                set_length_style_prop(&mut node.style, prop, length)
            } else {
                clear_style_prop(node, prop)
            }
        }
        PropKey::Gap => {
            if let Some(length) = parse_definite_length(value, rem_base) {
                set_gap_style_prop(&mut node.style, length)
            } else {
                clear_style_prop(node, prop)
            }
        }
        PropKey::Bg
        | PropKey::Color
        | PropKey::BorderColor
        | PropKey::HoverBg
        | PropKey::HoverColor
        | PropKey::HoverBorderColor
        | PropKey::ActiveBg
        | PropKey::ActiveColor
        | PropKey::ActiveBorderColor => {
            if let Some(color) = parse_color(value) {
                set_color_style_prop(node, prop, color)
            } else {
                clear_style_prop(node, prop)
            }
        }
        PropKey::FlexDir
        | PropKey::Items
        | PropKey::Justify
        | PropKey::Display
        | PropKey::OverflowWrap
        | PropKey::WordBreak
        | PropKey::Position => set_enum_style_prop_from_str(&mut node.style, prop, value)
            .then_some(StyleEffect::AppliedNeedsSync)
            .unwrap_or_else(|| clear_style_prop(node, prop)),
        PropKey::Cursor => {
            node.style.cursor = cursor::UzCursorIcon::parse(value);
            StyleEffect::Applied
        }
        PropKey::Visibility => set_bool_attribute(node, name, value == "visible"),
        PropKey::Flex => {
            if set_flex_string(&mut node.style, value) {
                StyleEffect::AppliedNeedsSync
            } else {
                let v = parse_px_scalar(value, rem_base).unwrap_or_default();
                set_f32_style_prop(node, prop, v)
            }
        }
        _ => {
            let v = parse_px_scalar(value, rem_base).unwrap_or_default();
            set_f32_style_prop(node, prop, v)
        }
    }
}

pub(super) fn set_number_attribute(node: &mut Node, name: &str, value: f32) -> StyleEffect {
    match name {
        "maxLength" => {
            if let Some(input) = node.as_text_input_mut() {
                input.max_length = parse_max_length(value);
                return StyleEffect::Applied;
            }
        }
        "disabled" | "multiline" | "secure" | "checked" | "scrollable" | "selectable" => {
            return set_bool_attribute(node, name, value > 0.5);
        }
        _ => {}
    }

    let Ok(prop) = name.parse::<PropKey>() else {
        return StyleEffect::Ignored;
    };

    match prop {
        PropKey::W
        | PropKey::H
        | PropKey::MinW
        | PropKey::MinH
        | PropKey::Top
        | PropKey::Right
        | PropKey::Bottom
        | PropKey::Left => set_length_style_prop(&mut node.style, prop, Length::Px(value)),
        PropKey::Gap => set_gap_style_prop(&mut node.style, DefiniteLength::Px(value)),
        PropKey::FlexDir
        | PropKey::Items
        | PropKey::Justify
        | PropKey::Display
        | PropKey::OverflowWrap
        | PropKey::WordBreak
        | PropKey::Position => {
            set_enum_style_prop(&mut node.style, prop, value as i32);
            StyleEffect::AppliedNeedsSync
        }
        PropKey::Visibility => set_bool_attribute(node, name, value > 0.5),
        _ => set_f32_style_prop(node, prop, value),
    }
}

pub(super) fn set_bool_attribute(node: &mut Node, name: &str, value: bool) -> StyleEffect {
    match name {
        "disabled" => {
            if let Some(input) = node.as_text_input_mut() {
                input.disabled = value;
                return StyleEffect::Applied;
            }
        }
        "multiline" => {
            if let Some(input) = node.as_text_input_mut() {
                input.multiline = value;
                return StyleEffect::Applied;
            }
        }
        "secure" => {
            if let Some(input) = node.as_text_input_mut() {
                input.secure = value;
                return StyleEffect::Applied;
            }
        }
        "checked" => {
            if let Some(checked) = node.as_checkbox_input_mut() {
                *checked = value;
                return StyleEffect::Applied;
            }
        }
        _ => {}
    }

    let Ok(prop) = name.parse::<PropKey>() else {
        return StyleEffect::Ignored;
    };
    set_f32_style_prop(node, prop, if value { 1.0 } else { 0.0 })
}

pub(super) fn clear_attribute(node: &mut Node, name: &str) -> StyleEffect {
    match name {
        "value" => {
            if let Some(input) = node.as_text_input_mut() {
                input.set_value("");
                return StyleEffect::Applied;
            }
        }
        "placeholder" => {
            if let Some(input) = node.as_text_input_mut() {
                input.placeholder.clear();
                return StyleEffect::Applied;
            }
        }
        "disabled" => {
            if let Some(input) = node.as_text_input_mut() {
                input.disabled = false;
                return StyleEffect::Applied;
            }
        }
        "maxLength" => {
            if let Some(input) = node.as_text_input_mut() {
                input.max_length = None;
                return StyleEffect::Applied;
            }
        }
        "multiline" => {
            if let Some(input) = node.as_text_input_mut() {
                input.multiline = false;
                return StyleEffect::Applied;
            }
        }
        "secure" => {
            if let Some(input) = node.as_text_input_mut() {
                input.secure = false;
                return StyleEffect::Applied;
            }
        }
        "checked" => {
            if let Some(checked) = node.as_checkbox_input_mut() {
                *checked = false;
                return StyleEffect::Applied;
            }
        }
        _ => {}
    }

    let Ok(prop) = name.parse::<PropKey>() else {
        return StyleEffect::Ignored;
    };
    clear_style_prop(node, prop)
}

pub(super) fn get_attribute(node: &Node, name: &str) -> Value {
    match name {
        "value" => {
            return node
                .as_text_input()
                .map(|v| json!(v.text()))
                .unwrap_or(Value::Null);
        }
        "placeholder" => {
            return node
                .as_text_input()
                .map(|v| json!(v.placeholder))
                .unwrap_or(Value::Null);
        }
        "disabled" => {
            return node
                .as_text_input()
                .map(|v| json!(v.disabled))
                .unwrap_or(Value::Null);
        }
        "maxLength" => {
            return node
                .as_text_input()
                .map(|v| v.max_length.map_or(Value::Null, |max| json!(max)))
                .unwrap_or(Value::Null);
        }
        "multiline" => {
            return node
                .as_text_input()
                .map(|v| json!(v.multiline))
                .unwrap_or(Value::Null);
        }
        "secure" => {
            return node
                .as_text_input()
                .map(|v| json!(v.secure))
                .unwrap_or(Value::Null);
        }
        "checked" => {
            return node
                .as_checkbox_input()
                .map(|v| json!(v))
                .unwrap_or(Value::Null);
        }
        _ => {}
    }

    let Ok(prop) = name.parse::<PropKey>() else {
        return Value::Null;
    };
    get_style_attribute(node, prop)
}

fn set_length_style_prop(style: &mut UzStyle, prop: PropKey, length: Length) -> StyleEffect {
    match prop {
        PropKey::W => style.size.width = length,
        PropKey::H => style.size.height = length,
        PropKey::MinW => style.min_size.width = length,
        PropKey::MinH => style.min_size.height = length,
        PropKey::Top => style.inset.top = length,
        PropKey::Right => style.inset.right = length,
        PropKey::Bottom => style.inset.bottom = length,
        PropKey::Left => style.inset.left = length,
        _ => return StyleEffect::Ignored,
    }
    StyleEffect::AppliedNeedsSync
}

fn set_gap_style_prop(style: &mut UzStyle, length: DefiniteLength) -> StyleEffect {
    style.gap = GapSize {
        width: length,
        height: length,
    };
    StyleEffect::AppliedNeedsSync
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

fn set_enum_style_prop_from_str(style: &mut UzStyle, prop: PropKey, value: &str) -> bool {
    let value = value.trim();
    let number = match prop {
        PropKey::FlexDir => match value {
            "row" => 0,
            "col" | "column" => 1,
            "row-reverse" => 2,
            "col-reverse" | "column-reverse" => 3,
            _ => return false,
        },
        PropKey::Items => match value {
            "flex-start" | "start" => 0,
            "flex-end" | "end" => 1,
            "center" => 2,
            "stretch" => 3,
            "baseline" => 4,
            _ => return false,
        },
        PropKey::Justify => match value {
            "flex-start" | "start" => 0,
            "flex-end" | "end" => 1,
            "center" => 2,
            "space-between" | "between" => 3,
            "space-around" | "around" => 4,
            "space-evenly" | "evenly" => 5,
            _ => return false,
        },
        PropKey::Display => match value {
            "none" => 0,
            "flex" => 1,
            "block" => 2,
            _ => return false,
        },
        PropKey::OverflowWrap => match value {
            "normal" => 0,
            "anywhere" => 1,
            "break-word" => 2,
            _ => return false,
        },
        PropKey::WordBreak => match value {
            "normal" => 0,
            "break-all" => 1,
            "keep-all" => 2,
            _ => return false,
        },
        PropKey::Position => match value {
            "relative" => 0,
            "absolute" => 1,
            _ => return false,
        },
        _ => return false,
    };
    set_enum_style_prop(style, prop, number)
}

fn clear_style_prop(node: &mut Node, prop: PropKey) -> StyleEffect {
    let default = UzStyle::default();
    match prop {
        PropKey::W => node.style.size.width = default.size.width,
        PropKey::H => node.style.size.height = default.size.height,
        PropKey::MinW => node.style.min_size.width = default.min_size.width,
        PropKey::MinH => node.style.min_size.height = default.min_size.height,
        PropKey::P => node.style.padding = default.padding,
        PropKey::Px => {
            node.style.padding.left = default.padding.left;
            node.style.padding.right = default.padding.right;
        }
        PropKey::Py => {
            node.style.padding.top = default.padding.top;
            node.style.padding.bottom = default.padding.bottom;
        }
        PropKey::Pt => node.style.padding.top = default.padding.top,
        PropKey::Pb => node.style.padding.bottom = default.padding.bottom,
        PropKey::Pl => node.style.padding.left = default.padding.left,
        PropKey::Pr => node.style.padding.right = default.padding.right,
        PropKey::M => node.style.margin = default.margin,
        PropKey::Mx => {
            node.style.margin.left = default.margin.left;
            node.style.margin.right = default.margin.right;
        }
        PropKey::My => {
            node.style.margin.top = default.margin.top;
            node.style.margin.bottom = default.margin.bottom;
        }
        PropKey::Mt => node.style.margin.top = default.margin.top,
        PropKey::Mb => node.style.margin.bottom = default.margin.bottom,
        PropKey::Ml => node.style.margin.left = default.margin.left,
        PropKey::Mr => node.style.margin.right = default.margin.right,
        PropKey::Flex => {
            node.style.display = default.display;
            node.style.flex_grow = default.flex_grow;
        }
        PropKey::FlexDir => node.style.flex_direction = default.flex_direction,
        PropKey::FlexGrow => node.style.flex_grow = default.flex_grow,
        PropKey::FlexShrink => node.style.flex_shrink = default.flex_shrink,
        PropKey::Items => node.style.align_items = default.align_items,
        PropKey::Justify => node.style.justify_content = default.justify_content,
        PropKey::Gap => node.style.gap = default.gap,
        PropKey::Bg => node.style.background = default.background,
        PropKey::Color => node.style.text.color = default.text.color,
        PropKey::FontSize => node.style.text.font_size = default.text.font_size,
        PropKey::FontWeight => node.style.text.font_weight = default.text.font_weight,
        PropKey::Rounded => node.style.corner_radii = default.corner_radii,
        PropKey::RoundedTL => node.style.corner_radii.top_left = default.corner_radii.top_left,
        PropKey::RoundedTR => node.style.corner_radii.top_right = default.corner_radii.top_right,
        PropKey::RoundedBR => {
            node.style.corner_radii.bottom_right = default.corner_radii.bottom_right
        }
        PropKey::RoundedBL => {
            node.style.corner_radii.bottom_left = default.corner_radii.bottom_left
        }
        PropKey::Border => node.style.border_widths = default.border_widths,
        PropKey::BorderTop => node.style.border_widths.top = default.border_widths.top,
        PropKey::BorderRight => node.style.border_widths.right = default.border_widths.right,
        PropKey::BorderBottom => node.style.border_widths.bottom = default.border_widths.bottom,
        PropKey::BorderLeft => node.style.border_widths.left = default.border_widths.left,
        PropKey::BorderColor => node.style.border_color = default.border_color,
        PropKey::Opacity => node.style.opacity = default.opacity,
        PropKey::Display => node.style.display = default.display,
        PropKey::Cursor => node.style.cursor = default.cursor,
        PropKey::Interactive => node.interactivity.js_interactive = false,
        PropKey::Visibility => node.style.visibility = default.visibility,
        PropKey::HoverBg
        | PropKey::HoverColor
        | PropKey::HoverOpacity
        | PropKey::HoverBorderColor => {
            if let Some(style) = node.interactivity.hover_style.as_mut() {
                match prop {
                    PropKey::HoverBg => style.background = None,
                    PropKey::HoverColor => style.text.color = None,
                    PropKey::HoverOpacity => style.opacity = None,
                    PropKey::HoverBorderColor => style.border_color = None,
                    _ => {}
                }
            }
            return StyleEffect::Applied;
        }
        PropKey::ActiveBg
        | PropKey::ActiveColor
        | PropKey::ActiveOpacity
        | PropKey::ActiveBorderColor => {
            if let Some(style) = node.interactivity.active_style.as_mut() {
                match prop {
                    PropKey::ActiveBg => style.background = None,
                    PropKey::ActiveColor => style.text.color = None,
                    PropKey::ActiveOpacity => style.opacity = None,
                    PropKey::ActiveBorderColor => style.border_color = None,
                    _ => {}
                }
            }
            return StyleEffect::Applied;
        }
        PropKey::Scrollable => {
            node.style.overflow_y = default.overflow_y;
            node.scroll_state = None;
        }
        PropKey::TextSelect => node.set_text_selectable(default.text_selectable),
        PropKey::OverflowWrap => node.style.text.overflow_wrap = default.text.overflow_wrap,
        PropKey::WordBreak => node.style.text.word_break = default.text.word_break,
        PropKey::Position => node.style.position = default.position,
        PropKey::Top => node.style.inset.top = default.inset.top,
        PropKey::Right => node.style.inset.right = default.inset.right,
        PropKey::Bottom => node.style.inset.bottom = default.inset.bottom,
        PropKey::Left => node.style.inset.left = default.inset.left,
    }
    match prop {
        PropKey::Interactive | PropKey::TextSelect | PropKey::Cursor => StyleEffect::Applied,
        _ => StyleEffect::AppliedNeedsSync,
    }
}

fn parse_bool(value: &str) -> bool {
    !matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "" | "0" | "false" | "hidden" | "none" | "no" | "off"
    )
}

fn parse_max_length(value: f32) -> Option<usize> {
    (value.is_finite() && value > 0.0).then_some(value as usize)
}

fn parse_px_scalar(value: &str, rem_base: f32) -> Option<f32> {
    let value = value.trim();
    if let Some(value) = value.strip_suffix("rem") {
        return value.trim().parse::<f32>().ok().map(|v| v * rem_base);
    }
    if let Some(value) = value.strip_suffix("px") {
        return value.trim().parse().ok();
    }
    value.parse().ok()
}

fn parse_length(value: &str, rem_base: f32) -> Option<Length> {
    let value = value.trim();
    if value == "auto" {
        return Some(Length::Auto);
    }
    if value == "full" {
        return Some(Length::Percent(1.0));
    }
    if let Some(value) = value.strip_suffix('%') {
        return value
            .trim()
            .parse::<f32>()
            .ok()
            .map(|value| Length::Percent(value / 100.0));
    }
    parse_px_scalar(value, rem_base).map(Length::Px)
}

fn parse_definite_length(value: &str, rem_base: f32) -> Option<DefiniteLength> {
    let value = value.trim();
    if value == "full" {
        return Some(DefiniteLength::Percent(1.0));
    }
    if let Some(value) = value.strip_suffix('%') {
        return value
            .trim()
            .parse::<f32>()
            .ok()
            .map(|value| DefiniteLength::Percent(value / 100.0));
    }
    parse_px_scalar(value, rem_base).map(DefiniteLength::Px)
}

fn parse_color(value: &str) -> Option<Color> {
    let value = value.trim();
    if let Some(color) = parse_named_color(value) {
        return Some(color);
    }
    if let Some(color) = parse_hex_color(value) {
        return Some(color);
    }
    parse_rgb_color(value)
}

fn parse_named_color(value: &str) -> Option<Color> {
    Some(match value.to_ascii_lowercase().as_str() {
        "aliceblue" => Color::rgb(240, 248, 255),
        "antiquewhite" => Color::rgb(250, 235, 215),
        "aqua" => Color::rgb(0, 255, 255),
        "aquamarine" => Color::rgb(127, 255, 212),
        "azure" => Color::rgb(240, 255, 255),
        "beige" => Color::rgb(245, 245, 220),
        "bisque" => Color::rgb(255, 228, 196),
        "black" => Color::BLACK,
        "blanchedalmond" => Color::rgb(255, 235, 205),
        "blue" => Color::rgb(0, 0, 255),
        "blueviolet" => Color::rgb(138, 43, 226),
        "brown" => Color::rgb(165, 42, 42),
        "burlywood" => Color::rgb(222, 184, 135),
        "cadetblue" => Color::rgb(95, 158, 160),
        "chartreuse" => Color::rgb(127, 255, 0),
        "chocolate" => Color::rgb(210, 105, 30),
        "coral" => Color::rgb(255, 127, 80),
        "cornflowerblue" => Color::rgb(100, 149, 237),
        "cornsilk" => Color::rgb(255, 248, 220),
        "crimson" => Color::rgb(220, 20, 60),
        "cyan" => Color::rgb(0, 255, 255),
        "darkblue" => Color::rgb(0, 0, 139),
        "darkcyan" => Color::rgb(0, 139, 139),
        "darkgoldenrod" => Color::rgb(184, 134, 11),
        "darkgray" => Color::rgb(169, 169, 169),
        "darkgreen" => Color::rgb(0, 100, 0),
        "darkgrey" => Color::rgb(169, 169, 169),
        "darkkhaki" => Color::rgb(189, 183, 107),
        "darkmagenta" => Color::rgb(139, 0, 139),
        "darkolivegreen" => Color::rgb(85, 107, 47),
        "darkorange" => Color::rgb(255, 140, 0),
        "darkorchid" => Color::rgb(153, 50, 204),
        "darkred" => Color::rgb(139, 0, 0),
        "darksalmon" => Color::rgb(233, 150, 122),
        "darkseagreen" => Color::rgb(143, 188, 143),
        "darkslateblue" => Color::rgb(72, 61, 139),
        "darkslategray" => Color::rgb(47, 79, 79),
        "darkslategrey" => Color::rgb(47, 79, 79),
        "darkturquoise" => Color::rgb(0, 206, 209),
        "darkviolet" => Color::rgb(148, 0, 211),
        "deeppink" => Color::rgb(255, 20, 147),
        "deepskyblue" => Color::rgb(0, 191, 255),
        "dimgray" => Color::rgb(105, 105, 105),
        "dimgrey" => Color::rgb(105, 105, 105),
        "dodgerblue" => Color::rgb(30, 144, 255),
        "firebrick" => Color::rgb(178, 34, 34),
        "floralwhite" => Color::rgb(255, 250, 240),
        "forestgreen" => Color::rgb(34, 139, 34),
        "fuchsia" => Color::rgb(255, 0, 255),
        "gainsboro" => Color::rgb(220, 220, 220),
        "ghostwhite" => Color::rgb(248, 248, 255),
        "gold" => Color::rgb(255, 215, 0),
        "goldenrod" => Color::rgb(218, 165, 32),
        "gray" => Color::rgb(128, 128, 128),
        "green" => Color::rgb(0, 128, 0),
        "greenyellow" => Color::rgb(173, 255, 47),
        "grey" => Color::rgb(128, 128, 128),
        "honeydew" => Color::rgb(240, 255, 240),
        "hotpink" => Color::rgb(255, 105, 180),
        "indianred" => Color::rgb(205, 92, 92),
        "indigo" => Color::rgb(75, 0, 130),
        "ivory" => Color::rgb(255, 255, 240),
        "khaki" => Color::rgb(240, 230, 140),
        "lavender" => Color::rgb(230, 230, 250),
        "lavenderblush" => Color::rgb(255, 240, 245),
        "lawngreen" => Color::rgb(124, 252, 0),
        "lemonchiffon" => Color::rgb(255, 250, 205),
        "lightblue" => Color::rgb(173, 216, 230),
        "lightcoral" => Color::rgb(240, 128, 128),
        "lightcyan" => Color::rgb(224, 255, 255),
        "lightgoldenrodyellow" => Color::rgb(250, 250, 210),
        "lightgray" => Color::rgb(211, 211, 211),
        "lightgreen" => Color::rgb(144, 238, 144),
        "lightgrey" => Color::rgb(211, 211, 211),
        "lightpink" => Color::rgb(255, 182, 193),
        "lightsalmon" => Color::rgb(255, 160, 122),
        "lightseagreen" => Color::rgb(32, 178, 170),
        "lightskyblue" => Color::rgb(135, 206, 250),
        "lightslategray" => Color::rgb(119, 136, 153),
        "lightslategrey" => Color::rgb(119, 136, 153),
        "lightsteelblue" => Color::rgb(176, 196, 222),
        "lightyellow" => Color::rgb(255, 255, 224),
        "lime" => Color::rgb(0, 255, 0),
        "limegreen" => Color::rgb(50, 205, 50),
        "linen" => Color::rgb(250, 240, 230),
        "magenta" => Color::rgb(255, 0, 255),
        "maroon" => Color::rgb(128, 0, 0),
        "mediumaquamarine" => Color::rgb(102, 205, 170),
        "mediumblue" => Color::rgb(0, 0, 205),
        "mediumorchid" => Color::rgb(186, 85, 211),
        "mediumpurple" => Color::rgb(147, 112, 219),
        "mediumseagreen" => Color::rgb(60, 179, 113),
        "mediumslateblue" => Color::rgb(123, 104, 238),
        "mediumspringgreen" => Color::rgb(0, 250, 154),
        "mediumturquoise" => Color::rgb(72, 209, 204),
        "mediumvioletred" => Color::rgb(199, 21, 133),
        "midnightblue" => Color::rgb(25, 25, 112),
        "mintcream" => Color::rgb(245, 255, 250),
        "mistyrose" => Color::rgb(255, 228, 225),
        "moccasin" => Color::rgb(255, 228, 181),
        "navajowhite" => Color::rgb(255, 222, 173),
        "navy" => Color::rgb(0, 0, 128),
        "oldlace" => Color::rgb(253, 245, 230),
        "olive" => Color::rgb(128, 128, 0),
        "olivedrab" => Color::rgb(107, 142, 35),
        "orange" => Color::rgb(255, 165, 0),
        "orangered" => Color::rgb(255, 69, 0),
        "orchid" => Color::rgb(218, 112, 214),
        "palegoldenrod" => Color::rgb(238, 232, 170),
        "palegreen" => Color::rgb(152, 251, 152),
        "paleturquoise" => Color::rgb(175, 238, 238),
        "palevioletred" => Color::rgb(219, 112, 147),
        "papayawhip" => Color::rgb(255, 239, 213),
        "peachpuff" => Color::rgb(255, 218, 185),
        "peru" => Color::rgb(205, 133, 63),
        "pink" => Color::rgb(255, 192, 203),
        "plum" => Color::rgb(221, 160, 221),
        "powderblue" => Color::rgb(176, 224, 230),
        "purple" => Color::rgb(128, 0, 128),
        "rebeccapurple" => Color::rgb(102, 51, 153),
        "red" => Color::rgb(255, 0, 0),
        "rosybrown" => Color::rgb(188, 143, 143),
        "royalblue" => Color::rgb(65, 105, 225),
        "saddlebrown" => Color::rgb(139, 69, 19),
        "salmon" => Color::rgb(250, 128, 114),
        "sandybrown" => Color::rgb(244, 164, 96),
        "seagreen" => Color::rgb(46, 139, 87),
        "seashell" => Color::rgb(255, 245, 238),
        "sienna" => Color::rgb(160, 82, 45),
        "silver" => Color::rgb(192, 192, 192),
        "skyblue" => Color::rgb(135, 206, 235),
        "slateblue" => Color::rgb(106, 90, 205),
        "slategray" => Color::rgb(112, 128, 144),
        "slategrey" => Color::rgb(112, 128, 144),
        "snow" => Color::rgb(255, 250, 250),
        "springgreen" => Color::rgb(0, 255, 127),
        "steelblue" => Color::rgb(70, 130, 180),
        "tan" => Color::rgb(210, 180, 140),
        "teal" => Color::rgb(0, 128, 128),
        "thistle" => Color::rgb(216, 191, 216),
        "tomato" => Color::rgb(255, 99, 71),
        "transparent" => Color::TRANSPARENT,
        "turquoise" => Color::rgb(64, 224, 208),
        "violet" => Color::rgb(238, 130, 238),
        "wheat" => Color::rgb(245, 222, 179),
        "white" => Color::WHITE,
        "whitesmoke" => Color::rgb(245, 245, 245),
        "yellow" => Color::rgb(255, 255, 0),
        "yellowgreen" => Color::rgb(154, 205, 50),
        _ => return None,
    })
}

fn parse_hex_color(value: &str) -> Option<Color> {
    let hex = value.strip_prefix('#')?;
    let component = |range: std::ops::Range<usize>| u8::from_str_radix(hex.get(range)?, 16).ok();
    let duplicate = |value: u8| (value << 4) | value;
    match hex.len() {
        3 | 4 => {
            let r = duplicate(component(0..1)?);
            let g = duplicate(component(1..2)?);
            let b = duplicate(component(2..3)?);
            let a = if hex.len() == 4 {
                duplicate(component(3..4)?)
            } else {
                255
            };
            Some(Color::rgba(r, g, b, a))
        }
        6 | 8 => {
            let r = component(0..2)?;
            let g = component(2..4)?;
            let b = component(4..6)?;
            let a = if hex.len() == 8 {
                component(6..8)?
            } else {
                255
            };
            Some(Color::rgba(r, g, b, a))
        }
        _ => None,
    }
}

fn parse_rgb_color(value: &str) -> Option<Color> {
    let inner = value
        .strip_prefix("rgb(")
        .and_then(|value| value.strip_suffix(')'))
        .or_else(|| {
            value
                .strip_prefix("rgba(")
                .and_then(|value| value.strip_suffix(')'))
        })?;
    let parts = inner.split(',').map(|part| part.trim()).collect::<Vec<_>>();
    if !(parts.len() == 3 || parts.len() == 4) {
        return None;
    }
    let channel = |value: &str| value.parse::<u8>().ok();
    let alpha = |value: &str| {
        if let Ok(alpha) = value.parse::<f32>() {
            Some((alpha.clamp(0.0, 1.0) * 255.0) as u8)
        } else {
            channel(value)
        }
    };
    Some(Color::rgba(
        channel(parts[0])?,
        channel(parts[1])?,
        channel(parts[2])?,
        parts.get(3).and_then(|value| alpha(value)).unwrap_or(255),
    ))
}

fn set_flex_string(style: &mut UzStyle, value: &str) -> bool {
    let dir = match value.trim() {
        "row" => FlexDirection::Row,
        "col" | "column" => FlexDirection::Column,
        "row-reverse" => FlexDirection::RowReverse,
        "col-reverse" | "column-reverse" => FlexDirection::ColumnReverse,
        _ => return false,
    };
    style.display = Display::Flex;
    style.flex_direction = dir;
    true
}

fn get_style_attribute(node: &Node, prop: PropKey) -> Value {
    let style = &node.style;
    match prop {
        PropKey::W => length_to_json(style.size.width),
        PropKey::H => length_to_json(style.size.height),
        PropKey::MinW => length_to_json(style.min_size.width),
        PropKey::MinH => length_to_json(style.min_size.height),
        PropKey::Bg => style.background.map(color_to_json).unwrap_or(Value::Null),
        PropKey::Color => color_to_json(style.text.color),
        PropKey::BorderColor => style.border_color.map(color_to_json).unwrap_or(Value::Null),
        PropKey::Opacity => json!(style.opacity),
        PropKey::Visibility => json!(matches!(style.visibility, Visibility::Visible)),
        PropKey::Scrollable => json!(matches!(style.overflow_y, Overflow::Scroll)),
        PropKey::TextSelect => json!(node.is_text_selectable()),
        PropKey::Top => length_to_json(style.inset.top),
        PropKey::Right => length_to_json(style.inset.right),
        PropKey::Bottom => length_to_json(style.inset.bottom),
        PropKey::Left => length_to_json(style.inset.left),
        PropKey::P => json!(style.padding.top),
        PropKey::M => json!(style.margin.top),
        PropKey::FlexGrow | PropKey::Flex => json!(style.flex_grow),
        PropKey::FlexShrink => json!(style.flex_shrink),
        PropKey::FontSize => json!(style.text.font_size),
        PropKey::Rounded => json!(style.corner_radii.top_left),
        PropKey::Border => json!(style.border_widths.top),
        _ => Value::Null,
    }
}

fn length_to_json(length: Length) -> Value {
    match length {
        Length::Auto => json!("auto"),
        Length::Px(value) => json!(value),
        Length::Percent(value) => json!(format!("{}%", value * 100.0)),
    }
}

fn color_to_json(color: Color) -> Value {
    json!({
        "r": color.r,
        "g": color.g,
        "b": color.b,
        "a": color.a,
    })
}

pub(super) fn update_cursor(entry: &mut WindowEntry) {
    if let Some(handle) = entry.handle.as_mut()
        && let Some(top) = entry.dom.hit_state.top_node
    {
        let icon = entry.dom.resolve_cursor(top);
        handle.set_cursor(icon);
    }
}

pub(super) fn sync_taffy(dom: &mut UIState, node_id: UzNodeId) {
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
