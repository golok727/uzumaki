use cosmic_text::Attrs;
use slotmap::{SlotMap, new_key_type};
use vello::Scene;

use crate::interactivity::{HitTestState, HitboxStore, Interactivity};
use crate::style::{Bounds, Color, Style};
use crate::text::TextRenderer;

new_key_type! {
    pub struct NodeId;
}

// ── Text content ─────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct TextContent {
    pub content: String,
}

// ── Element kind ─────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum ElementKind {
    /// Container element (div). Has visual style + children.
    View,
    /// Text leaf element.
    Text(TextContent),
}

// ── NodeContext for taffy ────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct NodeContext {
    pub dom_id: NodeId,
    pub text: Option<TextContent>,
    pub font_size: f32,
}

// ── Node ─────────────────────────────────────────────────────────────

pub struct Node {
    pub parent: Option<NodeId>,
    pub first_child: Option<NodeId>,
    pub last_child: Option<NodeId>,
    pub next_sibling: Option<NodeId>,
    pub prev_sibling: Option<NodeId>,
    pub taffy_node: taffy::NodeId,
    pub kind: ElementKind,
    /// The base style for this element. Converted to taffy for layout.
    pub style: Style,
    /// Interactivity: hover/active style overrides, hitbox, event listeners.
    pub interactivity: Interactivity,
}

// ── Dom ──────────────────────────────────────────────────────────────

pub struct Dom {
    pub nodes: SlotMap<NodeId, Node>,
    pub taffy: taffy::TaffyTree<NodeContext>,
    pub root: Option<NodeId>,
    /// Hitboxes registered during the last paint pass.
    pub hitbox_store: HitboxStore,
    /// Current hit test state (updated on mouse move).
    pub hit_state: HitTestState,
}

// Safety: Dom contains taffy's CompactLength which uses *const () as a tagged pointer
// for float storage. It never actually dereferences these pointers and is safe to send
// across threads. All other fields are Send+Sync.
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
        }
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
            kind: ElementKind::View,
            style,
            interactivity: Interactivity::new(),
        });
        self.taffy
            .set_node_context(
                taffy_node,
                Some(NodeContext {
                    dom_id: node_id,
                    text: None,
                    font_size: 16.0,
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
            kind: ElementKind::Text(text.clone()),
            style,
            interactivity: Interactivity::new(),
        });
        self.taffy
            .set_node_context(
                taffy_node,
                Some(NodeContext {
                    dom_id: node_id,
                    text: Some(text),
                    font_size,
                }),
            )
            .unwrap();
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
        node.kind = ElementKind::Text(tc.clone());
        let taffy_node = node.taffy_node;
        let font_size = node.style.text.font_size;
        self.taffy
            .set_node_context(
                taffy_node,
                Some(NodeContext {
                    dom_id: node_id,
                    text: Some(tc),
                    font_size,
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

    /// Render the DOM tree into the scene. Also rebuilds hitboxes.
    pub fn render(&mut self, scene: &mut Scene, text_renderer: &mut TextRenderer, scale: f64) {
        self.hitbox_store.clear();

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
        }

        let mut render_list: Vec<RenderInfo> = Vec::new();
        let mut stack: Vec<(NodeId, f64, f64)> = vec![(root_id, 0.0, 0.0)];

        while let Some((node_id, parent_x, parent_y)) = stack.pop() {
            let node = &self.nodes[node_id];
            let Ok(layout) = self.taffy.layout(node.taffy_node) else {
                continue;
            };

            let x = parent_x + layout.location.x as f64;
            let y = parent_y + layout.location.y as f64;
            let w = layout.size.width as f64;
            let h = layout.size.height as f64;

            let computed_style = node
                .interactivity
                .compute_style(&node.style, &self.hit_state);

            let text = match &node.kind {
                ElementKind::Text(tc) => Some((
                    tc.content.clone(),
                    computed_style.text.font_size,
                    computed_style.text.color,
                )),
                _ => None,
            };

            let needs_hitbox = node.interactivity.needs_hitbox();

            // Collect children in order, push in reverse for correct DFS
            let mut children = Vec::new();
            let mut child = node.first_child;
            while let Some(child_id) = child {
                children.push(child_id);
                child = self.nodes[child_id].next_sibling;
            }
            for &child_id in children.iter().rev() {
                stack.push((child_id, x, y));
            }

            render_list.push(RenderInfo {
                node_id,
                x,
                y,
                w,
                h,
                style: computed_style,
                text,
                needs_hitbox,
            });
        }

        // Paint all nodes in tree order
        for info in &render_list {
            let bounds = Bounds::new(info.x, info.y, info.w, info.h);

            // Register hitbox if needed
            if info.needs_hitbox {
                let hitbox_id = self.hitbox_store.insert(info.node_id, bounds);
                self.nodes[info.node_id].interactivity.hitbox_id = Some(hitbox_id);
            }

            match &info.text {
                Some((content, font_size, color)) => {
                    info.style.paint(bounds, scene, scale, |scene| {
                        text_renderer.draw_text(
                            scene,
                            content,
                            Attrs::new(),
                            *font_size,
                            info.w as f32,
                            info.h as f32,
                            (info.x as f32, info.y as f32),
                            color.to_vello(),
                            scale,
                        );
                    });
                }
                None => {
                    // View: paint bg + borders, children paint themselves in order
                    info.style.paint(bounds, scene, scale, |_scene| {});
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
