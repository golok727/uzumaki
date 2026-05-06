use slab::Slab;

use crate::{
    cursor::UzCursorIcon,
    element::{
        DragMode, ElementNode, ImageData, ImageNode, Node, ScrollAxis, ScrollState,
        ScrollThumbRect, ScrollWheelTarget, TextContent, TextNode, TextRunEntry, TextSelectRun,
        UzNodeId,
        scroll::{self, ScrollAlign, ScrollIntoViewOptions},
    },
    input::InputState,
    interactivity::{HitTestState, HitboxStore},
    layout::LayoutEngine,
    selection::TextSelection,
    style::{Length, UzStyle},
    text::TextRenderer,
};

pub struct UIState {
    pub nodes: Slab<Node>,

    pub layout_engine: LayoutEngine,
    pub root: Option<UzNodeId>,
    /// Hitboxes registered during the last paint pass.
    pub hitbox_store: HitboxStore,
    /// Current hit test state (updated on mouse move).
    pub hit_state: HitTestState,
    /// Currently focuswsed ndoe
    pub focused_node: Option<UzNodeId>,
    pending_scroll_node_into_view: Option<(UzNodeId, ScrollIntoViewOptions)>,
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
    /// Current UI-owned drag, following Blitz's document-level drag model.
    pub drag_mode: DragMode,
    /// Short-lived wheel routing capture for nested scroll continuity.
    pub wheel_capture: Option<ScrollWheelTarget>,
    /// Current text selection within a textSelect view. `root == None` means
    /// there is no active view selection
    pub text_selection: TextSelection,
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
            layout_engine: LayoutEngine::new(),
            root: None,
            hitbox_store: HitboxStore::default(),
            hit_state: HitTestState::default(),
            focused_node: None,
            pending_scroll_node_into_view: None,
            last_click_time: None,
            last_click_node: None,
            click_count: 0,
            window_focused: true,
            scroll_thumbs: Vec::new(),
            drag_mode: DragMode::None,
            wheel_capture: None,
            text_selection: TextSelection::default(),
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

        let style = node.style_variants.compute_style(
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

        // Walk ancestors. An explicit `cursor` style wins, but if any ancestor
        // is a text-selectable scope (`selectable` view) we should also show
        // the text cursor — otherwise an inner non-selectable `<text>` inside
        // a selectable view never gets the I-beam.
        let mut cur = node.parent;
        while let Some(id) = cur {
            let n = &self.nodes[id];
            let style = n.style_variants.compute_style(
                &n.style,
                id,
                &self.hit_state,
                self.focused_node == Some(id),
            );
            if let Some(c) = style.cursor {
                return c;
            }
            if n.is_text_selectable() {
                return UzCursorIcon::Text;
            }
            cur = n.parent;
        }
        UzCursorIcon::Default
    }

    /// Create a View element with a style.
    pub fn create_view(&mut self, style: UzStyle) -> UzNodeId {
        self.nodes.insert(Node::new(style, ElementNode::new_view()))
    }

    pub fn create_button(&mut self, style: UzStyle) -> UzNodeId {
        self.nodes
            .insert(Node::new(style, ElementNode::new_button()))
    }

    // leaf text node
    pub fn create_text_node(&mut self, content: String, style: UzStyle) -> UzNodeId {
        let text = TextContent {
            content: content.clone(),
        };
        self.nodes
            .insert(Node::new(style, TextNode::new(text.clone())))
    }

    /// Create a Text element. <text>
    pub fn create_text_element(&mut self, content: String, style: UzStyle) -> UzNodeId {
        let text = TextContent {
            content: content.clone(),
        };
        self.nodes
            .insert(Node::new(style, ElementNode::new_text(text.clone())))
    }

    /// Create an Input element.
    pub fn create_input(&mut self, style: UzStyle) -> UzNodeId {
        let is = InputState::new_single_line();

        let node_id = self
            .nodes
            .insert(Node::new(style, ElementNode::new_text_input(is)));
        self.nodes[node_id]
            .as_element_mut()
            .expect("input should be an element")
            .set_focussable(true);
        node_id
    }

