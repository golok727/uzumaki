use std::collections::HashMap;
use std::sync::Arc;

use cosmic_text::fontdb;
use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};
use unicode_segmentation::UnicodeSegmentation;
use vello::kurbo::Affine;
use vello::peniko::{Blob, Brush, Color, Fill, FontData};
use vello::{Glyph, Scene};

type FontId = fontdb::ID;

#[derive(Clone, Copy, Debug)]
pub struct GlyphPos2D {
    pub x: f32,
    pub y: f32,
}

pub struct TextRenderer {
    pub font_system: FontSystem,
    // Maps cosmic-text font IDs to vello Fonts.
    // cosmic-text identifies loaded fonts by fontdb::ID; vello needs its own Font
    // handle (built from the same raw bytes) to render glyph outlines on the GPU.
    font_cache: HashMap<FontId, FontData>,
}

impl TextRenderer {
    pub fn new() -> Self {
        let mut font_system = FontSystem::new();

        // Load bundled Roboto so we always have a known font available,
        // even on systems with limited installed fonts.
        let roboto = include_bytes!("../assets/Roboto-Regular.ttf");
        font_system.db_mut().load_font_data(roboto.to_vec());

        Self {
            font_system,
            font_cache: HashMap::new(),
        }
    }

    /// Extracts raw font file bytes from cosmic-text's fontdb and constructs
    /// a vello Font. This is the key bridge between the two libraries:
    /// cosmic-text uses the bytes for shaping/layout (via rustybuzz),
    /// vello uses the same bytes to read glyph outlines for GPU rendering (via skrifa).
    fn ensure_font_cached(&mut self, font_id: FontId) {
        if self.font_cache.contains_key(&font_id) {
            return;
        }
        // fontdb::Database::with_face_data gives us the raw font file bytes
        // and the face index within that file (relevant for .ttc collections).
        let font_data = self
            .font_system
            .db()
            .with_face_data(font_id, |data, index| (data.to_vec(), index));
        if let Some((data, index)) = font_data {
            let font = FontData::new(
                Blob::new(Arc::new(data) as Arc<dyn AsRef<[u8]> + Send + Sync>),
                index,
            );
            self.font_cache.insert(font_id, font);
        }
    }

    fn layout_buffer(
        &mut self,
        text: &str,
        attrs: Attrs<'_>,
        font_size: f32,
        width: Option<f32>,
        height: Option<f32>,
    ) -> Buffer {
        let metrics = Metrics::new(font_size, (font_size * 1.2).round());
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_text(&mut self.font_system, text, &attrs, Shaping::Advanced, None);
        buffer.set_size(&mut self.font_system, width, height);
        buffer.shape_until_scroll(&mut self.font_system, false);
        buffer
    }

