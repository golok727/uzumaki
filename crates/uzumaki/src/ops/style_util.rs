use serde_json::{Value, json};

use crate::app::WindowEntry;
use crate::cursor::UzCursorIcon;
use crate::interactivity::StyleVariantKind;
use crate::node::{Node, UzNodeId};
use crate::prop_keys::{AttrValue, AttributeKind, StyleProp};
use crate::style::*;
use crate::ui::UIState;

impl WindowEntry {
    pub(crate) fn set_attribute<'a>(
        &mut self,
        node_id: UzNodeId,
        name: &str,
        value: impl Into<AttrValue<'a>>,
    ) {
        let value = value.into();
        let kind = AttributeKind::parse(name);

        match kind {
            AttributeKind::Element(name) => {
                if let Some(node) = self.dom.nodes.get_mut(node_id)
                    && let Some(el) = node.as_element_mut()
                {
                    el.set_attr(name, value);
                }
            }
            AttributeKind::Style(prop, variant) => {
                set_node_style(&mut self.dom, node_id, prop, variant, value);
            }
        };
    }

    pub fn clear_attribute(&mut self, node_id: UzNodeId, name: &str) {
        let kind = AttributeKind::parse(name);
        let Some(node) = self.dom.nodes.get_mut(node_id) else {
            return;
        };

        match kind {
            AttributeKind::Element(name) => {
                if let Some(el) = node.as_element_mut() {
                    el.clear_attr(name);
                }
            }
            AttributeKind::Style(prop, variant) => {
                clear_node_style(&mut self.dom, node_id, prop, variant)
            }
        };
    }

    pub fn get_attribute(&self, node_id: UzNodeId, name: &str) -> Value {
        let kind = AttributeKind::parse(name);

        let Some(node) = self.dom.nodes.get(node_id) else {
            return Value::Null;
        };

        match kind {
            AttributeKind::Element(name) => node
                .as_element()
                .and_then(|el| el.get_attr(name))
                .unwrap_or(Value::Null),
            AttributeKind::Style(_, _variant) => Value::Null, // todo computed styls ?
        }
    }

    pub fn set_cursor(&mut self, _node_id: UzNodeId, icon: UzCursorIcon) {
        todo!()
    }
}

fn set_node_style(
    dom: &mut UIState,
    node_id: UzNodeId,
    prop: StyleProp,
    variant: StyleVariantKind,
    value: AttrValue<'_>,
) {
    todo!()
}

fn clear_node_style(
    dom: &mut UIState,
    node_id: UzNodeId,
    prop: StyleProp,
    variant: StyleVariantKind,
) {
    todo!()
}

fn set_variant_color(node: &mut Node, prop: StyleProp, variant: StyleVariantKind, color: Color) {
    let r = node.style_for(variant);

    match prop {
        StyleProp::Bg => r.background = Some(color),
        StyleProp::Color => r.text.color = Some(color),
        StyleProp::BorderColor => r.border_color = Some(color),
        StyleProp::OutlineColor => {
            let outline = r.outline.get_or_insert(Outline::FOCUS_RING);
            outline.color = color;
        }
        StyleProp::ScrollbarColor => r.scrollbar.color = Some(color),
        StyleProp::ScrollbarHoverColor => r.scrollbar.hover_color = Some(color),
        StyleProp::ScrollbarActiveColor => r.scrollbar.active_color = Some(color),
        _ => {}
    }
}

fn set_variant_length(node: &mut Node, prop: StyleProp, variant: StyleVariantKind, length: Length) {
    let r = node.style_for(variant);
    match prop {
        StyleProp::W => r.size.width = Some(length),
        StyleProp::H => r.size.height = Some(length),
        StyleProp::MinW => r.min_size.width = Some(length),
        StyleProp::MinH => r.min_size.height = Some(length),
        StyleProp::Top => r.inset.top = Some(length),
        StyleProp::Right => r.inset.right = Some(length),
        StyleProp::Bottom => r.inset.bottom = Some(length),
        StyleProp::Left => r.inset.left = Some(length),
        _ => {
            // rest doesnt affect length
        }
    }
}

