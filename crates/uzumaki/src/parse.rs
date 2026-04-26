use crate::style::*;

pub(crate) fn parse_bool(value: &str) -> bool {
    !matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "" | "0" | "false" | "hidden" | "none" | "no" | "off"
    )
}

pub(crate) fn parse_max_length(value: f32) -> Option<usize> {
    (value.is_finite() && value > 0.0).then_some(value as usize)
}

pub(crate) fn parse_px_scalar(value: &str, rem_base: f32) -> Option<f32> {
    let value = value.trim();
    if let Some(value) = value.strip_suffix("rem") {
        return value.trim().parse::<f32>().ok().map(|v| v * rem_base);
    }
    if let Some(value) = value.strip_suffix("px") {
        return value.trim().parse().ok();
    }
    value.parse().ok()
}

pub(crate) fn parse_length(value: &str, rem_base: f32) -> Option<Length> {
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

pub(crate) fn parse_definite_length(value: &str, rem_base: f32) -> Option<DefiniteLength> {
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

pub(crate) fn parse_color(value: &str) -> Option<Color> {
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
