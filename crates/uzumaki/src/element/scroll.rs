//! Shared scrollbar geometry and painting.
//!
//! The render walker registers per-axis hit rects and emits paint commands;
//! the actual visual style of the thumb (overlay, 4px, rounded) lives here so
//! every scrollable surface — views and multiline inputs — agrees.

use vello::Scene;
use vello::kurbo::{Affine, Rect, RoundedRect, RoundedRectRadii};
use vello::peniko::{Color as VelloColor, Fill};

use crate::element::ScrollAxis;
use crate::style::Bounds;

pub const THUMB_THICKNESS: f64 = 4.0;
pub const THUMB_MARGIN: f64 = 4.0;
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
/// the movable range along the drag axis (used by drag handlers).
#[derive(Clone, Copy, Debug)]
pub struct ThumbGeometry {
    pub local_x: f64,
    pub local_y: f64,
    pub width: f64,
    pub height: f64,
    pub track_range: f64,
}

pub fn thumb_geometry(axis: ScrollAxis, view: Bounds, info: ScrollAxisInfo) -> ThumbGeometry {
    let max_scroll = info.max_scroll();
    let scroll_ratio = if max_scroll > 0.0 {
        info.clamped_offset() / max_scroll
    } else {
        0.0
    };
    match axis {
        ScrollAxis::Y => {
            let track = view.height;
            let length =
                (track * info.visible_size / info.content_size.max(1.0)).max(THUMB_MIN_LENGTH);
            let track_range = (track - length).max(0.0);
            ThumbGeometry {
                local_x: view.width - THUMB_THICKNESS - THUMB_MARGIN,
                local_y: scroll_ratio * track_range,
                width: THUMB_THICKNESS,
                height: length,
                track_range,
            }
        }
        ScrollAxis::X => {
            let track = view.width;
            let length =
                (track * info.visible_size / info.content_size.max(1.0)).max(THUMB_MIN_LENGTH);
            let track_range = (track - length).max(0.0);
            ThumbGeometry {
                local_x: scroll_ratio * track_range,
                local_y: view.height - THUMB_THICKNESS - THUMB_MARGIN,
                width: length,
                height: THUMB_THICKNESS,
                track_range,
            }
        }
    }
}

pub fn paint_thumb(scene: &mut Scene, transform: Affine, geom: &ThumbGeometry, hovered: bool) {
    let alpha = if hovered { 140u8 } else { 90u8 };
    let color = VelloColor::from_rgba8(255, 255, 255, alpha);
    let radius = geom.width.min(geom.height) / 2.0;
    let rect = Rect::new(
        geom.local_x,
        geom.local_y,
        geom.local_x + geom.width,
        geom.local_y + geom.height,
    );
    let rounded = RoundedRect::from_rect(rect, RoundedRectRadii::from_single_radius(radius));
    scene.fill(Fill::NonZero, transform, color, None, &rounded);
}
