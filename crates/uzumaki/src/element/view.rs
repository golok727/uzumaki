use vello::Scene;

use crate::style::{Bounds, Style};

/// Paint a view element: background, borders, rounded corners.
/// The `paint_children` callback is invoked between background and any overlays.
pub fn paint_view(
    scene: &mut Scene,
    bounds: Bounds,
    style: &Style,
    scale: f64,
    paint_children: impl FnOnce(&mut Scene),
) {
    style.paint(bounds, scene, scale, paint_children);
}
