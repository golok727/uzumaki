use vello::Scene;
use vello::kurbo::{Affine, BezPath, Stroke as KStroke};
use vello::peniko::{Brush, Color as VelloColor, Fill};

use crate::style::Color;

pub fn render_svg_tree(
    scene: &mut Scene,
    tree: &usvg::Tree,
    transform: Affine,
    current_color: Option<Color>,
) {
    render_group(scene, tree.root(), transform, current_color);
}

fn render_group(
    scene: &mut Scene,
    group: &usvg::Group,
    parent_transform: Affine,
    current_color: Option<Color>,
) {
    let local_transform = parent_transform * to_affine(group.transform());
    let opacity = group.opacity().get();
    let has_layer = opacity < 1.0 || group.clip_path().is_some();

    if has_layer {
        scene.push_layer(
            Fill::NonZero,
            vello::peniko::Mix::Normal,
            opacity,
            local_transform,
            &vello::kurbo::Rect::new(f64::MIN, f64::MIN, f64::MAX, f64::MAX),
        );
    }

    for node in group.children() {
        match node {
            usvg::Node::Group(g) => render_group(scene, g, local_transform, current_color),
            usvg::Node::Path(path) => render_path(scene, path, local_transform, current_color),
            usvg::Node::Image(_) => {}
            usvg::Node::Text(text) => {
                render_group(scene, text.flattened(), local_transform, current_color)
            }
        }
    }

    if has_layer {
        scene.pop_layer();
    }
}

fn render_path(
    scene: &mut Scene,
    path: &usvg::Path,
    transform: Affine,
    current_color: Option<Color>,
) {
    if !path.is_visible() {
        return;
    }
    let bez = to_bez_path(path.data());

    let paint_fill = |scene: &mut Scene| {
        if let Some(fill) = path.fill() {
            let brush = paint_to_brush(fill.paint(), fill.opacity().get(), current_color);
            let rule = match fill.rule() {
                usvg::FillRule::NonZero => Fill::NonZero,
                usvg::FillRule::EvenOdd => Fill::EvenOdd,
            };
            scene.fill(rule, transform, &brush, None, &bez);
        }
    };
    let paint_stroke = |scene: &mut Scene| {
        if let Some(stroke) = path.stroke() {
            let brush = paint_to_brush(stroke.paint(), stroke.opacity().get(), current_color);
            let mut k = KStroke::new(stroke.width().get() as f64);
            k.miter_limit = stroke.miterlimit().get() as f64;
            scene.stroke(&k, transform, &brush, None, &bez);
        }
    };

    match path.paint_order() {
        usvg::PaintOrder::FillAndStroke => {
            paint_fill(scene);
            paint_stroke(scene);
        }
        usvg::PaintOrder::StrokeAndFill => {
            paint_stroke(scene);
            paint_fill(scene);
        }
    }
}

fn paint_to_brush(paint: &usvg::Paint, opacity: f32, current_color: Option<Color>) -> Brush {
    let alpha = (opacity.clamp(0.0, 1.0) * 255.0) as u8;
    match paint {
        usvg::Paint::Color(c) => {
            if let Some(color) = current_color
                && c.red == 0
                && c.green == 0
                && c.blue == 0
            {
                return Brush::Solid(VelloColor::from_rgba8(color.r, color.g, color.b, alpha));
            }
            Brush::Solid(VelloColor::from_rgba8(c.red, c.green, c.blue, alpha))
        }
        _ => Brush::Solid(VelloColor::from_rgba8(128, 128, 128, alpha)),
    }
}

fn to_affine(t: usvg::Transform) -> Affine {
    Affine::new([
        t.sx as f64,
        t.ky as f64,
        t.kx as f64,
        t.sy as f64,
        t.tx as f64,
        t.ty as f64,
    ])
}

fn to_bez_path(p: &usvg::tiny_skia_path::Path) -> BezPath {
    let mut bez = BezPath::new();
    for seg in p.segments() {
        match seg {
            usvg::tiny_skia_path::PathSegment::MoveTo(pt) => {
                bez.move_to((pt.x as f64, pt.y as f64));
            }
            usvg::tiny_skia_path::PathSegment::LineTo(pt) => {
                bez.line_to((pt.x as f64, pt.y as f64));
            }
            usvg::tiny_skia_path::PathSegment::QuadTo(p1, p2) => {
                bez.quad_to((p1.x as f64, p1.y as f64), (p2.x as f64, p2.y as f64));
            }
            usvg::tiny_skia_path::PathSegment::CubicTo(p1, p2, p3) => {
                bez.curve_to(
                    (p1.x as f64, p1.y as f64),
                    (p2.x as f64, p2.y as f64),
                    (p3.x as f64, p3.y as f64),
                );
            }
            usvg::tiny_skia_path::PathSegment::Close => {
                bez.close_path();
            }
        }
    }
    bez
}
