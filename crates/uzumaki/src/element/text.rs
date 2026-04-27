use vello::Scene;
use vello::kurbo::Affine;

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
    transform: Affine,
) {
    style.paint(bounds, scene, transform, |scene| {
        text_renderer.draw_text(
            scene,
            content,
            text_style,
            bounds.width as f32,
            bounds.height as f32,
            (0.0, 0.0),
            color.to_vello(),
            transform,
        );
    });
}
