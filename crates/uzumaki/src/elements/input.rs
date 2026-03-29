use vello::Scene;
use vello::kurbo::{Affine, Rect};
use vello::peniko::{Color as VelloColor, Fill};

use crate::style::{Bounds, Color, Corners, Edges, Style};
use crate::text::{GlyphPos2D, TextRenderer};

/// Snapshot of input state collected during the render-tree walk.
/// Decouples painting from the live InputState/Node.
pub struct InputRenderInfo {
    pub display_text: String,
    pub placeholder: String,
    pub font_size: f32,
    pub text_color: Color,
    pub focused: bool,
    pub sel_start: usize,
    pub sel_end: usize,
    pub cursor_pos: usize,
    pub scroll_offset: f32,
    pub scroll_offset_y: f32,
    pub blink_visible: bool,
    pub multiline: bool,
}

/// Paint an input element with its text, selection highlight, and cursor.
pub fn paint_input(
    scene: &mut Scene,
    text_renderer: &mut TextRenderer,
    bounds: Bounds,
    style: &Style,
    input: &InputRenderInfo,
    scale: f64,
) {
    let padding: f64 = 8.0;
    let text_x = bounds.x + padding;
    let text_y = bounds.y;
    let text_w = (bounds.width - padding * 2.0).max(0.0);
    let text_h = bounds.height;

    // Paint background with focus-aware border
    let mut paint_style = style.clone();
    if input.focused {
        paint_style.border_widths = Edges::all(2.0);
        paint_style.border_color = Some(Color::rgba(86, 156, 214, 255));
    } else {
        if !paint_style.border_widths.any_nonzero() {
            paint_style.border_widths = Edges::all(1.0);
        }
        if paint_style.border_color.is_none() {
            paint_style.border_color = Some(Color::rgba(60, 60, 60, 255));
        }
    }
    if paint_style.background.is_none() {
        paint_style.background = Some(Color::rgba(30, 30, 30, 255));
    }
    if !paint_style.corner_radii.any_nonzero() {
        paint_style.corner_radii = Corners::uniform(4.0);
    }

    paint_style.paint(bounds, scene, scale, |_| {});

    // Clip to text area
    let clip_rect = Rect::new(text_x, text_y, text_x + text_w, text_y + text_h);
    scene.push_clip_layer(Fill::NonZero, Affine::scale(scale), &clip_rect);

    let is_empty = input.display_text.is_empty();
    let line_height = (input.font_size * 1.2).round();

    if input.multiline {
        paint_multiline(
            scene,
            text_renderer,
            input,
            text_x,
            text_y,
            text_w,
            text_h,
            line_height,
            is_empty,
            style,
            scale,
        );
    } else {
        paint_singleline(
            scene,
            text_renderer,
            input,
            text_x,
            text_y,
            text_w,
            text_h,
            line_height,
            is_empty,
            scale,
        );
    }

    scene.pop_layer();
}

// ── Coordinate helpers ───────────────────────────────────────────────

/// Convert a position from `grapheme_positions_2d` to screen coordinates.
/// Single source of truth for the positions→screen transform used by both
/// cursor and selection painting.
pub(crate) fn to_screen(
    pos: GlyphPos2D,
    text_x: f64,
    text_y: f64,
    top_pad: f64,
    scroll_y: f64,
) -> (f64, f64) {
    (
        text_x + pos.x as f64,
        text_y + pos.y as f64 + top_pad - scroll_y,
    )
}

