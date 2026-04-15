use std::cell::Cell;
use std::rc::Rc;

use slab::Slab;
use unicode_segmentation::UnicodeSegmentation;

use crate::cursor::CursorIcon;
use crate::input::{BaseInputState, RangeProvider};
use crate::interactivity::{HitTestState, HitboxStore, Interactivity};
use crate::selection::{DomSelection, SelectionRange};
use crate::style::{Bounds, Style};
use crate::text::TextRenderer;

pub mod input;
pub mod render;
pub mod text;
pub mod view;

#[derive(Debug, Clone)]
pub struct SharedSelectionState {
    selection: Rc<Cell<Option<DomSelection>>>,
}

impl Default for SharedSelectionState {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedSelectionState {
    pub fn new() -> Self {
        Self {
            selection: Rc::new(Cell::new(None)),
        }
    }

    pub fn clear(&self) {
        self.selection.set(None);
    }

    pub fn range(&self) -> Option<SelectionRange> {
        self.selection.get().map(|s| s.range)
    }

    pub fn get(&self) -> Option<DomSelection> {
        self.selection.get()
    }

    pub fn is_empty(&self) -> bool {
        self.selection.get().is_none()
    }

    pub fn set(&self, selection: DomSelection) {
        self.selection.set(Some(selection));
    }
}

#[derive(Debug)]
pub struct DomRangeProvider {
    pub selection: SharedSelectionState,
}

impl RangeProvider for DomRangeProvider {
    fn get_range(&self) -> SelectionRange {
        self.selection.range().unwrap_or_default()
    }

    fn set_range(&mut self, range: SelectionRange) {
        if let Some(mut sel) = self.selection.get() {
            sel.range = range;
            self.selection.set(sel);
        }
    }
}

pub type InputState = BaseInputState<DomRangeProvider>;

pub type NodeId = usize;

pub struct ScrollState {
    pub scroll_offset_y: f32,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollState {
    pub fn new() -> Self {
        Self {
            scroll_offset_y: 0.0,
        }
    }
}

/// Active scroll-thumb drag. Stored on Dom (only one drag at a time).
pub struct ScrollDragState {
    pub node_id: NodeId,
    pub start_mouse_y: f64,
    pub start_scroll_offset: f32,
    /// Track length = visible_height - thumb_height (how far thumb can move).
    pub track_range: f64,
    /// Max scroll offset (content_height - visible_height).
    pub max_scroll: f32,
}

/// Rendered thumb rect, rebuilt each paint pass for hit testing.
pub struct ScrollThumbRect {
    pub node_id: NodeId,
    pub thumb_bounds: Bounds,
    pub view_bounds: Bounds,
    pub content_height: f32,
    pub visible_height: f32,
}

#[derive(Clone, Debug)]
pub struct TextContent {
    pub content: String,
}

// ── Inherited properties ─────────────────────────────────────────────
// General-purpose mechanism for properties that propagate from parent to child
// unless explicitly overridden. Designed for extension — future inheritable
// properties (font color, font size, line height, etc.) go here.

#[derive(Clone, Debug, Default)]
pub struct InheritedProperties {
    pub selectable: bool,
}

// ── View text selection ──────────────────────────────────────────────

/// One text node's contribution to a textSelect run.
pub struct TextRunEntry {
    pub node_id: NodeId,
    /// Start grapheme index of this node in the flat run.
    pub flat_start: usize,
    pub grapheme_count: usize,
}

/// The complete text run for a textSelect subtree.
/// Built each frame; maps between flat grapheme offsets and per-node positions.
pub struct TextSelectRun {
    pub root_id: NodeId,
    pub entries: Vec<TextRunEntry>,
    pub flat_text: String,
    pub total_graphemes: usize,
}

// ── Element trait ──────────────────────────────────────────────────────

pub trait ElementBehavior {
    fn as_input(&self) -> Option<&InputState> {
        None
    }
    fn as_input_mut(&mut self) -> Option<&mut InputState> {
        None
    }
    fn as_text(&self) -> Option<&TextContent> {
        None
    }
    fn as_text_mut(&mut self) -> Option<&mut TextContent> {
        None
    }
    fn is_input(&self) -> bool {
        false
    }
    fn is_text(&self) -> bool {
        false
    }