fn set_variant_gap(node: &mut Node, variant: StyleVariantKind, length: DefiniteLength) {
    let r = node.style_for(variant);
    r.gap.width = Some(length);
    r.gap.height = Some(length);
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

fn set_variant_enum_from_str(
    node: &mut Node,
    prop: StyleProp,
    variant: StyleVariantKind,
    value: &str,
) -> bool {
    let value = value.trim();
    let r = node.style_for(variant);
    match prop {
        StyleProp::FlexDir => {
            r.flex_direction = Some(match value {
                "row" => FlexDirection::Row,
                "col" | "column" => FlexDirection::Column,
                "row-reverse" => FlexDirection::RowReverse,
                "col-reverse" | "column-reverse" => FlexDirection::ColumnReverse,
                _ => return false,
            });
        }
        StyleProp::FlexWrap => {
            r.flex_wrap = Some(match value {
                "nowrap" | "no-wrap" => FlexWrap::NoWrap,
                "wrap" => FlexWrap::Wrap,
                "wrap-reverse" => FlexWrap::WrapReverse,
                _ => return false,
            });
        }
        StyleProp::Items => {
            r.align_items = Some(match value {
                "flex-start" | "start" => AlignItems::FlexStart,
                "flex-end" | "end" => AlignItems::FlexEnd,
                "center" => AlignItems::Center,
                "stretch" => AlignItems::Stretch,
                "baseline" => AlignItems::Baseline,
                _ => return false,
            });
        }
        StyleProp::Justify => {
            r.justify_content = Some(match value {
                "flex-start" | "start" => JustifyContent::FlexStart,
                "flex-end" | "end" => JustifyContent::FlexEnd,
                "center" => JustifyContent::Center,
                "space-between" | "between" => JustifyContent::SpaceBetween,
                "space-around" | "around" => JustifyContent::SpaceAround,
                "space-evenly" | "evenly" => JustifyContent::SpaceEvenly,
                _ => return false,
            });
        }
        StyleProp::Display => {
            r.display = Some(match value {
                "none" => Display::None,
                "flex" => Display::Flex,
                "block" => Display::Block,
                _ => return false,
            });
        }
        StyleProp::TextWrap => {
            let Some(value) = text_wrap_value(value) else {
                return false;
            };
        }
        StyleProp::WordBreak => {
            r.text.word_break = Some(match value {
                "normal" => WordBreak::Normal,
                "break-all" => WordBreak::BreakAll,
                "keep-all" => WordBreak::KeepAll,
                _ => return false,
            });
        }
        StyleProp::TextAlign => {
            r.text.text_align = Some(match value {
                "start" => TextAlign::Start,
                "end" => TextAlign::End,
                "left" => TextAlign::Left,
                "center" => TextAlign::Center,
                "right" => TextAlign::Right,
                "justify" => TextAlign::Justify,
                _ => return false,
            });
        }
        StyleProp::Position => {
            r.position = Some(match value {
                "relative" => Position::Relative,
                "absolute" => Position::Absolute,
                _ => return false,
            });
        }
        _ => return false,
    }
    true
}
fn set_variant_flex_string(node: &mut Node, variant: StyleVariantKind, value: &str) -> bool {
    let dir = match value.trim() {
        "row" => FlexDirection::Row,
        "col" | "column" => FlexDirection::Column,
        "row-reverse" => FlexDirection::RowReverse,
        "col-reverse" | "column-reverse" => FlexDirection::ColumnReverse,
        _ => return false,
    };
    let r = node.style_for(variant);
    r.display = Some(Display::Flex);
    r.flex_direction = Some(dir);
    true
}

fn clear_variant_prop(node: &mut Node, prop: StyleProp, variant: StyleVariantKind) {
    let style = node.style_for(variant);

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
        StyleProp::FlexWrap => style.flex_wrap = None,
        StyleProp::FlexGrow => style.flex_grow = None,
        StyleProp::FlexShrink => style.flex_shrink = None,
        StyleProp::Items => style.align_items = None,
        StyleProp::Justify => style.justify_content = None,
        StyleProp::Gap => style.gap = GapSizeRefinement::default(),
        StyleProp::Bg => style.background = None,
        StyleProp::Color => style.text.color = None,
        StyleProp::FontSize => style.text.font_size = None,
        StyleProp::FontWeight => style.text.font_weight = None,
        StyleProp::FontFamily => style.text.font_family = None,
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
        StyleProp::Visibility => style.visibility = None,
        StyleProp::Scroll => {
            style.overflow_x = None;
            style.overflow_y = None;
        }
        StyleProp::ScrollX => style.overflow_x = None,
        StyleProp::ScrollY => style.overflow_y = None,
        StyleProp::ScrollbarWidth => style.scrollbar.width = None,
        StyleProp::ScrollbarColor => style.scrollbar.color = None,
        StyleProp::ScrollbarHoverColor => style.scrollbar.hover_color = None,
        StyleProp::ScrollbarActiveColor => style.scrollbar.active_color = None,
        StyleProp::ScrollbarRadius => style.scrollbar.radius = None,
        StyleProp::TextSelect => style.text_selectable = None,
        StyleProp::TextWrap => {
            style.text.overflow_wrap = None;
            style.text.word_break = None;
        }
        StyleProp::WordBreak => style.text.word_break = None,
        StyleProp::TextAlign => style.text.text_align = None,
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

fn set_length_style_prop(style: &mut UzStyle, prop: StyleProp, length: Length) {
    match prop {
        StyleProp::W => style.size.width = length,
        StyleProp::H => style.size.height = length,
        StyleProp::MinW => style.min_size.width = length,
        StyleProp::MinH => style.min_size.height = length,
        StyleProp::Top => style.inset.top = length,
        StyleProp::Right => style.inset.right = length,
        StyleProp::Bottom => style.inset.bottom = length,
        StyleProp::Left => style.inset.left = length,
        _ => {}
    }
}

fn parse_font_weight_str(value: &str) -> Option<FontWeight> {
    match value.trim().to_ascii_lowercase().as_str() {
        "thin" => Some(FontWeight::Thin),
        "extra-light" | "extralight" | "ultra-light" | "ultralight" => Some(FontWeight::ExtraLight),
        "light" => Some(FontWeight::Light),
        "normal" | "regular" => Some(FontWeight::Regular),
        "medium" => Some(FontWeight::Medium),
        "semi-bold" | "semibold" | "demi-bold" | "demibold" => Some(FontWeight::SemiBold),
        "bold" => Some(FontWeight::Bold),
        "extra-bold" | "extrabold" | "ultra-bold" | "ultrabold" => Some(FontWeight::ExtraBold),
        "black" | "heavy" => Some(FontWeight::Black),
        value => value.parse::<f32>().ok().and_then(parse_font_weight_number),
    }
}

fn parse_font_weight_number(value: f32) -> Option<FontWeight> {
    if !value.is_finite() {
        return None;
    }
    let rounded = value.round();
    if (value - rounded).abs() > f32::EPSILON {
        return None;
    }
    let weight = rounded as i32;
    if weight % 100 != 0 {
        return None;
    }
    match weight {
        100 => Some(FontWeight::Thin),
        200 => Some(FontWeight::ExtraLight),
        300 => Some(FontWeight::Light),
        400 => Some(FontWeight::Regular),
        500 => Some(FontWeight::Medium),
        600 => Some(FontWeight::SemiBold),
        700 => Some(FontWeight::Bold),
        800 => Some(FontWeight::ExtraBold),
        900 => Some(FontWeight::Black),
        _ => None,
    }
}

// told todo remove after migrating
// fn set_bool_style_prop(node: &mut Node, prop: StyleProp, value: bool) {
//     match prop {
//         StyleProp::Scroll => {
//             if value {
//                 node.style.overflow_x = Overflow::Auto;
//                 node.style.overflow_y = Overflow::Auto;
//             } else {
//                 node.style.overflow_x = Overflow::Visible;
//                 node.style.overflow_y = Overflow::Visible;
//                 node.scroll_state = Default::default();
//             }
//         }
//         StyleProp::ScrollX => {
//             if value {
//                 node.style.overflow_x = Overflow::Auto;
//             } else {
//                 node.style.overflow_x = Overflow::Visible;
//                 if node.style.overflow_y == Overflow::Visible {
//                     node.scroll_state = Default::default();
//                 }
//             }
//         }
//         StyleProp::ScrollY => {
//             if value {
//                 node.style.overflow_y = Overflow::Auto;
//             } else {
//                 node.style.overflow_y = Overflow::Visible;
//                 if node.style.overflow_x == Overflow::Visible {
//                     node.scroll_state = Default::default();
//                 }
//             }
//         }
//         StyleProp::TextSelect => {
//             let text_selectable = value.into();
//             node.set_text_selectable(text_selectable);
//             node.interactivity.text_selectable = text_selectable);
//         }
//         _ => {}
//     }
// }

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

fn font_weight_to_number(weight: FontWeight) -> u16 {
    match weight {
        FontWeight::Thin => 100,
        FontWeight::ExtraLight => 200,
        FontWeight::Light => 300,
        FontWeight::Regular => 400,
        FontWeight::Medium => 500,
        FontWeight::SemiBold => 600,
        FontWeight::Bold => 700,
        FontWeight::ExtraBold => 800,
        FontWeight::Black => 900,
    }
}

#[cfg(test)]
mod tests {
    // use crate::element::ElementNode;

    // use super::{
    //     AttrValue, Display, FlexDirection, FontWeight, Node, Overflow, StyleVariant, UzStyle,
    //     parse_font_weight_number, parse_font_weight_str, set_bool_style_prop, set_flex_string,
    //     set_variant_bool, set_variant_flex_string,
    // };

    // #[test]
    // fn parses_font_weight_names() {
    //     assert_eq!(parse_font_weight_str("normal"), Some(FontWeight::Regular));
    //     assert_eq!(parse_font_weight_str("bold"), Some(FontWeight::Bold));
    //     assert_eq!(
    //         parse_font_weight_str("semi-bold"),
    //         Some(FontWeight::SemiBold)
    //     );
    //     assert_eq!(
    //         parse_font_weight_str("extraBold"),
    //         Some(FontWeight::ExtraBold)
    //     );
    //     assert_eq!(parse_font_weight_str("wat"), None);
    // }

    // #[test]
    // fn parses_exact_numeric_font_weights() {
    //     assert_eq!(parse_font_weight_str("700"), Some(FontWeight::Bold));
    //     assert_eq!(parse_font_weight_number(400.0), Some(FontWeight::Regular));
    //     assert_eq!(parse_font_weight_number(750.0), None);
    //     assert_eq!(parse_font_weight_number(0.0), None);
    // }

    // #[test]
    // fn flex_string_sets_display_and_direction() {
    //     let mut style = UzStyle::default();

    //     assert!(set_flex_string(&mut style, "col"));
    //     assert_eq!(style.display, Display::Flex);
    //     assert_eq!(style.flex_direction, FlexDirection::Column);

    //     assert!(set_flex_string(&mut style, "row"));
    //     assert_eq!(style.display, Display::Flex);
    //     assert_eq!(style.flex_direction, FlexDirection::Row);
    // }

    // #[test]
    // fn variant_flex_string_sets_display_and_direction() {
    //     let mut node = Node::new(UzStyle::default(), ElementNode::new_view());

    //     assert!(set_variant_flex_string(
    //         &mut node,
    //         StyleVariant::Hover,
    //         "col"
    //     ));

    //     let hover = node.style_variants.hover_style.as_ref().unwrap();
    //     assert_eq!(hover.display, Some(Display::Flex));
    //     assert_eq!(hover.flex_direction, Some(FlexDirection::Column));
    // }

    // #[test]
    // fn scroll_sets_both_axes_to_auto() {
    //     let mut node = Node::new(UzStyle::default(), ElementNode::new_view());

    //     set_bool_style_prop(&mut node, super::StyleProp::Scroll, true);

    //     assert_eq!(node.style.overflow_x, Overflow::Auto);
    //     assert_eq!(node.style.overflow_y, Overflow::Auto);
    //     assert_eq!(node.scroll_state.scroll_offset_x, 0.0);
    //     assert_eq!(node.scroll_state.scroll_offset_y, 0.0);
    // }

    // #[test]
    // fn variant_scroll_sets_both_axes_to_auto() {
    //     let mut node = Node::new(UzStyle::default(), ElementNode::new_view());

    //     set_variant_bool(
    //         &mut node,
    //         super::StyleProp::Scroll,
    //         StyleVariant::Hover,
    //         true,
    //     );

    //     let hover = node.style_variants.hover_style.as_ref().unwrap();
    //     assert_eq!(hover.overflow_x, Some(Overflow::Auto));
    //     assert_eq!(hover.overflow_y, Some(Overflow::Auto));
    // }

    // #[test]
    // fn string_scroll_true_sets_both_axes_to_auto() {
    //     let mut node = Node::new(UzStyle::default(), ElementNode::new_view());

    //     set_style_attr(
    //         &mut node,
    //         super::StyleProp::Scroll,
    //         StyleVariant::Base,
    //         AttrValue::String("true".into()),
    //         16.0,
    //     );

    //     assert_eq!(node.style.overflow_x, Overflow::Auto);
    //     assert_eq!(node.style.overflow_y, Overflow::Auto);
    // }

    // #[test]
    // fn string_scroll_false_clears_both_axes() {
    //     let mut node = Node::new(UzStyle::default(), ElementNode::new_view());
    //     node.style.overflow_x = Overflow::Auto;
    //     node.style.overflow_y = Overflow::Auto;
    //     node.scroll_state.scroll_offset_x = 10.0;
    //     node.scroll_state.scroll_offset_y = 20.0;

    //     set_style_attr(
    //         &mut node,
    //         super::StyleProp::Scroll,
    //         StyleVariant::Base,
    //         AttrValue("false".into()),
    //         16.0,
    //     );

    //     assert_eq!(node.style.overflow_x, Overflow::Visible);
    //     assert_eq!(node.style.overflow_y, Overflow::Visible);
    //     assert_eq!(node.scroll_state.scroll_offset_x, 0.0);
    //     assert_eq!(node.scroll_state.scroll_offset_y, 0.0);
    // }
}