    pub fn create_image(&mut self, style: UzStyle) -> UzNodeId {
        self.nodes.insert(Node::new(
            style,
            ElementNode::new_image(ImageNode::default()),
        ))
    }

    pub fn create_checkbox(&mut self, mut style: UzStyle) -> UzNodeId {
        if matches!(style.size.width, Length::Auto) {
            style.size.width = Length::Px(18.0);
        }
        if matches!(style.size.height, Length::Auto) {
            style.size.height = Length::Px(18.0);
        }

        let node_id = self
            .nodes
            .insert(Node::new(style, ElementNode::new_checkbox_input(false)));
        self.nodes[node_id]
            .as_element_mut()
            .expect("checkbox should be an element")
            .set_focussable(true);
        node_id
    }

    /// Update a node's style. Layout state is rebuilt on the next frame.
    pub fn set_style(&mut self, node_id: UzNodeId, style: UzStyle) {
        let node = &mut self.nodes[node_id];
        node.style = style;
    }

    pub fn set_root(&mut self, node_id: UzNodeId) {
        self.root = Some(node_id);
    }

    pub fn append_child(&mut self, parent_id: UzNodeId, child_id: UzNodeId) {
        if parent_id == child_id {
            return;
        }
        if self.nodes[parent_id].get_text_content().is_some() {
            return;
        }

        self.detach_from_parent(child_id);

        self.nodes[child_id].parent = Some(parent_id);
        self.nodes[parent_id].children.push(child_id);
    }

    pub fn insert_before(&mut self, parent_id: UzNodeId, child_id: UzNodeId, before_id: UzNodeId) {
        if child_id == before_id || parent_id == child_id {
            return;
        }
        if self.nodes[parent_id].get_text_content().is_some() {
            return;
        }
        self.detach_from_parent(child_id);
        self.nodes[child_id].parent = Some(parent_id);
        let Some(index) = self.nodes[parent_id]
            .children
            .iter()
            .position(|&id| id == before_id)
        else {
            self.nodes[child_id].parent = None;
            return;
        };
        self.nodes[parent_id].children.insert(index, child_id);
    }

    pub fn first_child(&self, node_id: UzNodeId) -> Option<UzNodeId> {
        self.nodes
            .get(node_id)
            .and_then(|node| node.children.first().copied())
    }

    pub fn last_child(&self, node_id: UzNodeId) -> Option<UzNodeId> {
        self.nodes
            .get(node_id)
            .and_then(|node| node.children.last().copied())
    }

    pub fn next_sibling(&self, node_id: UzNodeId) -> Option<UzNodeId> {
        let node = self.nodes.get(node_id)?;
        let parent_id = node.parent?;
        let siblings = &self.nodes.get(parent_id)?.children;
        let index = siblings.iter().position(|&id| id == node_id)?;
        siblings.get(index + 1).copied()
    }

    pub fn prev_sibling(&self, node_id: UzNodeId) -> Option<UzNodeId> {
        let node = self.nodes.get(node_id)?;
        let parent_id = node.parent?;
        let siblings = &self.nodes.get(parent_id)?.children;
        let index = siblings.iter().position(|&id| id == node_id)?;
        index
            .checked_sub(1)
            .and_then(|idx| siblings.get(idx).copied())
    }

    pub fn request_scroll_node_into_view(
        &mut self,
        node_id: UzNodeId,
        opts: ScrollIntoViewOptions,
    ) {
        self.pending_scroll_node_into_view = Some((node_id, opts));
    }

    /// Focus-flavored variant: text inputs scroll their parent (the wrapping
    /// view) instead of themselves, and we always use default options.
    pub fn request_scroll_focus_into_view(&mut self, node_id: UzNodeId) {
        let target = if self.nodes.get(node_id).is_some_and(|n| n.is_text_input()) {
            self.nodes
                .get(node_id)
                .and_then(|n| n.parent)
                .unwrap_or(node_id)
        } else {
            node_id
        };
        self.request_scroll_node_into_view(target, ScrollIntoViewOptions::default());
    }

    pub fn scroll_node_into_view(&mut self, node_id: UzNodeId, opts: ScrollIntoViewOptions) {
        if !self.nodes.contains(node_id) {
            return;
        }
        self.scroll_axis_into_view(node_id, ScrollAxis::Y, opts.block, opts.margin);
        self.scroll_axis_into_view(node_id, ScrollAxis::X, opts.inline, opts.margin);
    }

