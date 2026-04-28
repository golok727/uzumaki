use parley::BoundingBox;
use vello::Scene;
use vello::kurbo::{Affine, Rect};
use vello::peniko::{Color as VelloColor, Fill};

use crate::style::{Bounds, Color, Corners, Edges, TextStyle, UzStyle};
use crate::text::TextRenderer;

pub struct InputContentInfo {
    pub content_height: f64,
    pub visible_height: f64,
    pub scroll_offset_y: f64,
}

pub struct InputRenderInfo {
    pub display_text: String,
    pub placeholder: String,
    pub text_style: TextStyle,
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
    transform: Affine,
) -> Option<InputContentInfo> {
    let pad_l = style.padding.left as f64;
    let pad_r = style.padding.right as f64;
    let pad_t = style.padding.top as f64;
    let pad_b = style.padding.bottom as f64;
    let content_x = pad_l;
    let content_y = pad_t;
    let content_w = (bounds.width - pad_l - pad_r).max(0.0);
    let content_h = (bounds.height - pad_t - pad_b).max(0.0);

    let mut paint_style = style.clone();
    if !paint_style.border_widths.any_nonzero() {
        paint_style.border_widths = Edges::all(1.0);
    }
    if paint_style.border_color.is_none() {
        paint_style.border_color = Some(Color::rgba(60, 60, 60, 255));
    }
    if paint_style.background.is_none() {
        paint_style.background = Some(Color::rgba(30, 30, 30, 255));
    }
    if !paint_style.corner_radii.any_nonzero() {
        paint_style.corner_radii = Corners::uniform(4.0);
    }

    paint_style.paint(bounds, scene, transform, |_| {});

    // Clip to text area
    let clip_rect = Rect::new(
        content_x,
        content_y,
        content_x + content_w,
        content_y + content_h,
    );
    scene.push_clip_layer(Fill::NonZero, transform, &clip_rect);

    let is_empty = input.display_text.is_empty();
    let line_height = (input.text_style.font_size * input.text_style.line_height).round();
    let scroll_y = input.scroll_offset_y as f64;

    // Placeholder
    if is_empty && !input.placeholder.is_empty() {
        let py = if input.multiline {
            content_y as f32
        } else {
            content_y as f32 + ((content_h as f32 - line_height) / 2.0).max(0.0)
        };
        text_renderer.draw_text(
            scene,
            &input.placeholder,
            &input.text_style,
            content_w as f32,
            content_h as f32,
            (content_x as f32, py),
            VelloColor::from_rgba8(128, 128, 128, 255),
            transform,
        );
    }

    if !is_empty {
        // Selection highlights
        if input.focused && !input.selection_rects.is_empty() {
            let sel_color = VelloColor::from_rgba8(56, 121, 185, 128);
            let oy = if input.multiline {
                content_y - scroll_y
            } else {
                content_y + ((content_h - line_height as f64) / 2.0).max(0.0)
            };
            for rect in &input.selection_rects {
                let x1 = content_x + rect.x0
                    - if input.multiline {
                        0.0
                    } else {
                        input.scroll_offset as f64
                    };
                let x2 = content_x + rect.x1
                    - if input.multiline {
                        0.0
                    } else {
                        input.scroll_offset as f64
                    };
                let y1 = oy + rect.y0;
                let y2 = oy + rect.y1;
                scene.fill(
                    Fill::NonZero,
                    transform,
                    sel_color,
                    None,
                    &Rect::new(x1, y1, x2, y2),
                );
            }
        }

        // Text
        let ty = if input.multiline {
            (content_y - scroll_y) as f32
        } else {
            content_y as f32 + ((content_h as f32 - line_height) / 2.0).max(0.0)
        };
        let tw = if input.multiline {
            content_w as f32
        } else {
            content_w as f32 + input.scroll_offset + 10000.0
        };
        let tx = if input.multiline {
            content_x as f32
        } else {
            content_x as f32 - input.scroll_offset
        };
        text_renderer.draw_text(
            scene,
            &input.display_text,
            &input.text_style,
            tw,
            content_h as f32
                + if input.multiline {
                    input.scroll_offset_y + 10000.0
                } else {
                    0.0
                },
            (tx, ty),
            input.text_style.color.to_vello(),
            transform,
        );
    }

    // Preedit (IME composition text)
    if let Some(preedit) = &input.preedit
        && let Some(cr) = &input.cursor_rect
    {
        let oy = if input.multiline {
            content_y - scroll_y
        } else {
            content_y + ((content_h - line_height as f64) / 2.0).max(0.0)
        };
        let px = content_x + cr.x0
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
        scene.fill(Fill::NonZero, transform, preedit_bg, None, &preedit_rect);

        // Preedit text
        text_renderer.draw_text(
            scene,
            &preedit.text,
            &input.text_style,
            preedit.width + 100.0,
            content_h as f32,
            (px as f32, py as f32),
            input.text_style.color.to_vello(),
            transform,
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
            transform,
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
            content_y - scroll_y
        } else {
            content_y + ((content_h - line_height as f64) / 2.0).max(0.0)
        };
        let cx = content_x + cr.x0
            - if input.multiline {
                0.0
            } else {
                input.scroll_offset as f64
            };
        let cy = oy + cr.y0;
        let cursor_rect = Rect::new(cx, cy + 2.0, cx + 1.5, cy + cr.y1 - cr.y0 - 2.0);
        scene.fill(
            Fill::NonZero,
            transform,
            VelloColor::from_rgba8(212, 212, 212, 255),
            None,
            &cursor_rect,
        );
    }

    scene.pop_layer();

    if input.multiline {
        let content_height = input.layout_height as f64 + pad_t + pad_b;
        Some(InputContentInfo {
            content_height,
            visible_height: content_h,
            scroll_offset_y: scroll_y,
        })
    } else {
        None
    }
}
