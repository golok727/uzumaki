//! Layout-tree construction. Walks the DOM each frame and computes
//! `layout_children` for every container, splicing synthetic anonymous
//! wrapper nodes around runs of inline-level children when those inline
//! children sit alongside block-level siblings.
//! adapted from https://github.com/DioxusLabs/blitz

use crate::element::{ElementNode, InlineTextEntry, TextLayout};
use crate::node::{Node, NodeData, NodeFlags, UzNodeId};
use crate::style::Display;
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
            .filter(|(_, n)| n.flags.is_anonymous())
            .map(|(id, _)| id)
            .collect();

        for id in &anon_ids {
            self.nodes.remove(*id);
        }

        for (_, node) in self.nodes.iter_mut() {
            node.layout_children = None;
            node.layout_parent = node.parent;
            if let Some(element) = node.as_element_mut() {
                element.inline_layout = None;
            }
            node.flags.reset_construction_flags();
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

        let parent_display = self.nodes[node_id].style.display;
        let kinds: Vec<bool> = children
            .iter()
            .map(|&c| self.nodes.get(c).is_some_and(|n| n.is_inline_level()))
            .collect();

        let any_inline = kinds.iter().any(|&v| v);
        let any_block = kinds.iter().any(|&v| !v);

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

        if children.len() == 1
            && self
                .nodes
                .get(children[0])
                .is_some_and(|node| node.is_text_element())
        {
            let child_id = children[0];
            if let Some(child) = self.nodes.get_mut(child_id) {
                child.layout_parent = Some(node_id);
            }
            self.build_layout_children(child_id);
            return;
        }

        if !any_block && parent_display != Display::Flex {
            self.set_inline_layout(node_id, self.collect_inline_layout(&children));
            self.nodes[node_id].layout_children = Some(Vec::new());
            self.nodes[node_id].flags.insert(NodeFlags::INLINE_ROOT);
            for &cid in &children {
                if let Some(child) = self.nodes.get_mut(cid) {
                    child.layout_parent = Some(node_id);
                }
            }
            return;
        }

        // Wrap every contiguous run of inline children in an anonymous block.
        // The wrapper is measured and painted as one inline formatting context.
        // Flex containers only wrap bare text nodes, since flex items should
        // stay as independent flex children.
        let mut layout_children: Vec<UzNodeId> = Vec::with_capacity(children.len());
        let mut open_wrapper: Option<UzNodeId> = None;

        for (i, &cid) in children.iter().enumerate() {
            let should_wrap = if parent_display == Display::Flex {
                self.nodes.get(cid).is_some_and(|n| n.is_text_node())
            } else {
                kinds[i]
            };

            if should_wrap {
                let wrapper_id = match open_wrapper {
                    Some(id) => id,
                    None => {
                        let wrapper = self.create_anonymous_block(node_id);
                        layout_children.push(wrapper);
                        open_wrapper = Some(wrapper);
                        wrapper
                    }
                };
                if let Some(child) = self.nodes.get_mut(cid) {
                    child.layout_parent = Some(wrapper_id);
                }
                self.nodes[wrapper_id].children.push(cid);
                self.append_inline_layout(wrapper_id, cid);
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
            let is_anon = self.nodes.get(id).is_some_and(Node::is_anonymous);
            if !is_anon {
                self.build_layout_children(id);
            }
        }
    }

    fn create_anonymous_block(&mut self, parent_id: UzNodeId) -> UzNodeId {
        let style = anonymous_block_style(&self.nodes[parent_id].style);
        let mut node = Node::new(style, NodeData::AnonymousBlock(ElementNode::new_view()));
        node.flags.insert(NodeFlags::ANONYMOUS);
        node.flags.insert(NodeFlags::INLINE_ROOT);
        node.layout_parent = Some(parent_id);
        node.parent = None;
        self.nodes.insert(node)
    }

    fn set_inline_layout(&mut self, node_id: UzNodeId, inline: TextLayout) {
        if let Some(element) = self.nodes[node_id].as_element_mut() {
            element.inline_layout = Some(Box::new(inline));
        }
    }

    fn collect_inline_layout(&self, children: &[UzNodeId]) -> TextLayout {
        let mut inline = TextLayout::default();
        self.collect_inline_text_into(children, &mut inline);
        inline
    }

    fn append_inline_layout(&mut self, wrapper_id: UzNodeId, child_id: UzNodeId) {
        let mut next = self.collect_inline_layout(&[child_id]);
        let Some(element) = self.nodes[wrapper_id].as_element_mut() else {
            return;
        };
        let inline = element
            .inline_layout
            .get_or_insert_with(|| Box::new(TextLayout::default()));
        let offset = inline.text.len();
        inline.text.push_str(&next.text);
        inline
            .entries
            .extend(next.entries.drain(..).map(|mut entry| {
                entry.byte_start += offset;
                entry
            }));
    }

    fn collect_inline_text_into(&self, children: &[UzNodeId], inline: &mut TextLayout) {
        for &node_id in children {
            let Some(node) = self.nodes.get(node_id) else {
                continue;
            };
            if let Some(text) = node.get_text_content() {
                let byte_start = inline.text.len();
                inline.text.push_str(&text.content);
                inline.entries.push(InlineTextEntry {
                    node_id,
                    byte_start,
                    byte_len: text.content.len(),
                });
                continue;
            }
            if node.is_inline_level() {
                self.collect_inline_text_into(&node.children, inline);
            }
        }
    }
}