    /// Default cursor for this behavior when unset by style.
    fn default_cursor(&self) -> Option<CursorIcon> {
        None
    }
}

pub struct ViewBehavior;
impl ElementBehavior for ViewBehavior {}

pub struct TextBehavior {
    pub content: TextContent,
}

impl ElementBehavior for TextBehavior {
    fn as_text(&self) -> Option<&TextContent> {
        Some(&self.content)
    }
    fn as_text_mut(&mut self) -> Option<&mut TextContent> {
        Some(&mut self.content)
    }
    fn is_text(&self) -> bool {
        true
    }
}

pub struct InputBehavior {
    pub state: InputState,
}

impl InputBehavior {
    pub fn new(state: InputState) -> Self {
        Self { state }
    }

    pub fn new_single_line(mut state: InputState) -> Self {
        state.multiline = false;
        Self::new(state)
    }
}

impl ElementBehavior for InputBehavior {
    fn as_input(&self) -> Option<&InputState> {
        Some(&self.state)
    }
    fn as_input_mut(&mut self) -> Option<&mut InputState> {
        Some(&mut self.state)
    }
    fn is_input(&self) -> bool {
        true
    }
    fn default_cursor(&self) -> Option<CursorIcon> {
        if self.state.disabled {
            Some(CursorIcon::NotAllowed)
        } else {
            Some(CursorIcon::Text)
        }
    }
}

#[derive(Clone, Debug)]
pub struct NodeContext {
    pub dom_id: NodeId,
    pub text: Option<TextContent>,
    pub font_size: f32,
    pub is_input: bool,
}

pub struct Node {
    pub parent: Option<NodeId>,
    pub first_child: Option<NodeId>,
    pub last_child: Option<NodeId>,
    pub next_sibling: Option<NodeId>,
    pub prev_sibling: Option<NodeId>,
    pub taffy_node: taffy::NodeId,
    pub behavior: Box<dyn ElementBehavior>,
    /// The base style for this element. Converted to taffy for layout.
    pub style: Style,
    /// Interactivity: hover/active style overrides, hitbox, event listeners.
    pub interactivity: Interactivity,
    /// Scroll state, present only when overflow_y == Scroll.
    pub scroll_state: Option<ScrollState>,
    /// Whether text within this element is selectable.
    /// None = inherit from parent (default). Some(true) = selectable, Some(false) = not.
    pub selectable: Option<bool>,
}

pub struct ElementTree {
    pub nodes: Slab<Node>,
    pub taffy: taffy::TaffyTree<NodeContext>,
    pub root: Option<NodeId>,
    /// Hitboxes registered during the last paint pass.
    pub hitbox_store: HitboxStore,
    /// Current hit test state (updated on mouse move).
    pub hit_state: HitTestState,
    /// Currently focuswsed ndoe
    pub focused_node: Option<NodeId>,
    // oh god please move this to input state
    /// Input node being dragged for selection.
    pub dragging_input: Option<NodeId>,
    /// Last click time (for multi-click detection).
    pub last_click_time: Option<std::time::Instant>,
    /// Last clicked node (for multi-click detection).
    pub last_click_node: Option<NodeId>,
    /// Consecutive click count (1=normal, 2=word, 3=line, 4=select all).
    pub click_count: u8,
    /// Whether the OS window is focused.
    pub window_focused: bool,
    /// Scroll thumb rects from last paint pass (for hit testing).
    pub scroll_thumbs: Vec<ScrollThumbRect>,
    /// Active scroll-thumb drag state (only one at a time).
    pub scroll_drag: Option<ScrollDragState>,
    /// Scroll lock: when scrolling starts, lock to that node for a short duration
    /// to prevent inner scrollable views from stealing wheel events mid-scroll.
    pub scroll_lock: Option<(NodeId, std::time::Instant)>,
    /// Current text selection within a textSelect view.
    pub selection: SharedSelectionState,
    /// textSelect root being dragged for selection.
    pub dragging_view_selection: Option<NodeId>,
    /// Text runs for textSelect subtrees, rebuilt each frame.
    pub selectable_text_runs: Vec<TextSelectRun>,
}

// Safety:  We only access it from main thread
unsafe impl Send for ElementTree {}
unsafe impl Sync for ElementTree {}

impl Default for ElementTree {
    fn default() -> Self {
        Self::new()
    }
}

impl ElementTree {
    pub fn new() -> Self {
        Self {
            nodes: Slab::new(),
            taffy: taffy::TaffyTree::new(),
            root: None,
            hitbox_store: HitboxStore::default(),
            hit_state: HitTestState::default(),
            focused_node: None,
            dragging_input: None,
            last_click_time: None,
            last_click_node: None,
            click_count: 0,
            window_focused: true,
            scroll_thumbs: Vec::new(),
            scroll_drag: None,
            scroll_lock: None,
            selection: SharedSelectionState::new(),
            dragging_view_selection: None,
            selectable_text_runs: Vec::new(),
        }
    }

