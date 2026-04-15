use std::cell::Cell;
use std::rc::Rc;

use unicode_segmentation::UnicodeSegmentation;

use crate::input::RangeProvider;
use crate::selection::{DomSelection, SelectionRange};

use super::{ElementTree, NodeId, TextRunEntry, TextSelectRun};

#[derive(Debug, Clone)]
pub struct SharedSelectionState {
    selection: Rc<Cell<Option<DomSelection>>>,
}

impl Default for SharedSelectionState {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedSelectionState {
    pub fn new() -> Self {
        Self {
            selection: Rc::new(Cell::new(None)),
        }
    }

    pub fn clear(&self) {
        self.selection.set(None);
    }

    pub fn range(&self) -> Option<SelectionRange> {
        self.selection.get().map(|s| s.range)
    }

    pub fn get(&self) -> Option<DomSelection> {
        self.selection.get()
    }

    pub fn is_empty(&self) -> bool {
        self.selection.get().is_none()
    }

    pub fn set(&self, selection: DomSelection) {
        self.selection.set(Some(selection));
    }
}

#[derive(Debug)]
pub struct DomRangeProvider {
    pub selection: SharedSelectionState,
}

impl RangeProvider for DomRangeProvider {
    fn get_range(&self) -> SelectionRange {
        self.selection.range().unwrap_or_default()
    }

    fn set_range(&mut self, range: SelectionRange) {
        if let Some(mut sel) = self.selection.get() {
            sel.range = range;
            self.selection.set(sel);
        }
    }
}

impl ElementTree {
    /// Build text runs for all textSelect subtrees. Called each frame before render.
    pub fn build_text_select_runs(&mut self) {
        self.selectable_text_runs.clear();
        let Some(root) = self.root else { return };

        // DFS: (node_id, parent_resolved_text_select, current_run_index_or_none)
        let mut stack: Vec<(NodeId, bool, Option<usize>)> = vec![(root, false, None)];

        while let Some((node_id, parent_ts, run_idx)) = stack.pop() {
            let node = &self.nodes[node_id];
            let resolved_ts = node.selectable.unwrap_or(parent_ts);

            // A node that explicitly enables textSelect when the parent scope
            // doesn't have it starts a new selection scope.
            let current_run = if resolved_ts && run_idx.is_none() {
                let idx = self.selectable_text_runs.len();
                self.selectable_text_runs.push(TextSelectRun {
                    root_id: node_id,
                    entries: Vec::new(),
                    flat_text: String::new(),
                    total_graphemes: 0,
                });
                Some(idx)
            } else if resolved_ts {
                run_idx
            } else {
                None
            };

            // Add text nodes to the current run
            if let Some(tc) = node.behavior.as_text()
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
                stack.push((cid, resolved_ts, current_run));
            }
        }
    }

    /// Get the currently selected text content (input or view).
    pub fn selected_text(&self) -> String {
        let Some(sel) = self.selection.get() else {
            return String::new();
        };

        if sel.is_collapsed() {
            return String::new();
        }
        // Input selection: delegate to InputState
        if let Some(node) = self.nodes.get(sel.root)
            && let Some(is) = node.behavior.as_input()
        {
            return is.selected_text();
        }
        // View text selection: look up in text_select_runs
        let Some(run) = self
            .selectable_text_runs
            .iter()
            .find(|r| r.root_id == sel.root)
        else {
            return String::new();
        };
        let start = sel.start();
        let end = sel.end();
        run.flat_text
            .graphemes(true)
            .skip(start)
            .take(end - start)
            .collect::<String>()
    }

    /// Get the current selection range as flat grapheme offsets.
    /// Returns (start, end) where start <= end.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        let sel = self.selection.get()?;
        if sel.is_collapsed() {
            return None;
        }
        Some((sel.start(), sel.end()))
    }

    /// Get the full selection state: root node, anchor, and active offsets.
    /// Useful for text editors that need to know the direction of selection.
    pub fn selection_state(&self) -> Option<(NodeId, usize, usize)> {
        let sel = self.selection.get()?;
        Some((sel.root, sel.anchor(), sel.active()))
    }

    /// Get the total grapheme count in the text run containing the current selection.
    /// For input selections, returns the input's grapheme count.
    pub fn selection_run_length(&self) -> Option<usize> {
        let sel = self.selection.get()?;
        // Input selection
        if let Some(node) = self.nodes.get(sel.root)
            && let Some(is) = node.behavior.as_input()
        {
            return Some(is.grapheme_count());
        }
        // View text selection
        let run = self
            .selectable_text_runs
            .iter()
            .find(|r| r.root_id == sel.root)?;
        Some(run.total_graphemes)
    }

    pub fn selection(&self) -> Option<DomSelection> {
        self.selection.get()
    }

    pub fn set_selection(&mut self, selection: DomSelection) {
        let root = selection.root;

        // If the target node is focusable (input, future: content-editable),
        // handle focus transfer automatically.
        let is_focusable = self
            .nodes
            .get(root)
            .map(|n| n.behavior.is_input())
            .unwrap_or(false);

        if is_focusable {
            if let Some(old_id) = self.focused_node
                && old_id != root
                && let Some(old_node) = self.nodes.get_mut(old_id)
                && let Some(is) = old_node.behavior.as_input_mut()
            {
                is.focused = false;
            }
            self.focused_node = Some(root);
            if let Some(node) = self.nodes.get_mut(root)
                && let Some(is) = node.behavior.as_input_mut()
            {
                is.focused = true;
                is.reset_blink();
            }
        }

        self.selection.set(selection);
    }

    /// Clear the selection.
    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }
}
