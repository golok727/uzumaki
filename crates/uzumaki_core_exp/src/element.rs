use cosmic_text::Attrs;
use slotmap::{SlotMap, new_key_type};
use vello::Scene;
use vello::kurbo::{Affine, Rect, RoundedRect, RoundedRectRadii};
use vello::peniko::{Color as VelloColor, Fill};

use crate::elements::input::InputRenderInfo;
use crate::input::InputState;
use crate::interactivity::{HitTestState, HitboxStore, Interactivity};
use crate::style::{Bounds, Color, Style};
use crate::text::TextRenderer;

new_key_type! {
    pub struct NodeId;
}

pub struct ScrollState {
    pub scroll_offset_y: f32,
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

#[derive(Default)]
pub struct InputBehavior {
    pub state: InputState,
}

impl InputBehavior {
    pub fn new(state: InputState) -> Self {
        Self { state }
    }

    pub fn new_single_line() -> Self {
        Self::new(InputState::new(false))
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
}

pub struct Dom {
    pub nodes: SlotMap<NodeId, Node>,
    pub taffy: taffy::TaffyTree<NodeContext>,
    pub root: Option<NodeId>,
    /// Hitboxes registered during the last paint pass.
    pub hitbox_store: HitboxStore,
    /// Current hit test state (updated on mouse move).
    pub hit_state: HitTestState,
    /// Currently focused input node.
    pub focused_node: Option<NodeId>,
    // oh god please move this to input state
    /// Input node being dragged for selection.
    pub dragging_input: Option<NodeId>,
    /// Last click time (for double-click detection).
    pub last_click_time: Option<std::time::Instant>,
    /// Last clicked node (for double-click detection).
    pub last_click_node: Option<NodeId>,
    /// Whether the OS window is focused.
    pub window_focused: bool,
    /// Scroll thumb rects from last paint pass (for hit testing).
    pub scroll_thumbs: Vec<ScrollThumbRect>,
    /// Active scroll-thumb drag state (only one at a time).
    pub scroll_drag: Option<ScrollDragState>,
    /// Scroll lock: when scrolling starts, lock to that node for a short duration
    /// to prevent inner scrollable views from stealing wheel events mid-scroll.
    pub scroll_lock: Option<(NodeId, std::time::Instant)>,
}

// Safety:  We only access it from main thread
unsafe impl Send for Dom {}
unsafe impl Sync for Dom {}

impl Dom {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::with_key(),
            taffy: taffy::TaffyTree::new(),
            root: None,
            hitbox_store: HitboxStore::default(),
            hit_state: HitTestState::default(),
            focused_node: None,
            dragging_input: None,
            last_click_time: None,
            last_click_node: None,
            window_focused: true,
            scroll_thumbs: Vec::new(),
            scroll_drag: None,
            scroll_lock: None,
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
        focus
            .map(|id| self.nodes.get_mut(id).map(|node| update(node, id)))
            .flatten()
    }

    pub fn get_node(&self, node_id: NodeId) -> Option<&Node> {
        self.nodes.get(node_id)
    }

