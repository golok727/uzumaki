use slab::Slab;

use crate::{
    element::{ImageMeasureInfo, Node, TextContent, UzNodeId, render},
    interactivity::HitTestState,
    style::{TextStyle, UzStyle},
    text::TextRenderer,
};

#[derive(Clone, Debug)]
pub struct NodeContext {
    pub dom_id: UzNodeId,
    pub text: Option<TextContent>,
    pub text_style: TextStyle,
    pub is_input: bool,
    pub image: Option<ImageMeasureInfo>,
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
        hit_state: &HitTestState,
        focused_node: Option<UzNodeId>,
        width: f32,
        height: f32,
        text_renderer: &mut TextRenderer,
    ) {
        self.clear();
        let Some(root) = root else { return };
        let Some(root_taffy) = self.build_node(nodes, root, None, hit_state, focused_node) else {
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
                        known_dimensions,
                        available_space,
                        node_context,
                    )
                },
            )
            .unwrap();
    }

    fn build_node(
        &mut self,
        nodes: &Slab<Node>,
        node_id: UzNodeId,
        parent_style: Option<&UzStyle>,
        hit_state: &HitTestState,
        focused_node: Option<UzNodeId>,
    ) -> Option<taffy::NodeId> {
        let node = nodes.get(node_id)?;
        let parent = parent_style.unwrap_or(&node.style);
        let style = node.interactivity.compute_style_inherited(
            &node.style,
            parent,
            node_id,
            hit_state,
            focused_node == Some(node_id),
        );

        let mut children = Vec::new();
        for &child_id in &node.children {
            if let Some(taffy_child) =
                self.build_node(nodes, child_id, Some(&style), hit_state, focused_node)
            {
                children.push(taffy_child);
            }
        }

        let context = NodeContext {
            dom_id: node_id,
            text: node.get_text_content().cloned(),
            text_style: style.text.clone(),
            is_input: node.is_text_input(),
            image: node.as_image().and_then(|image| {
                image
                    .data
                    .natural_size()
                    .map(|(width, height)| ImageMeasureInfo { width, height })
            }),
        };

        let taffy_node = if children.is_empty() {
            self.taffy.new_leaf(style.to_taffy()).unwrap()
        } else {
            self.taffy
                .new_with_children(style.to_taffy(), &children)
                .unwrap()
        };
        self.taffy
            .set_node_context(taffy_node, Some(context))
            .unwrap();
        self.set_taffy_node(node_id, taffy_node);
        Some(taffy_node)
    }
}
