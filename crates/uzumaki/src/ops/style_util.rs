use serde_json::{Value, json};

use crate::app::WindowEntry;
use crate::cursor;
use crate::element::{self, Node, UzNodeId};
use crate::prop_keys::{AttributeKind, ElementProp, StyleProp, StyleVariant};
use crate::style::*;
use crate::ui::UIState;

use crate::parse::*;

impl WindowEntry {
    pub fn set_str_attribute(&mut self, node_id: UzNodeId, name: &str, value: &str) {
        let Some(kind) = name.parse::<AttributeKind>().ok() else {
            return;
        };

        let effect = match kind {
            AttributeKind::Element(ep) => {
                let Some(node) = self.dom.nodes.get_mut(node_id) else {
                    return;
                };
                set_element_str(node, ep, value, self.rem_base)
            }
            AttributeKind::Style(prop, variant) => {
                let Some(node) = self.dom.nodes.get_mut(node_id) else {
                    return;
                };
                set_style_str(node, prop, variant, value, self.rem_base)
            }
        };

        self.apply_side_effects(node_id, &kind, effect);
    }

    pub fn set_number_attribute(&mut self, node_id: UzNodeId, name: &str, value: f64) {
        let Some(kind) = name.parse::<AttributeKind>().ok() else {
            return;
        };

        let effect = match kind {
            AttributeKind::Element(ep) => {
                let Some(node) = self.dom.nodes.get_mut(node_id) else {
                    return;
                };
                set_element_number(node, ep, value as f32)
            }
            AttributeKind::Style(prop, variant) => {
                let Some(node) = self.dom.nodes.get_mut(node_id) else {
                    return;
                };
                set_style_number(node, prop, variant, value as f32)
            }
        };

        self.apply_side_effects(node_id, &kind, effect);
    }

    pub fn set_bool_attribute(&mut self, node_id: UzNodeId, name: &str, value: bool) {
        let Some(kind) = name.parse::<AttributeKind>().ok() else {
            return;
        };

        let effect = match kind {
            AttributeKind::Element(ep) => {
                let Some(node) = self.dom.nodes.get_mut(node_id) else {
                    return;
                };
                set_element_bool(node, ep, value)
            }
            AttributeKind::Style(prop, variant) => {
                let Some(node) = self.dom.nodes.get_mut(node_id) else {
                    return;
                };
                set_style_number(node, prop, variant, if value { 1.0 } else { 0.0 })
            }
        };

        self.apply_side_effects(node_id, &kind, effect);
    }

    pub fn clear_attribute(&mut self, node_id: UzNodeId, name: &str) {
        let Some(kind) = name.parse::<AttributeKind>().ok() else {
            return;
        };

        let effect = match kind {
            AttributeKind::Element(ep) => {
                let Some(node) = self.dom.nodes.get_mut(node_id) else {
                    return;
                };
                clear_element_prop(node, ep)
            }
            AttributeKind::Style(prop, variant) => {
                let Some(node) = self.dom.nodes.get_mut(node_id) else {
                    return;
                };
                clear_style_prop(node, prop, variant)
            }
        };

        self.apply_side_effects(node_id, &kind, effect);
    }

    pub fn get_attribute(&self, node_id: UzNodeId, name: &str) -> Value {
        let Ok(kind) = name.parse::<AttributeKind>() else {
            return Value::Null;
        };

        let Some(node) = self.dom.nodes.get(node_id) else {
            return Value::Null;
        };

        match kind {
            AttributeKind::Element(ep) => get_element_prop(node, ep),
            AttributeKind::Style(prop, _variant) => get_style_prop(node, prop),
        }
    }

    fn apply_side_effects(&mut self, node_id: UzNodeId, kind: &AttributeKind, effect: StyleEffect) {
        if matches!(effect, StyleEffect::AppliedNeedsSync) {
            sync_taffy(&mut self.dom, node_id);
        }
        if matches!(kind, AttributeKind::Style(StyleProp::Cursor, _)) {
            self.update_cursor();
        }
    }

    pub(crate) fn update_cursor(&mut self) {
        if let Some(handle) = self.handle.as_mut()
            && let Some(top) = self.dom.hit_state.top_node
        {
            let icon = self.dom.resolve_cursor(top);
            handle.set_cursor(icon);
        }
    }
}

pub(crate) enum StyleEffect {
    Ignored,
    Applied,
    AppliedNeedsSync,
}

fn set_element_str(node: &mut Node, prop: ElementProp, value: &str, _rem_base: f32) -> StyleEffect {
    match prop {
        ElementProp::Value => {
            if let Some(input) = node.as_text_input_mut() {
                input.set_value(value);
                return StyleEffect::Applied;
            }
        }
        ElementProp::Placeholder => {
            if let Some(input) = node.as_text_input_mut() {
                input.placeholder = value.to_string();
                return StyleEffect::Applied;
            }
        }
        ElementProp::MaxLength => {
            if let Some(input) = node.as_text_input_mut() {
                input.max_length = parse_max_length(value.parse::<f32>().unwrap_or(-1.0));
                return StyleEffect::Applied;
            }
        }
        ElementProp::Disabled
        | ElementProp::Multiline
        | ElementProp::Secure
        | ElementProp::Checked
        | ElementProp::Focusable => {
            return set_element_bool(node, prop, parse_bool(value));
        }
    }
    StyleEffect::Ignored
}

fn set_element_number(node: &mut Node, prop: ElementProp, value: f32) -> StyleEffect {
    match prop {
        ElementProp::MaxLength => {
            if let Some(input) = node.as_text_input_mut() {
                input.max_length = parse_max_length(value);
                return StyleEffect::Applied;
            }
        }
        ElementProp::Disabled
        | ElementProp::Multiline
        | ElementProp::Secure
        | ElementProp::Checked
        | ElementProp::Focusable => {
            return set_element_bool(node, prop, value > 0.5);
        }
        _ => {}
    }
    StyleEffect::Ignored
}

