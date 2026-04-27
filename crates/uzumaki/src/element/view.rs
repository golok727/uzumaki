use vello::Scene;
use vello::kurbo::Affine;

use crate::style::{Bounds, UzStyle};

/// Paint a view element: background, borders, rounded corners.
/// The `paint_children` callback is invoked between background and any overlays.
pub fn paint_view(
    scene: &mut Scene,
    bounds: Bounds,
    style: &UzStyle,
    transform: Affine,
    paint_children: impl FnOnce(&mut Scene),
) {
    style.paint(bounds, scene, transform, paint_children);
}