fn anonymous_block_style(parent: &crate::style::UzStyle) -> crate::style::UzStyle {
    use crate::style::UzStyle;
    UzStyle {
        display: Display::Block,
        text: parent.text.clone(),
        ..UzStyle::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::UzStyle;

    #[test]
    fn all_inline_children_make_parent_inline_root() {
        let mut dom = UIState::new();
        let parent = dom.create_view(UzStyle::default_for_element("view"));
        let first = dom.create_text_node("hello ".into(), UzStyle::default_for_element("#text"));
        let second = dom.create_text_element("world".into(), UzStyle::default_for_element("text"));

        dom.set_root(parent);
        dom.append_child(parent, first);
        dom.append_child(parent, second);
        dom.resolve_layout_children();

        assert!(dom.nodes[parent].flags.is_inline_root());
        assert_eq!(dom.nodes[parent].layout_children.as_deref(), Some(&[][..]));
        assert_eq!(
            dom.nodes[parent]
                .as_element()
                .and_then(|element| element.inline_layout.as_ref())
                .as_ref()
                .map(|text| text.text.as_str()),
            Some("hello world")
        );
    }

    #[test]
    fn mixed_inline_and_block_children_create_anonymous_block_runs() {
        let mut dom = UIState::new();
        let parent = dom.create_view(UzStyle::default_for_element("view"));
        let first = dom.create_text_node("hello".into(), UzStyle::default_for_element("#text"));
        let block = dom.create_view(UzStyle::default_for_element("view"));
        let second = dom.create_text_node("world".into(), UzStyle::default_for_element("#text"));

        dom.set_root(parent);
        dom.append_child(parent, first);
        dom.append_child(parent, block);
        dom.append_child(parent, second);
        dom.resolve_layout_children();

        let layout_children = dom.nodes[parent].layout_children.as_ref().unwrap();
        assert_eq!(layout_children.len(), 3);
        assert!(dom.nodes[layout_children[0]].flags.is_anonymous());
        assert_eq!(layout_children[1], block);
        assert!(dom.nodes[layout_children[2]].flags.is_anonymous());
    }

    #[test]
    fn flex_parent_does_not_become_inline_root() {
        let mut dom = UIState::new();
        let mut flex = UzStyle::default_for_element("view");
        flex.display = Display::Flex;
        let parent = dom.create_view(flex);
        let bare_text = dom.create_text_node("hello".into(), UzStyle::default_for_element("#text"));
        let text_element =
            dom.create_text_element("world".into(), UzStyle::default_for_element("text"));

        dom.set_root(parent);
        dom.append_child(parent, bare_text);
        dom.append_child(parent, text_element);
        dom.resolve_layout_children();

        let layout_children = dom.nodes[parent].layout_children.as_ref().unwrap();
        assert!(!dom.nodes[parent].flags.is_inline_root());
        assert!(dom.nodes[layout_children[0]].flags.is_anonymous());
        assert_eq!(layout_children[1], text_element);
    }

    #[test]
    fn single_text_element_stays_as_its_own_layout_box() {
        let mut dom = UIState::new();
        let parent = dom.create_view(UzStyle::default_for_element("view"));
        let text = dom.create_text_element("aligned".into(), UzStyle::default_for_element("text"));

        dom.set_root(parent);
        dom.append_child(parent, text);
        dom.resolve_layout_children();

        assert!(!dom.nodes[parent].flags.is_inline_root());
        assert_eq!(dom.nodes[text].layout_parent, Some(parent));
        assert!(dom.nodes[parent].layout_children.is_none());
    }
}
