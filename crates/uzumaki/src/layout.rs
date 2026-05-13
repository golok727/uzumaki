use slab::Slab;

use crate::{
    node::{Node, ScrollAxis, UzNodeId},
    paint::render,
    style::Bounds,
    text::TextRenderer,
};

#[derive(Clone, Debug)]
pub struct NodeContext {
    // NOTE FOR LLMS: DONT ADD ANYTHING ELSE
    pub node_id: UzNodeId,
}

pub trait TaffyLayoutExt {
    fn border_box_bounds(&self) -> Bounds;
    fn content_box_bounds(&self) -> Bounds;
    fn axis_location(&self, axis: ScrollAxis) -> f32;
    fn axis_size(&self, axis: ScrollAxis) -> f32;
    fn axis_content_box_size(&self, axis: ScrollAxis) -> f32;
    fn axis_scroll_overflow(&self, axis: ScrollAxis) -> f32;
    fn axis_scroll_content_size(&self, axis: ScrollAxis) -> f32;
}

impl TaffyLayoutExt for taffy::Layout {
    fn border_box_bounds(&self) -> Bounds {
        Bounds::new(0.0, 0.0, self.size.width as f64, self.size.height as f64)
    }

    fn content_box_bounds(&self) -> Bounds {
        Bounds::new(
            (self.border.left + self.padding.left) as f64,
            (self.border.top + self.padding.top) as f64,
            (self.content_box_width() - self.scrollbar_size.width).max(0.0) as f64,
            (self.content_box_height() - self.scrollbar_size.height).max(0.0) as f64,
        )
    }

    fn axis_location(&self, axis: ScrollAxis) -> f32 {
        match axis {
            ScrollAxis::X => self.location.x,
            ScrollAxis::Y => self.location.y,
        }
    }

    fn axis_size(&self, axis: ScrollAxis) -> f32 {
        match axis {
            ScrollAxis::X => self.size.width,
            ScrollAxis::Y => self.size.height,
        }
    }

    fn axis_content_box_size(&self, axis: ScrollAxis) -> f32 {
        match axis {
            ScrollAxis::X => self.content_box_bounds().width as f32,
            ScrollAxis::Y => self.content_box_bounds().height as f32,
        }
    }

    fn axis_scroll_overflow(&self, axis: ScrollAxis) -> f32 {
        match axis {
            ScrollAxis::X => self.scroll_width(),
            ScrollAxis::Y => self.scroll_height(),
        }
    }

    fn axis_scroll_content_size(&self, axis: ScrollAxis) -> f32 {
        self.axis_content_box_size(axis) + self.axis_scroll_overflow(axis)
    }
}

pub struct LayoutEngine {
    pub taffy: taffy::TaffyTree<NodeContext>,
    root: Option<taffy::NodeId>,
    node_to_taffy: Vec<Option<taffy::NodeId>>,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            taffy: taffy::TaffyTree::new(),
            root: None,
            node_to_taffy: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.taffy.clear();
        self.root = None;
        self.node_to_taffy.clear();
    }

    pub fn taffy_node(&self, node_id: UzNodeId) -> Option<taffy::NodeId> {
        self.node_to_taffy.get(node_id).copied().flatten()
    }

    pub fn layout(&self, node_id: UzNodeId) -> Option<&taffy::Layout> {
        self.taffy_node(node_id)
            .and_then(|taffy_node| self.taffy.layout(taffy_node).ok())
    }

    fn set_taffy_node(&mut self, node_id: UzNodeId, taffy_node: taffy::NodeId) {
        if self.node_to_taffy.len() <= node_id {
            self.node_to_taffy.resize(node_id + 1, None);
        }
        self.node_to_taffy[node_id] = Some(taffy_node);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn compute_layout(
        &mut self,
        nodes: &Slab<Node>,
        root: Option<UzNodeId>,
        width: f32,
        height: f32,
        text_renderer: &mut TextRenderer,
    ) {
        self.clear();
        let Some(root) = root else { return };
        let Some(root_taffy) = self.build_node(nodes, root) else {
            return;
        };
        self.root = Some(root_taffy);

        self.taffy
            .compute_layout_with_measure(
                root_taffy,
                taffy::Size {
                    width: taffy::AvailableSpace::Definite(width),
                    height: taffy::AvailableSpace::Definite(height),
                },
                |known_dimensions, available_space, _node_id, node_context, _style| {
                    render::measure(
                        text_renderer,
                        nodes,
                        known_dimensions,
                        available_space,
                        node_context,
                    )
                },
            )
            .unwrap();
    }

    fn build_node(&mut self, nodes: &Slab<Node>, node_id: UzNodeId) -> Option<taffy::NodeId> {
        let node = nodes.get(node_id)?;
        let style = node.computed_style();

        let taffy_node = self.taffy.new_leaf(style.to_taffy()).unwrap();
        self.taffy
            .set_node_context(taffy_node, Some(NodeContext { node_id }))
            .unwrap();
        self.set_taffy_node(node_id, taffy_node);

        let layout_children = node.layout_children.borrow();
        for &child_id in layout_children.iter() {
            if let Some(taffy_child) = self.build_node(nodes, child_id) {
                self.taffy.add_child(taffy_node, taffy_child).unwrap();
            }
        }
        Some(taffy_node)
    }
}