    fn cache_fonts_from_buffer(&mut self, buffer: &Buffer) {
        for run in buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                self.ensure_font_cached(glyph.font_id);
            }
        }
    }

    pub fn draw_text(
        &mut self,
        scene: &mut Scene,
        text: &str,
        attrs: Attrs<'_>,
        font_size: f32,
        width: f32,
        height: f32,
        position: (f32, f32),
        color: Color,
        scale: f64,
    ) {
        let buffer = self.layout_buffer(text, attrs, font_size, Some(width), Some(height));
        self.cache_fonts_from_buffer(&buffer);

        // Second pass: draw glyphs, grouping consecutive runs by font_id
        let (px, py) = position;
        for run in buffer.layout_runs() {
            // Group consecutive glyphs by font_id so each draw_glyphs call
            // uses a single font (required by the vello API).
            let mut by_font: Vec<(FontId, Vec<Glyph>)> = Vec::new();

            for glyph in run.glyphs.iter() {
                let vello_glyph = Glyph {
                    id: glyph.glyph_id as u32,
                    x: px + glyph.x,
                    y: py + run.line_y,
                };

                if let Some(last) = by_font.last_mut() {
                    if last.0 == glyph.font_id {
                        last.1.push(vello_glyph);
                        continue;
                    }
                }
                by_font.push((glyph.font_id, vec![vello_glyph]));
            }

            for (font_id, glyphs) in by_font {
                if let Some(vello_font) = self.font_cache.get(&font_id) {
                    scene
                        .draw_glyphs(vello_font)
                        .font_size(font_size)
                        .transform(Affine::scale(scale))
                        .brush(&Brush::Solid(color))
                        .draw(Fill::NonZero, glyphs.into_iter());
                }
            }
        }
    }

    /// Returns x-positions for each grapheme boundary in the text.
    /// Result has `grapheme_count + 1` entries: [0] = 0.0, [n] = end of text.
    pub fn grapheme_x_positions(&mut self, text: &str, font_size: f32) -> Vec<f32> {
        if text.is_empty() {
            return vec![0.0];
        }

        let buffer = self.layout_buffer(text, Attrs::new(), font_size, None, None);

        // Build byte offset → x position mapping from glyphs
        let mut byte_x: Vec<(usize, f32)> = Vec::new();
        byte_x.push((0, 0.0));

        for run in buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                byte_x.push((glyph.start, glyph.x));
                byte_x.push((glyph.end, glyph.x + glyph.w));
            }
        }

        byte_x.sort_by_key(|&(offset, _)| offset);
        byte_x.dedup_by_key(|entry| entry.0);

        // Map grapheme boundaries to x positions
        let mut positions = Vec::new();
        positions.push(lookup_byte_x(&byte_x, 0));

        let mut byte_offset = 0;
        for grapheme in text.graphemes(true) {
            byte_offset += grapheme.len();
            positions.push(lookup_byte_x(&byte_x, byte_offset));
        }

        positions
    }

    /// Hit-test an x-coordinate against text layout, returning the grapheme index
    /// (cursor position) closest to that x.
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

    /// Returns (x, y) positions for each grapheme boundary in multiline text.
    /// `wrap_width` controls line wrapping. Result has `grapheme_count + 1` entries.
    /// y values are line-top relative to buffer origin.
    pub fn grapheme_positions_2d(
        &mut self,
        text: &str,
        font_size: f32,
        wrap_width: Option<f32>,
    ) -> Vec<GlyphPos2D> {
        if text.is_empty() {
            return vec![GlyphPos2D { x: 0.0, y: 0.0 }];
        }

        let buffer = self.layout_buffer(text, Attrs::new(), font_size, wrap_width, None);

        let mut byte_pos: Vec<(usize, f32, f32)> = Vec::new();

        // Compute byte offset of each line start for mapping empty runs
        let mut line_starts: Vec<usize> = vec![0];
        for (i, ch) in text.char_indices() {
            if ch == '\n' {
                line_starts.push(i + 1);
            }
        }

        let line_height = (font_size * 1.2).round();
        let mut first_line_top: Option<f32> = None;
        let mut last_line_top: f32 = 0.0;

        for run in buffer.layout_runs() {
            let line_top = run.line_top;
            if first_line_top.is_none() {
                first_line_top = Some(line_top);
            }
            if line_top >= last_line_top {
                last_line_top = line_top;
            }

            // cosmic-text glyph byte offsets are relative to the line (paragraph),
            // not the full text. Convert to absolute text byte offsets.
            let line_byte_offset = line_starts.get(run.line_i).copied().unwrap_or(0);

            if run.glyphs.is_empty() {
                byte_pos.push((line_byte_offset, 0.0, line_top));
            } else {
                for glyph in run.glyphs.iter() {
                    byte_pos.push((line_byte_offset + glyph.start, glyph.x, line_top));
                    byte_pos.push((line_byte_offset + glyph.end, glyph.x + glyph.w, line_top));
                }
            }
        }

        let first_y = first_line_top.unwrap_or(0.0);

        if !byte_pos.iter().any(|&(off, _, _)| off == 0) {
            byte_pos.push((0, 0.0, first_y));
        }

        if text.ends_with('\n') {
            let end_byte = text.len();
            if !byte_pos.iter().any(|&(off, _, _)| off == end_byte) {
                byte_pos.push((end_byte, 0.0, last_line_top + line_height));
            }
        }

        if !byte_pos.iter().any(|&(off, _, _)| off == text.len()) {
            let last = byte_pos
                .last()
                .map(|&(_, x, y)| (x, y))
                .unwrap_or((0.0, first_y));
            byte_pos.push((text.len(), last.0, last.1));
        }

        byte_pos.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then(a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
        });

        // Map grapheme boundaries to (x, y), normalizing y to zero-based.
        // Always prefer next line at line breaks: at hard newlines (\n) this
        // picks the start of the next line; at soft-wrap boundaries this picks
        // (x=0, next_visual_line_y) instead of (end_x, current_line_y).
        // At non-break positions there is only one entry so the flag is a no-op.
        let mut positions = Vec::new();
        positions.push(GlyphPos2D { x: 0.0, y: 0.0 });

        let mut byte_offset = 0;
        for grapheme in text.graphemes(true) {
            byte_offset += grapheme.len();
            let mut pos = lookup_byte_pos_2d(&byte_pos, byte_offset, true);
            pos.y -= first_y;
            positions.push(pos);
        }

        // Normalize the first position too (it was set to 0.0 above, which is correct)
        positions
    }

    /// Hit-test an (x, y) coordinate against multiline text layout.
    pub fn hit_to_grapheme_2d(
        &mut self,
        text: &str,
        font_size: f32,
        wrap_width: Option<f32>,
        x: f32,
        y: f32,
    ) -> usize {
        let positions = self.grapheme_positions_2d(text, font_size, wrap_width);
        let line_height = (font_size * 1.2).round(); // match cosmic-text Metrics

        // Collect unique line y values
        let mut line_ys: Vec<f32> = Vec::new();
        for pos in &positions {
            if line_ys
                .last()
                .map_or(true, |&last| (pos.y - last).abs() > 1.0)
            {
                line_ys.push(pos.y);
            }
        }

        // Find which line the y coordinate falls on.
        // Use the line whose vertical range [ly, ly + line_height) contains y.
        let mut target_y = line_ys.first().copied().unwrap_or(0.0);
        for &ly in &line_ys {
            if y >= ly {
                target_y = ly;
            }
        }

        // Among positions on that line, find closest x
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
        attrs: Attrs<'_>,
        font_size: f32,
        max_width: Option<f32>,
        max_height: Option<f32>,
    ) -> (f32, f32) {
        let buffer = self.layout_buffer(text, attrs, font_size, max_width, max_height);
        self.cache_fonts_from_buffer(&buffer);

        let mut measured_width: f32 = 0.0;
        let mut measured_height: f32 = 0.0;

        for run in buffer.layout_runs() {
            // Use glyph extents to avoid relying on line_w when wrap width is tiny.
            for glyph in run.glyphs.iter() {
                measured_width = measured_width.max(glyph.x + glyph.w);
            }
            measured_height = measured_height.max(run.line_top + run.line_height);
        }

        if measured_height == 0.0 {
            measured_height = buffer.metrics().line_height;
        }
        if measured_width == 0.0 {
            measured_width = (text.len() as f32) * (font_size * 0.6);
        }

        (measured_width.ceil(), measured_height.ceil())
    }
}