    pub fn has_focused_node(&self) -> bool {
        self.focused_node.is_some()
    }

    pub(crate) fn with_focused_node<R>(
        &mut self,
        update: impl FnOnce(&mut Node, NodeId) -> R,
    ) -> Option<R> {
        let focus = self.focused_node;
        focus.and_then(|id| self.nodes.get_mut(id).map(|node| update(node, id)))
    }

    pub fn get_node(&self, node_id: NodeId) -> Option<&Node> {
        self.nodes.get(node_id)
    }

    pub fn get_node_mut(&mut self, node_id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(node_id)
    }

    /// Resolve the effective cursor for `node_id`.
    /// Precedence at the hit node: explicit style -> behavior default -> selectable
    /// text fallback. Otherwise walk ancestors honoring only explicit overrides.
    pub fn resolve_cursor(&self, node_id: NodeId) -> CursorIcon {
        let Some(node) = self.nodes.get(node_id) else {
            return CursorIcon::Default;
        };
        if let Some(c) = node.style.cursor {
            return c;
        }
        if let Some(c) = node.behavior.default_cursor() {
            return c;
        }
        if node.selectable == Some(true) {
            return CursorIcon::Text;
        }
        let mut cur = node.parent;
        while let Some(id) = cur {
            let n = &self.nodes[id];
            if let Some(c) = n.style.cursor {
                return c;
            }
            cur = n.parent;
        }
        CursorIcon::Default
    }

    /// Create a View element with a style.
    pub fn create_view(&mut self, style: Style) -> NodeId {
        let taffy_style = style.to_taffy();
        let taffy_node = self.taffy.new_leaf(taffy_style).unwrap();
        let node_id = self.nodes.insert(Node {
            parent: None,
            first_child: None,
            last_child: None,
            next_sibling: None,
            prev_sibling: None,
            taffy_node,
            behavior: Box::new(ViewBehavior),
            style,
            interactivity: Interactivity::new(),
            scroll_state: None,
            selectable: None,
        });
        self.taffy
            .set_node_context(
                taffy_node,
                Some(NodeContext {
                    dom_id: node_id,
                    text: None,
                    font_size: 16.0,
                    is_input: false,
                }),
            )
            .unwrap();
        node_id
    }

    /// Create a Text element.
    pub fn create_text(&mut self, content: String, style: Style) -> NodeId {
        let taffy_style = style.to_taffy();
        let taffy_node = self.taffy.new_leaf(taffy_style).unwrap();
        let text = TextContent {
            content: content.clone(),
        };
        let font_size = style.text.font_size;
        let node_id = self.nodes.insert(Node {
            parent: None,
            first_child: None,
            last_child: None,
            next_sibling: None,
            prev_sibling: None,
            taffy_node,
            behavior: Box::new(TextBehavior {
                content: text.clone(),
            }),
            style,
            interactivity: Interactivity::new(),
            scroll_state: None,
            selectable: None,
        });
        self.taffy
            .set_node_context(
                taffy_node,
                Some(NodeContext {
                    dom_id: node_id,
                    text: Some(text),
                    font_size,
                    is_input: false,
                }),
            )
            .unwrap();
        node_id
    }

    /// Create an Input element.
    pub fn create_input(&mut self, style: Style) -> NodeId {
        let taffy_style = style.to_taffy();
        let taffy_node = self.taffy.new_leaf(taffy_style).unwrap();
        let font_size = style.text.font_size;
        let node_id = self.nodes.insert(Node {
            parent: None,
            first_child: None,
            last_child: None,
            next_sibling: None,
            prev_sibling: None,
            taffy_node,
            behavior: Box::new(InputBehavior::new_single_line(InputState::new(
                DomRangeProvider {
                    selection: self.selection.clone(),
                },
            ))),
            style,
            interactivity: Interactivity::new(),
            scroll_state: None,
            selectable: None,
        });
        self.taffy
            .set_node_context(
                taffy_node,
                Some(NodeContext {
                    dom_id: node_id,
                    text: None,
                    font_size,
                    is_input: true,
                }),
            )
            .unwrap();
        // Input always needs a hitbox for click-to-focus
        self.nodes[node_id].interactivity.js_interactive = true;
        node_id
    }