fn set_element_bool(node: &mut Node, prop: ElementProp, value: bool) -> StyleEffect {
    match prop {
        ElementProp::Disabled => {
            if let Some(input) = node.as_text_input_mut() {
                input.disabled = value;
                return StyleEffect::Applied;
            }
        }
        ElementProp::Multiline => {
            if let Some(input) = node.as_text_input_mut() {
                input.multiline = value;
                return StyleEffect::Applied;
            }
        }
        ElementProp::Secure => {
            if let Some(input) = node.as_text_input_mut() {
                input.secure = value;
                return StyleEffect::Applied;
            }
        }
        ElementProp::Checked => {
            if let Some(checked) = node.as_checkbox_input_mut() {
                *checked = value;
                return StyleEffect::Applied;
            }
        }
        ElementProp::Focusable => {
            if let Some(element) = node.as_element_mut() {
                element.set_focussable(value);
                return StyleEffect::Applied;
            }
        }
        _ => {}
    }
    StyleEffect::Ignored
}

fn clear_element_prop(node: &mut Node, prop: ElementProp) -> StyleEffect {
    match prop {
        ElementProp::Value => {
            if let Some(input) = node.as_text_input_mut() {
                input.set_value("");
                return StyleEffect::Applied;
            }
        }
        ElementProp::Placeholder => {
            if let Some(input) = node.as_text_input_mut() {
                input.placeholder.clear();
                return StyleEffect::Applied;
            }
        }
        ElementProp::Disabled => {
            if let Some(input) = node.as_text_input_mut() {
                input.disabled = false;
                return StyleEffect::Applied;
            }
        }
        ElementProp::MaxLength => {
            if let Some(input) = node.as_text_input_mut() {
                input.max_length = None;
                return StyleEffect::Applied;
            }
        }
        ElementProp::Multiline => {
            if let Some(input) = node.as_text_input_mut() {
                input.multiline = false;
                return StyleEffect::Applied;
            }
        }
        ElementProp::Secure => {
            if let Some(input) = node.as_text_input_mut() {
                input.secure = false;
                return StyleEffect::Applied;
            }
        }
        ElementProp::Checked => {
            if let Some(checked) = node.as_checkbox_input_mut() {
                *checked = false;
                return StyleEffect::Applied;
            }
        }
        ElementProp::Focusable => {
            if let Some(element) = node.as_element_mut() {
                element.set_focussable(false);
                return StyleEffect::Applied;
            }
        }
    }
    StyleEffect::Ignored
}

fn get_element_prop(node: &Node, prop: ElementProp) -> Value {
    match prop {
        ElementProp::Value => node
            .as_text_input()
            .map(|v| json!(v.text()))
            .unwrap_or(Value::Null),
        ElementProp::Placeholder => node
            .as_text_input()
            .map(|v| json!(v.placeholder))
            .unwrap_or(Value::Null),
        ElementProp::Disabled => node
            .as_text_input()
            .map(|v| json!(v.disabled))
            .unwrap_or(Value::Null),
        ElementProp::MaxLength => node
            .as_text_input()
            .map(|v| v.max_length.map_or(Value::Null, |max| json!(max)))
            .unwrap_or(Value::Null),
        ElementProp::Multiline => node
            .as_text_input()
            .map(|v| json!(v.multiline))
            .unwrap_or(Value::Null),
        ElementProp::Secure => node
            .as_text_input()
            .map(|v| json!(v.secure))
            .unwrap_or(Value::Null),
        ElementProp::Checked => node
            .as_checkbox_input()
            .map(|v| json!(v))
            .unwrap_or(Value::Null),
        ElementProp::Focusable => node
            .as_element()
            .map(|v| json!(v.is_focussable()))
            .unwrap_or(Value::Null),
    }
}