    fn scroll_axis_into_view(
        &mut self,
        target_id: UzNodeId,
        axis: ScrollAxis,
        align: ScrollAlign,
        margin: f32,
    ) {
        let Some(scroller_id) = Self::nearest_overflow_scroller(&self.nodes, target_id, axis)
        else {
            return;
        };
        let Some((rel, scroller_abs)) =
            Self::accumulate_axis(&self.nodes, target_id, scroller_id, axis)
        else {
            return;
        };

        let Some(target_node) = self.nodes.get(target_id) else {
            return;
        };
        let Some(scroller_ref) = self.nodes.get(scroller_id) else {
            return;
        };

        let target_extent = axis_size(target_node.final_layout.size, axis);
        let content_extent = axis_size(scroller_ref.final_layout.content_size, axis);

        // Clamp viewport by the root rect so a scroller overflowing the window
        // doesn't think it has more visible space than the user can actually see.
        let root_extent = self
            .root
            .and_then(|r| self.nodes.get(r))
            .map(|n| axis_size(n.final_layout.size, axis))
            .unwrap_or(f32::MAX);
        let scroller_extent = axis_size(scroller_ref.final_layout.size, axis);
        let clipped_end = (scroller_abs + scroller_extent).min(root_extent);
        let true_viewport = (clipped_end - scroller_abs).max(0.0);

        // Match render: a horizontal scrollbar steals from the Y viewport, but
        // a vertical scrollbar does not steal from the X viewport.
        let viewport_extent = match axis {
            ScrollAxis::Y => scroll::vertical_scroll_visible_height(
                true_viewport,
                scroller_ref.final_layout.content_size.width,
                scroller_ref.final_layout.size.width,
                scroller_ref.style.overflow_x.is_scrollable(),
                scroller_ref.style.scrollbar.width,
            ),
            ScrollAxis::X => true_viewport,
        };

        let cur_offset = scroller_ref
            .scroll_state
            .as_ref()
            .map(|s| s.offset(axis))
            .unwrap_or(0.0);

        let Some(next_offset) = scroll::compute_scroll_offset(
            rel,
            target_extent,
            viewport_extent,
            content_extent,
            cur_offset,
            align,
            margin,
        ) else {
            return;
        };

        let ss = self.nodes[scroller_id]
            .scroll_state
            .get_or_insert(ScrollState::new());
        ss.set_offset(axis, next_offset);
    }

    fn nearest_overflow_scroller(
        nodes: &Slab<Node>,
        target_id: UzNodeId,
        axis: ScrollAxis,
    ) -> Option<UzNodeId> {
        let mut ancestor = nodes.get(target_id).and_then(|n| n.parent)?;
        loop {
            let node = nodes.get(ancestor)?;
            let scrollable = match axis {
                ScrollAxis::Y => node.style.overflow_y.is_scrollable(),
                ScrollAxis::X => node.style.overflow_x.is_scrollable(),
            };
            if scrollable {
                return Some(ancestor);
            }
            ancestor = node.parent?;
        }
    }

    /// Walks target -> root once, returning (rel of target inside scroller,
    /// layout-absolute position of scroller) along the given axis.
    fn accumulate_axis(
        nodes: &Slab<Node>,
        target_id: UzNodeId,
        scroller_id: UzNodeId,
        axis: ScrollAxis,
    ) -> Option<(f32, f32)> {
        let mut cur = target_id;
        let mut rel = 0.0f32;
        let mut scroller_abs = 0.0f32;
        let mut past_scroller = false;
        loop {
            let node = nodes.get(cur)?;
            let loc = axis_loc(node.final_layout.location, axis);
            if past_scroller {
                scroller_abs += loc;
            } else {
                rel += loc;
            }
            match node.parent {
                Some(pid) if !past_scroller && pid == scroller_id => {
                    past_scroller = true;
                    cur = pid;
                }
                Some(pid) => cur = pid,
                None => {
                    if past_scroller {
                        break;
                    } else {
                        return None;
                    }
                }
            }
        }
        Some((rel, scroller_abs))
    }

