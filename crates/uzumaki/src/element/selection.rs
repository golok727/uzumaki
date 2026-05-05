use unicode_segmentation::UnicodeSegmentation;

use crate::selection::{Affinity, SelectionEndpoint, TextSelection};
use crate::ui::UIState;

use super::{TextRunEntry, TextSelectRun, UzNodeId};

impl UIState {
    fn grapheme_to_byte(text: &str, grapheme_index: usize) -> usize {
        text.graphemes(true)
            .take(grapheme_index)
            .map(str::len)
            .sum::<usize>()
            .min(text.len())
    }

    fn byte_to_grapheme(text: &str, byte_offset: usize) -> usize {
        text.graphemes(true)
            .scan(0, |offset, grapheme| {
                let start = *offset;
                *offset += grapheme.len();
                Some(start)
            })
            .take_while(|&start| start < byte_offset)
            .count()
    }

    /// Build text runs for all textSelect subtrees. Called each frame before render.
    pub fn build_text_select_runs(&mut self) {
        self.selectable_text_runs.clear();
        let Some(root) = self.root else { return };
        self.visit_text_select(root, None, None);
    }

    fn visit_text_select(
        &mut self,
        node_id: UzNodeId,
        parent_style: Option<&crate::style::UzStyle>,
        run_idx: Option<usize>,
    ) {
        let style = self.computed_style(node_id, parent_style);
        let resolved_text_sel = style.text_selectable.selectable();

        // A node that explicitly enables textSelect when the parent scope
        // doesn't have it starts a new selection scope.
        let current_run = if resolved_text_sel && run_idx.is_none() {
            let idx = self.selectable_text_runs.len();
            self.selectable_text_runs.push(TextSelectRun {
                root_id: node_id,
                entries: Vec::new(),
                flat_text: String::new(),
                total_graphemes: 0,
            });
            Some(idx)
        } else if resolved_text_sel {
            run_idx
        } else {
            None
        };

        if let Some(idx) = current_run
            && let Some(tc) = self.nodes[node_id].get_text_content()
        {
            let gc = tc.content.graphemes(true).count();
            let run = &mut self.selectable_text_runs[idx];
            run.entries.push(TextRunEntry {
                node_id,
                flat_start: run.total_graphemes,
                grapheme_count: gc,
            });
            run.flat_text.push_str(&tc.content);
            run.total_graphemes += gc;
        }

        let child_count = self.nodes[node_id].children.len();
        for i in 0..child_count {
            let cid = self.nodes[node_id].children[i];
            self.visit_text_select(cid, Some(&style), current_run);
        }
    }

    /// Get the currently selected text content. Checks focused input first,
    /// then the active view selection.
    pub fn selected_text(&self) -> String {
        if let Some(fid) = self.focused_node
            && let Some(node) = self.nodes.get(fid)
            && let Some(is) = node.as_text_input()
        {
            return is.selected_text();
        }

        if self.text_selection.is_collapsed() {
            return String::new();
        }
        let Some((start, end)) = self.ordered_text_selection() else {
            return String::new();
        };
        let Some(run) = self.find_run_for_node(start.node) else {
            return String::new();
        };

        let mut out = String::new();
        let mut in_range = false;
        for entry in &run.entries {
            if entry.node_id == start.node {
                in_range = true;
            }
            if !in_range {
                continue;
            }

            let Some(text) = self
                .nodes
                .get(entry.node_id)
                .and_then(|n| n.get_text_content())
            else {
                continue;
            };
            let local_start = if entry.node_id == start.node {
                start.offset.min(text.content.len())
            } else {
                0
            };
            let local_end = if entry.node_id == end.node {
                end.offset.min(text.content.len())
            } else {
                text.content.len()
            };
            if local_start < local_end {
                out.push_str(&text.content[local_start..local_end]);
            }
            if entry.node_id == end.node {
                break;
            }
        }
        out
    }