    /// Update a node's style (also syncs taffy).
    pub fn set_style(&mut self, node_id: NodeId, style: Style) {
        let node = &mut self.nodes[node_id];
        let taffy_style = style.to_taffy();
        node.style = style;
        self.taffy.set_style(node.taffy_node, taffy_style).unwrap();
    }

    pub fn set_root(&mut self, node_id: NodeId) {
        self.root = Some(node_id);
    }

    pub fn append_child(&mut self, parent_id: NodeId, child_id: NodeId) {
        let parent_taffy = self.nodes[parent_id].taffy_node;
        let child_taffy = self.nodes[child_id].taffy_node;
        self.taffy.add_child(parent_taffy, child_taffy).unwrap();

        let old_last = self.nodes[parent_id].last_child;
        self.nodes[child_id].parent = Some(parent_id);
        self.nodes[child_id].prev_sibling = old_last;
        self.nodes[child_id].next_sibling = None;

        if let Some(old_last_id) = old_last {
            self.nodes[old_last_id].next_sibling = Some(child_id);
        } else {
            self.nodes[parent_id].first_child = Some(child_id);
        }
        self.nodes[parent_id].last_child = Some(child_id);
    }

    pub fn insert_before(&mut self, parent_id: NodeId, child_id: NodeId, before_id: NodeId) {
        let parent_taffy = self.nodes[parent_id].taffy_node;
        let child_taffy = self.nodes[child_id].taffy_node;
        let before_taffy = self.nodes[before_id].taffy_node;

        let children = self.taffy.children(parent_taffy).unwrap();
        let idx = children
            .iter()
            .position(|&c| c == before_taffy)
            .expect("before node not found in parent");
        self.taffy
            .insert_child_at_index(parent_taffy, idx, child_taffy)
            .unwrap();

        let prev = self.nodes[before_id].prev_sibling;
        self.nodes[child_id].parent = Some(parent_id);
        self.nodes[child_id].next_sibling = Some(before_id);
        self.nodes[child_id].prev_sibling = prev;
        self.nodes[before_id].prev_sibling = Some(child_id);

        if let Some(prev_id) = prev {
            self.nodes[prev_id].next_sibling = Some(child_id);
        } else {
            self.nodes[parent_id].first_child = Some(child_id);
        }
    }

    /// Single source of truth for clearing stale NodeId references when a node
    /// is about to be freed. With plain `usize` NodeIds (slab), any long-lived
    /// field holding a removed id would silently retarget to whatever node
    /// reuses the slot. Every removal path MUST funnel through here.
    ///
    /// When adding a new long-lived `NodeId` field to `Dom`, register it here.
    fn on_node_removed(&mut self, id: NodeId) {
        if self.focused_node == Some(id) {
            self.focused_node = None;
        }
        if self.dragging_input == Some(id) {
            self.dragging_input = None;
        }
        if self.dragging_view_selection == Some(id) {
            self.dragging_view_selection = None;
        }
        if self.last_click_node == Some(id) {
            self.last_click_node = None;
            self.click_count = 0;
            self.last_click_time = None;
        }
        if let Some(d) = &self.scroll_drag
            && d.node_id == id
        {
            self.scroll_drag = None;
        }
        if let Some((nid, _)) = self.scroll_lock
            && nid == id
        {
            self.scroll_lock = None;
        }
        if let Some(sel) = self.selection.get()
            && sel.root == id
        {
            self.selection.clear();
        }

        self.hit_state.hovered_nodes.retain(|&n| n != id);
        if self.hit_state.top_node == Some(id) {
            self.hit_state.top_node = None;
        }
        if self.hit_state.active_node == Some(id) {
            self.hit_state.active_node = None;
        }

        self.scroll_thumbs.retain(|t| t.node_id != id);
        self.hitbox_store.retain_by_node(|n| n != id);

        // Selectable text runs reference nodes as both roots and entries.
        self.selectable_text_runs
            .retain(|r| r.root_id != id && !r.entries.iter().any(|e| e.node_id == id));
    }

