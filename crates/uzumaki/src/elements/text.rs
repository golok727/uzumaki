use cosmic_text::Attrs;
use vello::Scene;

use crate::style::{Bounds, Color, Style};
use crate::text::TextRenderer;

/// Paint a text element: background/borders from style, then text content.
pub fn paint_text(
    scene: &mut Scene,
    text_renderer: &mut TextRenderer,
    bounds: Bounds,
    style: &Style,
    content: &str,
    font_size: f32,
    color: Color,
    scale: f64,
) {
    style.paint(bounds, scene, scale, |scene| {
        text_renderer.draw_text(
            scene,
            content,
            Attrs::new(),
            font_size,
            bounds.width as f32,
            bounds.height as f32,
            (bounds.x as f32, bounds.y as f32),
            color.to_vello(),
            scale,
        );
    });
}
