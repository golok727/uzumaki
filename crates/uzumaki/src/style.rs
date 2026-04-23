use parley::{LineHeight, StyleProperty};
use refineable::Refineable;
use vello::Scene;
use vello::kurbo::{Affine, Rect, RoundedRect, RoundedRectRadii, Stroke};
use vello::peniko::Color as VelloColor;

use crate::cursor::UzCursorIcon;
use crate::text::TextBrush;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Default for Color {
    fn default() -> Self {
        Self::TRANSPARENT
    }
}

impl Color {
    pub const TRANSPARENT: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn to_vello(self) -> VelloColor {
        VelloColor::from_rgba8(self.r, self.g, self.b, self.a)
    }

    pub fn with_opacity(self, opacity: f32) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a: (self.a as f32 * opacity.clamp(0.0, 1.0)) as u8,
        }
    }

    pub fn is_transparent(self) -> bool {
        self.a == 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Bounds {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px <= self.x + self.width && py >= self.y && py <= self.y + self.height
    }

    pub fn to_rect(&self) -> Rect {
        Rect::new(self.x, self.y, self.x + self.width, self.y + self.height)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Refineable)]
#[refineable(Debug)]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Edges {
    pub fn all(val: f32) -> Self {
        Self {
            top: val,
            right: val,
            bottom: val,
            left: val,
        }
    }

    pub fn any_nonzero(&self) -> bool {
        self.top > 0.0 || self.right > 0.0 || self.bottom > 0.0 || self.left > 0.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Refineable)]
#[refineable(Debug)]
pub struct Corners {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl Corners {
    pub fn uniform(val: f32) -> Self {
        Self {
            top_left: val,
            top_right: val,
            bottom_right: val,
            bottom_left: val,
        }
    }