/// Look up a byte offset in the (sorted, non-deduped) position list.
/// `prefer_next_line`: when true (after \n), pick the entry with highest y at this offset;
/// when false (soft-wrap), pick the entry with lowest y (end of current line).
fn lookup_byte_pos_2d(
    byte_pos: &[(usize, f32, f32)],
    byte_offset: usize,
    prefer_next_line: bool,
) -> GlyphPos2D {
    // Find range of entries matching this byte offset (array is sorted by offset then y).
    let start = byte_pos.partition_point(|&(off, _, _)| off < byte_offset);
    let end = byte_pos.partition_point(|&(off, _, _)| off <= byte_offset);

    if start < end {
        // One or more entries match. Since sorted by y ascending:
        // - lowest y (current line end) is at `start`
        // - highest y (next line start) is at `end - 1`
        let idx = if prefer_next_line { end - 1 } else { start };
        return GlyphPos2D {
            x: byte_pos[idx].1,
            y: byte_pos[idx].2,
        };
    }

    // No exact match — interpolate between neighbors
    if start == 0 {
        GlyphPos2D { x: 0.0, y: 0.0 }
    } else if start >= byte_pos.len() {
        byte_pos
            .last()
            .map(|&(_, x, y)| GlyphPos2D { x, y })
            .unwrap_or(GlyphPos2D { x: 0.0, y: 0.0 })
    } else {
        let (off0, x0, y0) = byte_pos[start - 1];
        let (off1, x1, y1) = byte_pos[start];
        if (y0 - y1).abs() > 1.0 {
            // Cross-line: snap to nearest entry
            let d0 = byte_offset - off0;
            let d1 = off1 - byte_offset;
            if d0 <= d1 {
                GlyphPos2D { x: x0, y: y0 }
            } else {
                GlyphPos2D { x: x1, y: y1 }
            }
        } else {
            let t = (byte_offset - off0) as f32 / (off1 - off0).max(1) as f32;
            GlyphPos2D {
                x: x0 + t * (x1 - x0),
                y: y0,
            }
        }
    }
}

