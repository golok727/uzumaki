use std::str::FromStr;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum StyleVariant {
    Base,
    Hover,
    Active,
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
    FlexGrow,
    FlexShrink,
    Items,
    Justify,
    Gap,
    Bg,
    Color,
    FontSize,
    FontWeight,
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
    Opacity,
    Display,
    Cursor,
    Interactive,
    Visibility,
    Scrollable,
    TextSelect,
    OverflowWrap,
    WordBreak,
    Position,
    Top,
    Right,
    Bottom,
    Left,
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
            "flexGrow" => Self::FlexGrow,
            "flexShrink" => Self::FlexShrink,
            "items" => Self::Items,
            "justify" => Self::Justify,
            "gap" => Self::Gap,
            "bg" => Self::Bg,
            "color" => Self::Color,
            "fontSize" => Self::FontSize,
            "fontWeight" => Self::FontWeight,
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
            "opacity" => Self::Opacity,
            "display" => Self::Display,
            "cursor" => Self::Cursor,
            "interactive" => Self::Interactive,
            "visibility" => Self::Visibility,
            "scrollable" => Self::Scrollable,
            "selectable" => Self::TextSelect,
            "overflowWrap" => Self::OverflowWrap,
            "wordBreak" => Self::WordBreak,
            "position" => Self::Position,
            "top" => Self::Top,
            "right" => Self::Right,
            "bottom" => Self::Bottom,
            "left" => Self::Left,
            _ => return Err(()),
        })
    }
}

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
            _ => return Err(()),
        })
    }
}
