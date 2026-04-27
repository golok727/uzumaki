use vello::Scene;
use vello::kurbo::Affine;

use crate::style::{Bounds, UzStyle};

#[derive(Clone)]
pub struct ImageRenderInfo {
    pub image: vello::peniko::ImageData,
}

pub fn paint_image(
    scene: &mut Scene,
    bounds: Bounds,
    style: &UzStyle,
    image: &ImageRenderInfo,
    transform: Affine,
) {
    if image.image.width == 0 || image.image.height == 0 {
        style.paint(bounds, scene, transform, |_| {});
        return;
    }

    style.paint(bounds, scene, transform, |scene| {
        let scale_x = bounds.width / image.image.width as f64;
        let scale_y = bounds.height / image.image.height as f64;
        let image_transform = transform * Affine::new([scale_x, 0.0, 0.0, scale_y, 0.0, 0.0]);
        scene.draw_image(&image.image, image_transform);
    });
}
