use vello::Scene;

use crate::style::{Bounds, Color, TextStyle, UzStyle};
use crate::text::TextRenderer;

#[allow(clippy::too_many_arguments)]
pub fn paint_text(
    scene: &mut Scene,
    text_renderer: &mut TextRenderer,
    bounds: Bounds,
    style: &UzStyle,
    content: &str,
    text_style: &TextStyle,
    color: Color,
    scale: f64,
) {
    style.paint(bounds, scene, scale, |scene| {
        text_renderer.draw_text(
            scene,
            content,
            text_style,
            bounds.width as f32,
            bounds.height as f32,
            (bounds.x as f32, bounds.y as f32),
            color.to_vello(),
            scale,
        );
    });
}
