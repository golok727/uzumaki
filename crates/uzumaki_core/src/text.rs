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

        let line_height = (font_size * 1.2).round(); // match cosmic-text Metrics
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

            if run.glyphs.is_empty() {
                // Empty line (e.g. after \n) — use line_i to find byte offset
                let line_idx = run.line_i;
                let byte_off = line_starts.get(line_idx).copied().unwrap_or(text.len());
                byte_pos.push((byte_off, 0.0, line_top));
            } else {
                for glyph in run.glyphs.iter() {
                    byte_pos.push((glyph.start, glyph.x, line_top));
                    byte_pos.push((glyph.end, glyph.x + glyph.w, line_top));
                }
            }
        }

        let first_y = first_line_top.unwrap_or(0.0);

        // Ensure byte 0 is in the map (safety net for text starting with non-printable chars)
        if !byte_pos.iter().any(|&(off, _, _)| off == 0) {
            byte_pos.push((0, 0.0, first_y));
        }

        // Ensure trailing newline has a position on the next line
        if text.ends_with('\n') {
            let end_byte = text.len();
            if !byte_pos.iter().any(|&(off, _, _)| off == end_byte) {
                byte_pos.push((end_byte, 0.0, last_line_top + line_height));
            }
        }

        // Ensure text.len() is always in the map
        if !byte_pos.iter().any(|&(off, _, _)| off == text.len()) {
            let last = byte_pos.last().map(|&(_, x, y)| (x, y)).unwrap_or((0.0, first_y));
            byte_pos.push((text.len(), last.0, last.1));
        }

        // Sort by byte offset, then by y descending so that at soft-wrap boundaries
        // (where two entries share the same byte offset on different lines), the
        // start-of-next-line entry comes first and is kept by dedup.
        byte_pos.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then(b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal))
        });
        byte_pos.dedup_by_key(|entry| entry.0);

        // Map grapheme boundaries to (x, y)
        let mut positions = Vec::new();
        positions.push(GlyphPos2D { x: 0.0, y: first_y });

        let mut byte_offset = 0;
        for grapheme in text.graphemes(true) {
            byte_offset += grapheme.len();
            positions.push(lookup_byte_pos_2d(&byte_pos, byte_offset));
        }

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
            if line_ys.last().map_or(true, |&last| (pos.y - last).abs() > 1.0) {
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

fn lookup_byte_pos_2d(byte_pos: &[(usize, f32, f32)], byte_offset: usize) -> GlyphPos2D {
    match byte_pos.binary_search_by_key(&byte_offset, |&(off, _, _)| off) {
        Ok(idx) => GlyphPos2D {
            x: byte_pos[idx].1,
            y: byte_pos[idx].2,
        },
        Err(idx) => {
            if idx == 0 {
                GlyphPos2D { x: 0.0, y: 0.0 }
            } else if idx >= byte_pos.len() {
                byte_pos
                    .last()
                    .map(|&(_, x, y)| GlyphPos2D { x, y })
                    .unwrap_or(GlyphPos2D { x: 0.0, y: 0.0 })
            } else {
                let (off0, x0, y0) = byte_pos[idx - 1];
                let (off1, x1, y1) = byte_pos[idx];
                // If on different lines (line break), snap to nearest entry
                if (y0 - y1).abs() > 1.0 {
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