    /// Get the current selection range as flat grapheme offsets.
    /// Returns (start, end) where start <= end.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        let sel = self.get_selection()?;
        if sel.is_collapsed() {
            return None;
        }
        let (start, end) = self.selection_flat_range(&sel)?;
        Some((start, end))
    }

    /// Unified selection. Prefers the focused input; falls back to the view
    /// selection. Returns `None` if neither is set.
    pub fn get_selection(&self) -> Option<TextSelection> {
        if let Some(fid) = self.focused_node
            && let Some(node) = self.nodes.get(fid)
            && let Some(is) = node.as_text_input()
        {
            let sel = is.editor.raw_selection();
            return Some(TextSelection::new(
                SelectionEndpoint::new(fid, sel.anchor().index(), sel.anchor().affinity().into()),
                SelectionEndpoint::new(fid, sel.focus().index(), sel.focus().affinity().into()),
            ));
        }
        self.get_text_selection()
    }

    /// Active view selection, if any. Returns `None` if `root` is unset.
    pub fn get_text_selection(&self) -> Option<TextSelection> {
        self.text_selection.is_set().then_some(self.text_selection)
    }

    /// Set the active view selection. Clears any focused input.
    pub fn set_selection(&mut self, selection: TextSelection) {
        if selection.is_set() {
            self.focused_node = None;
        }
        self.text_selection = selection;
    }

    pub fn selection_root(&self, selection: &TextSelection) -> Option<UzNodeId> {
        let anchor = selection.anchor?;
        let focus = selection.focus?;
        let anchor_root = self.find_run_for_node(anchor.node)?.root_id;
        let focus_root = self.find_run_for_node(focus.node)?.root_id;
        (anchor_root == focus_root).then_some(anchor_root)
    }

    pub fn ordered_text_selection(&self) -> Option<(SelectionEndpoint, SelectionEndpoint)> {
        let (start, end) = self.text_selection.ordered_with(|a, b| {
            if a.node == b.node {
                return a.offset <= b.offset;
            }
            let Some(run) = self.find_run_for_node(a.node) else {
                return true;
            };
            let a_pos = run.entries.iter().position(|entry| entry.node_id == a.node);
            let b_pos = run.entries.iter().position(|entry| entry.node_id == b.node);
            a_pos <= b_pos
        })?;
        if self.selection_root(&self.text_selection).is_some() {
            Some((*start, *end))
        } else {
            None
        }
    }

    pub fn endpoint_from_flat_index(
        &self,
        root_id: UzNodeId,
        flat_index: usize,
        affinity: Affinity,
    ) -> Option<SelectionEndpoint> {
        let run = self
            .selectable_text_runs
            .iter()
            .find(|r| r.root_id == root_id)?;
        for entry in &run.entries {
            let entry_end = entry.flat_start + entry.grapheme_count;
            if flat_index <= entry_end {
                let local_grapheme = flat_index.saturating_sub(entry.flat_start);
                let text = self.nodes.get(entry.node_id)?.get_text_content()?;
                let offset = Self::grapheme_to_byte(&text.content, local_grapheme);
                return Some(SelectionEndpoint::new(entry.node_id, offset, affinity));
            }
        }
        let entry = run.entries.last()?;
        let text = self.nodes.get(entry.node_id)?.get_text_content()?;
        Some(SelectionEndpoint::new(
            entry.node_id,
            text.content.len(),
            affinity,
        ))
    }

    pub fn flat_index_for_endpoint(&self, endpoint: SelectionEndpoint) -> Option<usize> {
        let (_run, entry) = self.find_run_entry_for_node(endpoint.node)?;
        let text = self.nodes.get(endpoint.node)?.get_text_content()?;
        Some(entry.flat_start + Self::byte_to_grapheme(&text.content, endpoint.offset))
    }

    pub fn selection_flat_range(&self, selection: &TextSelection) -> Option<(usize, usize)> {
        let (start, end) = selection.ordered_with(|a, b| {
            if a.node == b.node {
                return a.offset <= b.offset;
            }
            let Some(run) = self.find_run_for_node(a.node) else {
                return true;
            };
            let a_pos = run.entries.iter().position(|entry| entry.node_id == a.node);
            let b_pos = run.entries.iter().position(|entry| entry.node_id == b.node);
            a_pos <= b_pos
        })?;
        Some((
            self.flat_index_for_endpoint(*start)?,
            self.flat_index_for_endpoint(*end)?,
        ))
    }

    /// Focus an element node. Clears any active view selection and blurs the
    /// previously focused input.
    pub fn focus_element(&mut self, node_id: UzNodeId) {
        self.text_selection.clear();
        self.focused_node = Some(node_id);
        if let Some(node) = self.nodes.get_mut(node_id)
            && let Some(is) = node.as_text_input_mut()
        {
            is.reset_blink();
        }
    }

    /// Clear the view selection (does not touch focused input).
    pub fn clear_selection(&mut self) {
        self.text_selection.clear();
    }

    /// Walk the DOM in document order from `start_id`, returning the next node
    /// for which `filter` returns true. Wraps to the root once and stops if the
    /// traversal returns to `start_id` without a match.
    pub fn next_node(
        &self,
        start_id: UzNodeId,
        mut filter: impl FnMut(&super::Node) -> bool,
    ) -> Option<UzNodeId> {
        let mut node_id = start_id;
        let mut look_in_children = true;
        loop {
            let cur = self.nodes.get(node_id)?;
            let next_id = if look_in_children && let Some(first) = cur.children.first().copied() {
                first
            } else if let Some(parent_id) = cur.parent {
                if let Some(sibling) = self.next_sibling(node_id) {
                    look_in_children = true;
                    sibling
                } else {
                    look_in_children = false;
                    node_id = parent_id;
                    continue;
                }
            } else {
                look_in_children = true;
                self.root?
            };

            let next = self.nodes.get(next_id)?;
            if filter(next) {
                return Some(next_id);
            }
            if next_id == start_id {
                return None;
            }
            node_id = next_id;
        }
    }

    /// Walk the DOM in reverse document order. At each step go to the previous
    /// sibling's deepest-last descendant, or up to the parent. Wraps to the
    /// deepest-last descendant of root.
    pub fn prev_node(
        &self,
        start_id: UzNodeId,
        mut filter: impl FnMut(&super::Node) -> bool,
    ) -> Option<UzNodeId> {
        let mut node_id = start_id;
        loop {
            let cur = self.nodes.get(node_id)?;
            let next_id = if let Some(prev) = self.prev_sibling(node_id) {
                self.deepest_last(prev)
            } else if let Some(parent) = cur.parent {
                parent
            } else {
                self.deepest_last(self.root?)
            };

            let next = self.nodes.get(next_id)?;
            if filter(next) {
                return Some(next_id);
            }
            if next_id == start_id {
                return None;
            }
            node_id = next_id;
        }
    }

    fn deepest_last(&self, mut id: UzNodeId) -> UzNodeId {
        while let Some(last) = self.nodes.get(id).and_then(|n| n.children.last().copied()) {
            id = last;
        }
        id
    }

    /// Move focus to the next focusable element in document order.
    pub fn focus_next_node(&mut self) -> Option<FocusChange> {
        self.focus_step(false)
    }

    /// Move focus to the previous focusable element in document order.
    pub fn focus_prev_node(&mut self) -> Option<FocusChange> {
        self.focus_step(true)
    }

    fn focus_step(&mut self, backward: bool) -> Option<FocusChange> {
        let start_id = self.focused_node.or(self.root)?;
        let new_id = if backward {
            self.prev_node(start_id, |n| n.is_focusable())?
        } else {
            self.next_node(start_id, |n| n.is_focusable())?
        };

        let old = self.focused_node;
        if old == Some(new_id) {
            return None;
        }

        let is_input = self
            .nodes
            .get(new_id)
            .map(|n| n.is_text_input())
            .unwrap_or(false);
        if is_input {
            self.focus_element(new_id);
        } else {
            self.clear_selection();
            self.focused_node = Some(new_id);
        }

        Some(FocusChange { old, new: new_id })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FocusChange {
    pub old: Option<UzNodeId>,
    pub new: UzNodeId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{TextSelectable, UzStyle};

    fn selectable_style() -> UzStyle {
        UzStyle {
            text_selectable: TextSelectable::True,
            ..Default::default()
        }
    }

    #[test]
    fn selected_text_spans_sibling_text_nodes_with_byte_offsets() {
        let mut dom = UIState::new();
        let root = dom.create_view(selectable_style());
        let first = dom.create_text_element("hello".into(), Default::default());
        let second = dom.create_text_element(" world".into(), Default::default());
        dom.set_root(root);
        dom.append_child(root, first);
        dom.append_child(root, second);
        dom.build_text_select_runs();

        dom.set_selection(TextSelection::new(
            SelectionEndpoint::new(first, 2, Affinity::Downstream),
            SelectionEndpoint::new(second, 4, Affinity::Upstream),
        ));

        assert_eq!(dom.selected_text(), "llo wor");
    }

    #[test]
    fn removing_node_that_holds_endpoint_clears_selection() {
        let mut dom = UIState::new();
        let root = dom.create_view(selectable_style());
        let first = dom.create_text_element("hello".into(), Default::default());
        let second = dom.create_text_element(" world".into(), Default::default());
        dom.set_root(root);
        dom.append_child(root, first);
        dom.append_child(root, second);
        dom.build_text_select_runs();
        dom.set_selection(TextSelection::new(
            SelectionEndpoint::new(first, 1, Affinity::Downstream),
            SelectionEndpoint::new(second, 2, Affinity::Upstream),
        ));

        dom.remove_child(root, second);

        assert!(dom.get_text_selection().is_none());
    }
}
