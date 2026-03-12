use cosmic_text::Attrs;
use slotmap::{SlotMap, new_key_type};
use vello::Scene;

use crate::interactivity::{HitTestState, HitboxStore, Interactivity};
use crate::style::{Bounds, Color, Style};
use crate::text::TextRenderer;

new_key_type! {
    pub struct NodeId;
}

impl NodeId {
    pub fn to_string_id(self) -> String {
        self.0.as_ffi().to_string()
    }

    pub fn from_string_id(s: &str) -> Self {
        let ffi: u64 = s.parse().expect("invalid node id");
        Self(slotmap::KeyData::from_ffi(ffi))
    }
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
        let tc = TextContent {
            content: text,
        };
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
    pub fn render(&mut self, scene: &mut Scene, text_renderer: &mut TextRenderer) {
        self.hitbox_store.clear();

        if let Some(root) = self.root {
            self.render_tree(scene, text_renderer, root);
        }
    }

    fn render_tree(
        &mut self,
        scene: &mut Scene,
        text_renderer: &mut TextRenderer,
        root_id: NodeId,
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
                    info.style.paint(bounds, scene, |scene| {
                        text_renderer.draw_text(
                            scene,
                            content,
                            Attrs::new(),
                            *font_size,
                            info.w as f32,
                            info.h as f32,
                            (info.x as f32, info.y as f32),
                            color.to_vello(),
                        );
                    });
                }
                None => {
                    // View: paint bg + borders, children paint themselves in order
                    info.style.paint(bounds, scene, |_scene| {});
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

// ── Demo tree using the new Style system ─────────────────────────────

pub fn build_demo_tree() -> Dom {
    use crate::style::*;

    let mut dom = Dom::new();

    let base_bg = Color::rgb(15, 15, 15);
    let panel = Color::rgb(20, 20, 20);
    let border = Color::rgb(60, 60, 60);
    let text_color = Color::rgb(212, 212, 212);
    let subtext = Color::rgb(140, 140, 150);
    let accent_blue = Color::rgb(86, 156, 214);
    let accent_green = Color::rgb(102, 204, 153);
    let accent_orange = Color::rgb(206, 145, 120);
    let nav_active = Color::rgb(45, 45, 48);
    let hover_bg = Color::rgb(55, 55, 60);
    let active_bg = Color::rgb(65, 65, 70);

    // Root
    let root = dom.create_view(Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        size: Size {
            width: Length::Percent(1.0),
            height: Length::Percent(1.0),
        },
        background: Some(base_bg),
        ..Default::default()
    });
    dom.set_root(root);

    // Header
    let header = dom.create_view(Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        align_items: Some(AlignItems::Center),
        size: Size {
            width: Length::Auto,
            height: Length::Px(48.0),
        },
        padding: Edges::all(16.0),
        background: Some(panel),
        border_color: Some(border),
        border_widths: Edges::all(1.0),
        ..Default::default()
    });
    dom.append_child(root, header);

