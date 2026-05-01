use slab::Slab;

use crate::{
    cursor::UzCursorIcon,
    element::{
        ElementData, ElementNode, ImageData, ImageMeasureInfo, ImageNode, Node, NodeContext,
        ScrollDragState, ScrollThumbRect, TextNode, TextRunEntry, TextSelectRun, UzNodeId, render,
    },
    input::InputState,
    interactivity::{HitTestState, HitboxStore},
    selection::TextSelection,
    style::{Length, TextStyle, UzStyle},
    text::TextRenderer,
};

pub struct UIState {
    pub nodes: Slab<Node>,

    pub taffy: taffy::TaffyTree<NodeContext>,
    pub root: Option<UzNodeId>,
    /// Hitboxes registered during the last paint pass.
    pub hitbox_store: HitboxStore,
    /// Current hit test state (updated on mouse move).
    pub hit_state: HitTestState,
    /// Currently focuswsed ndoe
    pub focused_node: Option<UzNodeId>,
    /// Input node being dragged for selection.
    pub dragging_input: Option<UzNodeId>,
    /// Last click time (for multi-click detection).
    pub last_click_time: Option<std::time::Instant>,
    /// Last clicked node (for multi-click detection).
    pub last_click_node: Option<UzNodeId>,
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
    pub scroll_lock: Option<(UzNodeId, std::time::Instant)>,
    /// Current text selection within a textSelect view. `root == None` means
    /// there is no active view selection
    pub text_selection: TextSelection,
    /// textSelect root being dragged for selection.
    pub dragging_view_selection: Option<UzNodeId>,
    /// Text runs for textSelect subtrees, rebuilt each frame.
    pub selectable_text_runs: Vec<TextSelectRun>,
}

// Safety:  We only access it from main thread
unsafe impl Send for UIState {}
unsafe impl Sync for UIState {}

impl Default for UIState {
    fn default() -> Self {
        Self::new()
    }
}