/// Compute selection highlight rectangles in positions-relative coordinates.
/// Returns `(x1, y, x2, y + line_height)` rects — the caller applies the
/// screen offset via `to_screen` / the shared `(text_x, text_y + top_pad - scroll_y)`.
///
/// Coordinate system: x is relative to text-area left edge (0 = left),
/// y is the zero-based line-top from `grapheme_positions_2d`.
pub(crate) fn compute_selection_rects(
    positions: &[GlyphPos2D],
    sel_start: usize,
    sel_end: usize,
    text_w: f64,
    line_height: f64,
) -> Vec<[f64; 4]> {
    if sel_start >= sel_end || positions.is_empty() {
        return vec![];
    }

    let sel_end_idx = sel_end.min(positions.len() - 1);
    let start_x = positions[sel_start].x as f64;
    let end_x = positions[sel_end_idx].x as f64;

    // Collect unique visual line y-values within the selection range
    let mut line_ys: Vec<f32> = Vec::new();
    for i in sel_start..=sel_end_idx {
        let y = positions[i].y;
        if line_ys.last().map_or(true, |&ly| (y - ly).abs() > 1.0) {
            line_ys.push(y);
        }
    }

    let num_lines = line_ys.len();
    let mut rects = Vec::new();

    for (idx, &ly) in line_ys.iter().enumerate() {
        let y = ly as f64;

        let (x1, x2) = if num_lines == 1 {
            // Single visual line — exact start-to-end
            (start_x, end_x)
        } else if idx == 0 {
            // First line: from selection start to right edge
            (start_x, text_w)
        } else if idx == num_lines - 1 {
            // Last line: from left edge to selection end
            if end_x < 1.0 {
                // Selection ends at start-of-line (after a newline) — small stub
                (0.0, 8.0)
            } else {
                (0.0, end_x)
            }
        } else {
            // Middle line: full width, or stub for empty lines
            let has_content = (sel_start..=sel_end_idx)
                .any(|i| (positions[i].y - ly).abs() < 1.0 && positions[i].x > 1.0);
            if has_content {
                (0.0, text_w)
            } else {
                (0.0, 8.0)
            }
        };

        if x2 > x1 {
            rects.push([x1, y, x2, y + line_height]);
        }
    }

    rects
}

// ── Multiline input rendering ────────────────────────────────────────

fn paint_multiline(
    scene: &mut Scene,
    text_renderer: &mut TextRenderer,
    input: &InputRenderInfo,
    text_x: f64,
    text_y: f64,
    text_w: f64,
    text_h: f64,
    line_height: f32,
    is_empty: bool,
    style: &Style,
    scale: f64,
) {
    let top_pad: f64 = if style.padding.top > 0.0 {
        style.padding.top as f64
    } else {
        4.0
    };
    let scroll_y: f64 = input.scroll_offset_y as f64;
    let wrap_width = Some(text_w as f32);

    let positions = if !is_empty {
        text_renderer.grapheme_positions_2d(&input.display_text, input.font_size, wrap_width)
    } else {
        vec![GlyphPos2D { x: 0.0, y: 0.0 }]
    };

    if is_empty && !input.placeholder.is_empty() {
        text_renderer.draw_text(
            scene,
            &input.placeholder,
            cosmic_text::Attrs::new(),
            input.font_size,
            text_w as f32,
            text_h as f32,
            (text_x as f32, (text_y + top_pad - scroll_y) as f32),
            VelloColor::from_rgba8(128, 128, 128, 255),
            scale,
        );
    } else if !is_empty {
        // Draw selection highlight
        if input.focused
            && input.sel_start != input.sel_end
            && input.sel_start < positions.len()
            && input.sel_end <= positions.len()
        {
            paint_multiline_selection(
                scene,
                &positions,
                input,
                text_x,
                text_y,
                text_w,
                line_height,
                top_pad,
                scroll_y,
                scale,
            );
        }

        // Draw text with wrapping
        text_renderer.draw_text(
            scene,
            &input.display_text,
            cosmic_text::Attrs::new(),
            input.font_size,
            text_w as f32,
            text_h as f32 + input.scroll_offset_y + 10000.0,
            (text_x as f32, (text_y + top_pad - scroll_y) as f32),
            input.text_color.to_vello(),
            scale,
        );
    }

    // Draw cursor — uses same to_screen transform as selection
    if input.focused && input.blink_visible && !positions.is_empty() {
        let cp = if input.cursor_pos < positions.len() {
            positions[input.cursor_pos]
        } else {
            *positions.last().unwrap()
        };
        let (cx, cy) = to_screen(cp, text_x, text_y, top_pad, scroll_y);
        paint_cursor(scene, cx, cy, line_height, scale);
    }
}