fn lookup_byte_x(byte_x: &[(usize, f32)], byte_offset: usize) -> f32 {
    match byte_x.binary_search_by_key(&byte_offset, |&(off, _)| off) {
        Ok(idx) => byte_x[idx].1,
        Err(idx) => {
            if idx == 0 {
                0.0
            } else if idx >= byte_x.len() {
                byte_x.last().map(|&(_, x)| x).unwrap_or(0.0)
            } else {
                let (off0, x0) = byte_x[idx - 1];
                let (off1, x1) = byte_x[idx];
                let t = (byte_offset - off0) as f32 / (off1 - off0).max(1) as f32;
                x0 + t * (x1 - x0)
            }
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn renderer() -> TextRenderer {
        TextRenderer::new()
    }

    // ── lookup_byte_pos_2d (pure function) ──────────────────────────

    #[test]
    fn lookup_single_entry() {
        let byte_pos = vec![(0, 0.0f32, 0.0f32), (5, 50.0, 0.0)];
        let pos = lookup_byte_pos_2d(&byte_pos, 0, true);
        assert!((pos.x - 0.0).abs() < 0.01);
        assert!((pos.y - 0.0).abs() < 0.01);
    }

    #[test]
    fn lookup_prefers_next_line_at_wrap() {
        // Simulates a soft-wrap boundary at byte 3:
        // end-of-line-0 at (3, 30.0, 0.0) and start-of-line-1 at (3, 0.0, 20.0)
        let byte_pos = vec![
            (0, 0.0f32, 0.0f32),
            (3, 30.0, 0.0),
            (3, 0.0, 20.0),
            (6, 30.0, 20.0),
        ];

        let next = lookup_byte_pos_2d(&byte_pos, 3, true);
        assert!(
            (next.x - 0.0).abs() < 0.01,
            "prefer_next_line should pick x=0"
        );
        assert!(
            (next.y - 20.0).abs() < 0.01,
            "prefer_next_line should pick y=20"
        );

        let curr = lookup_byte_pos_2d(&byte_pos, 3, false);
        assert!(
            (curr.x - 30.0).abs() < 0.01,
            "prefer_current should pick x=30"
        );
        assert!(
            (curr.y - 0.0).abs() < 0.01,
            "prefer_current should pick y=0"
        );
    }

    #[test]
    fn lookup_interpolates_between_neighbors() {
        // Byte offsets 0 and 4 at x=0 and x=40 on same line
        let byte_pos = vec![(0, 0.0f32, 0.0f32), (4, 40.0, 0.0)];
        let pos = lookup_byte_pos_2d(&byte_pos, 2, false);
        assert!((pos.x - 20.0).abs() < 0.01, "should interpolate to x=20");
        assert!((pos.y - 0.0).abs() < 0.01);
    }

    // ── grapheme_positions_2d ───────────────────────────────────────

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
        // 5 graphemes (a, b, \n, c, d) + 1 = 6
        assert_eq!(pos.len(), 6);

        // First 3 positions (before-a, after-a, after-b) on line 0
        assert!(pos[0].y.abs() < 1.0);
        assert!(pos[1].y.abs() < 1.0);
        assert!(pos[2].y.abs() < 1.0);

        // After \n → start of line 1
        let line1_y = pos[3].y;
        assert!(line1_y > 1.0, "line 1 y should be > 0, got {}", line1_y);
        assert!(
            pos[3].x < 1.0,
            "start of line 1 should be at x≈0, got {}",
            pos[3].x
        );

        // Remaining on line 1
        assert!((pos[4].y - line1_y).abs() < 1.0);
        assert!((pos[5].y - line1_y).abs() < 1.0);
    }

    #[test]
    fn positions_2d_x_resets_on_new_line() {
        let mut r = renderer();
        let pos = r.grapheme_positions_2d("abc\nde", 16.0, None);
        // After \n = pos[4], should be at x≈0 on line 1
        assert!(pos[4].x < 1.0, "first char on line 1 should be near x=0");
        // After first char on line 1, x should increase
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
        // Graphemes: a, \n, \n, b → 4 graphemes + 1 = 5 positions
        assert_eq!(pos.len(), 5);

        // pos[2] = after first \n = start of empty line
        // pos[3] = after second \n = start of line with "b"
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
        // Graphemes: a, b, c, \n → 4 graphemes + 1 = 5 positions
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
        // Use very narrow width to force wrapping. At font_size 16, each char is ~9px.
        // "abcdef ghij" with wrap_width=50 should wrap.
        let pos = r.grapheme_positions_2d("abcdef ghij", 16.0, Some(50.0));

        // Collect unique y values
        let mut ys: Vec<f32> = vec![pos[0].y];
        for p in &pos[1..] {
            if ys.last().map_or(true, |&ly| (p.y - ly).abs() > 1.0) {
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
        // Force a wrap. With very narrow width, even "ab cd" wraps.
        let pos = r.grapheme_positions_2d("ab cd", 16.0, Some(30.0));

        // Find the first position on a second visual line
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
                // New line — reset x tracking
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
        // 11 graphemes + 1 = 12
        assert_eq!(pos.len(), 12);

        // Three distinct y values
        let y0 = pos[0].y;
        let y1 = pos[4].y; // after first \n
        let y2 = pos[8].y; // after second \n

        assert!(y1 > y0 + 1.0, "line 1 below line 0");
        assert!(y2 > y1 + 1.0, "line 2 below line 1");

        // Lines are evenly spaced
        let spacing_01 = y1 - y0;
        let spacing_12 = y2 - y1;
        assert!(
            (spacing_01 - spacing_12).abs() < 1.0,
            "line spacing should be consistent: {} vs {}",
            spacing_01,
            spacing_12
        );
    }

    // ── hit_to_grapheme_2d ──────────────────────────────────────────

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
        let line1_y = pos[4].y; // start of "def" line

        // Click at x=0 on line 1 → should be at start of line 1
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

    // ── grapheme_x_positions (singleline) ───────────────────────────

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
