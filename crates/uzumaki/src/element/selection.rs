use unicode_segmentation::UnicodeSegmentation;

use crate::selection::TextSelection;
use crate::ui::UIState;

use super::{TextRunEntry, TextSelectRun, UzNodeId};

impl UIState {
    /// Build text runs for all textSelect subtrees. Called each frame before render.
    pub fn build_text_select_runs(&mut self) {
        self.selectable_text_runs.clear();
        let Some(root) = self.root else { return };

        // DFS: (node_id, parent_style, current_run_index_or_none)
        let mut stack = vec![(root, None, None)];

        while let Some((node_id, parent_style, run_idx)) = stack.pop() {
            let node = &self.nodes[node_id];
            let style = self.computed_style(node_id, parent_style.as_deref());
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

            // Add text nodes to the current run
            if let Some(tc) = node.as_text_node()
                && let Some(idx) = current_run
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

            // Push children in reverse order for correct DFS traversal
            let mut children = Vec::new();
            let mut child = node.first_child;
            while let Some(cid) = child {
                children.push(cid);
                child = self.nodes[cid].next_sibling;
            }
            for &cid in children.iter().rev() {
                stack.push((cid, Some(Box::new(style.clone())), current_run));
            }
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

        let Some(root) = self.text_selection.root else {
            return String::new();
        };
        if self.text_selection.is_collapsed() {
            return String::new();
        }
        let Some(run) = self.selectable_text_runs.iter().find(|r| r.root_id == root) else {
            return String::new();
        };
        let start = self.text_selection.start();
        let end = self.text_selection.end();
        run.flat_text
            .graphemes(true)
            .skip(start)
            .take(end - start)
            .collect::<String>()
    }

    /// Get the current selection range as flat grapheme offsets.
    /// Returns (start, end) where start <= end.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        let sel = self.get_selection()?;
        if sel.is_collapsed() {
            return None;
        }
        Some((sel.start(), sel.end()))
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
                fid,
                sel.anchor().index(),
                sel.focus().index(),
            ));
        }
        self.get_text_selection()
    }

    /// Get the total grapheme count in the target containing the current selection.
    pub(crate) fn selection_run_length(&self) -> Option<usize> {
        if let Some(fid) = self.focused_node
            && let Some(node) = self.nodes.get(fid)
            && let Some(is) = node.as_text_input()
        {
            return Some(is.text().len());
        }
        let root = self.text_selection.root?;
        let run = self
            .selectable_text_runs
            .iter()
            .find(|r| r.root_id == root)?;
        Some(run.total_graphemes)
    }

    /// Active view selection, if any. Returns `None` if `root` is unset.
    pub fn get_text_selection(&self) -> Option<TextSelection> {
        self.text_selection.root.map(|_| self.text_selection)
    }

    /// Set the active view selection. Clears any focused input.
    pub fn set_selection(&mut self, selection: TextSelection) {
        if selection.root.is_some()
            && let Some(old_id) = self.focused_node.take()
            && let Some(old_node) = self.nodes.get_mut(old_id)
            && let Some(is) = old_node.as_text_input_mut()
        {
            is.focused = false;
        }
        self.text_selection = selection;
    }

    /// Focus an input node. Clears any active view selection and blurs the
    /// previously focused input.
    pub fn focus_input(&mut self, node_id: UzNodeId) {
        self.text_selection.clear();
        if let Some(old_id) = self.focused_node
            && old_id != node_id
            && let Some(old_node) = self.nodes.get_mut(old_id)
            && let Some(is) = old_node.as_text_input_mut()
        {
            is.focused = false;
        }
        self.focused_node = Some(node_id);
        if let Some(node) = self.nodes.get_mut(node_id)
            && let Some(is) = node.as_text_input_mut()
        {
            is.focused = true;
            is.reset_blink();
        }
    }

    /// Clear the view selection (does not touch focused input).
    pub fn clear_selection(&mut self) {
        self.text_selection.clear();
    }
}