impl UIState {
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
            text_selection: TextSelection::default(),
            dragging_view_selection: None,
            selectable_text_runs: Vec::new(),
        }
    }

    pub fn has_focused_node(&self) -> bool {
        self.focused_node.is_some()
    }

    pub(crate) fn with_focused_node<R>(
        &mut self,
        update: impl FnOnce(&mut Node, UzNodeId) -> R,
    ) -> Option<R> {
        let focus = self.focused_node;
        focus.and_then(|id| self.nodes.get_mut(id).map(|node| update(node, id)))
    }

    pub fn get_node(&self, node_id: UzNodeId) -> Option<&Node> {
        self.nodes.get(node_id)
    }

    pub fn get_node_mut(&mut self, node_id: UzNodeId) -> Option<&mut Node> {
        self.nodes.get_mut(node_id)
    }

    /// Resolve the effective cursor for `node_id`.
    /// Precedence at the hit node: explicit style -> behavior default -> selectable
    /// text fallback. Otherwise walk ancestors honoring only explicit overrides.
    pub fn resolve_cursor(&self, node_id: UzNodeId) -> UzCursorIcon {
        let Some(node) = self.nodes.get(node_id) else {
            return UzCursorIcon::Default;
        };

        let style = node.interactivity.compute_style(
            &node.style,
            node_id,
            &self.hit_state,
            self.focused_node == Some(node_id),
        );
        if let Some(c) = style.cursor {
            return c;
        }

        if let Some(c) = node.default_cursor() {
            return c;
        }

        if node.is_text_selectable() {
            return UzCursorIcon::Text;
        }

        let mut cur = node.parent;
        while let Some(id) = cur {
            let n = &self.nodes[id];
            let style = n.interactivity.compute_style(
                &n.style,
                id,
                &self.hit_state,
                self.focused_node == Some(id),
            );
            if let Some(c) = style.cursor {
                return c;
            }
            cur = n.parent;
        }
        UzCursorIcon::Default
    }

    /// Create a View element with a style.
    pub fn create_view(&mut self, style: UzStyle) -> UzNodeId {
        let taffy_style = style.to_taffy();
        let taffy_node = self.taffy.new_leaf(taffy_style).unwrap();
        let node_id = self.nodes.insert(Node::new(
            taffy_node,
            style,
            ElementNode::new(ElementData::None),
        ));

        self.taffy
            .set_node_context(
                taffy_node,
                Some(NodeContext {
                    dom_id: node_id,
                    text: None,
                    text_style: TextStyle::default(),
                    is_input: false,
                    image: None,
                }),
            )
            .unwrap();
        node_id
    }

    /// Create a Text element.
    pub fn create_text(&mut self, content: String, style: UzStyle) -> UzNodeId {
        let taffy_style = style.to_taffy();
        let taffy_node = self.taffy.new_leaf(taffy_style).unwrap();
        let text = TextNode {
            content: content.clone(),
        };
        let text_style = style.text.clone();
        let node_id = self
            .nodes
            .insert(Node::new(taffy_node, style, TextNode::new(content)));

        self.taffy
            .set_node_context(
                taffy_node,
                Some(NodeContext {
                    dom_id: node_id,
                    text: Some(text),
                    text_style,
                    is_input: false,
                    image: None,
                }),
            )
            .unwrap();
        node_id
    }

    /// Create an Input element.
    pub fn create_input(&mut self, style: UzStyle) -> UzNodeId {
        let taffy_style = style.to_taffy();
        let taffy_node = self.taffy.new_leaf(taffy_style).unwrap();
        let text_style = style.text.clone();
        let is = InputState::new_single_line();

        let node_id = self.nodes.insert(Node::new(
            taffy_node,
            style,
            ElementNode::new_text_input(is),
        ));

        self.taffy
            .set_node_context(
                taffy_node,
                Some(NodeContext {
                    dom_id: node_id,
                    text: None,
                    text_style,
                    is_input: true,
                    image: None,
                }),
            )
            .unwrap();
        // Input always needs a hitbox for click-to-focus
        self.nodes[node_id].interactivity.js_interactive = true;
        self.nodes[node_id]
            .as_element_mut()
            .expect("input should be an element")
            .set_focussable(true);
        node_id
    }

    pub fn create_image(&mut self, style: UzStyle) -> UzNodeId {
        let taffy_style = style.to_taffy();
        let taffy_node = self.taffy.new_leaf(taffy_style).unwrap();
        let node_id = self.nodes.insert(Node::new(
            taffy_node,
            style,
            ElementNode::new_image(ImageNode::default()),
        ));

        self.taffy
            .set_node_context(
                taffy_node,
                Some(NodeContext {
                    dom_id: node_id,
                    text: None,
                    text_style: TextStyle::default(),
                    is_input: false,
                    image: None,
                }),
            )
            .unwrap();
        node_id
    }

    pub fn create_checkbox(&mut self, mut style: UzStyle) -> UzNodeId {
        if matches!(style.size.width, Length::Auto) {
            style.size.width = Length::Px(18.0);
        }
        if matches!(style.size.height, Length::Auto) {
            style.size.height = Length::Px(18.0);
        }

        let taffy_style = style.to_taffy();
        let taffy_node = self.taffy.new_leaf(taffy_style).unwrap();
        let node_id = self.nodes.insert(Node::new(
            taffy_node,
            style,
            ElementNode::new_checkbox_input(false),
        ));

        self.taffy
            .set_node_context(
                taffy_node,
                Some(NodeContext {
                    dom_id: node_id,
                    text: None,
                    text_style: TextStyle::default(),
                    is_input: false,
                    image: None,
                }),
            )
            .unwrap();

        self.nodes[node_id].interactivity.js_interactive = true;
        self.nodes[node_id]
            .as_element_mut()
            .expect("checkbox should be an element")
            .set_focussable(true);
        node_id
    }

    /// Update a node's style (also syncs taffy).
    pub fn set_style(&mut self, node_id: UzNodeId, style: UzStyle) {
        let node = &mut self.nodes[node_id];
        let taffy_style = style.to_taffy();
        node.style = style;
        self.taffy.set_style(node.taffy_node, taffy_style).unwrap();
    }

    pub fn set_root(&mut self, node_id: UzNodeId) {
        self.root = Some(node_id);
    }

    pub fn append_child(&mut self, parent_id: UzNodeId, child_id: UzNodeId) {
        if parent_id == child_id {
            return;
        }
        if self.nodes[parent_id].as_text_node().is_some() {
            return;
        }
        self.detach_from_parent(child_id);

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

    pub fn insert_before(&mut self, parent_id: UzNodeId, child_id: UzNodeId, before_id: UzNodeId) {
        if child_id == before_id || parent_id == child_id {
            return;
        }
        if self.nodes[parent_id].as_text_node().is_some() {
            return;
        }
        self.detach_from_parent(child_id);

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

    fn detach_from_parent(&mut self, child_id: UzNodeId) {
        let Some(parent_id) = self.nodes[child_id].parent else {
            return;
        };

        let parent_taffy = self.nodes[parent_id].taffy_node;
        let child_taffy = self.nodes[child_id].taffy_node;
        let _ = self.taffy.remove_child(parent_taffy, child_taffy);

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

        self.nodes[child_id].parent = None;
        self.nodes[child_id].prev_sibling = None;
        self.nodes[child_id].next_sibling = None;
    }

    /// Single source of truth for clearing stale NodeId references when a node
    /// is about to be freed. With plain `usize` NodeIds (slab), any long-lived
    /// field holding a removed id would silently retarget to whatever node
    /// reuses the slot. Every removal path MUST funnel through here.
    ///
    /// When adding a new long-lived `NodeId` field to `Dom`, register it here.
    fn on_node_removed(&mut self, id: UzNodeId) {
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
        if self
            .text_selection
            .anchor
            .is_some_and(|endpoint| endpoint.node == id)
            || self
                .text_selection
                .focus
                .is_some_and(|endpoint| endpoint.node == id)
            || self.selection_root(&self.text_selection) == Some(id)
        {
            self.text_selection.clear();
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

    pub fn remove_child(&mut self, parent_id: UzNodeId, child_id: UzNodeId) {
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
    pub fn set_text_content(&mut self, node_id: UzNodeId, text: String) {
        let node = &mut self.nodes[node_id];
        let tc = TextNode {
            content: text.clone(),
        };

        if let Some(text_node) = node.as_text_node_mut() {
            text_node.content = text;
        }

        let taffy_node = node.taffy_node;
        let text_style = node.style.text.clone();
        self.taffy
            .set_node_context(
                taffy_node,
                Some(NodeContext {
                    dom_id: node_id,
                    text: Some(tc),
                    text_style,
                    is_input: false,
                    image: None,
                }),
            )
            .unwrap();
    }

    pub fn set_image_data(&mut self, node_id: UzNodeId, data: ImageData) {
        let Some(node) = self.nodes.get_mut(node_id) else {
            return;
        };
        let Some(image_node) = node.as_image_mut() else {
            return;
        };
        let measure = data.natural_size().map(|(w, h)| ImageMeasureInfo {
            width: w,
            height: h,
        });
        image_node.data = data;

        let taffy_node = node.taffy_node;
        if let Some(ctx) = self.taffy.get_node_context_mut(taffy_node) {
            ctx.image = measure;
        }
        let _ = self.taffy.mark_dirty(taffy_node);
    }

    pub fn clear_image_data(&mut self, node_id: UzNodeId) {
        let Some(node) = self.nodes.get_mut(node_id) else {
            return;
        };
        let Some(image_node) = node.as_image_mut() else {
            return;
        };
        image_node.clear();

        let taffy_node = node.taffy_node;
        if let Some(ctx) = self.taffy.get_node_context_mut(taffy_node) {
            ctx.image = None;
        }
        let _ = self.taffy.mark_dirty(taffy_node);
    }

    /// Remove all children (and their descendants) from `parent_id`, clearing
    /// the taffy tree and slotmap entries.  The parent node itself is kept.
    pub fn clear_children(&mut self, parent_id: UzNodeId) {
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
        let Some(root) = self.root else { return };
        let taffy_root = self.nodes[root].taffy_node;

        let mut computed_styles = Vec::new();
        self.collect_computed_styles(root, None, &mut computed_styles);

        for (taffy_node, taffy_style, text_style) in computed_styles {
            self.taffy.set_style(taffy_node, taffy_style).unwrap();
            if let Some(ctx) = self.taffy.get_node_context_mut(taffy_node) {
                ctx.text_style = text_style;
            }
        }

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

    pub(crate) fn computed_style(&self, node_id: UzNodeId, parent: Option<&UzStyle>) -> UzStyle {
        let node = &self.nodes[node_id];
        let parent = parent.unwrap_or(&node.style);
        node.interactivity.compute_style_inherited(
            &node.style,
            parent,
            node_id,
            &self.hit_state,
            self.focused_node == Some(node_id),
        )
    }

    fn collect_computed_styles(
        &self,
        node_id: UzNodeId,
        parent: Option<&UzStyle>,
        out: &mut Vec<(taffy::NodeId, taffy::Style, TextStyle)>,
    ) {
        let style = self.computed_style(node_id, parent);
        let node = &self.nodes[node_id];
        out.push((node.taffy_node, style.to_taffy(), style.text.clone()));

        let mut child = node.first_child;
        while let Some(child_id) = child {
            self.collect_computed_styles(child_id, Some(&style), out);
            child = self.nodes[child_id].next_sibling;
        }
    }

    /// Run hit test at the given mouse position and update hit_state.
    pub fn update_hit_test(&mut self, x: f64, y: f64) {
        let active = self.hit_state.active_node;
        let mut hit_state = self.hitbox_store.hit_test(x, y);
        if let Some(top) = hit_state.top_node {
            hit_state.hovered_nodes = self.hit_path(top);
        }
        hit_state.active_node = active;
        self.hit_state = hit_state;
    }

    fn hit_path(&self, top: UzNodeId) -> Vec<UzNodeId> {
        let mut path = Vec::new();
        let mut current = Some(top);
        while let Some(id) = current {
            if self.nodes.get(id).is_some() {
                path.push(id);
                current = self.nodes[id].parent;
            } else {
                break;
            }
        }
        path.reverse();
        path
    }

    fn dispatch_mouse_path(
        &self,
        x: f64,
        y: f64,
        button: crate::interactivity::MouseButton,
        listeners: impl Fn(
            &crate::interactivity::Interactivity,
        ) -> &[crate::interactivity::MouseEventListener],
    ) {
        let event = crate::interactivity::MouseEvent {
            position: (x, y),
            button,
        };

        let Some(target) = self.hit_state.top_node else {
            return;
        };

        let mut path = self.hit_path(target);
        path.reverse();
        for node_id in path {
            let Some(node) = self.nodes.get(node_id) else {
                continue;
            };
            let Some(bounds) = node
                .interactivity
                .hitbox_id
                .and_then(|hid| self.hitbox_store.get(hid))
                .map(|hitbox| hitbox.bounds)
            else {
                continue;
            };
            for listener in listeners(&node.interactivity) {
                listener(&event, &bounds);
            }
        }
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
    pub fn set_active(&mut self, node_id: Option<UzNodeId>) {
        self.hit_state.active_node = node_id;
    }

    /// Dispatch mouse down event to listeners on hovered elements.
    pub fn dispatch_mouse_down(&self, x: f64, y: f64, button: crate::interactivity::MouseButton) {
        self.dispatch_mouse_path(x, y, button, |i| &i.mouse_down_listeners);
    }

    /// Dispatch mouse up event.
    pub fn dispatch_mouse_up(&self, x: f64, y: f64, button: crate::interactivity::MouseButton) {
        self.dispatch_mouse_path(x, y, button, |i| &i.mouse_up_listeners);
    }

    /// Dispatch click event.
    pub fn dispatch_click(&self, x: f64, y: f64, button: crate::interactivity::MouseButton) {
        self.dispatch_mouse_path(x, y, button, |i| &i.click_listeners);
    }

    /// Find the text run that contains a given text node.
    pub fn find_run_for_node(&self, node_id: UzNodeId) -> Option<&TextSelectRun> {
        self.selectable_text_runs
            .iter()
            .find(|run| run.entries.iter().any(|e| e.node_id == node_id))
    }

    /// Find the text run entry for a given text node.
    pub fn find_run_entry_for_node(
        &self,
        node_id: UzNodeId,
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
    pub fn is_text_selectable(&self, node_id: UzNodeId) -> bool {
        self.selectable_text_runs
            .iter()
            .any(|run| run.entries.iter().any(|e| e.node_id == node_id))
    }
}

#[cfg(test)]
mod tests {
    use super::UIState;
    use crate::{
        cursor::UzCursorIcon,
        style::{Bounds, Length, Size, UzStyle},
        text::TextRenderer,
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[test]
    fn refresh_hit_test_retargets_stationary_pointer_after_hitboxes_change() {
        let mut dom = UIState::new();
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

    #[test]
    fn plain_text_inherits_cursor_from_parent() {
        let mut dom = UIState::new();
        let parent = dom.create_view(Default::default());
        let child = dom.create_text("pointer".into(), Default::default());

        dom.append_child(parent, child);
        dom.nodes[parent].style.cursor = Some(UzCursorIcon::Pointer);

        assert_eq!(dom.resolve_cursor(child), UzCursorIcon::Pointer);
    }

    #[test]
    fn append_child_to_text_node_is_noop() {
        let mut dom = UIState::new();
        let parent = dom.create_view(Default::default());
        let text = dom.create_text("leaf".into(), Default::default());
        let child = dom.create_view(Default::default());

        dom.append_child(parent, child);
        dom.append_child(text, child);

        assert_eq!(dom.nodes[text].first_child, None);
        assert_eq!(dom.nodes[text].last_child, None);
        assert_eq!(dom.nodes[child].parent, Some(parent));
    }

    #[test]
    fn insert_before_into_text_node_is_noop() {
        let mut dom = UIState::new();
        let parent = dom.create_view(Default::default());
        let text = dom.create_text("leaf".into(), Default::default());
        let child = dom.create_view(Default::default());
        let before = dom.create_view(Default::default());

        dom.append_child(parent, child);
        dom.insert_before(text, child, before);

        assert_eq!(dom.nodes[text].first_child, None);
        assert_eq!(dom.nodes[text].last_child, None);
        assert_eq!(dom.nodes[child].parent, Some(parent));
    }

    #[test]
    fn view_default_stacks_child_views_vertically() {
        let mut dom = UIState::new();
        let mut renderer = TextRenderer::new();

        let mut parent_style = UzStyle::default_for_element("view");
        parent_style.size = Size {
            width: Length::Px(100.0),
            height: Length::Px(100.0),
        };

        let mut child_style = UzStyle::default_for_element("view");
        child_style.size = Size {
            width: Length::Px(20.0),
            height: Length::Px(10.0),
        };

        let parent = dom.create_view(parent_style);
        let first = dom.create_view(child_style.clone());
        let second = dom.create_view(child_style);

        dom.set_root(parent);
        dom.append_child(parent, first);
        dom.append_child(parent, second);
        dom.compute_layout(100.0, 100.0, &mut renderer);

        let first_layout = dom.taffy.layout(dom.nodes[first].taffy_node).unwrap();
        let second_layout = dom.taffy.layout(dom.nodes[second].taffy_node).unwrap();

        assert_eq!(first_layout.location.y, 0.0);
        assert_eq!(second_layout.location.y, first_layout.size.height);
        assert_eq!(second_layout.location.x, 0.0);
    }

    #[test]
    fn hit_test_hover_and_dispatch_follow_top_node_ancestors_not_siblings() {
        let mut dom = UIState::new();
        let root = dom.create_view(Default::default());
        let sibling = dom.create_view(Default::default());
        let modal = dom.create_view(Default::default());
        dom.append_child(root, sibling);
        dom.append_child(root, modal);

        let sibling_clicks = Arc::new(AtomicUsize::new(0));
        let modal_clicks = Arc::new(AtomicUsize::new(0));

        {
            let clicks = Arc::clone(&sibling_clicks);
            dom.nodes[sibling].interactivity.on_click(move |_, _| {
                clicks.fetch_add(1, Ordering::Relaxed);
            });
        }
        {
            let clicks = Arc::clone(&modal_clicks);
            dom.nodes[modal].interactivity.on_click(move |_, _| {
                clicks.fetch_add(1, Ordering::Relaxed);
            });
        }

        let sibling_hitbox = dom
            .hitbox_store
            .insert(sibling, Bounds::new(0.0, 0.0, 100.0, 100.0));
        let modal_hitbox = dom
            .hitbox_store
            .insert(modal, Bounds::new(20.0, 20.0, 40.0, 40.0));
        dom.nodes[sibling].interactivity.hitbox_id = Some(sibling_hitbox);
        dom.nodes[modal].interactivity.hitbox_id = Some(modal_hitbox);

        dom.update_hit_test(30.0, 30.0);
        dom.dispatch_click(30.0, 30.0, crate::interactivity::MouseButton::Left);

        assert_eq!(dom.hit_state.top_node, Some(modal));
        assert_eq!(dom.hit_state.hovered_nodes, vec![root, modal]);
        assert_eq!(modal_clicks.load(Ordering::Relaxed), 1);
        assert_eq!(sibling_clicks.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn transformed_hitboxes_test_points_in_node_local_space() {
        use vello::kurbo::Affine;

        let mut dom = UIState::new();
        let node = dom.create_view(Default::default());
        dom.hitbox_store.insert_transformed(
            node,
            Bounds::new(0.0, 0.0, 10.0, 10.0),
            Affine::translate((50.0, 50.0))
                * Affine::rotate(std::f64::consts::FRAC_PI_4)
                * Affine::translate((-5.0, -5.0)),
        );

        dom.update_hit_test(50.0, 50.0);
        assert_eq!(dom.hit_state.top_node, Some(node));

        dom.update_hit_test(50.0, 42.0);
        assert_eq!(dom.hit_state.top_node, None);
    }
}
