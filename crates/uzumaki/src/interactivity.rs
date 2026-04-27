use refineable::Refineable;
use vello::kurbo::{Affine, Point};

use crate::element::UzNodeId;
use crate::style::{Bounds, UzStyle, UzStyleRefinement};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HitboxId(pub u64);

#[derive(Clone, Debug)]
pub struct Hitbox {
    pub id: HitboxId,
    pub node_id: UzNodeId,
    /// Axis-aligned logical bounds kept for legacy geometry consumers.
    pub bounds: Bounds,
    /// The node-local hit region before transform.
    pub local_bounds: Bounds,
    /// Logical-space transform from local node coordinates to window coordinates.
    pub transform: Affine,
}

impl Hitbox {
    /// Check if this hitbox is hovered according to the current hit test result.
    pub fn is_hovered(&self, hit_state: &HitTestState) -> bool {
        hit_state.is_hovered(self.node_id)
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        let local = self.transform.inverse() * Point::new(x, y);
        self.local_bounds.contains(local.x, local.y)
    }
}

/// Stores the result of a hit test: which hitboxes the mouse is currently over.
#[derive(Clone, Debug, Default)]
pub struct HitTestState {
    /// Mouse position in window coordinates.
    pub mouse_position: Option<(f64, f64)>,
    /// Set of node IDs that the mouse is currently over (back-to-front order).
    pub hovered_nodes: Vec<UzNodeId>,
    /// The topmost (frontmost) hovered node, if any.
    pub top_node: Option<UzNodeId>,
    /// Which node is currently pressed (mouse down without mouse up).
    pub active_node: Option<UzNodeId>,
}

impl HitTestState {
    pub fn is_hovered(&self, node_id: UzNodeId) -> bool {
        self.hovered_nodes.contains(&node_id)
    }

    pub fn is_active(&self, node_id: UzNodeId) -> bool {
        self.active_node == Some(node_id) && self.is_hovered(node_id)
    }
}

/// Stores all hitboxes registered during a paint pass. Order matters (back to front).
#[derive(Clone, Debug, Default)]
pub struct HitboxStore {
    hitboxes: Vec<Hitbox>,
    next_id: u64,
}

impl HitboxStore {
    pub fn clear(&mut self) {
        self.hitboxes.clear();
        self.next_id = 0;
    }

    /// Drop any hitbox whose `node_id` no longer passes `keep`.
    /// Used by Dom::on_node_removed to scrub stale references after a node is freed.
    pub fn retain_by_node(&mut self, mut keep: impl FnMut(UzNodeId) -> bool) {
        self.hitboxes.retain(|h| keep(h.node_id));
    }

    /// Register a hitbox and return its ID.
    pub fn insert(&mut self, node_id: UzNodeId, bounds: Bounds) -> HitboxId {
        self.insert_transformed(node_id, bounds, Affine::IDENTITY)
    }

    pub fn insert_transformed(
        &mut self,
        node_id: UzNodeId,
        local_bounds: Bounds,
        transform: Affine,
    ) -> HitboxId {
        let id = HitboxId(self.next_id);
        self.next_id += 1;
        let bounds = transformed_axis_aligned_bounds(local_bounds, transform);
        self.hitboxes.push(Hitbox {
            id,
            node_id,
            bounds,
            local_bounds,
            transform,
        });
        id
    }

    /// Get a hitbox by its ID.
    pub fn get(&self, id: HitboxId) -> Option<&Hitbox> {
        self.hitboxes.iter().find(|h| h.id == id)
    }

    /// Run a hit test at the given position. Walk hitboxes back-to-front
    /// (last registered = frontmost) and return all that contain the point.
    pub fn hit_test(&self, x: f64, y: f64) -> HitTestState {
        let mut hovered = Vec::new();
        let mut top_node = None;

        // Walk back-to-front: later entries are painted on top
        for hitbox in self.hitboxes.iter().rev() {
            if hitbox.contains(x, y) {
                if top_node.is_none() {
                    top_node = Some(hitbox.node_id);
                }
                if !hovered.contains(&hitbox.node_id) {
                    hovered.push(hitbox.node_id);
                }
            }
        }

        // Reverse so order is back-to-front (matching paint order)
        hovered.reverse();

        HitTestState {
            mouse_position: Some((x, y)),
            hovered_nodes: hovered,
            top_node,
            active_node: None, // Caller must preserve active state
        }
    }

    pub fn hitboxes(&self) -> &[Hitbox] {
        &self.hitboxes
    }
}

