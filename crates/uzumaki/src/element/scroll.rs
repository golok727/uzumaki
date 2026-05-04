//! Shared scrollbar geometry and painting.
//!
//! The render walker registers per-axis hit rects and emits paint commands;
//! the actual visual style of the thumb (width, colors, radius) lives in
//! `ScrollbarStyle` on the owning node and is plumbed through here so every
//! scrollable surface — views and multiline inputs — agrees.

use vello::Scene;
use vello::kurbo::{Affine, Rect, RoundedRect, RoundedRectRadii};
use vello::peniko::Fill;

use crate::element::ScrollAxis;
use crate::style::{Bounds, ScrollbarStyle};

pub const SCROLLBAR_SIDE_MARGIN: f64 = 4.0; // gap between thumb and container edge (perpendicular axis)
pub const SCROLLBAR_END_MARGIN: f64 = 8.0; // gap at track start/end (along scroll axis)
pub const THUMB_MIN_LENGTH: f64 = 24.0;

/// A scrollable extent on one axis. Geometry-only — the owner of this info
/// (e.g. `ScrollState` for views, `InputState` for multiline inputs) holds
/// the actual offset.
#[derive(Clone, Copy, Debug)]
pub struct ScrollAxisInfo {
    pub content_size: f64,
    pub visible_size: f64,
    pub offset: f64,
}

impl ScrollAxisInfo {
    pub fn max_scroll(&self) -> f64 {
        (self.content_size - self.visible_size).max(0.0)
    }

    pub fn overflows(&self) -> bool {
        self.content_size > self.visible_size
    }

    pub fn clamped_offset(&self) -> f64 {
        self.offset.clamp(0.0, self.max_scroll())
    }
}

/// Local-space rect of a scrollbar thumb in the view's transform space, plus
/// the movable range along the drag axis (used by drag handlers) and the
/// track rect (used by paint to draw the optional track background).
#[derive(Clone, Copy, Debug)]
pub struct ThumbGeometry {
    pub local_x: f64,
    pub local_y: f64,
    pub thumb_width: f64,
    pub thumb_height: f64,
    pub track_range: f64,
    // unused for now
    pub track_x: f64,
    pub track_y: f64,
    pub track_width: f64,
    pub track_height: f64,
}

pub fn thumb_geometry(
    axis: ScrollAxis,
    view: Bounds,
    info: ScrollAxisInfo,
    style: &ScrollbarStyle,
) -> ThumbGeometry {
    let thickness = (style.width as f64).max(0.0);
    let max_scroll = info.max_scroll();
    let scroll_ratio = if max_scroll > 0.0 {
        info.clamped_offset() / max_scroll
    } else {
        0.0
    };
    match axis {
        ScrollAxis::Y => {
            let track = view.height - SCROLLBAR_END_MARGIN - thickness;
            let length =
                (track * info.visible_size / info.content_size.max(1.0)).max(THUMB_MIN_LENGTH);
            let track_range = (track - length).max(0.0);
            let track_x = view.width - thickness - SCROLLBAR_SIDE_MARGIN;
            ThumbGeometry {
                local_x: track_x,
                local_y: SCROLLBAR_END_MARGIN + scroll_ratio * track_range,
                thumb_width: thickness,
                thumb_height: length,
                track_range,
                track_x,
                track_y: SCROLLBAR_END_MARGIN,
                track_width: thickness,
                track_height: track,
            }
        }
        ScrollAxis::X => {
            let track = view.width - SCROLLBAR_END_MARGIN - thickness;
            let length =
                (track * info.visible_size / info.content_size.max(1.0)).max(THUMB_MIN_LENGTH);
            let track_range = (track - length).max(0.0);
            let track_y = view.height - thickness - SCROLLBAR_SIDE_MARGIN;
            ThumbGeometry {
                local_x: SCROLLBAR_END_MARGIN + scroll_ratio * track_range,
                local_y: track_y,
                thumb_width: length,
                thumb_height: thickness,
                track_range,
                track_x: SCROLLBAR_END_MARGIN,
                track_y,
                track_width: track,
                track_height: thickness,
            }
        }
    }
}

pub fn paint_thumb(
    scene: &mut Scene,
    transform: Affine,
    geom: &ThumbGeometry,
    hovered: bool,
    style: &ScrollbarStyle,
) {
    if geom.thumb_width <= 0.0 || geom.thumb_height <= 0.0 {
        return;
    }

    let color = if hovered {
        style.hover_color
    } else {
        style.color
    };
    if color.is_transparent() {
        return;
    }

    let radius = style
        .radius
        .map(|r| r as f64)
        .unwrap_or_else(|| geom.thumb_width.min(geom.thumb_height) / 2.0)
        .max(0.0);
    let rect = Rect::new(
        geom.local_x,
        geom.local_y,
        geom.local_x + geom.thumb_width,
        geom.local_y + geom.thumb_height,
    );
    let rounded = RoundedRect::from_rect(rect, RoundedRectRadii::from_single_radius(radius));
    scene.fill(Fill::NonZero, transform, color.to_vello(), None, &rounded);
}
