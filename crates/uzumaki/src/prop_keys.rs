use std::str::FromStr;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum StyleVariant {
    Base,
    Hover,
    Active,
    Focus,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum StyleProp {
    W,
    H,
    MinW,
    MinH,
    P,
    Px,
    Py,
    Pt,
    Pb,
    Pl,
    Pr,
    M,
    Mx,
    My,
    Mt,
    Mb,
    Ml,
    Mr,
    Flex,
    FlexDir,
    FlexWrap,
    FlexGrow,
    FlexShrink,
    Items,
    Justify,
    Gap,
    Bg,
    Color,
    FontSize,
    FontWeight,
    FontFamily,
    Rounded,
    RoundedTL,
    RoundedTR,
    RoundedBR,
    RoundedBL,
    Border,
    BorderTop,
    BorderRight,
    BorderBottom,
    BorderLeft,
    BorderColor,
    Outline,
    OutlineColor,
    OutlineOffset,
    Opacity,
    Display,
    Cursor,
    Interactive,
    Visibility,
    Scroll,
    ScrollX,
    ScrollY,
    TextSelect,
    TextWrap,
    WordBreak,
    Position,
    Top,
    Right,
    Bottom,
    Left,
    TranslateX,
    TranslateY,
    Rotate,
    Scale,
    ScaleX,
    ScaleY,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum ElementProp {
    Value,
    Placeholder,
    Disabled,
    MaxLength,
    Multiline,
    Secure,
    Checked,
    Focusable,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum AttributeKind {
    Style(StyleProp, StyleVariant),
    Element(ElementProp),
}

impl FromStr for AttributeKind {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if let Ok(ep) = value.parse::<ElementProp>() {
            return Ok(AttributeKind::Element(ep));
        }

        if let Some(rest) = value.strip_prefix("hover:") {
            return rest
                .parse::<StyleProp>()
                .map(|p| AttributeKind::Style(p, StyleVariant::Hover));
        }
        if let Some(rest) = value.strip_prefix("active:") {
            return rest
                .parse::<StyleProp>()
                .map(|p| AttributeKind::Style(p, StyleVariant::Active));
        }
        if let Some(rest) = value.strip_prefix("focus:") {
            return rest
                .parse::<StyleProp>()
                .map(|p| AttributeKind::Style(p, StyleVariant::Focus));
        }

        value
            .parse::<StyleProp>()
            .map(|p| AttributeKind::Style(p, StyleVariant::Base))
    }
}

impl FromStr for StyleProp {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "w" => Self::W,
            "h" => Self::H,
            "minW" => Self::MinW,
            "minH" => Self::MinH,
            "p" => Self::P,
            "px" => Self::Px,
            "py" => Self::Py,
            "pt" => Self::Pt,
            "pb" => Self::Pb,
            "pl" => Self::Pl,
            "pr" => Self::Pr,
            "m" => Self::M,
            "mx" => Self::Mx,
            "my" => Self::My,
            "mt" => Self::Mt,
            "mb" => Self::Mb,
            "ml" => Self::Ml,
            "mr" => Self::Mr,
            "flex" => Self::Flex,
            "flexDir" => Self::FlexDir,
            "flexWrap" => Self::FlexWrap,
            "flexGrow" => Self::FlexGrow,
            "flexShrink" => Self::FlexShrink,
            "items" => Self::Items,
            "justify" => Self::Justify,
            "gap" => Self::Gap,
            "bg" => Self::Bg,
            "color" => Self::Color,
            "fontSize" => Self::FontSize,
            "fontWeight" => Self::FontWeight,
            "fontFamily" => Self::FontFamily,
            "rounded" => Self::Rounded,
            "roundedTL" => Self::RoundedTL,
            "roundedTR" => Self::RoundedTR,
            "roundedBR" => Self::RoundedBR,
            "roundedBL" => Self::RoundedBL,
            "border" => Self::Border,
            "borderTop" => Self::BorderTop,
            "borderRight" => Self::BorderRight,
            "borderBottom" => Self::BorderBottom,
            "borderLeft" => Self::BorderLeft,
            "borderColor" => Self::BorderColor,
            "outline" => Self::Outline,
            "outlineColor" => Self::OutlineColor,
            "outlineOffset" => Self::OutlineOffset,
            "opacity" => Self::Opacity,
            "display" => Self::Display,
            "cursor" => Self::Cursor,
            "interactive" => Self::Interactive,
            "visibility" => Self::Visibility,
            "scroll" | "scrollable" => Self::Scroll,
            "scrollX" | "scrollableX" => Self::ScrollX,
            "scrollY" | "scrollableY" => Self::ScrollY,
            "selectable" => Self::TextSelect,
            "textWrap" => Self::TextWrap,
            "wordBreak" => Self::WordBreak,
            "position" => Self::Position,
            "top" => Self::Top,
            "right" => Self::Right,
            "bottom" => Self::Bottom,
            "left" => Self::Left,
            "translateX" => Self::TranslateX,
            "translateY" => Self::TranslateY,
            "rotate" => Self::Rotate,
            "scale" => Self::Scale,
            "scaleX" => Self::ScaleX,
            "scaleY" => Self::ScaleY,
            _ => return Err(()),
        })
    }
}

// we should move these to Core*Element
impl FromStr for ElementProp {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "value" => Self::Value,
            "placeholder" => Self::Placeholder,
            "disabled" => Self::Disabled,
            "maxLength" => Self::MaxLength,
            "multiline" => Self::Multiline,
            "secure" => Self::Secure,
            "checked" => Self::Checked,
            "focusable" => Self::Focusable,
            _ => return Err(()),
        })
    }
}
