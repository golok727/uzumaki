use parley::{
    Affinity, BoundingBox, Cursor, FontContext, Layout, LayoutContext, LineHeight, Selection,
    StyleProperty,
};
use unicode_segmentation::UnicodeSegmentation;
use vello::Scene;
use vello::kurbo::Affine;
use vello::peniko::{Brush, Color, Fill};

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
        font_size: f32,
        max_width: Option<f32>,
    ) -> Layout<TextBrush> {
        let mut builder = self
            .layout_ctx
            .ranged_builder(&mut self.font_ctx, text, 1.0, true);
        builder.push_default(StyleProperty::FontSize(font_size));
        builder.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(1.2)));
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
        font_size: f32,
        width: f32,
        _height: f32,
        position: (f32, f32),
        color: Color,
        scale: f64,
    ) {
        let layout = self.build_layout(text, font_size, Some(width));
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
                        .transform(Affine::scale(scale))
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

    pub fn grapheme_x_positions(&mut self, text: &str, font_size: f32) -> Vec<f32> {
        if text.is_empty() {
            return vec![0.0];
        }

        let layout = self.build_layout(text, font_size, None);
        let layout_width = layout.width();
        let boundaries = Self::grapheme_boundaries(text);

        let mut positions = Vec::with_capacity(boundaries.len());
        positions.push(0.0);
        for &byte_offset in boundaries.iter().skip(1) {
            let cursor = Cursor::from_byte_index(&layout, byte_offset, Affinity::Downstream);
            let geom = cursor.geometry(&layout, layout_width);
            positions.push(geom.x0 as f32);
        }

        positions
    }

    pub fn hit_to_grapheme(&mut self, text: &str, font_size: f32, x: f32) -> usize {
        self.hit_to_grapheme_2d(text, font_size, None, x, 0.0)
    }

    pub fn hit_to_grapheme_2d(
        &mut self,
        text: &str,
        font_size: f32,
        wrap_width: Option<f32>,
        x: f32,
        y: f32,
    ) -> usize {
        if text.is_empty() {
            return 0;
        }

        let layout = self.build_layout(text, font_size, wrap_width);
        let boundaries = Self::grapheme_boundaries(text);
        let byte_index = Cursor::from_point(&layout, x, y).index();
        Self::byte_to_grapheme(&boundaries, byte_index)
    }

    pub fn word_range_at_point(
        &mut self,
        text: &str,
        font_size: f32,
        wrap_width: Option<f32>,
        x: f32,
        y: f32,
    ) -> (usize, usize) {
        if text.is_empty() {
            return (0, 0);
        }

        let layout = self.build_layout(text, font_size, wrap_width);
        let boundaries = Self::grapheme_boundaries(text);
        let selection = Selection::word_from_point(&layout, x, y);
        let range = selection.text_range();
        (
            Self::byte_to_grapheme(&boundaries, range.start),
            Self::byte_to_grapheme(&boundaries, range.end),
        )
    }

    pub fn line_range_at_point(
        &mut self,
        text: &str,
        font_size: f32,
        wrap_width: Option<f32>,
        x: f32,
        y: f32,
    ) -> (usize, usize) {
        if text.is_empty() {
            return (0, 0);
        }

        let layout = self.build_layout(text, font_size, wrap_width);
        let boundaries = Self::grapheme_boundaries(text);
        let selection = Selection::line_from_point(&layout, x, y);
        let range = selection.text_range();
        (
            Self::byte_to_grapheme(&boundaries, range.start),
            Self::byte_to_grapheme(&boundaries, range.end),
        )
    }

    pub fn selection_rects(
        &mut self,
        text: &str,
        font_size: f32,
        wrap_width: Option<f32>,
        start: usize,
        end: usize,
    ) -> Vec<BoundingBox> {
        if text.is_empty() || start >= end {
            return Vec::new();
        }

        let layout = self.build_layout(text, font_size, wrap_width);
        let boundaries = Self::grapheme_boundaries(text);
        let anchor = Self::grapheme_to_byte(&boundaries, start);
        let focus = Self::grapheme_to_byte(&boundaries, end);
        let selection = Selection::new(
            Cursor::from_byte_index(&layout, anchor, Affinity::Downstream),
            Cursor::from_byte_index(&layout, focus, Affinity::Upstream),
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
        font_size: f32,
        max_width: Option<f32>,
        _max_height: Option<f32>,
    ) -> (f32, f32) {
        let layout = self.build_layout(text, font_size, max_width);

        let measured_width = layout.width();
        let measured_height = layout.height();
        let line_height = (font_size * 1.2).round();

        let w = if measured_width == 0.0 {
            (text.len() as f32) * (font_size * 0.6)
        } else {
            measured_width
        };

        let h = if measured_height == 0.0 {
            line_height
        } else {
            measured_height
        };

        (w.ceil(), h.ceil())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn renderer() -> TextRenderer {
        TextRenderer::new()
    }

    #[test]
    fn hit_2d_start_of_text() {
        let mut r = renderer();
        let idx = r.hit_to_grapheme_2d("abc\ndef", 16.0, None, 0.0, 0.0);
        assert_eq!(idx, 0);
    }

    #[test]
    fn hit_2d_second_line() {
        let mut r = renderer();
        let idx = r.hit_to_grapheme_2d("abc\ndef", 16.0, None, 0.0, 24.0);
        assert_eq!(
            idx, 4,
            "clicking at start of line 1 should give index 4 (after \\n)"
        );
    }

    #[test]
    fn hit_2d_past_end_snaps_to_last() {
        let mut r = renderer();
        let pos = r.grapheme_x_positions("abc", 16.0);
        let last_x = *pos.last().unwrap();

        let idx = r.hit_to_grapheme_2d("abc", 16.0, None, last_x + 100.0, 0.0);
        assert_eq!(idx, 3, "clicking past end should give last position");
    }

    #[test]
    fn x_positions_count() {
        let mut r = renderer();
        let pos = r.grapheme_x_positions("hello", 16.0);
        assert_eq!(pos.len(), 6, "5 graphemes + 1 = 6 boundaries");
    }

    #[test]
    fn x_positions_start_at_zero() {
        let mut r = renderer();
        let pos = r.grapheme_x_positions("abc", 16.0);
        assert!((pos[0] - 0.0).abs() < 0.01, "first position should be 0");
    }

    #[test]
    fn x_positions_monotonic() {
        let mut r = renderer();
        let pos = r.grapheme_x_positions("hello world", 16.0);
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
        let (start, end) = r.word_range_at_point("hello world", 16.0, None, 2.0, 0.0);
        assert_eq!((start, end), (0, 5));
    }

    #[test]
    fn line_range_tracks_visual_line() {
        let mut r = renderer();
        let (start, end) = r.line_range_at_point("abc\ndef", 16.0, None, 0.0, 24.0);
        assert_eq!((start, end), (4, 7));
    }

    #[test]
    fn selection_rects_split_across_lines() {
        let mut r = renderer();
        let rects = r.selection_rects("ab\ncd", 16.0, None, 1, 4);
        assert_eq!(rects.len(), 2);
        assert!(rects[0].x1 > rects[0].x0);
        assert!(rects[1].y0 > rects[0].y0);
    }
}