    pub fn remove_child(&mut self, parent_id: NodeId, child_id: NodeId) {
        let parent_taffy = self.nodes[parent_id].taffy_node;
        let child_taffy = self.nodes[child_id].taffy_node;
        self.taffy.remove_child(parent_taffy, child_taffy).unwrap();

        let prev = self.nodes[child_id].prev_sibling;
        let next = self.nodes[child_id].next_sibling;

        if let Some(prev_id) = prev {
            self.nodes[prev_id].next_sibling = next;
        } else {
            self.nodes[parent_id].first_child = next;
        }

        if let Some(next_id) = next {
            self.nodes[next_id].prev_sibling = prev;
        } else {
            self.nodes[parent_id].last_child = prev;
        }

        // Collect the entire subtree rooted at child_id (BFS)
        let mut to_remove = Vec::new();
        let mut stack = vec![child_id];
        while let Some(nid) = stack.pop() {
            to_remove.push(nid);
            let mut c = self.nodes[nid].first_child;
            while let Some(cid) = c {
                stack.push(cid);
                c = self.nodes[cid].next_sibling;
            }
        }

        // remove taffy and slab nodes
        for nid in to_remove {
            let tn = self.nodes[nid].taffy_node;
            let _ = self.taffy.remove(tn);
            self.on_node_removed(nid);
            self.nodes.remove(nid);
        }
    }

    /// Update a text node's content.
    pub fn set_text_content(&mut self, node_id: NodeId, text: String) {
        let node = &mut self.nodes[node_id];
        let tc = TextContent { content: text };
        if let Some(existing) = node.behavior.as_text_mut() {
            existing.content = tc.content.clone();
        } else {
            node.behavior = Box::new(TextBehavior {
                content: tc.clone(),
            });
        }
        let taffy_node = node.taffy_node;
        let font_size = node.style.text.font_size;
        self.taffy
            .set_node_context(
                taffy_node,
                Some(NodeContext {
                    dom_id: node_id,
                    text: Some(tc),
                    font_size,
                    is_input: false,
                }),
            )
            .unwrap();
    }

    /// Remove all children (and their descendants) from `parent_id`, clearing
    /// the taffy tree and slotmap entries.  The parent node itself is kept.
    pub fn clear_children(&mut self, parent_id: NodeId) {
        // Collect every descendant via BFS
        let mut to_remove = Vec::new();
        let mut stack = Vec::new();

        let mut child = self.nodes[parent_id].first_child;
        while let Some(cid) = child {
            stack.push(cid);
            child = self.nodes[cid].next_sibling;
        }
        while let Some(nid) = stack.pop() {
            to_remove.push(nid);
            let mut child = self.nodes[nid].first_child;
            while let Some(cid) = child {
                stack.push(cid);
                child = self.nodes[cid].next_sibling;
            }
        }

        // Detach all taffy children from parent
        let parent_taffy = self.nodes[parent_id].taffy_node;
        let taffy_children: Vec<_> = self.taffy.children(parent_taffy).unwrap();
        for tc in taffy_children {
            let _ = self.taffy.remove_child(parent_taffy, tc);
        }

        // Remove descendants from taffy + slab; scrub stale NodeId references first.
        for nid in to_remove {
            let tn = self.nodes[nid].taffy_node;
            let _ = self.taffy.remove(tn);
            self.on_node_removed(nid);
            self.nodes.remove(nid);
        }

        // Reset parent pointers
        self.nodes[parent_id].first_child = None;
        self.nodes[parent_id].last_child = None;
    }

    pub fn compute_layout(&mut self, width: f32, height: f32, text_renderer: &mut TextRenderer) {
        if let Some(root) = self.root {
            let taffy_root = self.nodes[root].taffy_node;
            self.taffy
                .compute_layout_with_measure(
                    taffy_root,
                    taffy::Size {
                        width: taffy::AvailableSpace::Definite(width),
                        height: taffy::AvailableSpace::Definite(height),
                    },
                    |known_dimensions, available_space, _node_id, node_context, _style| {
                        render::measure(
                            text_renderer,
                            known_dimensions,
                            available_space,
                            node_context,
                        )
                    },
                )
                .unwrap();
        }
    }

    /// Run hit test at the given mouse position and update hit_state.
    pub fn update_hit_test(&mut self, x: f64, y: f64) {
        let active = self.hit_state.active_node;
        self.hit_state = self.hitbox_store.hit_test(x, y);
        self.hit_state.active_node = active;
    }

