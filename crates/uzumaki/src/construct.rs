//! Layout-tree construction. Walks the DOM each frame and computes
//! `layout_children` for every container, splicing synthetic anonymous
//! wrapper nodes around runs of inline-level children when those inline
//! children sit alongside block-level siblings.
//!
//! Mirrors Blitz's `collect_layout_children`: anonymous wrappers are real
//! entries in `Slab<Node>` flagged `is_anonymous`, and they are torn down
//! and rebuilt every frame. They appear only in `Node::layout_children`,
//! never in the user-facing `Node::children` of the wrapped originals.

use crate::element::ElementNode;
use crate::node::{Node, UzNodeId};
use crate::ui::UIState;

impl UIState {
    /// Tear down anonymous wrappers from the previous frame and rebuild the
    /// layout tree. Must be called before `layout_engine.compute_layout`.
    pub fn resolve_layout_children(&mut self) {
        self.tear_down_anonymous_nodes();

        let Some(root) = self.root else { return };
        self.build_layout_children(root);
    }

    fn tear_down_anonymous_nodes(&mut self) {
        let anon_ids: Vec<UzNodeId> = self
            .nodes
            .iter()
            .filter(|(_, n)| n.is_anonymous)
            .map(|(id, _)| id)
            .collect();

        for id in &anon_ids {
            self.nodes.remove(*id);
        }

        for (_, node) in self.nodes.iter_mut() {
            node.layout_children = None;
            node.layout_parent = node.parent;
        }
    }

    fn build_layout_children(&mut self, node_id: UzNodeId) {
        let Some(node) = self.nodes.get(node_id) else {
            return;
        };
        let children = node.children.clone();
        if children.is_empty() {
            return;
        }

        let kinds: Vec<bool> = children
            .iter()
            .map(|&c| self.nodes.get(c).is_some_and(|n| n.is_inline_level()))
            .collect();

        let any_inline = kinds.iter().any(|&v| v);

        if !any_inline {
            // Pure block container: layout_children == children.
            for &cid in &children {
                if let Some(child) = self.nodes.get_mut(cid) {
                    child.layout_parent = Some(node_id);
                }
            }
            self.build_layout_children_recurse(&children);
            return;
        }

        // Wrap every contiguous run of inline children in an anonymous
        // flex-row box so they flow horizontally regardless of the parent's
        // display mode. Block children pass through unwrapped.
        let mut layout_children: Vec<UzNodeId> = Vec::with_capacity(children.len());
        let mut open_wrapper: Option<UzNodeId> = None;

        for (i, &cid) in children.iter().enumerate() {
            if kinds[i] {
                let wrapper_id = match open_wrapper {
                    Some(id) => id,
                    None => {
                        let wrapper = self.create_anonymous_inline_wrapper(node_id);
                        layout_children.push(wrapper);
                        open_wrapper = Some(wrapper);
                        wrapper
                    }
                };
                if let Some(child) = self.nodes.get_mut(cid) {
                    child.layout_parent = Some(wrapper_id);
                }
                self.nodes[wrapper_id].children.push(cid);
            } else {
                open_wrapper = None;
                if let Some(child) = self.nodes.get_mut(cid) {
                    child.layout_parent = Some(node_id);
                }
                layout_children.push(cid);
            }
        }

        self.nodes[node_id].layout_children = Some(layout_children.clone());
        self.build_layout_children_recurse(&layout_children);
    }

    fn build_layout_children_recurse(&mut self, ids: &[UzNodeId]) {
        for &id in ids {
            // Recurse into the original DOM children; anonymous wrappers'
            // own children are already inline leaves with no further layout
            // structure to construct.
            let is_anon = self.nodes.get(id).is_some_and(|n| n.is_anonymous);
            if !is_anon {
                self.build_layout_children(id);
            }
        }
    }

    fn create_anonymous_inline_wrapper(&mut self, parent_id: UzNodeId) -> UzNodeId {
        let style = anonymous_inline_style(&self.nodes[parent_id].style);
        let mut node = Node::new(style, ElementNode::new_anonymous());
        node.is_anonymous = true;
        node.layout_parent = Some(parent_id);
        node.parent = None; // anonymous nodes are layout-only; not in DOM tree
        self.nodes.insert(node)
    }
}

fn anonymous_inline_style(parent: &crate::style::UzStyle) -> crate::style::UzStyle {
    use crate::style::{Display, FlexDirection, UzStyle};
    UzStyle {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        // Inherit text style so children sized via measure_text get the right
        // metrics; everything else stays default.
        text: parent.text.clone(),
        ..UzStyle::default()
    }
}