    fn remove_child_ref(&mut self, parent_id: UzNodeId, child_id: UzNodeId) -> bool {
        let Some(index) = self.nodes[parent_id]
            .children
            .iter()
            .position(|&id| id == child_id)
        else {
            return false;
        };
        self.nodes[parent_id].children.remove(index);
        true
    }

    fn detach_from_parent(&mut self, child_id: UzNodeId) {
        let Some(parent_id) = self.nodes[child_id].parent else {
            return;
        };

        self.remove_child_ref(parent_id, child_id);
        self.nodes[child_id].parent = None;
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
        if matches!(self.pending_scroll_node_into_view, Some((nid, _)) if nid == id) {
            self.pending_scroll_node_into_view = None;
        }
        if self.last_click_node == Some(id) {
            self.last_click_node = None;
            self.click_count = 0;
            self.last_click_time = None;
        }
        if self.drag_mode.node_id() == Some(id) {
            self.drag_mode = DragMode::None;
        }
        if self
            .wheel_capture
            .as_ref()
            .is_some_and(|capture| capture.node_id == id)
        {
            self.wheel_capture = None;
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
        if !self.remove_child_ref(parent_id, child_id) {
            return;
        }
        self.nodes[child_id].parent = None;
        // The node lives on in the slab until its JS wrapper is collected, but
        // every long-lived NodeId field (selection, focus, hit-state, scroll
        // locks…) refers to nodes by their place in the tree. Once detached
        // those references are stale, so scrub them now — same contract as
        // before the GC refactor.
        self.on_detached_subtree(child_id);
    }

    pub fn destroy_node(&mut self, node_id: UzNodeId) {
        if !self.nodes.contains(node_id) || self.root == Some(node_id) {
            return;
        }

        // eprintln!("[uzumaki] removing native node {node_id}",);

        if let Some(parent_id) = self.nodes[node_id].parent
            && self.nodes.contains(parent_id)
        {
            self.remove_child_ref(parent_id, node_id);
        }

        let children = std::mem::take(&mut self.nodes[node_id].children);
        for child_id in children {
            if let Some(child) = self.nodes.get_mut(child_id)
                && child.parent == Some(node_id)
            {
                child.parent = None;
            }
        }

        self.on_node_removed(node_id);
        self.nodes.remove(node_id);
    }

    /// Update a text node's content.
    pub fn set_text_content(&mut self, node_id: UzNodeId, text: String) {
        let node = &mut self.nodes[node_id];
        if let Some(text_node) = node.text_content_mut() {
            text_node.content = text;
        }
    }

    pub fn set_image_data(&mut self, node_id: UzNodeId, data: ImageData) {
        let Some(node) = self.nodes.get_mut(node_id) else {
            return;
        };
        let Some(image_node) = node.as_image_mut() else {
            return;
        };
        image_node.data = data;
    }

    pub fn clear_image_data(&mut self, node_id: UzNodeId) {
        let Some(node) = self.nodes.get_mut(node_id) else {
            return;
        };
        let Some(image_node) = node.as_image_mut() else {
            return;
        };
        image_node.clear();
    }

    /// Detach all children from `parent_id`. Nodes themselves stay alive in
    /// the slab;
    pub fn clear_children(&mut self, parent_id: UzNodeId) {
        let children = std::mem::take(&mut self.nodes[parent_id].children);
        for child_id in children {
            self.nodes[child_id].parent = None;
            self.on_detached_subtree(child_id);
        }
    }

    /// Walk the subtree rooted at `id` (still wired via `children` since we
    /// only detach the root from its parent) and run `on_node_removed` on
    /// every entry. The slab nodes themselves stay alive.
    fn on_detached_subtree(&mut self, id: UzNodeId) {
        let children: Vec<UzNodeId> = self
            .nodes
            .get(id)
            .map(|node| node.children.clone())
            .unwrap_or_default();
        for child in children {
            self.on_detached_subtree(child);
        }
        self.on_node_removed(id);
    }

    pub fn compute_layout(&mut self, width: f32, height: f32, text_renderer: &mut TextRenderer) {
        self.layout_engine.compute_layout(
            &self.nodes,
            self.root,
            &self.hit_state,
            self.focused_node,
            width,
            height,
            text_renderer,
        );
        self.copy_final_layouts();
        self.refresh_text_layouts(text_renderer);
        if let Some((nid, opts)) = self.pending_scroll_node_into_view.take() {
            self.scroll_node_into_view(nid, opts);
        }
    }

    /// Copy taffy's layout result onto each node so the paint pass can read
    /// `node.final_layout` directly without going through the layout engine's
    /// id → taffy_id → slab indirection.
    fn copy_final_layouts(&mut self) {
        for (node_id, node) in self.nodes.iter_mut() {
            node.final_layout = self
                .layout_engine
                .layout(node_id)
                .copied()
                .unwrap_or_else(taffy::Layout::new);
        }
    }

    /// Rebuild cached parley layouts for every text-bearing node at its final
    /// taffy width. Runs once per frame after layout. Paint / selection /
    /// hit-test then reuse `node.text_layout` instead of rebuilding.
    fn refresh_text_layouts(&mut self, text_renderer: &mut TextRenderer) {
        let Some(root) = self.root else { return };
        self.refresh_text_layouts_at(root, None, text_renderer);
    }

    fn refresh_text_layouts_at(
        &mut self,
        node_id: UzNodeId,
        parent_style: Option<&UzStyle>,
        text_renderer: &mut TextRenderer,
    ) {
        let computed = self.computed_style(node_id, parent_style);
        let children = self.nodes[node_id].children.clone();

        let is_input = self.nodes[node_id].is_text_input();
        let text = (!is_input)
            .then(|| {
                self.nodes[node_id]
                    .get_text_content()
                    .map(|t| t.content.clone())
            })
            .flatten();

        if let Some(text) = text {
            let width = Some(self.nodes[node_id].final_layout.size.width);
            let layout = text_renderer.build_layout(&text, &computed.text, width);
            self.nodes[node_id].text_layout = Some(layout);
        } else {
            self.nodes[node_id].text_layout = None;
        }

        for cid in children {
            self.refresh_text_layouts_at(cid, Some(&computed), text_renderer);
        }
    }

    pub(crate) fn computed_style(&self, node_id: UzNodeId, parent: Option<&UzStyle>) -> UzStyle {
        let node = &self.nodes[node_id];
        let parent = parent.unwrap_or(&node.style);
        node.style_variants.compute_style_inherited(
            &node.style,
            parent,
            node_id,
            &self.hit_state,
            self.focused_node == Some(node_id),
        )
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

    pub fn nearest_button_ancestor(&self, node_id: UzNodeId) -> Option<UzNodeId> {
        self.nearest_ancestor_matching(node_id, |node| node.is_button())
    }

    fn nearest_ancestor_matching(
        &self,
        node_id: UzNodeId,
        mut matches: impl FnMut(&Node) -> bool,
    ) -> Option<UzNodeId> {
        let mut current = Some(node_id);
        while let Some(id) = current {
            let node = self.nodes.get(id)?;
            if matches(node) {
                return Some(id);
            }
            current = node.parent;
        }
        None
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

    /// Walk up from `node_id` and return the root of the enclosing
    /// text-selectable run, if any. This lets a click anywhere inside a
    /// `selectable` container ,  not just on a text node , start selection
    pub fn containing_text_run_root(&self, node_id: UzNodeId) -> Option<UzNodeId> {
        let mut cur = Some(node_id);
        while let Some(id) = cur {
            if self
                .selectable_text_runs
                .iter()
                .any(|run| run.root_id == id)
            {
                return Some(id);
            }
            cur = self.nodes.get(id).and_then(|n| n.parent);
        }
        None
    }
}

#[inline]
fn axis_size(size: taffy::Size<f32>, axis: ScrollAxis) -> f32 {
    match axis {
        ScrollAxis::X => size.width,
        ScrollAxis::Y => size.height,
    }
}

#[inline]
fn axis_loc(point: taffy::Point<f32>, axis: ScrollAxis) -> f32 {
    match axis {
        ScrollAxis::X => point.x,
        ScrollAxis::Y => point.y,
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
        let child = dom.create_text_element("pointer".into(), Default::default());

        dom.append_child(parent, child);
        dom.nodes[parent].style.cursor = Some(UzCursorIcon::Pointer);

        assert_eq!(dom.resolve_cursor(child), UzCursorIcon::Pointer);
    }

    #[test]
    fn button_child_press_activates_button_ancestor() {
        let mut dom = UIState::new();
        let button = dom.create_button(Default::default());
        let label = dom.create_text_element("Press".into(), Default::default());

        dom.append_child(button, label);
        dom.hitbox_store
            .insert(button, Bounds::new(0.0, 0.0, 100.0, 40.0));
        dom.hitbox_store
            .insert(label, Bounds::new(20.0, 10.0, 60.0, 20.0));

        dom.update_hit_test(30.0, 20.0);
        assert_eq!(dom.hit_state.top_node, Some(label));
        assert_eq!(dom.nearest_button_ancestor(label), Some(button));

        dom.set_active(dom.nearest_button_ancestor(label).or(Some(label)));
        assert!(dom.hit_state.is_active(button));
    }

    #[test]
    fn non_button_press_keeps_hit_node_active() {
        let mut dom = UIState::new();
        let view = dom.create_view(Default::default());

        dom.hitbox_store
            .insert(view, Bounds::new(0.0, 0.0, 100.0, 40.0));
        dom.update_hit_test(30.0, 20.0);

        let active = dom
            .hit_state
            .top_node
            .and_then(|nid| dom.nearest_button_ancestor(nid).or(Some(nid)));
        dom.set_active(active);

        assert!(dom.hit_state.is_active(view));
    }

    #[test]
    fn keyboard_activation_is_limited_to_element_kind() {
        let mut dom = UIState::new();
        let focusable_view = dom.create_view(Default::default());
        let button = dom.create_button(Default::default());

        dom.nodes[focusable_view]
            .as_element_mut()
            .unwrap()
            .set_focussable(true);

        assert!(dom.nodes[focusable_view].is_focusable());
        assert!(!dom.nodes[focusable_view].is_keyboard_activatable());
        assert!(dom.nodes[button].is_keyboard_activatable());
    }

    #[test]
    fn append_child_to_text_node_is_noop() {
        let mut dom = UIState::new();
        let parent = dom.create_view(Default::default());
        let text = dom.create_text_element("leaf".into(), Default::default());
        let child = dom.create_view(Default::default());

        dom.append_child(parent, child);
        dom.append_child(text, child);

        assert!(dom.nodes[text].children.is_empty());
        assert_eq!(dom.nodes[child].parent, Some(parent));
    }

    #[test]
    fn insert_before_into_text_node_is_noop() {
        let mut dom = UIState::new();
        let parent = dom.create_view(Default::default());
        let text = dom.create_text_element("leaf".into(), Default::default());
        let child = dom.create_view(Default::default());
        let before = dom.create_view(Default::default());

        dom.append_child(parent, child);
        dom.insert_before(text, child, before);

        assert!(dom.nodes[text].children.is_empty());
        assert_eq!(dom.nodes[child].parent, Some(parent));
    }

    #[test]
    fn remove_node_detaches_links_and_clears_long_lived_refs() {
        let mut dom = UIState::new();
        let parent = dom.create_view(Default::default());
        let child = dom.create_view(Default::default());
        let grandchild = dom.create_view(Default::default());

        dom.append_child(parent, child);
        dom.append_child(child, grandchild);
        dom.focused_node = Some(child);
        dom.hit_state.top_node = Some(child);

        dom.destroy_node(child);

        assert!(!dom.nodes.contains(child));
        assert!(dom.nodes[parent].children.is_empty());
        assert_eq!(dom.nodes[grandchild].parent, None);
        assert_eq!(dom.focused_node, None);
        assert_eq!(dom.hit_state.top_node, None);
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

        let first_layout = &dom.nodes[first].final_layout;
        let second_layout = &dom.nodes[second].final_layout;

        assert_eq!(first_layout.location.y, 0.0);
        assert_eq!(second_layout.location.y, first_layout.size.height);
        assert_eq!(second_layout.location.x, 0.0);
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
