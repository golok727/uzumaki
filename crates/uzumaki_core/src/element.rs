use slotmap::{new_key_type, SlotMap};
use vello::kurbo::{Affine, Rect, RoundedRect, Stroke};
use vello::peniko::Color;
use vello::Scene;

new_key_type! { pub struct NodeId; }

#[derive(Clone, Debug)]
pub struct ViewProps {
    pub background_color: Color,
    pub border_radius: f64,
    pub border_color: Color,
    pub border_width: f64,
}

impl Default for ViewProps {
    fn default() -> Self {
        Self {
            background_color: Color::TRANSPARENT,
            border_radius: 0.0,
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TextProps {
    pub content: String,
    pub font_size: f32,
    pub color: Color,
}

#[derive(Clone, Debug)]
pub enum Element {
    Root,
    View(ViewProps),
    Text(TextProps),
}

pub struct Node {
    pub parent: Option<NodeId>,
    pub first_child: Option<NodeId>,
    pub last_child: Option<NodeId>,
    pub next_sibling: Option<NodeId>,
    pub prev_sibling: Option<NodeId>,
    pub taffy_node: taffy::NodeId,
    pub element: Element,
}

pub struct Dom {
    pub nodes: SlotMap<NodeId, Node>,
    pub taffy: taffy::TaffyTree,
    pub root: Option<NodeId>,
}

impl Dom {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::with_key(),
            taffy: taffy::TaffyTree::new(),
            root: None,
        }
    }

    pub fn create_element(&mut self, element: Element, style: taffy::Style) -> NodeId {
        let taffy_node = self.taffy.new_leaf(style).unwrap();
        self.nodes.insert(Node {
            parent: None,
            first_child: None,
            last_child: None,
            next_sibling: None,
            prev_sibling: None,
            taffy_node,
            element,
        })
    }

    pub fn set_root(&mut self, node_id: NodeId) {
        self.root = Some(node_id);
    }

    pub fn append_child(&mut self, parent_id: NodeId, child_id: NodeId) {
        // Sync taffy tree
        let parent_taffy = self.nodes[parent_id].taffy_node;
        let child_taffy = self.nodes[child_id].taffy_node;
        self.taffy.add_child(parent_taffy, child_taffy).unwrap();

        // Update linked list
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
        // Sync taffy tree
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

        // Update linked list
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
        // Sync taffy tree
        let parent_taffy = self.nodes[parent_id].taffy_node;
        let child_taffy = self.nodes[child_id].taffy_node;
        self.taffy
            .remove_child(parent_taffy, child_taffy)
            .unwrap();

        // Update linked list
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

    pub fn compute_layout(&mut self, width: f32, height: f32) {
        if let Some(root) = self.root {
            let taffy_root = self.nodes[root].taffy_node;
            self.taffy
                .compute_layout(
                    taffy_root,
                    taffy::Size {
                        width: taffy::AvailableSpace::Definite(width),
                        height: taffy::AvailableSpace::Definite(height),
                    },
                )
                .unwrap();
        }
    }

    pub fn render(&self, scene: &mut Scene) {
        if let Some(root) = self.root {
            self.render_node(scene, root, 0.0, 0.0);
        }
    }

    fn render_node(&self, scene: &mut Scene, node_id: NodeId, parent_x: f64, parent_y: f64) {
        let node = &self.nodes[node_id];
        let Ok(layout) = self.taffy.layout(node.taffy_node) else {
            return;
        };

        let x = parent_x + layout.location.x as f64;
        let y = parent_y + layout.location.y as f64;
        let w = layout.size.width as f64;
        let h = layout.size.height as f64;

        match &node.element {
            Element::View(props) => {
                if props.border_radius > 0.0 {
                    let shape =
                        RoundedRect::from_rect(Rect::new(x, y, x + w, y + h), props.border_radius);
                    scene.fill(
                        vello::peniko::Fill::NonZero,
                        Affine::IDENTITY,
                        props.background_color,
                        None,
                        &shape,
                    );
                    if props.border_width > 0.0 {
                        scene.stroke(
                            &Stroke::new(props.border_width),
                            Affine::IDENTITY,
                            props.border_color,
                            None,
                            &shape,
                        );
                    }
                } else {
                    let shape = Rect::new(x, y, x + w, y + h);
                    scene.fill(
                        vello::peniko::Fill::NonZero,
                        Affine::IDENTITY,
                        props.background_color,
                        None,
                        &shape,
                    );
                    if props.border_width > 0.0 {
                        scene.stroke(
                            &Stroke::new(props.border_width),
                            Affine::IDENTITY,
                            props.border_color,
                            None,
                            &shape,
                        );
                    }
                }
            }
            Element::Root => {}
            Element::Text(_) => {
                // TODO: text rendering with parley
            }
        }

        // Traverse children via linked list
        let mut child = node.first_child;
        while let Some(child_id) = child {
            self.render_node(scene, child_id, x, y);
            child = self.nodes[child_id].next_sibling;
        }
    }
}

// Helpers for uniform taffy geometry
fn length_rect(val: f32) -> taffy::Rect<taffy::LengthPercentage> {
    let v = taffy::LengthPercentage::length(val);
    taffy::Rect {
        left: v,
        right: v,
        top: v,
        bottom: v,
    }
}

fn length_size(val: f32) -> taffy::Size<taffy::LengthPercentage> {
    let v = taffy::LengthPercentage::length(val);
    taffy::Size {
        width: v,
        height: v,
    }
}

/// Builds a hardcoded demo UI tree: dashboard layout with header, sidebar, cards, footer.
pub fn build_demo_tree() -> Dom {
    use taffy::*;

    let mut dom = Dom::new();

    // Root
    let root = dom.create_element(
        Element::Root,
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            size: Size {
                width: Dimension::percent(1.0),
                height: Dimension::percent(1.0),
            },
            ..Default::default()
        },
    );
    dom.set_root(root);

    // Header
    let header = dom.create_element(
        Element::View(ViewProps {
            background_color: Color::from_rgba8(91, 33, 182, 255),
            ..Default::default()
        }),
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::Center),
            size: Size {
                width: Dimension::auto(),
                height: Dimension::length(56.0),
            },
            padding: length_rect(16.0),
            ..Default::default()
        },
    );
    dom.append_child(root, header);

    // Body
    let body = dom.create_element(
        Element::Root,
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            flex_grow: 1.0,
            padding: length_rect(12.0),
            gap: length_size(12.0),
            ..Default::default()
        },
    );
    dom.append_child(root, body);

    // Sidebar
    let sidebar = dom.create_element(
        Element::View(ViewProps {
            background_color: Color::from_rgba8(30, 27, 75, 255),
            border_radius: 8.0,
            ..Default::default()
        }),
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            size: Size {
                width: Dimension::length(200.0),
                height: Dimension::auto(),
            },
            padding: length_rect(12.0),
            gap: length_size(8.0),
            ..Default::default()
        },
    );
    dom.append_child(body, sidebar);

    // Sidebar nav items
    for _ in 0..4 {
        let nav = dom.create_element(
            Element::View(ViewProps {
                background_color: Color::from_rgba8(67, 56, 202, 255),
                border_radius: 6.0,
                ..Default::default()
            }),
            Style {
                size: Size {
                    width: Dimension::auto(),
                    height: Dimension::length(40.0),
                },
                ..Default::default()
            },
        );
        dom.append_child(sidebar, nav);
    }

    // Main content area
    let main_area = dom.create_element(
        Element::Root,
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            gap: length_size(12.0),
            ..Default::default()
        },
    );
    dom.append_child(body, main_area);

    // Top card row
    let card_row = dom.create_element(
        Element::Root,
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            gap: length_size(12.0),
            size: Size {
                width: Dimension::auto(),
                height: Dimension::length(140.0),
            },
            ..Default::default()
        },
    );
    dom.append_child(main_area, card_row);

    // Three colored cards
    let card_colors = [
        Color::from_rgba8(220, 38, 38, 255),
        Color::from_rgba8(5, 150, 105, 255),
        Color::from_rgba8(37, 99, 235, 255),
    ];
    for color in card_colors {
        let card = dom.create_element(
            Element::View(ViewProps {
                background_color: color,
                border_radius: 8.0,
                ..Default::default()
            }),
            Style {
                flex_grow: 1.0,
                ..Default::default()
            },
        );
        dom.append_child(card_row, card);
    }

    // Bottom panel
    let bottom = dom.create_element(
        Element::View(ViewProps {
            background_color: Color::from_rgba8(31, 41, 55, 255),
            border_radius: 8.0,
            border_color: Color::from_rgba8(75, 85, 99, 255),
            border_width: 1.0,
        }),
        Style {
            flex_grow: 1.0,
            ..Default::default()
        },
    );
    dom.append_child(main_area, bottom);

    // Footer
    let footer = dom.create_element(
        Element::View(ViewProps {
            background_color: Color::from_rgba8(59, 7, 100, 255),
            ..Default::default()
        }),
        Style {
            size: Size {
                width: Dimension::auto(),
                height: Dimension::length(36.0),
            },
            ..Default::default()
        },
    );
    dom.append_child(root, footer);

    dom
}