fn paint_multiline_selection(
    scene: &mut Scene,
    positions: &[GlyphPos2D],
    input: &InputRenderInfo,
    text_x: f64,
    text_y: f64,
    text_w: f64,
    line_height: f32,
    top_pad: f64,
    scroll_y: f64,
    scale: f64,
) {
    let sel_color = VelloColor::from_rgba8(56, 121, 185, 128);

    let rects = compute_selection_rects(
        positions,
        input.sel_start,
        input.sel_end,
        text_w,
        line_height as f64,
    );

    // Apply the same screen offset used by cursor and text drawing
    let ox = text_x;
    let oy = text_y + top_pad - scroll_y;

    for [x1, y1, x2, y2] in rects {
        scene.fill(
            Fill::NonZero,
            Affine::scale(scale),
            sel_color,
            None,
            &Rect::new(ox + x1, oy + y1, ox + x2, oy + y2),
        );
    }
}

// ── Single-line input rendering ──────────────────────────────────────

fn paint_singleline(
    scene: &mut Scene,
    text_renderer: &mut TextRenderer,
    input: &InputRenderInfo,
    text_x: f64,
    text_y: f64,
    text_w: f64,
    text_h: f64,
    line_height: f32,
    is_empty: bool,
    scale: f64,
) {
    let text_offset_y = ((text_h as f32 - line_height) / 2.0).max(0.0);

    let positions = if !is_empty {
        text_renderer.grapheme_x_positions(&input.display_text, input.font_size)
    } else {
        vec![0.0]
    };

    if is_empty && !input.placeholder.is_empty() {
        text_renderer.draw_text(
            scene,
            &input.placeholder,
            cosmic_text::Attrs::new(),
            input.font_size,
            text_w as f32,
            text_h as f32,
            (text_x as f32, text_y as f32 + text_offset_y),
            VelloColor::from_rgba8(128, 128, 128, 255),
            scale,
        );
    } else if !is_empty {
        // Draw selection highlight
        if input.focused
            && input.sel_start != input.sel_end
            && input.sel_start < positions.len()
            && input.sel_end <= positions.len()
        {
            let x1 = (positions[input.sel_start] - input.scroll_offset) as f64;
            let x2 = (positions[input.sel_end] - input.scroll_offset) as f64;
            let sel_rect = Rect::new(
                text_x + x1,
                text_y + text_offset_y as f64,
                text_x + x2,
                text_y + text_offset_y as f64 + line_height as f64,
            );
            scene.fill(
                Fill::NonZero,
                Affine::scale(scale),
                VelloColor::from_rgba8(56, 121, 185, 128),
                None,
                &sel_rect,
            );
        }

        text_renderer.draw_text(
            scene,
            &input.display_text,
            cosmic_text::Attrs::new(),
            input.font_size,
            text_w as f32 + input.scroll_offset + 10000.0,
            text_h as f32,
            (
                text_x as f32 - input.scroll_offset,
                text_y as f32 + text_offset_y,
            ),
            input.text_color.to_vello(),
            scale,
        );
    }

    // Draw cursor
    if input.focused && input.blink_visible {
        let cursor_x = if input.cursor_pos < positions.len() {
            (positions[input.cursor_pos] - input.scroll_offset) as f64
        } else {
            0.0
        };
        paint_cursor(
            scene,
            text_x + cursor_x,
            text_y + text_offset_y as f64,
            line_height,
            scale,
        );
    }
}

// ── Shared helpers ───────────────────────────────────────────────────