    pub fn get_node_mut(&mut self, node_id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(node_id)
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
            behavior: Box::new(InputBehavior::new_single_line()),
            style,
            interactivity: Interactivity::new(),
            scroll_state: None,
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

        self.nodes[child_id].parent = None;
        self.nodes[child_id].prev_sibling = None;
        self.nodes[child_id].next_sibling = None;
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

        // Remove descendants from taffy + slotmap
        for nid in to_remove {
            let tn = self.nodes[nid].taffy_node;
            let _ = self.taffy.remove(tn);
            self.nodes.remove(nid);
        }

        // Reset parent pointers
        self.nodes[parent_id].first_child = None;
        self.nodes[parent_id].last_child = None;

        // Stale hitboxes reference removed nodes
        self.hitbox_store.clear();
        self.hit_state = HitTestState::default();
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
                        Self::measure(
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
        let active = self.hit_state.active_hitbox;
        self.hit_state = self.hitbox_store.hit_test(x, y);
        self.hit_state.active_hitbox = active;
    }

    /// Set active hitbox (mouse down on an element).
    pub fn set_active(&mut self, hitbox_id: Option<crate::interactivity::HitboxId>) {
        self.hit_state.active_hitbox = hitbox_id;
    }

    /// Render the DOM tree into the scene. Also rebuilds hitboxes and scroll thumbs.
    pub fn render(&mut self, scene: &mut Scene, text_renderer: &mut TextRenderer, scale: f64) {
        self.hitbox_store.clear();
        self.scroll_thumbs.clear();

        if let Some(root) = self.root {
            self.render_tree(scene, text_renderer, root, scale);
        }
    }

    fn render_tree(
        &mut self,
        scene: &mut Scene,
        text_renderer: &mut TextRenderer,
        root_id: NodeId,
        scale: f64,
    ) {
        // Collect render info for all nodes in DFS order
        struct RenderInfo {
            node_id: NodeId,
            x: f64,
            y: f64,
            w: f64,
            h: f64,
            style: Style,
            text: Option<(String, f32, Color)>,
            needs_hitbox: bool,
            input: Option<InputRenderInfo>,
        }

        struct ThumbInfo {
            node_id: NodeId,
            view_x: f64,
            view_y: f64,
            view_w: f64,
            view_h: f64,
            scroll_offset_y: f32,
            content_height: f32,
            visible_height: f32,
            thumb_hovered: bool,
            scale: f64,
        }

        enum RenderCommand {
            PaintNode(RenderInfo),
            PushClip(Rect, f64),
            PopClip,
            PaintThumb(ThumbInfo),
        }

        enum StackItem {
            Visit(NodeId, f64, f64),
            PushClip(Rect, f64),
            PopClip,
            PaintThumb(ThumbInfo),
        }

        let mut render_list: Vec<RenderCommand> = Vec::new();
        let mut stack: Vec<StackItem> = vec![StackItem::Visit(root_id, 0.0, 0.0)];

        while let Some(item) = stack.pop() {
            match item {
                StackItem::PushClip(rect, s) => {
                    render_list.push(RenderCommand::PushClip(rect, s));
                    continue;
                }
                StackItem::PopClip => {
                    render_list.push(RenderCommand::PopClip);
                    continue;
                }
                StackItem::PaintThumb(info) => {
                    render_list.push(RenderCommand::PaintThumb(info));
                    continue;
                }
                StackItem::Visit(node_id, parent_x, parent_y) => {
                    // Extract all needed data from the node (immutable borrow scope)
                    let (
                        taffy_node,
                        computed_style,
                        text,
                        input,
                        needs_hitbox,
                        is_scrollable,
                        first_child,
                    ) = {
                        let node = &self.nodes[node_id];
                        let taffy_node = node.taffy_node;
                        let computed_style = node
                            .interactivity
                            .compute_style(&node.style, &self.hit_state);

                        let text = node.behavior.as_text().map(|tc| {
                            (
                                tc.content.clone(),
                                computed_style.text.font_size,
                                computed_style.text.color,
                            )
                        });

                        let input = node.behavior.as_input().map(|is| InputRenderInfo {
                            display_text: is.display_text(),
                            placeholder: is.placeholder.clone(),
                            font_size: computed_style.text.font_size,
                            text_color: computed_style.text.color,
                            focused: is.focused,
                            sel_start: is.selection.start(),
                            sel_end: is.selection.end(),
                            cursor_pos: is.selection.active,
                            scroll_offset: is.scroll_offset,
                            scroll_offset_y: is.scroll_offset_y,
                            blink_visible: is.blink_visible(self.window_focused),
                            multiline: is.multiline,
                        });

                        let needs_hitbox = node.interactivity.needs_hitbox();
                        let is_scrollable = node.scroll_state.is_some();
                        let first_child = node.first_child;

                        (
                            taffy_node,
                            computed_style,
                            text,
                            input,
                            needs_hitbox,
                            is_scrollable,
                            first_child,
                        )
                    };
                    // immutable borrow of self.nodes is now dropped

                    let Ok(layout) = self.taffy.layout(taffy_node) else {
                        continue;
                    };

                    let x = parent_x + layout.location.x as f64;
                    let y = parent_y + layout.location.y as f64;
                    let w = layout.size.width as f64;
                    let h = layout.size.height as f64;

                    // Compute scroll info and clamp offset (mutable borrow is safe now)
                    let scroll_info = if is_scrollable {
                        let content_height = layout.content_size.height;
                        let visible_height = layout.size.height;
                        let max_scroll = (content_height - visible_height).max(0.0);
                        if let Some(ss) = self.nodes[node_id].scroll_state.as_mut() {
                            if ss.scroll_offset_y > max_scroll {
                                ss.scroll_offset_y = max_scroll;
                            }
                        }
                        let clamped_offset = self.nodes[node_id]
                            .scroll_state
                            .as_ref()
                            .map(|ss| ss.scroll_offset_y)
                            .unwrap_or(0.0);
                        Some((content_height, visible_height, clamped_offset))
                    } else {
                        None
                    };

                    // Collect children in order
                    let mut children = Vec::new();
                    let mut child = first_child;
                    while let Some(child_id) = child {
                        children.push(child_id);
                        child = self.nodes[child_id].next_sibling;
                    }

                    if let Some((content_height, visible_height, clamped_offset)) = scroll_info {
                        let overflows = content_height > visible_height;
                        let thumb_hovered = self
                            .scroll_drag
                            .as_ref()
                            .map_or(false, |d| d.node_id == node_id)
                            || self.scroll_thumbs.iter().any(|t| {
                                t.node_id == node_id
                                    && self
                                        .hit_state
                                        .mouse_position
                                        .map_or(false, |(mx, my)| t.thumb_bounds.contains(mx, my))
                            });

                        // Push in reverse order for LIFO stack:
                        // 6. PaintThumb (last to execute)
                        if overflows {
                            stack.push(StackItem::PaintThumb(ThumbInfo {
                                node_id,
                                view_x: x,
                                view_y: y,
                                view_w: w,
                                view_h: h,
                                scroll_offset_y: clamped_offset,
                                content_height,
                                visible_height,
                                thumb_hovered,
                                scale,
                            }));
                        }
                        // 5. PopClip
                        stack.push(StackItem::PopClip);
                        // 4-3. Children (reversed for correct order)
                        for &child_id in children.iter().rev() {
                            stack.push(StackItem::Visit(child_id, x, y - clamped_offset as f64));
                        }
                        // 2. PushClip
                        let clip_rect = Rect::new(x, y, x + w, y + h);
                        stack.push(StackItem::PushClip(clip_rect, scale));
                    } else {
                        // Normal (non-scrollable) node: push children
                        for &child_id in children.iter().rev() {
                            stack.push(StackItem::Visit(child_id, x, y));
                        }
                    }

                    // 1. PaintNode (always first — the node's own bg/borders)
                    render_list.push(RenderCommand::PaintNode(RenderInfo {
                        node_id,
                        x,
                        y,
                        w,
                        h,
                        style: computed_style,
                        text,
                        needs_hitbox,
                        input,
                    }));
                }
            }
        }

        // Paint all commands in order
        for cmd in &render_list {
            match cmd {
                RenderCommand::PaintNode(info) => {
                    let bounds = Bounds::new(info.x, info.y, info.w, info.h);

                    // Register hitbox if needed
                    if info.needs_hitbox {
                        let hitbox_id = self.hitbox_store.insert(info.node_id, bounds);
                        self.nodes[info.node_id].interactivity.hitbox_id = Some(hitbox_id);
                    }

                    if let Some(input_info) = &info.input {
                        crate::elements::input::paint_input(
                            scene,
                            text_renderer,
                            bounds,
                            &info.style,
                            input_info,
                            scale,
                        );
                    } else if let Some((content, font_size, color)) = &info.text {
                        crate::elements::text::paint_text(
                            scene,
                            text_renderer,
                            bounds,
                            &info.style,
                            content,
                            *font_size,
                            *color,
                            scale,
                        );
                    } else {
                        crate::elements::view::paint_view(
                            scene,
                            bounds,
                            &info.style,
                            scale,
                            |_| {},
                        );
                    }
                }
                RenderCommand::PushClip(rect, s) => {
                    scene.push_clip_layer(Fill::NonZero, Affine::scale(*s), rect);
                }
                RenderCommand::PopClip => {
                    scene.pop_layer();
                }
                RenderCommand::PaintThumb(thumb) => {
                    // Scrollbar thumb: 4px wide, 4px margin from right edge
                    let thumb_width = 4.0;
                    let thumb_margin = 4.0;

                    let ratio = thumb.visible_height as f64 / thumb.content_height as f64;
                    let thumb_height = (thumb.view_h * ratio).max(24.0);
                    let track_height = thumb.view_h;
                    let max_scroll = (thumb.content_height - thumb.visible_height).max(0.0) as f64;
                    let scroll_ratio = if max_scroll > 0.0 {
                        thumb.scroll_offset_y as f64 / max_scroll
                    } else {
                        0.0
                    };
                    let thumb_y = thumb.view_y + scroll_ratio * (track_height - thumb_height);
                    let thumb_x = thumb.view_x + thumb.view_w - thumb_width - thumb_margin;

                    let thumb_bounds = Bounds::new(thumb_x, thumb_y, thumb_width, thumb_height);

                    // Register for hit testing
                    self.scroll_thumbs.push(ScrollThumbRect {
                        node_id: thumb.node_id,
                        thumb_bounds,
                        view_bounds: Bounds::new(
                            thumb.view_x,
                            thumb.view_y,
                            thumb.view_w,
                            thumb.view_h,
                        ),
                        content_height: thumb.content_height,
                        visible_height: thumb.visible_height,
                    });

                    // Paint the thumb as a filled rounded rect
                    let alpha = if thumb.thumb_hovered { 140u8 } else { 90u8 };
                    let color = VelloColor::from_rgba8(255, 255, 255, alpha);
                    let radius = thumb_width / 2.0;
                    let rect = Rect::new(
                        thumb_x,
                        thumb_y,
                        thumb_x + thumb_width,
                        thumb_y + thumb_height,
                    );
                    let rounded =
                        RoundedRect::from_rect(rect, RoundedRectRadii::from_single_radius(radius));
                    scene.fill(
                        Fill::NonZero,
                        Affine::scale(thumb.scale),
                        color,
                        None,
                        &rounded,
                    );
                }
            }
        }
    }

    fn measure(
        text_renderer: &mut TextRenderer,
        known_dimensions: taffy::Size<Option<f32>>,
        available_space: taffy::Size<taffy::AvailableSpace>,
        node_context: Option<&mut NodeContext>,
    ) -> taffy::Size<f32> {
        let default_size = taffy::Size {
            width: known_dimensions.width.unwrap_or(0.0),
            height: known_dimensions.height.unwrap_or(0.0),
        };

        let Some(ctx) = node_context else {
            return default_size;
        };

        if ctx.is_input {
            return taffy::Size {
                width: known_dimensions
                    .width
                    .or_else(|| available_as_option(available_space.width))
                    .unwrap_or(200.0),
                height: known_dimensions
                    .height
                    .unwrap_or(ctx.font_size * 1.2 + 16.0),
            };
        }

        if let Some(text) = &ctx.text {
            let (measured_width, measured_height) = text_renderer.measure_text(
                &text.content,
                Attrs::new(),
                ctx.font_size,
                known_dimensions
                    .width
                    .or_else(|| available_as_option(available_space.width)),
                known_dimensions
                    .height
                    .or_else(|| available_as_option(available_space.height)),
            );

            return taffy::Size {
                width: measured_width,
                height: measured_height,
            };
        }

        default_size
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
}

fn available_as_option(space: taffy::AvailableSpace) -> Option<f32> {
    match space {
        taffy::AvailableSpace::Definite(v) => Some(v),
        _ => None,
    }
}
