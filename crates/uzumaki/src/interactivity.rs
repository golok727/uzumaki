use refineable::Refineable;

use crate::element::NodeId;
use crate::style::{Bounds, Style, StyleRefinement};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HitboxId(pub u64);

#[derive(Clone, Debug)]
pub struct Hitbox {
    pub id: HitboxId,
    pub node_id: NodeId,
    pub bounds: Bounds,
}

impl Hitbox {
    /// Check if this hitbox is hovered according to the current hit test result.
    pub fn is_hovered(&self, hit_state: &HitTestState) -> bool {
        hit_state.hovered_hitboxes.contains(&self.id)
    }
}

/// Stores the result of a hit test: which hitboxes the mouse is currently over.
#[derive(Clone, Debug, Default)]
pub struct HitTestState {
    /// Mouse position in window coordinates.
    pub mouse_position: Option<(f64, f64)>,
    /// Set of hitbox IDs that the mouse is currently over (back-to-front order).
    pub hovered_hitboxes: Vec<HitboxId>,
    /// The topmost (frontmost) hovered hitbox, if any.
    pub top_hit: Option<HitboxId>,
    /// Which hitbox is currently pressed (mouse down without mouse up).
    pub active_hitbox: Option<HitboxId>,
}

impl HitTestState {
    pub fn is_hovered(&self, id: HitboxId) -> bool {
        self.hovered_hitboxes.contains(&id)
    }

    pub fn is_active(&self, id: HitboxId) -> bool {
        self.active_hitbox == Some(id) && self.is_hovered(id)
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

    /// Register a hitbox and return its ID.
    pub fn insert(&mut self, node_id: NodeId, bounds: Bounds) -> HitboxId {
        let id = HitboxId(self.next_id);
        self.next_id += 1;
        self.hitboxes.push(Hitbox {
            id,
            node_id,
            bounds,
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
        let mut top_hit = None;

        // Walk back-to-front: later entries are painted on top
        for hitbox in self.hitboxes.iter().rev() {
            if hitbox.bounds.contains(x, y) {
                if top_hit.is_none() {
                    top_hit = Some(hitbox.id);
                }
                hovered.push(hitbox.id);
            }
        }

        // Reverse so order is back-to-front (matching paint order)
        hovered.reverse();

        HitTestState {
            mouse_position: Some((x, y)),
            hovered_hitboxes: hovered,
            top_hit,
            active_hitbox: None, // Caller must preserve active state
        }
    }

    pub fn hitboxes(&self) -> &[Hitbox] {
        &self.hitboxes
    }
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
pub struct Interactivity {
    /// Base style refinement (always applied).
    pub base_style: StyleRefinement,
    /// Applied when the element's hitbox is hovered.
    pub hover_style: Option<Box<StyleRefinement>>,
    /// Applied when the element's hitbox is active (mouse pressed on it).
    pub active_style: Option<Box<StyleRefinement>>,

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

impl Default for Interactivity {
    fn default() -> Self {
        Self {
            base_style: StyleRefinement::default(),
            hover_style: None,
            active_style: None,
            hitbox_id: None,
            mouse_down_listeners: Vec::new(),
            mouse_up_listeners: Vec::new(),
            click_listeners: Vec::new(),
            js_interactive: false,
        }
    }
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
    pub fn compute_style(&self, base: &Style, hit_state: &HitTestState) -> Style {
        let mut style = base.clone();

        // Apply base style refinement
        style.refine(&self.base_style);

        // Apply hover style if hovered
        if let Some(hitbox_id) = self.hitbox_id {
            if hit_state.is_hovered(hitbox_id) {
                if let Some(hover) = &self.hover_style {
                    style.refine(hover);
                }
            }

            // Apply active style if active (hovered + mouse pressed)
            if hit_state.is_active(hitbox_id) {
                if let Some(active) = &self.active_style {
                    style.refine(active);
                }
            }
        }

        style
    }

    /// Set the hover style refinement.
    pub fn on_hover(&mut self, style: StyleRefinement) {
        self.hover_style = Some(Box::new(style));
    }

    /// Set the active (pressed) style refinement.
    pub fn on_active(&mut self, style: StyleRefinement) {
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
