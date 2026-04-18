use vello::Scene;

use crate::style::{Bounds, Color, UzStyle};
use crate::text::TextRenderer;

#[allow(clippy::too_many_arguments)]
pub fn paint_text(
    scene: &mut Scene,
    text_renderer: &mut TextRenderer,
    bounds: Bounds,
    style: &UzStyle,
    content: &str,
    font_size: f32,
    color: Color,
    scale: f64,
) {
    style.paint(bounds, scene, scale, |scene| {
        text_renderer.draw_text(
            scene,
            content,
            font_size,
            bounds.width as f32,
            bounds.height as f32,
            (bounds.x as f32, bounds.y as f32),
            color.to_vello(),
            scale,
        );
    });
}
