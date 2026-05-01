use parley::{
    Affinity as ParleyAffinity, BoundingBox, Cursor, FontContext, Layout, LayoutContext, Selection,
};
use unicode_segmentation::UnicodeSegmentation;
use vello::Scene;
use vello::kurbo::Affine;
use vello::peniko::{Brush, Color, Fill};

use crate::style::TextStyle;

impl From<crate::selection::Affinity> for ParleyAffinity {
    fn from(value: crate::selection::Affinity) -> Self {
        match value {
            crate::selection::Affinity::Downstream => Self::Downstream,
            crate::selection::Affinity::Upstream => Self::Upstream,
        }
    }
}

impl From<ParleyAffinity> for crate::selection::Affinity {
    fn from(value: ParleyAffinity) -> Self {
        match value {
            ParleyAffinity::Downstream => Self::Downstream,
            ParleyAffinity::Upstream => Self::Upstream,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TextBrush;

pub struct TextRenderer {
    pub font_ctx: FontContext,
    pub layout_ctx: LayoutContext<TextBrush>,
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextRenderer {
    pub fn new() -> Self {
        let mut font_ctx = FontContext::default();

        let roboto = include_bytes!("../assets/Roboto-Regular.ttf");
        font_ctx
            .collection
            .register_fonts(roboto.to_vec().into(), None);

        Self {
            font_ctx,
            layout_ctx: LayoutContext::new(),
        }
    }

    fn build_layout(
        &mut self,
        text: &str,
        style: &TextStyle,
        max_width: Option<f32>,
    ) -> Layout<TextBrush> {
        let mut builder = self
            .layout_ctx
            .ranged_builder(&mut self.font_ctx, text, 1.0, true);
        for prop in style.to_parley_styles() {
            builder.push_default(prop);
        }
        let mut layout = builder.build(text);
        layout.break_all_lines(max_width);
        layout
    }

    fn grapheme_boundaries(text: &str) -> Vec<usize> {
        let mut boundaries = Vec::with_capacity(text.graphemes(true).count() + 1);
        boundaries.push(0);
        let mut byte_offset = 0;
        for grapheme in text.graphemes(true) {
            byte_offset += grapheme.len();
            boundaries.push(byte_offset);
        }
        boundaries
    }

    fn grapheme_to_byte(boundaries: &[usize], grapheme_index: usize) -> usize {
        boundaries
            .get(grapheme_index)
            .copied()
            .unwrap_or_else(|| *boundaries.last().unwrap_or(&0))
    }

    fn byte_to_grapheme(boundaries: &[usize], byte_index: usize) -> usize {
        boundaries
            .partition_point(|&boundary| boundary <= byte_index)
            .saturating_sub(1)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_text(
        &mut self,
        scene: &mut Scene,
        text: &str,
        style: &TextStyle,
        width: f32,
        height: f32,
        position: (f32, f32),
        color: Color,
        transform: Affine,
    ) {
        let _ = height;
        let layout = self.build_layout(text, style, Some(width));
        let (px, py) = position;

        for line in layout.lines() {
            for item in line.items() {
                if let parley::PositionedLayoutItem::GlyphRun(glyph_run) = item {
                    let run = glyph_run.run();
                    let font = run.font().clone();
                    let run_font_size = run.font_size();
                    let synthesis = run.synthesis();
                    let glyph_xform = synthesis
                        .skew()
                        .map(|angle| Affine::skew(angle.to_radians().tan() as f64, 0.0));

                    scene
                        .draw_glyphs(&font)
                        .font_size(run_font_size)
                        .transform(transform)
                        .glyph_transform(glyph_xform)
                        .brush(&Brush::Solid(color))
                        .draw(
                            Fill::NonZero,
                            glyph_run.positioned_glyphs().map(|g| vello::Glyph {
                                id: g.id,
                                x: px + g.x,
                                y: py + g.y,
                            }),
                        );
                }
            }
        }
    }

    pub fn grapheme_x_positions(&mut self, text: &str, style: &TextStyle) -> Vec<f32> {
        if text.is_empty() {
            return vec![0.0];
        }

        let layout = self.build_layout(text, style, None);
        let layout_width = layout.width();
        let boundaries = Self::grapheme_boundaries(text);

        let mut positions = Vec::with_capacity(boundaries.len());
        positions.push(0.0);
        for &byte_offset in boundaries.iter().skip(1) {
            let cursor = Cursor::from_byte_index(&layout, byte_offset, ParleyAffinity::Downstream);
            let geom = cursor.geometry(&layout, layout_width);
            positions.push(geom.x0 as f32);
        }

        positions
    }

    pub fn hit_to_grapheme(&mut self, text: &str, style: &TextStyle, x: f32) -> usize {
        self.hit_to_grapheme_2d(text, style, None, x, 0.0)
    }

    pub fn hit_to_grapheme_2d(
        &mut self,
        text: &str,
        style: &TextStyle,
        wrap_width: Option<f32>,
        x: f32,
        y: f32,
    ) -> usize {
        if text.is_empty() {
            return 0;
        }

        let layout = self.build_layout(text, style, wrap_width);
        let boundaries = Self::grapheme_boundaries(text);
        let cursor = Cursor::from_point(&layout, x, y);
        Self::byte_to_grapheme(&boundaries, cursor.index())
    }

    pub fn hit_to_text_position(
        &mut self,
        text: &str,
        style: &TextStyle,
        wrap_width: Option<f32>,
        x: f32,
        y: f32,
    ) -> (usize, crate::selection::Affinity) {
        if text.is_empty() {
            return (0, crate::selection::Affinity::Downstream);
        }

        let layout = self.build_layout(text, style, wrap_width);
        let cursor = Cursor::from_point(&layout, x, y);
        (cursor.index().min(text.len()), cursor.affinity().into())
    }

    pub fn cursor_geometry(
        &mut self,
        text: &str,
        style: &TextStyle,
        wrap_width: Option<f32>,
        grapheme_index: usize,
    ) -> BoundingBox {
        let layout = self.build_layout(text, style, wrap_width);
        let boundaries = Self::grapheme_boundaries(text);
        let byte_index = Self::grapheme_to_byte(&boundaries, grapheme_index);
        let cursor = Cursor::from_byte_index(&layout, byte_index, ParleyAffinity::Downstream);
        cursor.geometry(&layout, layout.width())
    }

    pub fn word_range_at_point(
        &mut self,
        text: &str,
        style: &TextStyle,
        wrap_width: Option<f32>,
        x: f32,
        y: f32,
    ) -> (usize, usize) {
        if text.is_empty() {
            return (0, 0);
        }

        let layout = self.build_layout(text, style, wrap_width);
        let boundaries = Self::grapheme_boundaries(text);
        let selection = Selection::word_from_point(&layout, x, y);
        let range = selection.text_range();
        (
            Self::byte_to_grapheme(&boundaries, range.start),
            Self::byte_to_grapheme(&boundaries, range.end),
        )
    }

    pub fn word_byte_range_at_point(
        &mut self,
        text: &str,
        style: &TextStyle,
        wrap_width: Option<f32>,
        x: f32,
        y: f32,
    ) -> (usize, usize) {
        if text.is_empty() {
            return (0, 0);
        }

        let layout = self.build_layout(text, style, wrap_width);
        let range = Selection::word_from_point(&layout, x, y).text_range();
        (range.start.min(text.len()), range.end.min(text.len()))
    }

    pub fn line_range_at_point(
        &mut self,
        text: &str,
        style: &TextStyle,
        wrap_width: Option<f32>,
        x: f32,
        y: f32,
    ) -> (usize, usize) {
        if text.is_empty() {
            return (0, 0);
        }

        let layout = self.build_layout(text, style, wrap_width);
        let boundaries = Self::grapheme_boundaries(text);
        let selection = Selection::line_from_point(&layout, x, y);
        let range = selection.text_range();
        (
            Self::byte_to_grapheme(&boundaries, range.start),
            Self::byte_to_grapheme(&boundaries, range.end),
        )
    }

    pub fn line_byte_range_at_point(
        &mut self,
        text: &str,
        style: &TextStyle,
        wrap_width: Option<f32>,
        x: f32,
        y: f32,
    ) -> (usize, usize) {
        if text.is_empty() {
            return (0, 0);
        }

        let layout = self.build_layout(text, style, wrap_width);
        let range = Selection::line_from_point(&layout, x, y).text_range();
        (range.start.min(text.len()), range.end.min(text.len()))
    }

    pub fn selection_rects(
        &mut self,
        text: &str,
        style: &TextStyle,
        wrap_width: Option<f32>,
        start: usize,
        end: usize,
    ) -> Vec<BoundingBox> {
        if text.is_empty() || start >= end {
            return Vec::new();
        }

        let layout = self.build_layout(text, style, wrap_width);
        let anchor = start.min(text.len());
        let focus = end.min(text.len());
        let selection = Selection::new(
            Cursor::from_byte_index(&layout, anchor, ParleyAffinity::Downstream),
            Cursor::from_byte_index(&layout, focus, ParleyAffinity::Upstream),
        );

        selection
            .geometry(&layout)
            .into_iter()
            .map(|(rect, _)| rect)
            .collect()
    }

    pub fn measure_text(
        &mut self,
        text: &str,
        style: &TextStyle,
        max_width: Option<f32>,
        _max_height: Option<f32>,
    ) -> (f32, f32) {
        let layout = self.build_layout(text, style, max_width);

        let measured_width = layout.full_width();
        let measured_height = layout.height();
        let fallback_height = (style.font_size * style.line_height).round();

        let w = if measured_width == 0.0 {
            (text.len() as f32) * (style.font_size * 0.6)
        } else {
            measured_width
        };

        let h = if measured_height == 0.0 {
            fallback_height
        } else {
            measured_height
        };

        (w.ceil(), h.ceil())
    }
}

pub fn apply_text_style_to_editor(editor: &mut parley::PlainEditor<TextBrush>, style: &TextStyle) {
    let styles = editor.edit_styles();
    for prop in style.to_parley_styles() {
        styles.insert(prop);
    }
}

pub fn secure_cursor_geometry(
    editor: &parley::PlainEditor<TextBrush>,
    width: f32,
    style: &TextStyle,
    text_renderer: &mut TextRenderer,
) -> Option<BoundingBox> {
    let sel = editor.raw_selection();
    let byte_idx = sel.focus().index();
    let char_count = editor.raw_text()[..byte_idx].chars().count();
    let masked = "\u{2022}".repeat(char_count);
    let positions = text_renderer.grapheme_x_positions(&masked, style);
    let cursor_x = *positions.last().unwrap_or(&0.0);
    let line_height = (style.font_size * style.line_height).round();
    Some(BoundingBox {
        x0: cursor_x as f64,
        y0: 0.0,
        x1: (cursor_x + width) as f64,
        y1: line_height as f64,
    })
}

pub fn secure_selection_geometry(
    editor: &parley::PlainEditor<TextBrush>,
    style: &TextStyle,
    text_renderer: &mut TextRenderer,
) -> Vec<BoundingBox> {
    let sel = editor.raw_selection();
    if sel.is_collapsed() {
        return vec![];
    }
    let text = editor.raw_text();
    let anchor_chars = text[..sel.anchor().index()].chars().count();
    let focus_chars = text[..sel.focus().index()].chars().count();
    let (start, end) = if anchor_chars < focus_chars {
        (anchor_chars, focus_chars)
    } else {
        (focus_chars, anchor_chars)
    };
    let full_masked = "\u{2022}".repeat(end);
    let positions = text_renderer.grapheme_x_positions(&full_masked, style);
    let x0 = if start < positions.len() {
        positions[start]
    } else {
        0.0
    };
    let x1 = if end < positions.len() {
        positions[end]
    } else {
        *positions.last().unwrap_or(&0.0)
    };
    let line_height = (style.font_size * style.line_height).round();
    vec![BoundingBox {
        x0: x0 as f64,
        y0: 0.0,
        x1: x1 as f64,
        y1: line_height as f64,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn renderer() -> TextRenderer {
        TextRenderer::new()
    }

    fn default_style() -> TextStyle {
        TextStyle::default()
    }

    #[test]
    fn hit_2d_start_of_text() {
        let mut r = renderer();
        let idx = r.hit_to_grapheme_2d("abc\ndef", &default_style(), None, 0.0, 0.0);
        assert_eq!(idx, 0);
    }

    #[test]
    fn hit_2d_second_line() {
        let mut r = renderer();
        let idx = r.hit_to_grapheme_2d("abc\ndef", &default_style(), None, 0.0, 24.0);
        assert_eq!(
            idx, 4,
            "clicking at start of line 1 should give index 4 (after \\n)"
        );
    }

    #[test]
    fn hit_2d_past_end_snaps_to_last() {
        let mut r = renderer();
        let style = default_style();
        let pos = r.grapheme_x_positions("abc", &style);
        let last_x = *pos.last().unwrap();

        let idx = r.hit_to_grapheme_2d("abc", &style, None, last_x + 100.0, 0.0);
        assert_eq!(idx, 3, "clicking past end should give last position");
    }

    #[test]
    fn x_positions_count() {
        let mut r = renderer();
        let pos = r.grapheme_x_positions("hello", &default_style());
        assert_eq!(pos.len(), 6, "5 graphemes + 1 = 6 boundaries");
    }

    #[test]
    fn x_positions_start_at_zero() {
        let mut r = renderer();
        let pos = r.grapheme_x_positions("abc", &default_style());
        assert!((pos[0] - 0.0).abs() < 0.01, "first position should be 0");
    }

    #[test]
    fn x_positions_monotonic() {
        let mut r = renderer();
        let pos = r.grapheme_x_positions("hello world", &default_style());
        for w in pos.windows(2) {
            assert!(
                w[1] >= w[0] - 0.01,
                "x should increase: {} >= {}",
                w[1],
                w[0]
            );
        }
    }

    #[test]
    fn word_range_uses_layout_boundaries() {
        let mut r = renderer();
        let (start, end) = r.word_range_at_point("hello world", &default_style(), None, 2.0, 0.0);
        assert_eq!((start, end), (0, 5));
    }

    #[test]
    fn line_range_tracks_visual_line() {
        let mut r = renderer();
        let (start, end) = r.line_range_at_point("abc\ndef", &default_style(), None, 0.0, 24.0);
        assert_eq!((start, end), (4, 7));
    }

    #[test]
    fn selection_rects_split_across_lines() {
        let mut r = renderer();
        let rects = r.selection_rects("ab\ncd", &default_style(), None, 1, 4);
        assert_eq!(rects.len(), 2);
        assert!(rects[0].x1 > rects[0].x0);
        assert!(rects[1].y0 > rects[0].y0);
    }
}