fn paint_cursor(scene: &mut Scene, x: f64, y: f64, line_height: f32, scale: f64) {
    let cursor_rect = Rect::new(x, y + 2.0, x + 1.5, y + line_height as f64 - 2.0);
    scene.fill(
        Fill::NonZero,
        Affine::scale(scale),
        VelloColor::from_rgba8(212, 212, 212, 255),
        None,
        &cursor_rect,
    );
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> GlyphPos2D {
        GlyphPos2D { x, y }
    }

    // ── to_screen ───────────────────────────────────────────────────

    #[test]
    fn to_screen_basic() {
        let (sx, sy) = to_screen(p(10.0, 20.0), 100.0, 50.0, 4.0, 0.0);
        assert_eq!(sx, 110.0); // text_x + pos.x
        assert_eq!(sy, 74.0); // text_y + pos.y + top_pad
    }

    #[test]
    fn to_screen_with_scroll() {
        let (sx, sy) = to_screen(p(10.0, 20.0), 100.0, 50.0, 4.0, 10.0);
        assert_eq!(sx, 110.0);
        assert_eq!(sy, 64.0); // 50 + 20 + 4 - 10
    }

    #[test]
    fn to_screen_origin() {
        let (sx, sy) = to_screen(p(0.0, 0.0), 8.0, 0.0, 4.0, 0.0);
        assert_eq!(sx, 8.0);
        assert_eq!(sy, 4.0);
    }

    #[test]
    fn to_screen_consistency_across_lines() {
        // Cursor on line 0 and line 1 should differ by exactly line_y_delta
        let line_height = 20.0f32;
        let (_, y0) = to_screen(p(5.0, 0.0), 0.0, 0.0, 4.0, 0.0);
        let (_, y1) = to_screen(p(5.0, line_height), 0.0, 0.0, 4.0, 0.0);
        assert!((y1 - y0 - line_height as f64).abs() < 0.01);
    }

    // ── compute_selection_rects ─────────────────────────────────────

    #[test]
    fn sel_rect_empty_selection() {
        let positions = vec![p(0.0, 0.0), p(10.0, 0.0)];
        let rects = compute_selection_rects(&positions, 1, 1, 200.0, 20.0);
        assert!(rects.is_empty());
    }

    #[test]
    fn sel_rect_empty_positions() {
        let rects = compute_selection_rects(&[], 0, 1, 200.0, 20.0);
        assert!(rects.is_empty());
    }

    #[test]
    fn sel_rect_single_line() {
        // "abc" on line 0: positions at x = 0, 10, 20, 30
        let positions = vec![p(0.0, 0.0), p(10.0, 0.0), p(20.0, 0.0), p(30.0, 0.0)];
        let rects = compute_selection_rects(&positions, 1, 3, 200.0, 20.0);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0], [10.0, 0.0, 30.0, 20.0]);
    }

    #[test]
    fn sel_rect_single_line_from_start() {
        let positions = vec![p(0.0, 0.0), p(10.0, 0.0), p(20.0, 0.0)];
        let rects = compute_selection_rects(&positions, 0, 2, 200.0, 20.0);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0], [0.0, 0.0, 20.0, 20.0]);
    }

    #[test]
    fn sel_rect_two_lines() {
        // Line 0: positions 0-2 at y=0, Line 1: positions 3-5 at y=20
        let positions = vec![
            p(0.0, 0.0),
            p(10.0, 0.0),
            p(20.0, 0.0), // line 0
            p(0.0, 20.0),
            p(10.0, 20.0),
            p(20.0, 20.0), // line 1
        ];
        let rects = compute_selection_rects(&positions, 1, 4, 200.0, 20.0);
        assert_eq!(rects.len(), 2);
        // First line: start_x=10 to text_w=200
        assert_eq!(rects[0], [10.0, 0.0, 200.0, 20.0]);
        // Last line: 0 to end_x=10
        assert_eq!(rects[1], [0.0, 20.0, 10.0, 40.0]);
    }

    #[test]
    fn sel_rect_three_lines_middle_full_width() {
        let positions = vec![
            p(0.0, 0.0),
            p(10.0, 0.0), // line 0
            p(0.0, 20.0),
            p(15.0, 20.0), // line 1
            p(0.0, 40.0),
            p(10.0, 40.0), // line 2
        ];
        let rects = compute_selection_rects(&positions, 0, 5, 200.0, 20.0);
        assert_eq!(rects.len(), 3);
        // First line → right edge
        assert_eq!(rects[0], [0.0, 0.0, 200.0, 20.0]);
        // Middle line → full width (has content at x=15)
        assert_eq!(rects[1], [0.0, 20.0, 200.0, 40.0]);
        // Last line → left to end_x
        assert_eq!(rects[2], [0.0, 40.0, 10.0, 60.0]);
    }

    #[test]
    fn sel_rect_last_line_at_x_zero_gets_stub() {
        // Selection ends at start of new line (after \n)
        let positions = vec![
            p(0.0, 0.0),
            p(10.0, 0.0),
            p(20.0, 0.0),
            p(30.0, 0.0), // line 0: "abc"
            p(0.0, 20.0), // line 1: after \n, x=0
        ];
        let rects = compute_selection_rects(&positions, 0, 4, 200.0, 20.0);
        assert_eq!(rects.len(), 2);
        // First line full width
        assert_eq!(rects[0], [0.0, 0.0, 200.0, 20.0]);
        // Last line: stub (end_x < 1.0)
        assert_eq!(rects[1], [0.0, 20.0, 8.0, 40.0]);
    }

    #[test]
    fn sel_rect_empty_middle_line_gets_stub() {
        // Line 0 has content, line 1 is empty (only x=0), line 2 has content
        let positions = vec![
            p(0.0, 0.0),
            p(10.0, 0.0), // line 0
            p(0.0, 20.0), // line 1 (empty — only x=0 position)
            p(0.0, 40.0),
            p(10.0, 40.0), // line 2
        ];
        let rects = compute_selection_rects(&positions, 0, 4, 200.0, 20.0);
        assert_eq!(rects.len(), 3);
        assert_eq!(rects[0], [0.0, 0.0, 200.0, 20.0]); // first line
        assert_eq!(rects[1], [0.0, 20.0, 8.0, 40.0]); // empty middle → stub
        assert_eq!(rects[2], [0.0, 40.0, 10.0, 60.0]); // last line
    }

    #[test]
    fn sel_rect_sel_end_clamped_to_positions_len() {
        let positions = vec![p(0.0, 0.0), p(10.0, 0.0), p(20.0, 0.0)];
        // sel_end beyond positions.len() should be clamped
        let rects = compute_selection_rects(&positions, 0, 100, 200.0, 20.0);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0], [0.0, 0.0, 20.0, 20.0]);
    }

    // ── Integration: to_screen + compute_selection_rects ────────────

    #[test]
    fn cursor_and_selection_use_same_offset() {
        // Verify that cursor at sel_start and selection rect start coincide
        let positions = vec![
            p(0.0, 0.0),
            p(10.0, 0.0),
            p(20.0, 0.0),
            p(0.0, 20.0),
            p(10.0, 20.0),
        ];
        let text_x = 50.0;
        let text_y = 100.0;
        let top_pad = 4.0;
        let scroll_y = 0.0;

        // Cursor at position 1
        let (cx, cy) = to_screen(positions[1], text_x, text_y, top_pad, scroll_y);

        // Selection starting at position 1
        let rects = compute_selection_rects(&positions, 1, 3, 200.0, 20.0);
        let sel_x = text_x + rects[0][0]; // offset applied by caller
        let sel_y = text_y + top_pad - scroll_y + rects[0][1];

        assert!(
            (cx - sel_x).abs() < 0.01,
            "cursor x and selection start x must match: {} vs {}",
            cx,
            sel_x
        );
        assert!(
            (cy - sel_y).abs() < 0.01,
            "cursor y and selection start y must match: {} vs {}",
            cy,
            sel_y
        );
    }
}