    /// Refresh hit-testing using the current pointer position after layout or
    /// paint invalidates the previous frame's hitboxes.
    pub fn refresh_hit_test(&mut self) -> bool {
        let Some((x, y)) = self.hit_state.mouse_position else {
            return false;
        };

        let old_top = self.hit_state.top_node;
        let old_hovered = self.hit_state.hovered_nodes.clone();
        self.update_hit_test(x, y);

        old_top != self.hit_state.top_node || old_hovered != self.hit_state.hovered_nodes
    }

    /// Set the active node (mouse down on an element).
    pub fn set_active(&mut self, node_id: Option<NodeId>) {
        self.hit_state.active_node = node_id;
    }

    /// Dispatch mouse down event to listeners on hovered elements.
    pub fn dispatch_mouse_down(&self, x: f64, y: f64, button: crate::interactivity::MouseButton) {
        let event = crate::interactivity::MouseEvent {
            position: (x, y),
            button,
        };

        for hitbox in self.hitbox_store.hitboxes().iter().rev() {
            if hitbox.bounds.contains(x, y) {
                let node = &self.nodes[hitbox.node_id];
                for listener in &node.interactivity.mouse_down_listeners {
                    listener(&event, &hitbox.bounds);
                }
            }
        }
    }

    /// Dispatch mouse up event.
    pub fn dispatch_mouse_up(&self, x: f64, y: f64, button: crate::interactivity::MouseButton) {
        let event = crate::interactivity::MouseEvent {
            position: (x, y),
            button,
        };

        for hitbox in self.hitbox_store.hitboxes().iter().rev() {
            if hitbox.bounds.contains(x, y) {
                let node = &self.nodes[hitbox.node_id];
                for listener in &node.interactivity.mouse_up_listeners {
                    listener(&event, &hitbox.bounds);
                }
            }
        }
    }

    /// Dispatch click event.
    pub fn dispatch_click(&self, x: f64, y: f64, button: crate::interactivity::MouseButton) {
        let event = crate::interactivity::MouseEvent {
            position: (x, y),
            button,
        };

        for hitbox in self.hitbox_store.hitboxes().iter().rev() {
            if hitbox.bounds.contains(x, y) {
                let node = &self.nodes[hitbox.node_id];
                for listener in &node.interactivity.click_listeners {
                    listener(&event, &hitbox.bounds);
                }
            }
        }
    }

    // ── Text selection ──────────────────────────────────────────────

    /// Build text runs for all textSelect subtrees. Called each frame before render.
    pub fn build_text_select_runs(&mut self) {
        self.selectable_text_runs.clear();
        let Some(root) = self.root else { return };

        // DFS: (node_id, parent_resolved_text_select, current_run_index_or_none)
        let mut stack: Vec<(NodeId, bool, Option<usize>)> = vec![(root, false, None)];

        while let Some((node_id, parent_ts, run_idx)) = stack.pop() {
            let node = &self.nodes[node_id];
            let resolved_ts = node.selectable.unwrap_or(parent_ts);

            // A node that explicitly enables textSelect when the parent scope
            // doesn't have it starts a new selection scope.
            let current_run = if resolved_ts && run_idx.is_none() {
                let idx = self.selectable_text_runs.len();
                self.selectable_text_runs.push(TextSelectRun {
                    root_id: node_id,
                    entries: Vec::new(),
                    flat_text: String::new(),
                    total_graphemes: 0,
                });
                Some(idx)
            } else if resolved_ts {
                run_idx
            } else {
                None
            };

            // Add text nodes to the current run
            if let Some(tc) = node.behavior.as_text()
                && let Some(idx) = current_run
            {
                let gc = tc.content.graphemes(true).count();
                let run = &mut self.selectable_text_runs[idx];
                run.entries.push(TextRunEntry {
                    node_id,
                    flat_start: run.total_graphemes,
                    grapheme_count: gc,
                });
                run.flat_text.push_str(&tc.content);
                run.total_graphemes += gc;
            }

            // Push children in reverse order for correct DFS traversal
            let mut children = Vec::new();
            let mut child = node.first_child;
            while let Some(cid) = child {
                children.push(cid);
                child = self.nodes[cid].next_sibling;
            }
            for &cid in children.iter().rev() {
                stack.push((cid, resolved_ts, current_run));
            }
        }
    }

    /// Find the text run that contains a given text node.
    pub fn find_run_for_node(&self, node_id: NodeId) -> Option<&TextSelectRun> {
        self.selectable_text_runs
            .iter()
            .find(|run| run.entries.iter().any(|e| e.node_id == node_id))
    }