fn transformed_axis_aligned_bounds(bounds: Bounds, transform: Affine) -> Bounds {
    let points = [
        transform * Point::new(bounds.x, bounds.y),
        transform * Point::new(bounds.x + bounds.width, bounds.y),
        transform * Point::new(bounds.x + bounds.width, bounds.y + bounds.height),
        transform * Point::new(bounds.x, bounds.y + bounds.height),
    ];

    let (mut min_x, mut min_y) = (points[0].x, points[0].y);
    let (mut max_x, mut max_y) = (points[0].x, points[0].y);
    for point in points.iter().skip(1) {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }

    Bounds::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

#[derive(Clone, Debug)]
pub struct MouseEvent {
    pub position: (f64, f64),
    pub button: MouseButton,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

pub type MouseEventListener = Box<dyn Fn(&MouseEvent, &Bounds) + Send + Sync>;

/// Holds the style states and event listeners for an interactive element.
/// Elements embed this struct and delegate styling through it.
#[derive(Default)]
pub struct Interactivity {
    /// Base style refinement (always applied).
    pub base_style: UzStyleRefinement,
    /// Applied when the element's hitbox is hovered.
    pub hover_style: Option<Box<UzStyleRefinement>>,
    /// Applied when the element's hitbox is active (mouse pressed on it).
    pub active_style: Option<Box<UzStyleRefinement>>,

    /// The hitbox ID assigned to this element during paint. None if not interactive.
    pub hitbox_id: Option<HitboxId>,

    /// Mouse event listeners.
    pub mouse_down_listeners: Vec<MouseEventListener>,
    pub mouse_up_listeners: Vec<MouseEventListener>,
    pub click_listeners: Vec<MouseEventListener>,

    // todo remove
    /// Set from JS side when a node has JS event listeners.
    pub js_interactive: bool,
}

impl Interactivity {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if this element needs a hitbox (has hover/active styles or listeners).
    pub fn needs_hitbox(&self) -> bool {
        self.js_interactive
            || self.hover_style.is_some()
            || self.active_style.is_some()
            || !self.mouse_down_listeners.is_empty()
            || !self.mouse_up_listeners.is_empty()
            || !self.click_listeners.is_empty()
    }

    /// Compute the final Style for this element by starting with the base style
    /// and refining with hover/active styles based on the current hit test state.
    pub fn compute_style(
        &self,
        base: &UzStyle,
        node_id: UzNodeId,
        hit_state: &HitTestState,
    ) -> UzStyle {
        let mut style = base.clone();

        // Apply base style refinement
        style.refine(&self.base_style);

        // Hover/active state must be keyed by the stable DOM node identity, not the
        // paint-frame hitbox ID, otherwise a redraw between mouse down/up breaks clicks.
        if hit_state.is_hovered(node_id)
            && let Some(hover) = &self.hover_style
        {
            style.refine(hover);
        }

        if hit_state.is_active(node_id)
            && let Some(active) = &self.active_style
        {
            style.refine(active);
        }

        style
    }

    pub fn compute_style_inherited(
        &self,
        base: &UzStyle,
        parent: &UzStyle,
        node_id: UzNodeId,
        hit_state: &HitTestState,
    ) -> UzStyle {
        let mut style = base.clone();
        style.inherit_from(parent);
        style.refine(&self.base_style);

        if hit_state.is_hovered(node_id)
            && let Some(hover) = &self.hover_style
        {
            style.refine(hover);
        }

        if hit_state.is_active(node_id)
            && let Some(active) = &self.active_style
        {
            style.refine(active);
        }

        style
    }

    /// Set the hover style refinement.
    pub fn on_hover(&mut self, style: UzStyleRefinement) {
        self.hover_style = Some(Box::new(style));
    }

    /// Set the active (pressed) style refinement.
    pub fn on_active(&mut self, style: UzStyleRefinement) {
        self.active_style = Some(Box::new(style));
    }

    /// Add a click listener.
    pub fn on_click(&mut self, listener: impl Fn(&MouseEvent, &Bounds) + Send + Sync + 'static) {
        self.click_listeners.push(Box::new(listener));
    }

    /// Add a mouse down listener.
    pub fn on_mouse_down(
        &mut self,
        listener: impl Fn(&MouseEvent, &Bounds) + Send + Sync + 'static,
    ) {
        self.mouse_down_listeners.push(Box::new(listener));
    }

    /// Add a mouse up listener.
    pub fn on_mouse_up(&mut self, listener: impl Fn(&MouseEvent, &Bounds) + Send + Sync + 'static) {
        self.mouse_up_listeners.push(Box::new(listener));
    }
}
