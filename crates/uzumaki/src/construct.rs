//! Layout-tree construction. Walks the DOM each frame and computes
//! `layout_children` for every container, splicing synthetic anonymous
//! wrapper nodes around runs of inline-level children when those inline
//! children sit alongside block-level siblings.
//! adapted from https://github.com/DioxusLabs/blitz

use crate::element::{ElementNode, InlineTextEntry, TextLayout};
use crate::node::{Node, NodeData, NodeFlags, UzNodeId};
use crate::style::{Display, UzStyle};
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
        self.nodes.retain(|_, node| !node.flags.is_anonymous());

        for (_, node) in self.nodes.iter_mut() {
            *node.layout_children.get_mut() = node.children.clone();
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
        *self.nodes[node_id].layout_children.borrow_mut() = children.clone();
        if children.is_empty() {
            return;
        }

        let parent_display = self.nodes[node_id].computed_style().display;

        let mut has_inline_nodes = false;
        let mut has_block_nodes = false;

        for &cid in &children {
            let is_inline = self.nodes.get(cid).is_some_and(|n| n.is_inline_level());

            has_inline_nodes |= is_inline;
            has_block_nodes |= !is_inline;

            if has_inline_nodes && has_block_nodes {
                break;
            }
        }

        if !has_inline_nodes {
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

        if !has_block_nodes && parent_display != Display::Flex {
            self.set_inline_layout(node_id, self.collect_inline_layout(&children));
            // Inline-flow text descendants are represented by the parent's
            // Parley layout. They are not layout or paint children.
            self.nodes[node_id].flags.insert(NodeFlags::INLINE_ROOT);
            for &cid in &children {
                if let Some(child) = self.nodes.get_mut(cid) {
                    child.layout_parent = Some(node_id);
                }
            }
            self.nodes[node_id].layout_children.borrow_mut().clear();
            return;
        }

        // Wrap every contiguous run of inline children in an anonymous block.
        // The wrapper is measured and painted as one inline formatting context.
        // Flex containers only wrap bare text nodes, since flex items should
        // stay as independent flex children.
        let mut layout_children: Vec<UzNodeId> = Vec::with_capacity(children.len());
        let mut open_wrapper: Option<UzNodeId> = None;

        for cid in children {
            let Some(node) = self.nodes.get(cid) else {
                continue;
            };

            let should_wrap = if parent_display == Display::Flex {
                node.is_text_node()
            } else {
                node.is_inline_level()
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

        self.build_layout_children_recurse(&layout_children);
        *self.nodes[node_id].layout_children.borrow_mut() = layout_children;
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
        let mut node = Node::new(
            UzStyle {
                display: Display::Block,
                ..Default::default()
            },
            NodeData::AnonymousBlock(ElementNode::new_view()),
        );
        node.flags.insert(NodeFlags::ANONYMOUS);
        node.flags.insert(NodeFlags::INLINE_ROOT);
        node.layout_parent = Some(parent_id);
        // Anonymous wrappers participate in the DOM tree for traversal
        // (e.g. selection-root walks) but are never DOM children of their
        // parent — they only live in `layout_children`.
        node.parent = Some(parent_id);
        let id = self.nodes.insert(node);
        // Cascade inheritable styles (text, color, visibility,
        // text_selectable, cursor) from the parent's already-resolved
        // style. The main style cascade pass runs before construct and
        // walks DOM `children`, so it never visits these synthetic
        // wrappers — we have to seed their computed_style here or every
        // consumer that reads it would see uncascaded defaults.
        let parent_style = self.nodes[parent_id].computed_style().clone();
        self.nodes[id].compute_styles(false, false, false, Some(&parent_style));
        id
    }

    fn set_inline_layout(&mut self, node_id: UzNodeId, inline: TextLayout) {
        if let Some(element) = self.nodes[node_id].as_element_mut() {
            element.inline_layout = Some(Box::new(inline));
        }
    }

    fn collect_inline_layout(&self, children: &[UzNodeId]) -> TextLayout {
        let mut inline = TextLayout::default();
        self.collect_inline_text_into(children, &mut inline, None);
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

    fn collect_inline_text_into(
        &self,
        children: &[UzNodeId],
        inline: &mut TextLayout,
        style_owner: Option<UzNodeId>,
    ) {
        for &node_id in children {
            let Some(node) = self.nodes.get(node_id) else {
                continue;
            };
            match &node.data {
                NodeData::Text(text_node) => {
                    let owner = style_owner.unwrap_or(node_id);
                    self.push_inline_text(inline, owner, text_node.content.as_str());
                }
                NodeData::Element(el) if node.is_inline_level() => {
                    let content = el
                        .data
                        .get_text_content()
                        .map(|content| content.content.as_str());
                    if let Some(content) = content {
                        self.push_inline_text(inline, node_id, content);
                    }
                    let children = node.children.clone();
                    self.collect_inline_text_into(&children, inline, Some(node_id));
                }
                _ => {}
            }
        }
    }

    fn push_inline_text(&self, inline: &mut TextLayout, node_id: UzNodeId, content: &str) {
        if !content.is_empty() {
            if let Some(last) = inline.entries.last_mut()
                && last.node_id == node_id
                && last.byte_start + last.byte_len == inline.text.len()
            {
                inline.text.push_str(content);
                last.byte_len += content.len();
                return;
            }

            let byte_start = inline.text.len();
            inline.text.push_str(content);
            inline.entries.push(InlineTextEntry {
                node_id,
                byte_start,
                byte_len: content.len(),
            });
        }
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
        assert!(dom.nodes[parent].layout_children.borrow().is_empty());
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

        let layout_children = dom.nodes[parent].layout_children.borrow();
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

        let layout_children = dom.nodes[parent].layout_children.borrow();
        assert!(!dom.nodes[parent].flags.is_inline_root());
        assert!(dom.nodes[layout_children[0]].flags.is_anonymous());
        assert_eq!(layout_children[1], text_element);
    }

    #[test]
    fn nested_text_node_uses_inline_element_as_style_owner() {
        let mut dom = UIState::new();
        let parent = dom.create_view(UzStyle::default_for_element("view"));
        let text_element =
            dom.create_text_element(String::new(), UzStyle::default_for_element("text"));
        let raw_text = dom.create_text_node("styled".into(), UzStyle::default_for_element("#text"));

        dom.set_root(parent);
        dom.append_child(parent, text_element);
        dom.append_child(text_element, raw_text);
        dom.resolve_layout_children();

        let inline = dom.nodes[parent]
            .as_element()
            .and_then(|element| element.inline_layout.as_ref())
            .expect("parent should own inline layout");

        assert_eq!(inline.text, "styled");
        assert_eq!(inline.entries.len(), 1);
        assert_eq!(inline.entries[0].node_id, text_element);
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
        assert_eq!(
            dom.nodes[parent].layout_children.borrow().as_slice(),
            &[text]
        );
    }
}