    /// Find the text run entry for a given text node.
    pub fn find_run_entry_for_node(
        &self,
        node_id: NodeId,
    ) -> Option<(&TextSelectRun, &TextRunEntry)> {
        for run in &self.selectable_text_runs {
            for entry in &run.entries {
                if entry.node_id == node_id {
                    return Some((run, entry));
                }
            }
        }
        None
    }

    /// Check whether a node is a text node inside an active textSelect scope.
    pub fn is_text_selectable(&self, node_id: NodeId) -> bool {
        self.selectable_text_runs
            .iter()
            .any(|run| run.entries.iter().any(|e| e.node_id == node_id))
    }

    // ── Selection query API ─────────────────────────────────────────
    // Designed for clipboard and text editor consumers.

    /// Get the currently selected text content (input or view).
    pub fn selected_text(&self) -> String {
        let Some(sel) = self.selection.get() else {
            return String::new();
        };

        if sel.is_collapsed() {
            return String::new();
        }
        // Input selection: delegate to InputState
        if let Some(node) = self.nodes.get(sel.root)
            && let Some(is) = node.behavior.as_input()
        {
            return is.selected_text();
        }
        // View text selection: look up in text_select_runs
        let Some(run) = self
            .selectable_text_runs
            .iter()
            .find(|r| r.root_id == sel.root)
        else {
            return String::new();
        };
        let start = sel.start();
        let end = sel.end();
        run.flat_text
            .graphemes(true)
            .skip(start)
            .take(end - start)
            .collect::<String>()
    }

    /// Get the current selection range as flat grapheme offsets.
    /// Returns (start, end) where start <= end.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        let sel = self.selection.get()?;
        if sel.is_collapsed() {
            return None;
        }
        Some((sel.start(), sel.end()))
    }

    /// Get the full selection state: root node, anchor, and active offsets.
    /// Useful for text editors that need to know the direction of selection.
    pub fn selection_state(&self) -> Option<(NodeId, usize, usize)> {
        let sel = self.selection.get()?;
        Some((sel.root, sel.anchor(), sel.active()))
    }

    /// Get the total grapheme count in the text run containing the current selection.
    /// For input selections, returns the input's grapheme count.
    pub fn selection_run_length(&self) -> Option<usize> {
        let sel = self.selection.get()?;
        // Input selection
        if let Some(node) = self.nodes.get(sel.root)
            && let Some(is) = node.behavior.as_input()
        {
            return Some(is.grapheme_count());
        }
        // View text selection
        let run = self
            .selectable_text_runs
            .iter()
            .find(|r| r.root_id == sel.root)?;
        Some(run.total_graphemes)
    }

    pub fn selection(&self) -> Option<DomSelection> {
        self.selection.get()
    }

    pub fn set_selection(&mut self, selection: DomSelection) {
        let root = selection.root;

        // If the target node is focusable (input, future: content-editable),
        // handle focus transfer automatically.
        let is_focusable = self
            .nodes
            .get(root)
            .map(|n| n.behavior.is_input())
            .unwrap_or(false);

        if is_focusable {
            if let Some(old_id) = self.focused_node
                && old_id != root
                && let Some(old_node) = self.nodes.get_mut(old_id)
                && let Some(is) = old_node.behavior.as_input_mut()
            {
                is.focused = false;
            }
            self.focused_node = Some(root);
            if let Some(node) = self.nodes.get_mut(root)
                && let Some(is) = node.behavior.as_input_mut()
            {
                is.focused = true;
                is.reset_blink();
            }
        }

        self.selection.set(selection);
    }

    /// Clear the selection.
    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::ElementTree;
    use crate::style::Bounds;

    #[test]
    fn refresh_hit_test_retargets_stationary_pointer_after_hitboxes_change() {
        let mut dom = ElementTree::new();
        let first = dom.create_view(Default::default());
        let second = dom.create_view(Default::default());

        dom.hitbox_store.insert(
            first,
            Bounds {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
        );
        dom.update_hit_test(10.0, 10.0);
        dom.set_active(Some(first));

        dom.hitbox_store.clear();
        dom.hitbox_store.insert(
            second,
            Bounds {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
        );

        assert!(dom.refresh_hit_test());
        assert_eq!(dom.hit_state.top_node, Some(second));
        assert_eq!(dom.hit_state.active_node, Some(first));
    }
}
