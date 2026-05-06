use std::collections::VecDeque;
use std::time::Instant;

use super::EditKind;

#[derive(Clone, Debug, Default)]
pub(super) struct SelectionSnapshot {
    pub(super) anchor_byte: usize,
    pub(super) focus_byte: usize,
}

#[derive(Clone, Debug)]
pub(super) struct ChangeItem {
    pub(super) start_byte: usize,
    pub(super) end_byte: usize,
    pub(super) text: String,
    pub(super) insert: bool,
}

#[derive(Clone, Debug, Default)]
pub(super) struct Change {
    pub(super) items: Vec<ChangeItem>,
    pub(super) before_selection: SelectionSnapshot,
    pub(super) after_selection: SelectionSnapshot,
}

/// Maximum number of undo entries to keep per input.
const MAX_HISTORY: usize = 100;

/// Consecutive edits within this window are merged into one undo batch.
const BATCH_TIMEOUT_MS: u128 = 500;

pub(super) struct EditHistory {
    pub(super) undo_stack: VecDeque<Change>,
    pub(super) redo_stack: Vec<Change>,
    /// Timestamp of the last edit (for time-based batch breaking).
    pub(super) last_edit_time: Option<Instant>,
}

impl EditHistory {
    pub(super) fn new() -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            last_edit_time: None,
        }
    }

    /// Determine whether the incoming edit should start a new undo batch.
    pub(super) fn should_start_new_batch(&self, kind: EditKind, inserted: Option<&str>) -> bool {
        // Non-batchable edits (paste, cut, word-delete, history) always start a new batch.
        if !kind.is_batchable() {
            return true;
        }

        // No previous edit means this is the first batch.
        let Some(last_edit_insert) = self
            .undo_stack
            .back()
            .and_then(|change| change.items.last())
            .map(|item| item.insert)
        else {
            return true;
        };

        let Some(last_time) = self.last_edit_time else {
            return true;
        };

        // Time gap exceeds threshold.
        if last_time.elapsed().as_millis() > BATCH_TIMEOUT_MS {
            return true;
        }

        // Switching between insert and delete batches.
        if last_edit_insert != kind.is_insert_batch() {
            return true;
        }

        // Newline insertion always starts a new batch.
        if kind == EditKind::Insert && inserted.is_some_and(|s| s.contains('\n')) {
            return true;
        }

        false
    }

    pub(super) fn push_with_inserted(
        &mut self,
        change: Change,
        kind: EditKind,
        inserted: Option<&str>,
    ) {
        if self.should_start_new_batch(kind, inserted) {
            self.undo_stack.push_back(change);
            if self.undo_stack.len() > MAX_HISTORY {
                self.undo_stack.pop_front();
            }
        } else if let Some(last) = self.undo_stack.back_mut() {
            last.items.extend(change.items);
            last.after_selection = change.after_selection;
        } else {
            self.undo_stack.push_back(change);
        }
    }

    pub(super) fn break_batch(&mut self) {
        self.last_edit_time = None;
    }

    pub(super) fn reset_batching(&mut self) {
        self.last_edit_time = None;
    }

    pub(super) fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.reset_batching();
    }
}