    pub fn any_nonzero(&self) -> bool {
        self.top_left > 0.0
            || self.top_right > 0.0
            || self.bottom_right > 0.0
            || self.bottom_left > 0.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxShadow {
    pub color: Color,
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur_radius: f32,
    pub spread_radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Length {
    #[default]
    Auto,
    Px(f32),
    Percent(f32),
}

impl Length {
    pub fn px(val: f32) -> Self {
        Length::Px(val)
    }

    pub fn percent(val: f32) -> Self {
        Length::Percent(val)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DefiniteLength {
    Px(f32),
    Percent(f32),
}

impl Default for DefiniteLength {
    fn default() -> Self {
        DefiniteLength::Px(0.0)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Display {
    None,
    #[default]
    Flex,
    Block,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Position {
    #[default]
    Relative,
    Absolute,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Visibility {
    #[default]
    Visible,
    Hidden,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum AlignItems {
    FlexStart,
    FlexEnd,
    Center,
    #[default]
    Stretch,
    Baseline,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum JustifyContent {
    #[default]
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum AlignSelf {
    #[default]
    Auto,
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
    Baseline,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Scroll,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Refineable)]
#[refineable(Debug)]
pub struct Size {
    pub width: Length,
    pub height: Length,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Refineable)]
#[refineable(Debug)]
pub struct GapSize {
    pub width: DefiniteLength,
    pub height: DefiniteLength,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    #[default]
    Regular,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}

impl FontWeight {
    pub fn to_parley(self) -> parley::FontWeight {
        match self {
            Self::Thin => parley::FontWeight::THIN,
            Self::ExtraLight => parley::FontWeight::EXTRA_LIGHT,
            Self::Light => parley::FontWeight::LIGHT,
            Self::Regular => parley::FontWeight::NORMAL,
            Self::Medium => parley::FontWeight::MEDIUM,
            Self::SemiBold => parley::FontWeight::SEMI_BOLD,
            Self::Bold => parley::FontWeight::BOLD,
            Self::ExtraBold => parley::FontWeight::EXTRA_BOLD,
            Self::Black => parley::FontWeight::BLACK,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OverflowWrap {
    Normal,
    Anywhere,
    #[default]
    BreakWord,
}

impl OverflowWrap {
    pub fn to_parley(self) -> parley::OverflowWrap {
        match self {
            Self::Normal => parley::OverflowWrap::Normal,
            Self::Anywhere => parley::OverflowWrap::Anywhere,
            Self::BreakWord => parley::OverflowWrap::BreakWord,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WordBreak {
    #[default]
    Normal,
    BreakAll,
    KeepAll,
}

impl WordBreak {
    pub fn to_parley(self) -> parley::WordBreak {
        match self {
            Self::Normal => parley::WordBreak::Normal,
            Self::BreakAll => parley::WordBreak::BreakAll,
            Self::KeepAll => parley::WordBreak::KeepAll,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Refineable)]
#[refineable(Debug)]
pub struct TextStyle {
    pub font_size: f32,
    pub color: Color,
    pub line_height: f32,
    pub font_weight: FontWeight,
    pub letter_spacing: f32,
    pub word_spacing: f32,
    pub overflow_wrap: OverflowWrap,
    pub word_break: WordBreak,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextSelectable {
    #[default]
    Inherit,
    True,
    False,
}

impl From<bool> for TextSelectable {
    fn from(value: bool) -> Self {
        if value { Self::True } else { Self::False }
    }
}

impl TextSelectable {
    pub fn as_value(&self) -> Option<bool> {
        (!matches!(self, Self::Inherit)).then_some(self == &Self::True)
    }

    pub fn selectable(&self) -> bool {
        self == &Self::True
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size: 16.0,
            color: Color::WHITE,
            line_height: 1.2,
            font_weight: FontWeight::default(),
            letter_spacing: 0.0,
            word_spacing: 0.0,
            overflow_wrap: OverflowWrap::default(),
            word_break: WordBreak::default(),
        }
    }
}

impl TextStyle {
    pub fn to_parley_styles(&self) -> impl Iterator<Item = StyleProperty<'static, TextBrush>> {
        let letter_spacing = (self.letter_spacing != 0.0)
            .then_some(StyleProperty::LetterSpacing(self.letter_spacing));
        let word_spacing =
            (self.word_spacing != 0.0).then_some(StyleProperty::WordSpacing(self.word_spacing));

        [
            StyleProperty::FontSize(self.font_size),
            StyleProperty::LineHeight(LineHeight::FontSizeRelative(self.line_height)),
            StyleProperty::FontWeight(self.font_weight.to_parley()),
            StyleProperty::OverflowWrap(self.overflow_wrap.to_parley()),
            StyleProperty::WordBreak(self.word_break.to_parley()),
        ]
        .into_iter()
        .chain(letter_spacing)
        .chain(word_spacing)
    }
}

#[derive(Clone, Debug, PartialEq, Refineable)]
#[refineable(Debug)]
pub struct UzStyle {
    // Visibility
    pub display: Display,
    pub visibility: Visibility,
    pub position: Position,
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,

    // Sizing
    #[refineable]
    pub size: Size,
    #[refineable]
    pub min_size: Size,
    #[refineable]
    pub max_size: Size,
    pub aspect_ratio: Option<f32>,

    // Spacing
    #[refineable]
    pub margin: Edges,
    #[refineable]
    pub padding: Edges,
    #[refineable]
    pub inset: Edges,

    // Flex layout
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Length,
    pub align_items: Option<AlignItems>,
    pub align_self: Option<AlignSelf>,
    pub justify_content: Option<JustifyContent>,
    #[refineable]
    pub gap: GapSize,

    // Visual
    pub background: Option<Color>,
    pub border_color: Option<Color>,
    #[refineable]
    pub border_widths: Edges,
    #[refineable]
    pub corner_radii: Corners,
    pub opacity: f32,
    pub box_shadow: Option<BoxShadow>,

    pub cursor: Option<UzCursorIcon>,

    // Text (inherited)
    #[refineable]
    pub text: TextStyle,

    /// Whether text within this element is selectable.
    /// None = inherit from parent (default). Some(true) = selectable, Some(false) = not.
    /// toro move to style
    pub text_selectable: TextSelectable,
}

impl Default for UzStyle {
    fn default() -> Self {
        Self {
            display: Display::default(),
            visibility: Visibility::default(),
            position: Position::default(),
            overflow_x: Overflow::default(),
            overflow_y: Overflow::default(),

            size: Size::default(),
            min_size: Size::default(),
            max_size: Size::default(),
            aspect_ratio: None,

            margin: Edges::default(),
            padding: Edges::default(),
            inset: Edges::default(),

            flex_direction: FlexDirection::default(),
            flex_wrap: FlexWrap::default(),
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Length::Auto,
            align_items: None,
            align_self: None,
            justify_content: None,
            gap: GapSize::default(),

            background: None,
            border_color: None,
            border_widths: Edges::default(),
            corner_radii: Corners::default(),
            opacity: 1.0,
            box_shadow: None,

            cursor: None,

            text: TextStyle::default(),
            text_selectable: TextSelectable::Inherit,
        }
    }
}

impl UzStyle {
    pub fn to_taffy(&self) -> taffy::Style {
        taffy::Style {
            display: match self.display {
                Display::None => taffy::Display::None,
                Display::Flex => taffy::Display::Flex,
                Display::Block => taffy::Display::Block,
            },
            position: match self.position {
                Position::Relative => taffy::Position::Relative,
                Position::Absolute => taffy::Position::Absolute,
            },
            overflow: taffy::Point {
                x: overflow_to_taffy(self.overflow_x),
                y: overflow_to_taffy(self.overflow_y),
            },
            size: taffy::Size {
                width: length_to_dimension(self.size.width),
                height: length_to_dimension(self.size.height),
            },
            min_size: taffy::Size {
                width: length_to_dimension(self.min_size.width),
                height: length_to_dimension(self.min_size.height),
            },
            max_size: taffy::Size {
                width: length_to_dimension(self.max_size.width),
                height: length_to_dimension(self.max_size.height),
            },
            aspect_ratio: self.aspect_ratio,
            margin: edges_to_taffy_lp_auto(&self.margin),
            padding: edges_to_taffy_lp(&self.padding),
            border: edges_to_taffy_lp(&self.border_widths),
            inset: edges_to_taffy_lp_auto(&self.inset),
            flex_direction: match self.flex_direction {
                FlexDirection::Row => taffy::FlexDirection::Row,
                FlexDirection::Column => taffy::FlexDirection::Column,
                FlexDirection::RowReverse => taffy::FlexDirection::RowReverse,
                FlexDirection::ColumnReverse => taffy::FlexDirection::ColumnReverse,
            },
            flex_wrap: match self.flex_wrap {
                FlexWrap::NoWrap => taffy::FlexWrap::NoWrap,
                FlexWrap::Wrap => taffy::FlexWrap::Wrap,
                FlexWrap::WrapReverse => taffy::FlexWrap::WrapReverse,
            },
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            flex_basis: length_to_dimension(self.flex_basis),
            align_items: self.align_items.map(align_items_to_taffy),
            align_self: self.align_self.map(align_self_to_taffy),
            justify_content: self.justify_content.map(justify_content_to_taffy),
            gap: taffy::Size {
                width: definite_to_taffy(self.gap.width),
                height: definite_to_taffy(self.gap.height),
            },
            ..taffy::Style::default()
        }
    }

    /// Paint the visual properties into the scene at bounds.
    /// `continuation` is called between background and borders to paint children.
    pub fn paint(
        &self,
        bounds: Bounds,
        scene: &mut Scene,
        scale: f64,
        continuation: impl FnOnce(&mut Scene),
    ) {
        if self.visibility == Visibility::Hidden || self.opacity <= 0.0 {
            return;
        }

        let opacity = self.opacity;

        // 1. Box shadow
        if let Some(shadow) = &self.box_shadow {
            self.paint_shadow(bounds, scene, shadow, opacity, scale);
        }

        // 2. Background
        if let Some(bg) = self.background
            && !bg.is_transparent()
        {
            let vbg = bg.with_opacity(opacity).to_vello();
            if self.corner_radii.any_nonzero() {
                let shape = rounded_rect(bounds, &self.corner_radii);
                scene.fill(
                    vello::peniko::Fill::NonZero,
                    Affine::scale(scale),
                    vbg,
                    None,
                    &shape,
                );
            } else {
                scene.fill(
                    vello::peniko::Fill::NonZero,
                    Affine::scale(scale),
                    vbg,
                    None,
                    &bounds.to_rect(),
                );
            }
        }

        // 3. Children
        continuation(scene);

        // 4. Borders
        if self.border_widths.any_nonzero()
            && let Some(bc) = self.border_color
            && !bc.is_transparent()
        {
            let vbc = bc.with_opacity(opacity).to_vello();
            if self.corner_radii.any_nonzero() {
                self.paint_rounded_borders(bounds, scene, vbc, scale);
            } else {
                self.paint_rect_borders(bounds, scene, vbc, scale);
            }
        }
    }

    fn paint_shadow(
        &self,
        bounds: Bounds,
        scene: &mut Scene,
        shadow: &BoxShadow,
        opacity: f32,
        scale: f64,
    ) {
        let spread = shadow.spread_radius as f64;
        let ox = shadow.offset_x as f64;
        let oy = shadow.offset_y as f64;
        let blur = shadow.blur_radius as f64;

        let expanded = Bounds::new(
            bounds.x + ox - spread - blur * 0.5,
            bounds.y + oy - spread - blur * 0.5,
            bounds.width + spread * 2.0 + blur,
            bounds.height + spread * 2.0 + blur,
        );

        let vc = shadow.color.with_opacity(opacity).to_vello();

        if self.corner_radii.any_nonzero() {
            let shape = rounded_rect(expanded, &self.corner_radii);
            scene.fill(
                vello::peniko::Fill::NonZero,
                Affine::scale(scale),
                vc,
                None,
                &shape,
            );
        } else {
            scene.fill(
                vello::peniko::Fill::NonZero,
                Affine::scale(scale),
                vc,
                None,
                &expanded.to_rect(),
            );
        }
    }

    fn paint_rounded_borders(
        &self,
        bounds: Bounds,
        scene: &mut Scene,
        color: VelloColor,
        scale: f64,
    ) {
        let bw = &self.border_widths;

        if let Some(width) = border_widths_equal(bw) {
            if width > 0.0 {
                let shape = rounded_rect(bounds, &self.corner_radii);
                scene.stroke(
                    &Stroke::new(width as f64),
                    Affine::scale(scale),
                    color,
                    None,
                    &shape,
                );
            }
            return;
        }

        // Fill outer, carve inner
        let outer = rounded_rect(bounds, &self.corner_radii);
        scene.fill(
            vello::peniko::Fill::NonZero,
            Affine::scale(scale),
            color,
            None,
            &outer,
        );

        let inner_rect = Rect::new(
            bounds.x + bw.left as f64,
            bounds.y + bw.top as f64,
            bounds.x + bounds.width - bw.right as f64,
            bounds.y + bounds.height - bw.bottom as f64,
        );
        if inner_rect.width() <= 0.0 || inner_rect.height() <= 0.0 {
            return;
        }

        let inner_radii = inset_radii(&self.corner_radii, bw);
        let inner = rounded_rect(
            Bounds::new(
                inner_rect.x0,
                inner_rect.y0,
                inner_rect.width(),
                inner_rect.height(),
            ),
            &inner_radii,
        );

        let bg = self.background.unwrap_or(Color::TRANSPARENT).to_vello();
        scene.fill(
            vello::peniko::Fill::NonZero,
            Affine::scale(scale),
            bg,
            None,
            &inner,
        );
    }

    fn paint_rect_borders(&self, bounds: Bounds, scene: &mut Scene, color: VelloColor, scale: f64) {
        let bw = &self.border_widths;
        let x = bounds.x;
        let y = bounds.y;
        let w = bounds.width;
        let h = bounds.height;

        if let Some(width) = border_widths_equal(bw) {
            if width > 0.0 {
                scene.stroke(
                    &Stroke::new(width as f64),
                    Affine::scale(scale),
                    color,
                    None,
                    &Rect::new(x, y, x + w, y + h),
                );
            }
            return;
        }

        let fill = |scene: &mut Scene, rect: Rect| {
            scene.fill(
                vello::peniko::Fill::NonZero,
                Affine::scale(scale),
                color,
                None,
                &rect,
            );
        };

        if bw.top > 0.0 {
            fill(scene, Rect::new(x, y, x + w, y + bw.top as f64));
        }
        if bw.bottom > 0.0 {
            fill(scene, Rect::new(x, y + h - bw.bottom as f64, x + w, y + h));
        }
        if bw.left > 0.0 {
            fill(scene, Rect::new(x, y, x + bw.left as f64, y + h));
        }
        if bw.right > 0.0 {
            fill(scene, Rect::new(x + w - bw.right as f64, y, x + w, y + h));
        }
    }
}

fn border_widths_equal(bw: &Edges) -> Option<f32> {
    let first = bw.top;
    if first > 0.0
        && (bw.right - first).abs() < f32::EPSILON
        && (bw.bottom - first).abs() < f32::EPSILON
        && (bw.left - first).abs() < f32::EPSILON
    {
        Some(first)
    } else {
        None
    }
}

fn rounded_rect(bounds: Bounds, radii: &Corners) -> RoundedRect {
    let w = bounds.width;
    let h = bounds.height;
    let clamp = |r: f32| r.max(0.0).min(w as f32 * 0.5).min(h as f32 * 0.5);
    let rect = bounds.to_rect();
    let rr = RoundedRectRadii::new(
        clamp(radii.top_left) as f64,
        clamp(radii.top_right) as f64,
        clamp(radii.bottom_right) as f64,
        clamp(radii.bottom_left) as f64,
    );
    RoundedRect::from_rect(rect, rr)
}

fn inset_radii(radii: &Corners, widths: &Edges) -> Corners {
    Corners {
        top_left: (radii.top_left - widths.top.max(widths.left)).max(0.0),
        top_right: (radii.top_right - widths.top.max(widths.right)).max(0.0),
        bottom_right: (radii.bottom_right - widths.bottom.max(widths.right)).max(0.0),
        bottom_left: (radii.bottom_left - widths.bottom.max(widths.left)).max(0.0),
    }
}

fn length_to_dimension(l: Length) -> taffy::Dimension {
    match l {
        Length::Auto => taffy::Dimension::auto(),
        Length::Px(v) => taffy::Dimension::length(v),
        Length::Percent(v) => taffy::Dimension::percent(v),
    }
}

fn definite_to_taffy(l: DefiniteLength) -> taffy::LengthPercentage {
    match l {
        DefiniteLength::Px(v) => taffy::LengthPercentage::length(v),
        DefiniteLength::Percent(v) => taffy::LengthPercentage::percent(v),
    }
}

fn edges_to_taffy_lp_auto(e: &Edges) -> taffy::Rect<taffy::LengthPercentageAuto> {
    taffy::Rect {
        left: taffy::LengthPercentageAuto::length(e.left),
        right: taffy::LengthPercentageAuto::length(e.right),
        top: taffy::LengthPercentageAuto::length(e.top),
        bottom: taffy::LengthPercentageAuto::length(e.bottom),
    }
}

fn edges_to_taffy_lp(e: &Edges) -> taffy::Rect<taffy::LengthPercentage> {
    taffy::Rect {
        left: taffy::LengthPercentage::length(e.left),
        right: taffy::LengthPercentage::length(e.right),
        top: taffy::LengthPercentage::length(e.top),
        bottom: taffy::LengthPercentage::length(e.bottom),
    }
}

fn overflow_to_taffy(o: Overflow) -> taffy::Overflow {
    match o {
        Overflow::Visible => taffy::Overflow::Visible,
        Overflow::Hidden => taffy::Overflow::Hidden,
        Overflow::Scroll => taffy::Overflow::Scroll,
    }
}

fn align_items_to_taffy(a: AlignItems) -> taffy::AlignItems {
    match a {
        AlignItems::FlexStart => taffy::AlignItems::FlexStart,
        AlignItems::FlexEnd => taffy::AlignItems::FlexEnd,
        AlignItems::Center => taffy::AlignItems::Center,
        AlignItems::Stretch => taffy::AlignItems::Stretch,
        AlignItems::Baseline => taffy::AlignItems::Baseline,
    }
}

fn align_self_to_taffy(a: AlignSelf) -> taffy::AlignSelf {
    match a {
        AlignSelf::Auto => taffy::AlignSelf::Start,
        AlignSelf::FlexStart => taffy::AlignSelf::FlexStart,
        AlignSelf::FlexEnd => taffy::AlignSelf::FlexEnd,
        AlignSelf::Center => taffy::AlignSelf::Center,
        AlignSelf::Stretch => taffy::AlignSelf::Stretch,
        AlignSelf::Baseline => taffy::AlignSelf::Baseline,
    }
}

fn justify_content_to_taffy(j: JustifyContent) -> taffy::JustifyContent {
    match j {
        JustifyContent::FlexStart => taffy::JustifyContent::FlexStart,
        JustifyContent::FlexEnd => taffy::JustifyContent::FlexEnd,
        JustifyContent::Center => taffy::JustifyContent::Center,
        JustifyContent::SpaceBetween => taffy::JustifyContent::SpaceBetween,
        JustifyContent::SpaceAround => taffy::JustifyContent::SpaceAround,
        JustifyContent::SpaceEvenly => taffy::JustifyContent::SpaceEvenly,
    }
}
