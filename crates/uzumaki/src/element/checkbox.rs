use vello::Scene;
use vello::kurbo::{Affine, BezPath, Cap, Join, RoundedRect, RoundedRectRadii, Stroke};
use vello::peniko::Fill;

use crate::style::{Bounds, Color, Corners, Edges, UzStyle};

pub struct CheckboxRenderInfo {
    pub checked: bool,
    pub focused: bool,
}

pub fn paint_checkbox(
    scene: &mut Scene,
    bounds: Bounds,
    style: &UzStyle,
    checkbox: &CheckboxRenderInfo,
    scale: f64,
) {
    let mut paint_style = style.clone();
    let accent = paint_style.background.unwrap_or(Color::rgb(59, 130, 246));
    let border = paint_style.border_color.unwrap_or(if checkbox.checked {
        accent
    } else {
        Color::rgba(148, 163, 184, 255)
    });

    if !paint_style.border_widths.any_nonzero() {
        paint_style.border_widths = Edges::all(1.5);
    }
    if !paint_style.corner_radii.any_nonzero() {
        paint_style.corner_radii =
            Corners::uniform((bounds.width.min(bounds.height) * 0.22) as f32);
    }
    paint_style.border_color = Some(border);
    paint_style.background = if checkbox.checked {
        Some(accent)
    } else {
        Some(Color::TRANSPARENT)
    };

    if checkbox.focused {
        let halo = RoundedRect::from_rect(
            vello::kurbo::Rect::new(
                bounds.x - 3.0,
                bounds.y - 3.0,
                bounds.x + bounds.width + 3.0,
                bounds.y + bounds.height + 3.0,
            ),
            RoundedRectRadii::from_single_radius((bounds.width.min(bounds.height) * 0.22) + 3.0),
        );
        scene.fill(
            Fill::NonZero,
            Affine::scale(scale),
            accent.with_opacity(0.22).to_vello(),
            None,
            &halo,
        );
    }

    paint_style.paint(bounds, scene, scale, |_| {});

    if checkbox.checked {
        let mut path = BezPath::new();
        path.move_to((
            bounds.x + bounds.width * 0.24,
            bounds.y + bounds.height * 0.52,
        ));
        path.line_to((
            bounds.x + bounds.width * 0.43,
            bounds.y + bounds.height * 0.71,
        ));
        path.line_to((
            bounds.x + bounds.width * 0.76,
            bounds.y + bounds.height * 0.30,
        ));

        let stroke = Stroke {
            width: (bounds.width.min(bounds.height) * 0.14).max(2.0),
            join: Join::Round,
            miter_limit: 10.0,
            start_cap: Cap::Round,
            end_cap: Cap::Round,
            dash_pattern: Default::default(),
            dash_offset: 0.0,
        };

        scene.stroke(
            &stroke,
            Affine::scale(scale),
            style.text.color.to_vello(),
            None,
            &path,
        );
    }
}
