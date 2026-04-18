use parley::{Affinity, Cursor, FontContext, Layout, LayoutContext, LineHeight, StyleProperty};
use unicode_segmentation::UnicodeSegmentation;
use vello::Scene;
use vello::kurbo::Affine;
use vello::peniko::{Brush, Color, Fill};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TextBrush;

#[derive(Clone, Copy, Debug)]
pub struct GlyphPos2D {
    pub x: f32,
    pub y: f32,
}

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

        let mut positions = Vec::new();
        positions.push(0.0);

        let mut byte_offset = 0;
        for grapheme in text.graphemes(true) {
            byte_offset += grapheme.len();
            let cursor = Cursor::from_byte_index(&layout, byte_offset, Affinity::Downstream);
            let geom = cursor.geometry(&layout, layout_width);
            positions.push(geom.x0 as f32);
        }

        positions
    }

    pub fn hit_to_grapheme(&mut self, text: &str, font_size: f32, x: f32) -> usize {
        let positions = self.grapheme_x_positions(text, font_size);
        let mut best_idx = 0;
        let mut best_dist = f32::MAX;
        for (i, &pos) in positions.iter().enumerate() {
            let dist = (pos - x).abs();
            if dist < best_dist {
                best_dist = dist;
                best_idx = i;
            }
        }
        best_idx
    }

    pub fn grapheme_positions_2d(
        &mut self,
        text: &str,
        font_size: f32,
        wrap_width: Option<f32>,
    ) -> Vec<GlyphPos2D> {
        if text.is_empty() {
            return vec![GlyphPos2D { x: 0.0, y: 0.0 }];
        }

        let layout = self.build_layout(text, font_size, wrap_width);
        let layout_width = layout.width().max(wrap_width.unwrap_or(f32::MAX));
        let line_height = (font_size * 1.2).round();

        let first_line_y = layout
            .lines()
            .next()
            .map(|l| l.metrics().baseline - l.metrics().ascent)
            .unwrap_or(0.0);

        let mut positions = Vec::new();
        positions.push(GlyphPos2D { x: 0.0, y: 0.0 });

        let mut byte_offset = 0;
        for grapheme in text.graphemes(true) {
            byte_offset += grapheme.len();
            let cursor = Cursor::from_byte_index(&layout, byte_offset, Affinity::Downstream);
            let geom = cursor.geometry(&layout, layout_width);
            let x = geom.x0 as f32;
            let y = geom.y0 as f32 - first_line_y;
            positions.push(GlyphPos2D { x, y });
        }

        // Handle trailing newline: cursor should be on a new line at x=0
        if text.ends_with('\n') {
            let len = positions.len();
            let last = &positions[len - 1];
            let needs_fix =
                last.x.abs() > 1.0 || (len > 2 && (last.y - positions[len - 2].y).abs() < 1.0);
            if needs_fix {
                let prev_y = if len > 2 { positions[len - 2].y } else { 0.0 };
                let last = positions.last_mut().unwrap();
                last.x = 0.0;
                last.y = prev_y + line_height;
            }
        }

        positions
    }

    pub fn hit_to_grapheme_2d(
        &mut self,
        text: &str,
        font_size: f32,
        wrap_width: Option<f32>,
        x: f32,
        y: f32,
    ) -> usize {
        let positions = self.grapheme_positions_2d(text, font_size, wrap_width);
        let line_height = (font_size * 1.2).round();

        let mut line_ys: Vec<f32> = Vec::new();
        for pos in &positions {
            if line_ys
                .last()
                .is_none_or(|&last| (pos.y - last).abs() > 1.0)
            {
                line_ys.push(pos.y);
            }
        }

        let mut target_y = line_ys.first().copied().unwrap_or(0.0);
        for &ly in &line_ys {
            if y >= ly {
                target_y = ly;
            }
        }

        let mut best_idx = 0;
        let mut best_dist = f32::MAX;
        for (i, pos) in positions.iter().enumerate() {
            if (pos.y - target_y).abs() < line_height * 0.5 {
                let dist = (pos.x - x).abs();
                if dist < best_dist {
                    best_dist = dist;
                    best_idx = i;
                }
            }
        }
        best_idx
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
    fn positions_2d_single_line_count() {
        let mut r = renderer();
        let pos = r.grapheme_positions_2d("abc", 16.0, None);
        assert_eq!(pos.len(), 4, "3 graphemes + 1 boundary = 4 entries");
    }

    #[test]
    fn positions_2d_single_line_all_on_line_zero() {
        let mut r = renderer();
        let pos = r.grapheme_positions_2d("hello", 16.0, None);
        for p in &pos {
            assert!(
                p.y.abs() < 1.0,
                "all positions should have y≈0 on a single line, got y={}",
                p.y
            );
        }
    }

    #[test]
    fn positions_2d_x_monotonically_increases() {
        let mut r = renderer();
        let pos = r.grapheme_positions_2d("abcdef", 16.0, None);
        for w in pos.windows(2) {
            assert!(
                w[1].x >= w[0].x - 0.01,
                "x should increase: {} >= {}",
                w[1].x,
                w[0].x
            );
        }
    }

    #[test]
    fn positions_2d_hard_newline_two_lines() {
        let mut r = renderer();
        let pos = r.grapheme_positions_2d("ab\ncd", 16.0, None);
        assert_eq!(pos.len(), 6);

        assert!(pos[0].y.abs() < 1.0);
        assert!(pos[1].y.abs() < 1.0);
        assert!(pos[2].y.abs() < 1.0);

        let line1_y = pos[3].y;
        assert!(line1_y > 1.0, "line 1 y should be > 0, got {}", line1_y);
        assert!(
            pos[3].x < 1.0,
            "start of line 1 should be at x≈0, got {}",
            pos[3].x
        );

        assert!((pos[4].y - line1_y).abs() < 1.0);
        assert!((pos[5].y - line1_y).abs() < 1.0);
    }

    #[test]
    fn positions_2d_x_resets_on_new_line() {
        let mut r = renderer();
        let pos = r.grapheme_positions_2d("abc\nde", 16.0, None);
        assert!(pos[4].x < 1.0, "first char on line 1 should be near x=0");
        assert!(
            pos[5].x > pos[4].x,
            "x should increase on line 1: {} > {}",
            pos[5].x,
            pos[4].x
        );
    }

    #[test]
    fn positions_2d_empty_line_between() {
        let mut r = renderer();
        let pos = r.grapheme_positions_2d("a\n\nb", 16.0, None);
        assert_eq!(pos.len(), 5);

        assert!(
            pos[3].y > pos[2].y,
            "line after empty should be below: {} > {}",
            pos[3].y,
            pos[2].y
        );
        assert!(pos[3].x < 1.0, "start of last line should be x≈0");
    }

    #[test]
    fn positions_2d_trailing_newline() {
        let mut r = renderer();
        let pos = r.grapheme_positions_2d("abc\n", 16.0, None);
        assert_eq!(pos.len(), 5);
        let last = pos.last().unwrap();
        assert!(
            last.x < 1.0,
            "after trailing \\n should be x≈0, got {}",
            last.x
        );
        assert!(
            last.y > pos[0].y,
            "after trailing \\n should be on a new line"
        );
    }

    #[test]
    fn positions_2d_wrapping_creates_multiple_visual_lines() {
        let mut r = renderer();
        let pos = r.grapheme_positions_2d("abcdef ghij", 16.0, Some(50.0));

        let mut ys: Vec<f32> = vec![pos[0].y];
        for p in &pos[1..] {
            if ys.last().is_none_or(|&ly| (p.y - ly).abs() > 1.0) {
                ys.push(p.y);
            }
        }
        assert!(
            ys.len() >= 2,
            "narrow width should produce at least 2 visual lines, got {}",
            ys.len()
        );
    }

    #[test]
    fn positions_2d_wrap_break_at_start_of_next_line() {
        let mut r = renderer();
        let pos = r.grapheme_positions_2d("ab cd", 16.0, Some(30.0));

        let line0_y = pos[0].y;
        let first_on_line1 = pos.iter().find(|p| (p.y - line0_y).abs() > 1.0);
        if let Some(p) = first_on_line1 {
            assert!(
                p.x < 5.0,
                "first position on wrapped line should be near x=0, got {}",
                p.x
            );
        }
    }

    #[test]
    fn positions_2d_x_monotonic_per_visual_line() {
        let mut r = renderer();
        let pos = r.grapheme_positions_2d("hello world\nfoo bar baz", 16.0, None);

        let mut current_y = pos[0].y;
        let mut prev_x = -1.0f32;
        for p in &pos {
            if (p.y - current_y).abs() > 1.0 {
                current_y = p.y;
                prev_x = -1.0;
            }
            assert!(
                p.x >= prev_x - 0.01,
                "x should increase on each visual line: {} >= {}",
                p.x,
                prev_x
            );
            prev_x = p.x;
        }
    }

    #[test]
    fn positions_2d_three_lines() {
        let mut r = renderer();
        let pos = r.grapheme_positions_2d("aaa\nbbb\nccc", 16.0, None);
        assert_eq!(pos.len(), 12);

        let y0 = pos[0].y;
        let y1 = pos[4].y;
        let y2 = pos[8].y;

        assert!(y1 > y0 + 1.0, "line 1 below line 0");
        assert!(y2 > y1 + 1.0, "line 2 below line 1");

        let spacing_01 = y1 - y0;
        let spacing_12 = y2 - y1;
        assert!(
            (spacing_01 - spacing_12).abs() < 1.0,
            "line spacing should be consistent: {} vs {}",
            spacing_01,
            spacing_12
        );
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
        let pos = r.grapheme_positions_2d("abc\ndef", 16.0, None);
        let line1_y = pos[4].y;

        let idx = r.hit_to_grapheme_2d("abc\ndef", 16.0, None, 0.0, line1_y + 2.0);
        assert_eq!(
            idx, 4,
            "clicking at start of line 1 should give index 4 (after \\n)"
        );
    }

    #[test]
    fn hit_2d_past_end_snaps_to_last() {
        let mut r = renderer();
        let pos = r.grapheme_positions_2d("abc", 16.0, None);
        let last_x = pos.last().unwrap().x;

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
}
