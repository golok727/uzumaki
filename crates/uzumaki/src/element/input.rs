use parley::BoundingBox;
use vello::Scene;
use vello::kurbo::{Affine, Rect};
use vello::peniko::{Color as VelloColor, Fill};

use crate::style::{Bounds, Color, Corners, Edges, UzStyle};
use crate::text::TextRenderer;

pub struct InputContentInfo {
    pub content_height: f64,
    pub visible_height: f64,
    pub scroll_offset_y: f64,
}

pub struct InputRenderInfo {
    pub display_text: String,
    pub placeholder: String,
    pub font_size: f32,
    pub text_color: Color,
    pub focused: bool,
    pub cursor_rect: Option<BoundingBox>,
    pub selection_rects: Vec<BoundingBox>,
    pub scroll_offset: f32,
    pub scroll_offset_y: f32,
    pub blink_visible: bool,
    pub multiline: bool,
    pub layout_height: f32,
    pub preedit: Option<PreeditRenderInfo>,
}

pub struct PreeditRenderInfo {
    pub text: String,
    pub cursor_x: f32,
    pub width: f32,
}

/// Paint an input element with its text, selection highlight, and cursor.
/// Returns the content height for multiline inputs (for scrollbar rendering).
pub fn paint_input(
    scene: &mut Scene,
    text_renderer: &mut TextRenderer,
    bounds: Bounds,
    style: &UzStyle,
    input: &InputRenderInfo,
    scale: f64,
) -> Option<InputContentInfo> {
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
    let scroll_y = input.scroll_offset_y as f64;

    let top_pad: f64 = if style.padding.top > 0.0 {
        style.padding.top as f64
    } else {
        4.0
    };

    // Placeholder
    if is_empty && !input.placeholder.is_empty() {
        let py = if input.multiline {
            (text_y + top_pad) as f32
        } else {
            text_y as f32 + ((text_h as f32 - line_height) / 2.0).max(0.0)
        };
        text_renderer.draw_text(
            scene,
            &input.placeholder,
            input.font_size,
            text_w as f32,
            text_h as f32,
            (text_x as f32, py),
            VelloColor::from_rgba8(128, 128, 128, 255),
            scale,
        );
    }

    if !is_empty {
        // Selection highlights
        if input.focused && !input.selection_rects.is_empty() {
            let sel_color = VelloColor::from_rgba8(56, 121, 185, 128);
            let oy = if input.multiline {
                text_y + top_pad - scroll_y
            } else {
                text_y + ((text_h - line_height as f64) / 2.0).max(0.0)
            };
            for rect in &input.selection_rects {
                let x1 = text_x + rect.x0
                    - if input.multiline {
                        0.0
                    } else {
                        input.scroll_offset as f64
                    };
                let x2 = text_x + rect.x1
                    - if input.multiline {
                        0.0
                    } else {
                        input.scroll_offset as f64
                    };
                let y1 = oy + rect.y0;
                let y2 = oy + rect.y1;
                scene.fill(
                    Fill::NonZero,
                    Affine::scale(scale),
                    sel_color,
                    None,
                    &Rect::new(x1, y1, x2, y2),
                );
            }
        }

        // Text
        let ty = if input.multiline {
            (text_y + top_pad - scroll_y) as f32
        } else {
            text_y as f32 + ((text_h as f32 - line_height) / 2.0).max(0.0)
        };
        let tw = if input.multiline {
            text_w as f32
        } else {
            text_w as f32 + input.scroll_offset + 10000.0
        };
        let tx = if input.multiline {
            text_x as f32
        } else {
            text_x as f32 - input.scroll_offset
        };
        text_renderer.draw_text(
            scene,
            &input.display_text,
            input.font_size,
            tw,
            text_h as f32
                + if input.multiline {
                    input.scroll_offset_y + 10000.0
                } else {
                    0.0
                },
            (tx, ty),
            input.text_color.to_vello(),
            scale,
        );
    }

    // Preedit (IME composition text)
    if let Some(preedit) = &input.preedit
        && let Some(cr) = &input.cursor_rect
    {
        let oy = if input.multiline {
            text_y + top_pad - scroll_y
        } else {
            text_y + ((text_h - line_height as f64) / 2.0).max(0.0)
        };
        let px = text_x + cr.x0
            - if input.multiline {
                0.0
            } else {
                input.scroll_offset as f64
            };
        let py = oy + cr.y0;
        let preedit_h = cr.y1 - cr.y0;

        // Background highlight for preedit
        let preedit_bg = VelloColor::from_rgba8(50, 50, 60, 180);
        let preedit_rect = Rect::new(px, py, px + preedit.width as f64, py + preedit_h);
        scene.fill(
            Fill::NonZero,
            Affine::scale(scale),
            preedit_bg,
            None,
            &preedit_rect,
        );

        // Preedit text
        text_renderer.draw_text(
            scene,
            &preedit.text,
            input.font_size,
            preedit.width + 100.0,
            text_h as f32,
            (px as f32, py as f32),
            input.text_color.to_vello(),
            scale,
        );

        // Underline
        let underline_y = py + preedit_h - 1.0;
        let underline = Rect::new(
            px,
            underline_y,
            px + preedit.width as f64,
            underline_y + 1.0,
        );
        scene.fill(
            Fill::NonZero,
            Affine::scale(scale),
            VelloColor::from_rgba8(180, 180, 180, 255),
            None,
            &underline,
        );
    }

    // Cursor (hide during preedit)
    if input.focused
        && input.blink_visible
        && input.preedit.is_none()
        && let Some(cr) = &input.cursor_rect
    {
        let oy = if input.multiline {
            text_y + top_pad - scroll_y
        } else {
            text_y + ((text_h - line_height as f64) / 2.0).max(0.0)
        };
        let cx = text_x + cr.x0
            - if input.multiline {
                0.0
            } else {
                input.scroll_offset as f64
            };
        let cy = oy + cr.y0;
        let cursor_rect = Rect::new(cx, cy + 2.0, cx + 1.5, cy + cr.y1 - cr.y0 - 2.0);
        scene.fill(
            Fill::NonZero,
            Affine::scale(scale),
            VelloColor::from_rgba8(212, 212, 212, 255),
            None,
            &cursor_rect,
        );
    }

    scene.pop_layer();

    if input.multiline {
        let content_height = input.layout_height as f64 + top_pad;
        Some(InputContentInfo {
            content_height,
            visible_height: text_h,
            scroll_offset_y: scroll_y,
        })
    } else {
        None
    }
}