fn set_style_str(
    node: &mut Node,
    prop: StyleProp,
    variant: StyleVariant,
    value: &str,
    rem_base: f32,
) -> StyleEffect {
    if variant != StyleVariant::Base {
        return set_variant_style_str(node, prop, variant, value, rem_base);
    }

    match prop {
        StyleProp::W
        | StyleProp::H
        | StyleProp::MinW
        | StyleProp::MinH
        | StyleProp::Top
        | StyleProp::Right
        | StyleProp::Bottom
        | StyleProp::Left => {
            if let Some(length) = parse_length(value, rem_base) {
                set_length_style_prop(&mut node.style, prop, length)
            } else {
                clear_style_prop(node, prop, variant)
            }
        }
        StyleProp::Gap => {
            if let Some(length) = parse_definite_length(value, rem_base) {
                set_gap_style_prop(&mut node.style, length)
            } else {
                clear_style_prop(node, prop, variant)
            }
        }
        StyleProp::Bg | StyleProp::Color | StyleProp::BorderColor => {
            if let Some(color) = parse_color(value) {
                set_color_style_prop(node, prop, color)
            } else {
                clear_style_prop(node, prop, variant)
            }
        }
        StyleProp::FlexDir
        | StyleProp::Items
        | StyleProp::Justify
        | StyleProp::Display
        | StyleProp::TextWrap
        | StyleProp::WordBreak
        | StyleProp::Position => {
            if set_enum_style_prop_from_str(&mut node.style, prop, value) {
                remember_inherited_enum(node, prop);
                StyleEffect::AppliedNeedsSync
            } else {
                clear_style_prop(node, prop, variant)
            }
        }
        StyleProp::Cursor => {
            node.style.cursor = cursor::UzCursorIcon::parse(value);
            StyleEffect::Applied
        }
        StyleProp::Visibility => set_style_number(
            node,
            prop,
            variant,
            if value == "visible" { 1.0 } else { 0.0 },
        ),
        StyleProp::Flex => {
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

fn set_variant_style_str(
    node: &mut Node,
    prop: StyleProp,
    variant: StyleVariant,
    value: &str,
    rem_base: f32,
) -> StyleEffect {
    match prop {
        StyleProp::W
        | StyleProp::H
        | StyleProp::MinW
        | StyleProp::MinH
        | StyleProp::Top
        | StyleProp::Right
        | StyleProp::Bottom
        | StyleProp::Left => {
            if let Some(length) = parse_length(value, rem_base) {
                set_variant_length(node, prop, variant, length)
            } else {
                clear_style_prop(node, prop, variant)
            }
        }
        StyleProp::Gap => {
            if let Some(length) = parse_definite_length(value, rem_base) {
                set_variant_gap(node, variant, length)
            } else {
                clear_style_prop(node, prop, variant)
            }
        }
        StyleProp::Bg | StyleProp::Color | StyleProp::BorderColor => {
            if let Some(color) = parse_color(value) {
                set_variant_color(node, prop, variant, color)
            } else {
                clear_style_prop(node, prop, variant)
            }
        }
        StyleProp::FlexDir
        | StyleProp::Items
        | StyleProp::Justify
        | StyleProp::Display
        | StyleProp::TextWrap
        | StyleProp::WordBreak
        | StyleProp::Position => {
            if set_variant_enum_from_str(node, prop, variant, value) {
                StyleEffect::Applied
            } else {
                clear_style_prop(node, prop, variant)
            }
        }
        StyleProp::Cursor => {
            get_or_init_variant_style(node, variant).cursor = cursor::UzCursorIcon::parse(value);
            StyleEffect::Applied
        }
        StyleProp::Visibility => set_variant_number(
            node,
            prop,
            variant,
            if value == "visible" { 1.0 } else { 0.0 },
        ),
        StyleProp::Flex => {
            if set_variant_flex_string(node, variant, value) {
                StyleEffect::Applied
            } else {
                let v = parse_px_scalar(value, rem_base).unwrap_or_default();
                set_variant_number(node, prop, variant, v)
            }
        }
        StyleProp::Opacity
        | StyleProp::TranslateX
        | StyleProp::TranslateY
        | StyleProp::Rotate
        | StyleProp::Scale
        | StyleProp::ScaleX
        | StyleProp::ScaleY => {
            let v = parse_px_scalar(value, rem_base).unwrap_or_default();
            set_variant_number(node, prop, variant, v)
        }
        _ => {
            let v = parse_px_scalar(value, rem_base).unwrap_or_default();
            set_variant_number(node, prop, variant, v)
        }
    }
}

fn set_style_number(
    node: &mut Node,
    prop: StyleProp,
    variant: StyleVariant,
    value: f32,
) -> StyleEffect {
    if variant != StyleVariant::Base {
        return set_variant_number(node, prop, variant, value);
    }

    match prop {
        StyleProp::W
        | StyleProp::H
        | StyleProp::MinW
        | StyleProp::MinH
        | StyleProp::Top
        | StyleProp::Right
        | StyleProp::Bottom
        | StyleProp::Left => set_length_style_prop(&mut node.style, prop, Length::Px(value)),
        StyleProp::Gap => set_gap_style_prop(&mut node.style, DefiniteLength::Px(value)),
        StyleProp::FlexDir
        | StyleProp::Items
        | StyleProp::Justify
        | StyleProp::Display
        | StyleProp::TextWrap
        | StyleProp::WordBreak
        | StyleProp::Position => {
            set_enum_style_prop(&mut node.style, prop, value as i32);
            remember_inherited_enum(node, prop);
            StyleEffect::AppliedNeedsSync
        }
        StyleProp::Visibility => {
            node.style.visibility = if value > 0.5 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            StyleEffect::AppliedNeedsSync
        }
        _ => set_f32_style_prop(node, prop, value),
    }
}

// ---------------------------------------------------------------------------
// Variant (hover/active) style helpers
// ---------------------------------------------------------------------------

fn get_or_init_variant_style(node: &mut Node, variant: StyleVariant) -> &mut UzStyleRefinement {
    match variant {
        StyleVariant::Hover => node
            .interactivity
            .hover_style
            .get_or_insert_with(|| Box::new(UzStyleRefinement::default())),
        StyleVariant::Active => node
            .interactivity
            .active_style
            .get_or_insert_with(|| Box::new(UzStyleRefinement::default())),
        StyleVariant::Focus => node
            .interactivity
            .focus_style
            .get_or_insert_with(|| Box::new(UzStyleRefinement::default())),
        StyleVariant::Base => unreachable!(),
    }
}

fn set_variant_color(
    node: &mut Node,
    prop: StyleProp,
    variant: StyleVariant,
    color: Color,
) -> StyleEffect {
    let r = get_or_init_variant_style(node, variant);
    match prop {
        StyleProp::Bg => r.background = Some(color),
        StyleProp::Color => r.text.color = Some(color),
        StyleProp::BorderColor => r.border_color = Some(color),
        StyleProp::OutlineColor => {
            let outline = r.outline.get_or_insert(Outline::FOCUS_RING);
            outline.color = color;
        }
        _ => return StyleEffect::Ignored,
    }
    StyleEffect::Applied
}

fn set_variant_length(
    node: &mut Node,
    prop: StyleProp,
    variant: StyleVariant,
    length: Length,
) -> StyleEffect {
    let r = get_or_init_variant_style(node, variant);
    match prop {
        StyleProp::W => r.size.width = Some(length),
        StyleProp::H => r.size.height = Some(length),
        StyleProp::MinW => r.min_size.width = Some(length),
        StyleProp::MinH => r.min_size.height = Some(length),
        StyleProp::Top => r.inset.top = Some(length),
        StyleProp::Right => r.inset.right = Some(length),
        StyleProp::Bottom => r.inset.bottom = Some(length),
        StyleProp::Left => r.inset.left = Some(length),
        _ => return StyleEffect::Ignored,
    }
    StyleEffect::Applied
}

fn set_variant_gap(node: &mut Node, variant: StyleVariant, length: DefiniteLength) -> StyleEffect {
    let r = get_or_init_variant_style(node, variant);
    r.gap.width = Some(length);
    r.gap.height = Some(length);
    StyleEffect::Applied
}

fn text_wrap_value(value: &str) -> Option<i32> {
    match value.trim() {
        "wrap" => Some(0),
        "nowrap" | "none" => Some(1),
        "anywhere" => Some(2),
        "break-word" => Some(3),
        _ => None,
    }
}

fn set_text_wrap(style: &mut UzStyle, value: i32) {
    let (overflow_wrap, word_break) = match value {
        1 => (OverflowWrap::Normal, WordBreak::KeepAll),
        2 => (OverflowWrap::Anywhere, WordBreak::Normal),
        3 => (OverflowWrap::BreakWord, WordBreak::Normal),
        _ => (OverflowWrap::BreakWord, WordBreak::Normal),
    };
    style.text.overflow_wrap = overflow_wrap;
    style.text.word_break = word_break;
}

fn set_text_wrap_refinement(style: &mut UzStyleRefinement, value: i32) {
    let mut resolved = UzStyle::default();
    set_text_wrap(&mut resolved, value);
    style.text.overflow_wrap = Some(resolved.text.overflow_wrap);
    style.text.word_break = Some(resolved.text.word_break);
}

fn set_variant_number(
    node: &mut Node,
    prop: StyleProp,
    variant: StyleVariant,
    value: f32,
) -> StyleEffect {
    match prop {
        StyleProp::W
        | StyleProp::H
        | StyleProp::MinW
        | StyleProp::MinH
        | StyleProp::Top
        | StyleProp::Right
        | StyleProp::Bottom
        | StyleProp::Left => {
            return set_variant_length(node, prop, variant, Length::Px(value));
        }
        StyleProp::Gap => return set_variant_gap(node, variant, DefiniteLength::Px(value)),
        StyleProp::FlexDir
        | StyleProp::Items
        | StyleProp::Justify
        | StyleProp::Display
        | StyleProp::TextWrap
        | StyleProp::WordBreak
        | StyleProp::Position => {
            if set_variant_enum(node, prop, variant, value as i32) {
                return StyleEffect::Applied;
            }
            return StyleEffect::Ignored;
        }
        _ => {}
    }

    let r = get_or_init_variant_style(node, variant);
    match prop {
        StyleProp::Scrollable => {
            r.overflow_y = Some(if value > 0.5 {
                Overflow::Scroll
            } else {
                Overflow::Visible
            });
        }
        StyleProp::TextSelect => {
            r.text_selectable = Some((value > 0.5).into());
        }
        StyleProp::TranslateX => r.transform.translate_x = Some(value),
        StyleProp::TranslateY => r.transform.translate_y = Some(value),
        StyleProp::Rotate => r.transform.rotate = Some(value),
        StyleProp::Scale => {
            r.transform.scale_x = Some(value);
            r.transform.scale_y = Some(value);
        }
        StyleProp::ScaleX => r.transform.scale_x = Some(value),
        StyleProp::ScaleY => r.transform.scale_y = Some(value),
        StyleProp::P => {
            r.padding = EdgesRefinement {
                top: Some(value),
                right: Some(value),
                bottom: Some(value),
                left: Some(value),
            }
        }
        StyleProp::Px => {
            r.padding.left = Some(value);
            r.padding.right = Some(value);
        }
        StyleProp::Py => {
            r.padding.top = Some(value);
            r.padding.bottom = Some(value);
        }
        StyleProp::Pt => r.padding.top = Some(value),
        StyleProp::Pb => r.padding.bottom = Some(value),
        StyleProp::Pl => r.padding.left = Some(value),
        StyleProp::Pr => r.padding.right = Some(value),
        StyleProp::M => {
            r.margin = EdgesRefinement {
                top: Some(value),
                right: Some(value),
                bottom: Some(value),
                left: Some(value),
            }
        }
        StyleProp::Mx => {
            r.margin.left = Some(value);
            r.margin.right = Some(value);
        }
        StyleProp::My => {
            r.margin.top = Some(value);
            r.margin.bottom = Some(value);
        }
        StyleProp::Mt => r.margin.top = Some(value),
        StyleProp::Mb => r.margin.bottom = Some(value),
        StyleProp::Ml => r.margin.left = Some(value),
        StyleProp::Mr => r.margin.right = Some(value),
        StyleProp::Flex => {
            r.display = Some(Display::Flex);
            r.flex_grow = Some(value);
        }
        StyleProp::FlexGrow => r.flex_grow = Some(value),
        StyleProp::FlexShrink => r.flex_shrink = Some(value),
        StyleProp::FontSize => r.text.font_size = Some(value),
        StyleProp::FontWeight => {}
        StyleProp::Rounded => {
            r.corner_radii = CornersRefinement {
                top_left: Some(value),
                top_right: Some(value),
                bottom_right: Some(value),
                bottom_left: Some(value),
            }
        }
        StyleProp::RoundedTL => r.corner_radii.top_left = Some(value),
        StyleProp::RoundedTR => r.corner_radii.top_right = Some(value),
        StyleProp::RoundedBR => r.corner_radii.bottom_right = Some(value),
        StyleProp::RoundedBL => r.corner_radii.bottom_left = Some(value),
        StyleProp::Border => {
            r.border_widths = EdgesRefinement {
                top: Some(value),
                right: Some(value),
                bottom: Some(value),
                left: Some(value),
            }
        }
        StyleProp::BorderTop => r.border_widths.top = Some(value),
        StyleProp::BorderRight => r.border_widths.right = Some(value),
        StyleProp::BorderBottom => r.border_widths.bottom = Some(value),
        StyleProp::BorderLeft => r.border_widths.left = Some(value),
        StyleProp::Outline => {
            let outline = r.outline.get_or_insert(Outline::FOCUS_RING);
            outline.width = value;
        }
        StyleProp::OutlineOffset => {
            let outline = r.outline.get_or_insert(Outline::FOCUS_RING);
            outline.offset = value;
        }
        StyleProp::Opacity => r.opacity = Some(value),
        StyleProp::Visibility => {
            r.visibility = Some(if value > 0.5 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            });
        }
        StyleProp::Interactive => return StyleEffect::Ignored,
        _ => return StyleEffect::Ignored,
    }
    StyleEffect::Applied
}

fn set_variant_enum(node: &mut Node, prop: StyleProp, variant: StyleVariant, value: i32) -> bool {
    let r = get_or_init_variant_style(node, variant);
    match prop {
        StyleProp::FlexDir => {
            r.flex_direction = Some(match value {
                0 => FlexDirection::Row,
                1 => FlexDirection::Column,
                2 => FlexDirection::RowReverse,
                3 => FlexDirection::ColumnReverse,
                _ => FlexDirection::Row,
            });
        }
        StyleProp::Items => {
            r.align_items = Some(match value {
                0 => AlignItems::FlexStart,
                1 => AlignItems::FlexEnd,
                2 => AlignItems::Center,
                3 => AlignItems::Stretch,
                4 => AlignItems::Baseline,
                _ => AlignItems::Stretch,
            });
        }
        StyleProp::Justify => {
            r.justify_content = Some(match value {
                0 => JustifyContent::FlexStart,
                1 => JustifyContent::FlexEnd,
                2 => JustifyContent::Center,
                3 => JustifyContent::SpaceBetween,
                4 => JustifyContent::SpaceAround,
                5 => JustifyContent::SpaceEvenly,
                _ => JustifyContent::FlexStart,
            });
        }
        StyleProp::Display => {
            r.display = Some(match value {
                0 => Display::None,
                1 => Display::Flex,
                2 => Display::Block,
                _ => Display::Flex,
            });
        }
        StyleProp::TextWrap => set_text_wrap_refinement(r, value),
        StyleProp::WordBreak => {
            r.text.word_break = Some(match value {
                0 => WordBreak::Normal,
                1 => WordBreak::BreakAll,
                2 => WordBreak::KeepAll,
                _ => WordBreak::Normal,
            });
        }
        StyleProp::Position => {
            r.position = Some(match value {
                0 => Position::Relative,
                1 => Position::Absolute,
                _ => Position::Relative,
            });
        }
        _ => return false,
    }
    true
}

fn set_variant_enum_from_str(
    node: &mut Node,
    prop: StyleProp,
    variant: StyleVariant,
    value: &str,
) -> bool {
    let value = value.trim();
    let number = match prop {
        StyleProp::FlexDir => match value {
            "row" => 0,
            "col" | "column" => 1,
            "row-reverse" => 2,
            "col-reverse" | "column-reverse" => 3,
            _ => return false,
        },
        StyleProp::Items => match value {
            "flex-start" | "start" => 0,
            "flex-end" | "end" => 1,
            "center" => 2,
            "stretch" => 3,
            "baseline" => 4,
            _ => return false,
        },
        StyleProp::Justify => match value {
            "flex-start" | "start" => 0,
            "flex-end" | "end" => 1,
            "center" => 2,
            "space-between" | "between" => 3,
            "space-around" | "around" => 4,
            "space-evenly" | "evenly" => 5,
            _ => return false,
        },
        StyleProp::Display => match value {
            "none" => 0,
            "flex" => 1,
            "block" => 2,
            _ => return false,
        },
        StyleProp::TextWrap => match text_wrap_value(value) {
            Some(v) => v,
            None => return false,
        },
        StyleProp::WordBreak => match value {
            "normal" => 0,
            "break-all" => 1,
            "keep-all" => 2,
            _ => return false,
        },
        StyleProp::Position => match value {
            "relative" => 0,
            "absolute" => 1,
            _ => return false,
        },
        _ => return false,
    };
    set_variant_enum(node, prop, variant, number)
}

fn set_variant_flex_string(node: &mut Node, variant: StyleVariant, value: &str) -> bool {
    let dir = match value.trim() {
        "row" => FlexDirection::Row,
        "col" | "column" => FlexDirection::Column,
        "row-reverse" => FlexDirection::RowReverse,
        "col-reverse" | "column-reverse" => FlexDirection::ColumnReverse,
        _ => return false,
    };
    let r = get_or_init_variant_style(node, variant);
    r.display = Some(Display::Flex);
    r.flex_direction = Some(dir);
    true
}

fn clear_variant_prop(node: &mut Node, prop: StyleProp, variant: StyleVariant) -> StyleEffect {
    let style = match variant {
        StyleVariant::Hover => node.interactivity.hover_style.as_mut(),
        StyleVariant::Active => node.interactivity.active_style.as_mut(),
        StyleVariant::Focus => node.interactivity.focus_style.as_mut(),
        StyleVariant::Base => unreachable!(),
    };
    if let Some(style) = style {
        match prop {
            StyleProp::W => style.size.width = None,
            StyleProp::H => style.size.height = None,
            StyleProp::MinW => style.min_size.width = None,
            StyleProp::MinH => style.min_size.height = None,
            StyleProp::P => style.padding = EdgesRefinement::default(),
            StyleProp::Px => {
                style.padding.left = None;
                style.padding.right = None;
            }
            StyleProp::Py => {
                style.padding.top = None;
                style.padding.bottom = None;
            }
            StyleProp::Pt => style.padding.top = None,
            StyleProp::Pb => style.padding.bottom = None,
            StyleProp::Pl => style.padding.left = None,
            StyleProp::Pr => style.padding.right = None,
            StyleProp::M => style.margin = EdgesRefinement::default(),
            StyleProp::Mx => {
                style.margin.left = None;
                style.margin.right = None;
            }
            StyleProp::My => {
                style.margin.top = None;
                style.margin.bottom = None;
            }
            StyleProp::Mt => style.margin.top = None,
            StyleProp::Mb => style.margin.bottom = None,
            StyleProp::Ml => style.margin.left = None,
            StyleProp::Mr => style.margin.right = None,
            StyleProp::Flex => {
                style.display = None;
                style.flex_grow = None;
            }
            StyleProp::FlexDir => style.flex_direction = None,
            StyleProp::FlexGrow => style.flex_grow = None,
            StyleProp::FlexShrink => style.flex_shrink = None,
            StyleProp::Items => style.align_items = None,
            StyleProp::Justify => style.justify_content = None,
            StyleProp::Gap => style.gap = GapSizeRefinement::default(),
            StyleProp::Bg => style.background = None,
            StyleProp::Color => style.text.color = None,
            StyleProp::FontSize => style.text.font_size = None,
            StyleProp::FontWeight => style.text.font_weight = None,
            StyleProp::Rounded => style.corner_radii = CornersRefinement::default(),
            StyleProp::RoundedTL => style.corner_radii.top_left = None,
            StyleProp::RoundedTR => style.corner_radii.top_right = None,
            StyleProp::RoundedBR => style.corner_radii.bottom_right = None,
            StyleProp::RoundedBL => style.corner_radii.bottom_left = None,
            StyleProp::Border => style.border_widths = EdgesRefinement::default(),
            StyleProp::BorderTop => style.border_widths.top = None,
            StyleProp::BorderRight => style.border_widths.right = None,
            StyleProp::BorderBottom => style.border_widths.bottom = None,
            StyleProp::BorderLeft => style.border_widths.left = None,
            StyleProp::Opacity => style.opacity = None,
            StyleProp::BorderColor => style.border_color = None,
            StyleProp::Outline | StyleProp::OutlineColor | StyleProp::OutlineOffset => {
                style.outline = None;
            }
            StyleProp::Display => style.display = None,
            StyleProp::Cursor => style.cursor = None,
            StyleProp::Interactive => {}
            StyleProp::Visibility => style.visibility = None,
            StyleProp::Scrollable => style.overflow_y = None,
            StyleProp::TextSelect => style.text_selectable = None,
            StyleProp::TextWrap => {
                style.text.overflow_wrap = None;
                style.text.word_break = None;
            }
            StyleProp::WordBreak => style.text.word_break = None,
            StyleProp::Position => style.position = None,
            StyleProp::Top => style.inset.top = None,
            StyleProp::Right => style.inset.right = None,
            StyleProp::Bottom => style.inset.bottom = None,
            StyleProp::Left => style.inset.left = None,
            StyleProp::TranslateX => style.transform.translate_x = None,
            StyleProp::TranslateY => style.transform.translate_y = None,
            StyleProp::Rotate => style.transform.rotate = None,
            StyleProp::Scale => {
                style.transform.scale_x = None;
                style.transform.scale_y = None;
            }
            StyleProp::ScaleX => style.transform.scale_x = None,
            StyleProp::ScaleY => style.transform.scale_y = None,
        }
    }
    StyleEffect::Applied
}

// ---------------------------------------------------------------------------
// Base style prop helpers
// ---------------------------------------------------------------------------

fn set_length_style_prop(style: &mut UzStyle, prop: StyleProp, length: Length) -> StyleEffect {
    match prop {
        StyleProp::W => style.size.width = length,
        StyleProp::H => style.size.height = length,
        StyleProp::MinW => style.min_size.width = length,
        StyleProp::MinH => style.min_size.height = length,
        StyleProp::Top => style.inset.top = length,
        StyleProp::Right => style.inset.right = length,
        StyleProp::Bottom => style.inset.bottom = length,
        StyleProp::Left => style.inset.left = length,
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

fn set_color_style_prop(node: &mut Node, prop: StyleProp, color: Color) -> StyleEffect {
    match prop {
        StyleProp::Bg => {
            node.style.background = Some(color);
            StyleEffect::AppliedNeedsSync
        }
        StyleProp::Color => {
            node.style.text.color = color;
            node.interactivity.base_style.text.color = Some(color);
            StyleEffect::AppliedNeedsSync
        }
        StyleProp::BorderColor => {
            node.style.border_color = Some(color);
            StyleEffect::AppliedNeedsSync
        }
        StyleProp::OutlineColor => {
            let outline = node.style.outline.get_or_insert(Outline::FOCUS_RING);
            outline.color = color;
            StyleEffect::Applied
        }
        _ => StyleEffect::Ignored,
    }
}

fn set_f32_style_prop(node: &mut Node, prop: StyleProp, v: f32) -> StyleEffect {
    match prop {
        StyleProp::Interactive => {
            node.interactivity.js_interactive = v > 0.5;
            return StyleEffect::Applied;
        }
        StyleProp::Scrollable => {
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
        StyleProp::TextSelect => {
            let text_selectable = (v > 0.5).into();
            node.set_text_selectable(text_selectable);
            node.interactivity.base_style.text_selectable = Some(text_selectable);
            return StyleEffect::Applied;
        }
        _ => {}
    }

    match prop {
        StyleProp::TranslateX => {
            node.style.transform.translate_x = v;
            return StyleEffect::Applied;
        }
        StyleProp::TranslateY => {
            node.style.transform.translate_y = v;
            return StyleEffect::Applied;
        }
        StyleProp::Rotate => {
            node.style.transform.rotate = v;
            return StyleEffect::Applied;
        }
        StyleProp::Scale => {
            node.style.transform.scale_x = v;
            node.style.transform.scale_y = v;
            return StyleEffect::Applied;
        }
        StyleProp::ScaleX => {
            node.style.transform.scale_x = v;
            return StyleEffect::Applied;
        }
        StyleProp::ScaleY => {
            node.style.transform.scale_y = v;
            return StyleEffect::Applied;
        }
        _ => {}
    }

    let style = &mut node.style;
    match prop {
        StyleProp::P => style.padding = Edges::all(v),
        StyleProp::Px => {
            style.padding.left = v;
            style.padding.right = v;
        }
        StyleProp::Py => {
            style.padding.top = v;
            style.padding.bottom = v;
        }
        StyleProp::Pt => style.padding.top = v,
        StyleProp::Pb => style.padding.bottom = v,
        StyleProp::Pl => style.padding.left = v,
        StyleProp::Pr => style.padding.right = v,
        StyleProp::M => style.margin = Edges::all(v),
        StyleProp::Mx => {
            style.margin.left = v;
            style.margin.right = v;
        }
        StyleProp::My => {
            style.margin.top = v;
            style.margin.bottom = v;
        }
        StyleProp::Mt => style.margin.top = v,
        StyleProp::Mb => style.margin.bottom = v,
        StyleProp::Ml => style.margin.left = v,
        StyleProp::Mr => style.margin.right = v,
        StyleProp::Flex => {
            style.display = Display::Flex;
            style.flex_grow = v;
        }
        StyleProp::FlexGrow => style.flex_grow = v,
        StyleProp::FlexShrink => style.flex_shrink = v,
        StyleProp::Gap => {
            style.gap = GapSize {
                width: DefiniteLength::Px(v),
                height: DefiniteLength::Px(v),
            };
        }
        StyleProp::FontSize => {
            style.text.font_size = v;
            node.interactivity.base_style.text.font_size = Some(v);
        }
        StyleProp::FontWeight => {}
        StyleProp::Rounded => style.corner_radii = Corners::uniform(v),
        StyleProp::RoundedTL => style.corner_radii.top_left = v,
        StyleProp::RoundedTR => style.corner_radii.top_right = v,
        StyleProp::RoundedBR => style.corner_radii.bottom_right = v,
        StyleProp::RoundedBL => style.corner_radii.bottom_left = v,
        StyleProp::Border => style.border_widths = Edges::all(v),
        StyleProp::BorderTop => style.border_widths.top = v,
        StyleProp::BorderRight => style.border_widths.right = v,
        StyleProp::BorderBottom => style.border_widths.bottom = v,
        StyleProp::BorderLeft => style.border_widths.left = v,
        StyleProp::Outline => {
            let outline = style.outline.get_or_insert(Outline::FOCUS_RING);
            outline.width = v;
        }
        StyleProp::OutlineOffset => {
            let outline = style.outline.get_or_insert(Outline::FOCUS_RING);
            outline.offset = v;
        }
        StyleProp::Opacity => style.opacity = v,
        StyleProp::Visibility => {
            style.visibility = if v > 0.5 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
        StyleProp::Top => style.inset.top = Length::Px(v),
        StyleProp::Right => style.inset.right = Length::Px(v),
        StyleProp::Bottom => style.inset.bottom = Length::Px(v),
        StyleProp::Left => style.inset.left = Length::Px(v),
        _ => return StyleEffect::Ignored,
    }
    StyleEffect::AppliedNeedsSync
}

fn set_enum_style_prop(style: &mut UzStyle, prop: StyleProp, value: i32) -> bool {
    match prop {
        StyleProp::FlexDir => {
            style.flex_direction = match value {
                0 => FlexDirection::Row,
                1 => FlexDirection::Column,
                2 => FlexDirection::RowReverse,
                3 => FlexDirection::ColumnReverse,
                _ => FlexDirection::Row,
            };
        }
        StyleProp::Items => {
            style.align_items = Some(match value {
                0 => AlignItems::FlexStart,
                1 => AlignItems::FlexEnd,
                2 => AlignItems::Center,
                3 => AlignItems::Stretch,
                4 => AlignItems::Baseline,
                _ => AlignItems::Stretch,
            });
        }
        StyleProp::Justify => {
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
        StyleProp::Display => {
            style.display = match value {
                0 => Display::None,
                1 => Display::Flex,
                2 => Display::Block,
                _ => Display::Flex,
            };
        }
        StyleProp::TextWrap => set_text_wrap(style, value),
        StyleProp::WordBreak => {
            style.text.word_break = match value {
                0 => WordBreak::Normal,
                1 => WordBreak::BreakAll,
                2 => WordBreak::KeepAll,
                _ => WordBreak::Normal,
            };
        }
        StyleProp::Position => {
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

fn set_enum_style_prop_from_str(style: &mut UzStyle, prop: StyleProp, value: &str) -> bool {
    let value = value.trim();
    let number = match prop {
        StyleProp::FlexDir => match value {
            "row" => 0,
            "col" | "column" => 1,
            "row-reverse" => 2,
            "col-reverse" | "column-reverse" => 3,
            _ => return false,
        },
        StyleProp::Items => match value {
            "flex-start" | "start" => 0,
            "flex-end" | "end" => 1,
            "center" => 2,
            "stretch" => 3,
            "baseline" => 4,
            _ => return false,
        },
        StyleProp::Justify => match value {
            "flex-start" | "start" => 0,
            "flex-end" | "end" => 1,
            "center" => 2,
            "space-between" | "between" => 3,
            "space-around" | "around" => 4,
            "space-evenly" | "evenly" => 5,
            _ => return false,
        },
        StyleProp::Display => match value {
            "none" => 0,
            "flex" => 1,
            "block" => 2,
            _ => return false,
        },
        StyleProp::TextWrap => match text_wrap_value(value) {
            Some(v) => v,
            None => return false,
        },
        StyleProp::WordBreak => match value {
            "normal" => 0,
            "break-all" => 1,
            "keep-all" => 2,
            _ => return false,
        },
        StyleProp::Position => match value {
            "relative" => 0,
            "absolute" => 1,
            _ => return false,
        },
        _ => return false,
    };
    set_enum_style_prop(style, prop, number)
}

fn remember_inherited_enum(node: &mut Node, prop: StyleProp) {
    match prop {
        StyleProp::TextWrap => {
            node.interactivity.base_style.text.overflow_wrap = Some(node.style.text.overflow_wrap);
            node.interactivity.base_style.text.word_break = Some(node.style.text.word_break);
        }
        StyleProp::WordBreak => {
            node.interactivity.base_style.text.word_break = Some(node.style.text.word_break);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Clear style prop
// ---------------------------------------------------------------------------

fn clear_style_prop(node: &mut Node, prop: StyleProp, variant: StyleVariant) -> StyleEffect {
    if variant != StyleVariant::Base {
        return clear_variant_prop(node, prop, variant);
    }

    let default = UzStyle::default();
    match prop {
        StyleProp::W => node.style.size.width = default.size.width,
        StyleProp::H => node.style.size.height = default.size.height,
        StyleProp::MinW => node.style.min_size.width = default.min_size.width,
        StyleProp::MinH => node.style.min_size.height = default.min_size.height,
        StyleProp::P => node.style.padding = default.padding,
        StyleProp::Px => {
            node.style.padding.left = default.padding.left;
            node.style.padding.right = default.padding.right;
        }
        StyleProp::Py => {
            node.style.padding.top = default.padding.top;
            node.style.padding.bottom = default.padding.bottom;
        }
        StyleProp::Pt => node.style.padding.top = default.padding.top,
        StyleProp::Pb => node.style.padding.bottom = default.padding.bottom,
        StyleProp::Pl => node.style.padding.left = default.padding.left,
        StyleProp::Pr => node.style.padding.right = default.padding.right,
        StyleProp::M => node.style.margin = default.margin,
        StyleProp::Mx => {
            node.style.margin.left = default.margin.left;
            node.style.margin.right = default.margin.right;
        }
        StyleProp::My => {
            node.style.margin.top = default.margin.top;
            node.style.margin.bottom = default.margin.bottom;
        }
        StyleProp::Mt => node.style.margin.top = default.margin.top,
        StyleProp::Mb => node.style.margin.bottom = default.margin.bottom,
        StyleProp::Ml => node.style.margin.left = default.margin.left,
        StyleProp::Mr => node.style.margin.right = default.margin.right,
        StyleProp::Flex => {
            node.style.display = default.display;
            node.style.flex_grow = default.flex_grow;
        }
        StyleProp::FlexDir => node.style.flex_direction = default.flex_direction,
        StyleProp::FlexGrow => node.style.flex_grow = default.flex_grow,
        StyleProp::FlexShrink => node.style.flex_shrink = default.flex_shrink,
        StyleProp::Items => node.style.align_items = default.align_items,
        StyleProp::Justify => node.style.justify_content = default.justify_content,
        StyleProp::Gap => node.style.gap = default.gap,
        StyleProp::Bg => node.style.background = default.background,
        StyleProp::Color => {
            node.style.text.color = default.text.color;
            node.interactivity.base_style.text.color = None;
        }
        StyleProp::FontSize => {
            node.style.text.font_size = default.text.font_size;
            node.interactivity.base_style.text.font_size = None;
        }
        StyleProp::FontWeight => node.style.text.font_weight = default.text.font_weight,
        StyleProp::Rounded => node.style.corner_radii = default.corner_radii,
        StyleProp::RoundedTL => node.style.corner_radii.top_left = default.corner_radii.top_left,
        StyleProp::RoundedTR => node.style.corner_radii.top_right = default.corner_radii.top_right,
        StyleProp::RoundedBR => {
            node.style.corner_radii.bottom_right = default.corner_radii.bottom_right
        }
        StyleProp::RoundedBL => {
            node.style.corner_radii.bottom_left = default.corner_radii.bottom_left
        }
        StyleProp::Border => node.style.border_widths = default.border_widths,
        StyleProp::BorderTop => node.style.border_widths.top = default.border_widths.top,
        StyleProp::BorderRight => node.style.border_widths.right = default.border_widths.right,
        StyleProp::BorderBottom => node.style.border_widths.bottom = default.border_widths.bottom,
        StyleProp::BorderLeft => node.style.border_widths.left = default.border_widths.left,
        StyleProp::BorderColor => node.style.border_color = default.border_color,
        StyleProp::Outline | StyleProp::OutlineColor | StyleProp::OutlineOffset => {
            node.style.outline = default.outline;
        }
        StyleProp::Opacity => node.style.opacity = default.opacity,
        StyleProp::TranslateX => node.style.transform.translate_x = default.transform.translate_x,
        StyleProp::TranslateY => node.style.transform.translate_y = default.transform.translate_y,
        StyleProp::Rotate => node.style.transform.rotate = default.transform.rotate,
        StyleProp::Scale => {
            node.style.transform.scale_x = default.transform.scale_x;
            node.style.transform.scale_y = default.transform.scale_y;
        }
        StyleProp::ScaleX => node.style.transform.scale_x = default.transform.scale_x,
        StyleProp::ScaleY => node.style.transform.scale_y = default.transform.scale_y,
        StyleProp::Display => node.style.display = default.display,
        StyleProp::Cursor => node.style.cursor = default.cursor,
        StyleProp::Interactive => node.interactivity.js_interactive = false,
        StyleProp::Visibility => node.style.visibility = default.visibility,
        StyleProp::Scrollable => {
            node.style.overflow_y = default.overflow_y;
            node.scroll_state = None;
        }
        StyleProp::TextSelect => {
            node.set_text_selectable(default.text_selectable);
            node.interactivity.base_style.text_selectable = None;
        }
        StyleProp::TextWrap => {
            node.style.text.overflow_wrap = default.text.overflow_wrap;
            node.style.text.word_break = default.text.word_break;
            node.interactivity.base_style.text.overflow_wrap = None;
            node.interactivity.base_style.text.word_break = None;
        }
        StyleProp::WordBreak => {
            node.style.text.word_break = default.text.word_break;
            node.interactivity.base_style.text.word_break = None;
        }
        StyleProp::Position => node.style.position = default.position,
        StyleProp::Top => node.style.inset.top = default.inset.top,
        StyleProp::Right => node.style.inset.right = default.inset.right,
        StyleProp::Bottom => node.style.inset.bottom = default.inset.bottom,
        StyleProp::Left => node.style.inset.left = default.inset.left,
    }
    match prop {
        StyleProp::Interactive
        | StyleProp::TextSelect
        | StyleProp::Cursor
        | StyleProp::TranslateX
        | StyleProp::TranslateY
        | StyleProp::Rotate
        | StyleProp::Scale
        | StyleProp::ScaleX
        | StyleProp::ScaleY => StyleEffect::Applied,
        _ => StyleEffect::AppliedNeedsSync,
    }
}

fn get_style_prop(node: &Node, prop: StyleProp) -> Value {
    let style = &node.style;
    match prop {
        StyleProp::W => length_to_json(style.size.width),
        StyleProp::H => length_to_json(style.size.height),
        StyleProp::MinW => length_to_json(style.min_size.width),
        StyleProp::MinH => length_to_json(style.min_size.height),
        StyleProp::Bg => style.background.map(color_to_json).unwrap_or(Value::Null),
        StyleProp::Color => color_to_json(style.text.color),
        StyleProp::BorderColor => style.border_color.map(color_to_json).unwrap_or(Value::Null),
        StyleProp::Outline => style.outline.map(|o| json!(o.width)).unwrap_or(Value::Null),
        StyleProp::OutlineColor => style
            .outline
            .map(|o| color_to_json(o.color))
            .unwrap_or(Value::Null),
        StyleProp::OutlineOffset => style
            .outline
            .map(|o| json!(o.offset))
            .unwrap_or(Value::Null),
        StyleProp::Opacity => json!(style.opacity),
        StyleProp::Visibility => json!(matches!(style.visibility, Visibility::Visible)),
        StyleProp::Scrollable => json!(matches!(style.overflow_y, Overflow::Scroll)),
        StyleProp::TextSelect => json!(node.is_text_selectable()),
        StyleProp::Top => length_to_json(style.inset.top),
        StyleProp::Right => length_to_json(style.inset.right),
        StyleProp::Bottom => length_to_json(style.inset.bottom),
        StyleProp::Left => length_to_json(style.inset.left),
        StyleProp::P => json!(style.padding.top),
        StyleProp::M => json!(style.margin.top),
        StyleProp::FlexGrow | StyleProp::Flex => json!(style.flex_grow),
        StyleProp::FlexShrink => json!(style.flex_shrink),
        StyleProp::FontSize => json!(style.text.font_size),
        StyleProp::Rounded => json!(style.corner_radii.top_left),
        StyleProp::Border => json!(style.border_widths.top),
        StyleProp::TranslateX => json!(style.transform.translate_x),
        StyleProp::TranslateY => json!(style.transform.translate_y),
        StyleProp::Rotate => json!(style.transform.rotate),
        StyleProp::Scale => json!(style.transform.scale_x),
        StyleProp::ScaleX => json!(style.transform.scale_x),
        StyleProp::ScaleY => json!(style.transform.scale_y),
        _ => Value::Null,
    }
}

pub(crate) fn sync_taffy(dom: &mut UIState, node_id: UzNodeId) {
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