    let header_text = dom.create_text(
        "Uzumaki".to_string(),
        Style {
            flex_shrink: 0.0,
            text: TextStyle {
                font_size: 18.0,
                color: accent_blue,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    dom.append_child(header, header_text);

    // Body
    let body = dom.create_view(Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        flex_grow: 1.0,
        background: Some(base_bg),
        ..Default::default()
    });
    dom.append_child(root, body);

    // Sidebar
    let sidebar = dom.create_view(Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        size: Size {
            width: Length::Px(400.0),
            height: Length::Auto,
        },
        padding: Edges::all(12.0),
        gap: GapSize {
            width: DefiniteLength::Px(4.0),
            height: DefiniteLength::Px(4.0),
        },
        background: Some(panel),
        border_color: Some(border),
        border_widths: Edges {
            top: 0.0,
            right: 1.0,
            bottom: 0.0,
            left: 0.0,
        },
        ..Default::default()
    });
    dom.append_child(body, sidebar);

    // Sidebar nav items with hover/active
    let nav_labels = ["Dashboard", "Analytics", "Projects", "Settings"];
    for (i, label) in nav_labels.iter().enumerate() {
        let nav = dom.create_view(Style {
            display: Display::Flex,
            align_items: Some(AlignItems::Center),
            size: Size {
                width: Length::Auto,
                height: Length::Px(36.0),
            },
            padding: Edges::all(8.0),
            flex_shrink: 0.0,
            background: if i == 0 { Some(nav_active) } else { None },
            corner_radii: Corners::uniform(6.0),
            ..Default::default()
        });
        dom.append_child(sidebar, nav);

        // Add hover + active interactivity
        {
            let node = dom.get_node_mut(nav).unwrap();
            let label = label.to_string();
            node.interactivity.on_click(move |_, _| {
                println!("Clicked: {}", label);
            });
            node.interactivity.on_hover({
                let mut s = StyleRefinement::default();
                s.background = Some(hover_bg);
                s
            });
            node.interactivity.on_active({
                let mut s = StyleRefinement::default();
                s.background = Some(active_bg);
                s
            });
        }

        let nav_text = dom.create_text(
            label.to_string(),
            Style {
                text: TextStyle {
                    font_size: 20.0,
                    color: if i == 0 { text_color } else { subtext },
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        dom.append_child(nav, nav_text);
    }

    // Main content area
    let main_area = dom.create_view(Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        flex_grow: 1.0,
        padding: Edges::all(16.0),
        gap: GapSize {
            width: DefiniteLength::Px(16.0),
            height: DefiniteLength::Px(16.0),
        },
        ..Default::default()
    });
    dom.append_child(body, main_area);

    // Page title
    let page_title = dom.create_text(
        "Dashboard".to_string(),
        Style {
            text: TextStyle {
                font_size: 22.0,
                color: text_color,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    dom.append_child(main_area, page_title);

    // Card row
    let card_row = dom.create_view(Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        gap: GapSize {
            width: DefiniteLength::Px(12.0),
            height: DefiniteLength::Px(12.0),
        },
        size: Size {
            width: Length::Auto,
            height: Length::Px(100.0),
        },
        ..Default::default()
    });
    dom.append_child(main_area, card_row);

    // Metric cards with hover
    let cards = [
        ("Revenue", "$12,400", accent_blue),
        ("Users", "1,240", accent_green),
        ("Growth", "+24%", accent_orange),
    ];
    for (title, value, accent) in cards {
        let card = dom.create_view(Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            padding: Edges::all(16.0),
            gap: GapSize {
                width: DefiniteLength::Px(8.0),
                height: DefiniteLength::Px(8.0),
            },
            background: Some(panel),
            corner_radii: Corners::uniform(8.0),
            border_color: Some(border),
            border_widths: Edges::all(1.0),
            ..Default::default()
        });
        dom.append_child(card_row, card);

        {
            let node = &mut dom.nodes[card];
            node.interactivity.on_hover({
                let mut s = StyleRefinement::default();
                s.background = Some(hover_bg);
                s
            });
        }

        let card_title = dom.create_text(
            title.to_string(),
            Style {
                text: TextStyle {
                    font_size: 16.0,
                    color: subtext,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        dom.append_child(card, card_title);

        let card_value = dom.create_text(
            value.to_string(),
            Style {
                text: TextStyle {
                    font_size: 24.0,
                    color: accent,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        dom.append_child(card, card_value);
    }

    // Border radius samples
    let samples = dom.create_view(Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        gap: GapSize {
            width: DefiniteLength::Px(12.0),
            height: DefiniteLength::Px(12.0),
        },
        size: Size {
            width: Length::Auto,
            height: Length::Px(80.0),
        },
        ..Default::default()
    });
    dom.append_child(main_area, samples);

    let chip = dom.create_view(Style {
        display: Display::Flex,
        align_items: Some(AlignItems::Center),
        justify_content: Some(JustifyContent::Center),
        size: Size {
            width: Length::Px(180.0),
            height: Length::Percent(1.0),
        },
        background: Some(panel),
        border_color: Some(border),
        border_widths: Edges::all(2.0),
        corner_radii: Corners {
            top_left: 12.0,
            top_right: 4.0,
            bottom_right: 12.0,
            bottom_left: 4.0,
        },
        ..Default::default()
    });
    dom.append_child(samples, chip);

    let chip_text = dom.create_text(
        "Asymmetric corners".to_string(),
        Style {
            text: TextStyle {
                font_size: 14.0,
                color: text_color,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    dom.append_child(chip, chip_text);

    let pill = dom.create_view(Style {
        display: Display::Flex,
        align_items: Some(AlignItems::Center),
        justify_content: Some(JustifyContent::Center),
        size: Size {
            width: Length::Px(200.0),
            height: Length::Percent(1.0),
        },
        background: Some(panel),
        border_color: Some(accent_blue),
        border_widths: Edges::all(5.0),
        corner_radii: Corners {
            top_left: 20.0,
            top_right: 20.0,
            bottom_right: 6.0,
            bottom_left: 6.0,
        },
        ..Default::default()
    });
    dom.append_child(samples, pill);

    let pill_text = dom.create_text(
        "Edge-specific stroke".to_string(),
        Style {
            text: TextStyle {
                font_size: 16.0,
                color: accent_blue,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    dom.append_child(pill, pill_text);

    // Bottom panel
    let bottom = dom.create_view(Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        flex_grow: 1.0,
        padding: Edges::all(16.0),
        gap: GapSize {
            width: DefiniteLength::Px(8.0),
            height: DefiniteLength::Px(8.0),
        },
        background: Some(panel),
        corner_radii: Corners::uniform(8.0),
        border_color: Some(border),
        border_widths: Edges::all(1.0),
        ..Default::default()
    });
    dom.append_child(main_area, bottom);

    let panel_title = dom.create_text(
        "Recent Activity".to_string(),
        Style {
            text: TextStyle {
                font_size: 16.0,
                color: text_color,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    dom.append_child(bottom, panel_title);

    let panel_text = dom.create_text(
        "No recent activity to display.".to_string(),
        Style {
            text: TextStyle {
                font_size: 16.0,
                color: subtext,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    dom.append_child(bottom, panel_text);

    // Footer (keeping original demo intact)
    let footer = dom.create_view(Style {
        display: Display::Flex,
        align_items: Some(AlignItems::Center),
        size: Size {
            width: Length::Auto,
            height: Length::Px(32.0),
        },
        padding: Edges::all(16.0),
        background: Some(panel),
        border_color: Some(border),
        border_widths: Edges::all(1.0),
        ..Default::default()
    });
    dom.append_child(root, footer);

    let footer_text = dom.create_text(
        "Uzumaki v0.1.0".to_string(),
        Style {
            text: TextStyle {
                font_size: 16.0,
                color: subtext,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    dom.append_child(footer, footer_text);

    dom
}
