use std::collections::VecDeque;
use std::time::{Duration, Instant};

use parley::PlainEditor;
use unicode_segmentation::UnicodeSegmentation;
use winit::keyboard::{Key, NamedKey};

use crate::text::{TextBrush, TextRenderer};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditKind {
    Insert,
    InsertFromPaste,
    DeleteBackward,
    DeleteForward,
    DeleteWordBackward,
    DeleteWordForward,
    DeleteByCut,
    HistoryUndo,
    HistoryRedo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditGroup {
    Insert,
    Delete,
}

impl EditKind {
    fn group_category(self) -> Option<EditGroup> {
        match self {
            EditKind::Insert => Some(EditGroup::Insert),
            EditKind::DeleteBackward | EditKind::DeleteForward => Some(EditGroup::Delete),
            _ => None,
        }
    }

    /// Whether consecutive edits of this kind can be merged into a single undo group.
    /// Paste, cut, word-delete, and history operations always stand alone.
    fn is_groupable(self) -> bool {
        self.group_category().is_some()
    }

    pub(crate) fn input_type(self) -> &'static str {
        match self {
            EditKind::Insert => "insertText",
            EditKind::InsertFromPaste => "insertFromPaste",
            EditKind::DeleteBackward => "deleteContentBackward",
            EditKind::DeleteForward => "deleteContentForward",
            EditKind::DeleteWordBackward => "deleteWordBackward",
            EditKind::DeleteWordForward => "deleteWordForward",
            EditKind::DeleteByCut => "deleteByCut",
            EditKind::HistoryUndo => "historyUndo",
            EditKind::HistoryRedo => "historyRedo",
        }
    }
}

#[derive(Clone, Debug)]
pub struct EditEvent {
    pub kind: EditKind,
    pub inserted: Option<String>,
}

pub enum KeyResult {
    Edit(EditEvent),
    Blur,
    Handled,
    Ignored,
}

pub struct InputState {
    pub editor: PlainEditor<TextBrush>,
    pub placeholder: String,
    pub scroll_offset: f32,
    pub scroll_offset_y: f32,
    pub focused: bool,
    pub blink_reset: Instant,
    pub disabled: bool,
    pub secure: bool,
    pub multiline: bool,
    pub max_length: Option<usize>,
    pub preedit: Option<PreeditState>,
    history: EditHistory,
}

#[derive(Clone, Debug)]
pub struct PreeditState {
    pub text: String,
    pub cursor: Option<(usize, usize)>,
}

// ── History types ───────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct HistoryEntry {
    text: String,
    anchor_byte: usize,
    focus_byte: usize,
}

/// Maximum number of undo entries to keep per input.
const MAX_HISTORY: usize = 100;

/// Consecutive edits within this window are merged into one undo group.
const GROUP_TIMEOUT_MS: u128 = 500;

struct EditHistory {
    undo_stack: VecDeque<HistoryEntry>,
    redo_stack: Vec<HistoryEntry>,
    /// The kind of the last edit that was pushed (for grouping decisions).
    last_edit_kind: Option<EditKind>,
    /// Timestamp of the last edit (for time-based group breaking).
    last_edit_time: Option<Instant>,
    /// Whether the current group is still open (accepting merges).
    group_open: bool,
}

impl EditHistory {
    fn new() -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            last_edit_kind: None,
            last_edit_time: None,
            group_open: false,
        }
    }

    /// Determine whether the incoming edit should start a new undo group
    /// (pushing a snapshot) or merge into the current one.
    fn should_start_new_group(&self, kind: EditKind, inserted: Option<&str>) -> bool {
        // Non-groupable edits (paste, cut, word-delete, history) always start a new group.
        if !kind.is_groupable() {
            return true;
        }

        // No previous edit → first group.
        let Some(last_kind) = self.last_edit_kind else {
            return true;
        };

        // Group was broken by cursor movement.
        if !self.group_open {
            return true;
        }

        // Time gap exceeds threshold.
        if let Some(last_time) = self.last_edit_time {
            if last_time.elapsed().as_millis() > GROUP_TIMEOUT_MS {
                return true;
            }
        }

        // Switching between insert and delete categories.
        if last_kind.group_category() != kind.group_category() {
            return true;
        }

        // Newline insertion always starts a new group.
        if kind == EditKind::Insert && inserted.is_some_and(|s| s.contains('\n')) {
            return true;
        }

        false
    }

    fn break_group(&mut self) {
        self.group_open = false;
    }

    fn reset_grouping(&mut self) {
        self.last_edit_kind = None;
        self.last_edit_time = None;
        self.group_open = false;
    }

    fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.reset_grouping();
    }
}

// ── InputState ──────────────────────────────────────────────────────────────

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

impl InputState {
    const BLINK_ON_MS: u128 = 530;
    const BLINK_CYCLE_MS: u128 = 1060;

    pub fn new() -> Self {
        Self {
            editor: PlainEditor::new(16.0),
            placeholder: String::new(),
            scroll_offset: 0.0,
            scroll_offset_y: 0.0,
            focused: false,
            blink_reset: Instant::now(),
            disabled: false,
            secure: false,
            multiline: true,
            max_length: None,
            preedit: None,
            history: EditHistory::new(),
        }
    }

    pub fn new_single_line() -> Self {
        let mut this = Self::new();
        this.multiline = false;
        this
    }

    pub fn text(&self) -> String {
        self.editor.text().to_string()
    }

    pub fn display_text(&self) -> String {
        if self.secure {
            "\u{2022}".repeat(self.editor.raw_text().chars().count())
        } else {
            self.editor.raw_text().to_string()
        }
    }

    pub fn has_selection(&self) -> bool {
        !self.editor.raw_selection().is_collapsed()
    }

    pub fn selected_text(&self) -> String {
        self.editor
            .selected_text()
            .map(|s| s.to_string())
            .unwrap_or_default()
    }

    fn with_driver(
        &mut self,
        renderer: &mut TextRenderer,
        f: impl FnOnce(&mut parley::PlainEditorDriver<'_, TextBrush>),
    ) {
        let mut driver = self
            .editor
            .driver(&mut renderer.font_ctx, &mut renderer.layout_ctx);
        f(&mut driver);
    }

    fn text_changed(&self, old_gen: parley::Generation) -> bool {
        self.editor.generation() != old_gen
    }

    // ── History helpers ─────────────────────────────────────────────────

    /// Capture the current editor state as a snapshot.
    fn snapshot(&self) -> HistoryEntry {
        let text = self.editor.raw_text().to_string();
        let sel = self.editor.raw_selection();
        HistoryEntry {
            text,
            anchor_byte: sel.anchor().index(),
            focus_byte: sel.focus().index(),
        }
    }

    fn grapheme_count_to_byte(text: &str, byte_idx: usize) -> usize {
        let clamped = byte_idx.min(text.len());
        text.get(..clamped).unwrap_or(text).graphemes(true).count()
    }

    /// Conditionally push a pre-edit snapshot onto the undo stack.
    /// Called after a successful edit to record the state that preceded it.
    fn push_history(&mut self, snapshot: HistoryEntry, kind: EditKind, inserted: Option<&str>) {
        if self.history.should_start_new_group(kind, inserted) {
            self.history.undo_stack.push_back(snapshot);
            if self.history.undo_stack.len() > MAX_HISTORY {
                self.history.undo_stack.pop_front();
            }
        }
        self.history.redo_stack.clear();
        self.history.last_edit_kind = Some(kind);
        self.history.last_edit_time = Some(Instant::now());
        self.history.group_open = kind.is_groupable();
    }

    /// Restore the editor to a previously captured state.
    fn restore_state(&mut self, entry: &HistoryEntry, renderer: &mut TextRenderer) {
        self.editor.set_text(&entry.text);
        let anchor_byte = entry.anchor_byte.min(entry.text.len());
        let focus_byte = entry.focus_byte.min(entry.text.len());
        let anchor_graphemes = Self::grapheme_count_to_byte(&entry.text, anchor_byte);
        let focus_graphemes = Self::grapheme_count_to_byte(&entry.text, focus_byte);

        self.with_driver(renderer, |d| {
            d.move_to_text_start();
            for _ in 0..anchor_graphemes {
                d.move_right();
            }
            if focus_graphemes >= anchor_graphemes {
                for _ in 0..(focus_graphemes - anchor_graphemes) {
                    d.select_right();
                }
            } else {
                for _ in 0..(anchor_graphemes - focus_graphemes) {
                    d.select_left();
                }
            }
        });
    }

    /// Break the current undo group. The next edit will start a new group.
    pub fn break_undo_group(&mut self) {
        self.history.break_group();
    }

    // ── Undo / Redo ─────────────────────────────────────────────────────

    pub fn undo(&mut self, renderer: &mut TextRenderer) -> Option<EditEvent> {
        if self.disabled || self.history.undo_stack.is_empty() {
            return None;
        }

        let entry = self.history.undo_stack.pop_back().unwrap();
        self.history.redo_stack.push(self.snapshot());

        self.restore_state(&entry, renderer);
        self.history.reset_grouping();
        self.reset_blink();

        Some(EditEvent {
            kind: EditKind::HistoryUndo,
            inserted: None,
        })
    }

    pub fn redo(&mut self, renderer: &mut TextRenderer) -> Option<EditEvent> {
        if self.disabled || self.history.redo_stack.is_empty() {
            return None;
        }

        let entry = self.history.redo_stack.pop().unwrap();
        self.history.undo_stack.push_back(self.snapshot());

        self.restore_state(&entry, renderer);
        self.history.reset_grouping();
        self.reset_blink();

        Some(EditEvent {
            kind: EditKind::HistoryRedo,
            inserted: None,
        })
    }

    // ── Edit operations (with history) ──────────────────────────────────

    /// Shared insert logic for both typed input and paste.
    fn do_insert(
        &mut self,
        text: &str,
        kind: EditKind,
        renderer: &mut TextRenderer,
    ) -> Option<EditEvent> {
        if self.disabled {
            return None;
        }
        let input = if !self.multiline {
            let filtered: String = text.chars().filter(|&c| c != '\n' && c != '\r').collect();
            if filtered.is_empty() {
                return None;
            }
            filtered
        } else {
            text.to_string()
        };

        if let Some(max) = self.max_length {
            let current = self.editor.raw_text().chars().count()
                - self
                    .editor
                    .selected_text()
                    .map(|s| s.chars().count())
                    .unwrap_or(0);
            if current + input.chars().count() > max {
                return None;
            }
        }

        let pre = self.snapshot();
        let generation = self.editor.generation();
        self.with_driver(renderer, |d| d.insert_or_replace_selection(&input));
        if self.text_changed(generation) {
            self.push_history(pre, kind, Some(&input));
            self.reset_blink();
            Some(EditEvent {
                kind,
                inserted: Some(input),
            })
        } else {
            None
        }
    }

    pub fn insert_text(&mut self, text: &str, renderer: &mut TextRenderer) -> Option<EditEvent> {
        self.do_insert(text, EditKind::Insert, renderer)
    }

    pub fn paste_text(&mut self, text: &str, renderer: &mut TextRenderer) -> Option<EditEvent> {
        self.do_insert(text, EditKind::InsertFromPaste, renderer)
    }

    pub fn cut_selected_text(
        &mut self,
        renderer: &mut TextRenderer,
    ) -> Option<(String, EditEvent)> {
        if self.disabled {
            return None;
        }
        let text = self.selected_text();
        if text.is_empty() {
            return None;
        }
        let pre = self.snapshot();
        self.with_driver(renderer, |d| d.delete_selection());
        self.push_history(pre, EditKind::DeleteByCut, None);
        self.reset_blink();
        Some((
            text,
            EditEvent {
                kind: EditKind::DeleteByCut,
                inserted: None,
            },
        ))
    }

    /// Shared delete logic — snapshots, runs the driver action, records history.
    fn do_delete(
        &mut self,
        kind: EditKind,
        renderer: &mut TextRenderer,
        action: fn(&mut parley::PlainEditorDriver<'_, TextBrush>),
    ) -> Option<EditEvent> {
        if self.disabled {
            return None;
        }
        let pre = self.snapshot();
        let generation = self.editor.generation();
        self.with_driver(renderer, |d| action(d));
        if self.text_changed(generation) {
            self.push_history(pre, kind, None);
            self.reset_blink();
            Some(EditEvent {
                kind,
                inserted: None,
            })
        } else {
            None
        }
    }

    pub fn delete_backward(&mut self, renderer: &mut TextRenderer) -> Option<EditEvent> {
        self.do_delete(EditKind::DeleteBackward, renderer, |d| d.backdelete())
    }

    pub fn delete_forward(&mut self, renderer: &mut TextRenderer) -> Option<EditEvent> {
        self.do_delete(EditKind::DeleteForward, renderer, |d| d.delete())
    }

    pub fn delete_word_backward(&mut self, renderer: &mut TextRenderer) -> Option<EditEvent> {
        self.do_delete(EditKind::DeleteWordBackward, renderer, |d| {
            d.backdelete_word()
        })
    }

    pub fn delete_word_forward(&mut self, renderer: &mut TextRenderer) -> Option<EditEvent> {
        self.do_delete(EditKind::DeleteWordForward, renderer, |d| d.delete_word())
    }

    // ── Cursor movement (breaks undo group) ─────────────────────────────

    pub fn move_left(&mut self, extend: bool, renderer: &mut TextRenderer) {
        self.break_undo_group();
        if extend {
            self.with_driver(renderer, |d| d.select_left());
        } else {
            self.with_driver(renderer, |d| d.move_left());
        }
        self.reset_blink();
    }

    pub fn move_right(&mut self, extend: bool, renderer: &mut TextRenderer) {
        self.break_undo_group();
        if extend {
            self.with_driver(renderer, |d| d.select_right());
        } else {
            self.with_driver(renderer, |d| d.move_right());
        }
        self.reset_blink();
    }

    pub fn move_word_left(&mut self, extend: bool, renderer: &mut TextRenderer) {
        self.break_undo_group();
        if extend {
            self.with_driver(renderer, |d| d.select_word_left());
        } else {
            self.with_driver(renderer, |d| d.move_word_left());
        }
        self.reset_blink();
    }

    pub fn move_word_right(&mut self, extend: bool, renderer: &mut TextRenderer) {
        self.break_undo_group();
        if extend {
            self.with_driver(renderer, |d| d.select_word_right());
        } else {
            self.with_driver(renderer, |d| d.move_word_right());
        }
        self.reset_blink();
    }

    pub fn move_up(&mut self, extend: bool, renderer: &mut TextRenderer) {
        self.break_undo_group();
        if extend {
            self.with_driver(renderer, |d| d.select_up());
        } else {
            self.with_driver(renderer, |d| d.move_up());
        }
    }

    pub fn move_down(&mut self, extend: bool, renderer: &mut TextRenderer) {
        self.break_undo_group();
        if extend {
            self.with_driver(renderer, |d| d.select_down());
        } else {
            self.with_driver(renderer, |d| d.move_down());
        }
    }

    pub fn move_home(&mut self, extend: bool, renderer: &mut TextRenderer) {
        self.break_undo_group();
        if extend {
            self.with_driver(renderer, |d| d.select_to_line_start());
        } else {
            self.with_driver(renderer, |d| d.move_to_line_start());
        }
        self.reset_blink();
    }

    pub fn move_end(&mut self, extend: bool, renderer: &mut TextRenderer) {
        self.break_undo_group();
        if extend {
            self.with_driver(renderer, |d| d.select_to_line_end());
        } else {
            self.with_driver(renderer, |d| d.move_to_line_end());
        }
        self.reset_blink();
    }

    pub fn move_absolute_home(&mut self, extend: bool, renderer: &mut TextRenderer) {
        self.break_undo_group();
        if extend {
            self.with_driver(renderer, |d| d.select_to_text_start());
        } else {
            self.with_driver(renderer, |d| d.move_to_text_start());
        }
        self.reset_blink();
    }

    pub fn move_absolute_end(&mut self, extend: bool, renderer: &mut TextRenderer) {
        self.break_undo_group();
        if extend {
            self.with_driver(renderer, |d| d.select_to_text_end());
        } else {
            self.with_driver(renderer, |d| d.move_to_text_end());
        }
        self.reset_blink();
    }

    pub fn move_to_point(&mut self, x: f32, y: f32, renderer: &mut TextRenderer) {
        self.break_undo_group();
        self.with_driver(renderer, |d| d.move_to_point(x, y));
        self.reset_blink();
    }

    pub fn extend_selection_to_point(&mut self, x: f32, y: f32, renderer: &mut TextRenderer) {
        self.with_driver(renderer, |d| d.extend_selection_to_point(x, y));
    }

    pub fn select_word_at_point(&mut self, x: f32, y: f32, renderer: &mut TextRenderer) {
        self.break_undo_group();
        self.with_driver(renderer, |d| d.select_word_at_point(x, y));
        self.reset_blink();
    }

    pub fn select_line_at_point(&mut self, x: f32, y: f32, renderer: &mut TextRenderer) {
        self.break_undo_group();
        self.with_driver(renderer, |d| d.select_line_at_point(x, y));
        self.reset_blink();
    }

    pub fn select_all(&mut self, renderer: &mut TextRenderer) {
        self.break_undo_group();
        self.with_driver(renderer, |d| d.select_all());
        self.reset_blink();
    }

    // ── IME ─────────────────────────────────────────────────────────────

    pub fn set_preedit(&mut self, text: String, cursor: Option<(usize, usize)>) {
        if text.is_empty() {
            self.preedit = None;
        } else {
            self.preedit = Some(PreeditState { text, cursor });
        }
    }

    pub fn clear_preedit(&mut self) {
        self.preedit = None;
    }

    pub fn commit_ime_text(
        &mut self,
        text: &str,
        renderer: &mut TextRenderer,
    ) -> Option<EditEvent> {
        self.preedit = None;
        // Each IME commit is a separate undo group.
        self.break_undo_group();
        self.insert_text(text, renderer)
    }

    // ── Programmatic value setter ───────────────────────────────────────

    pub fn set_value(&mut self, value: &str) {
        if self.editor.raw_text() != value {
            self.editor.set_text(value);
            self.history.clear();
        }
    }

    pub fn set_width(&mut self, width: Option<f32>) {
        self.editor.set_width(width);
    }

    pub fn set_scale(&mut self, scale: f32) {
        self.editor.set_scale(scale);
    }

    // ── Blink ───────────────────────────────────────────────────────────

    pub fn reset_blink(&mut self) {
        self.blink_reset = Instant::now();
    }

    pub fn blink_visible(&self, window_focused: bool) -> bool {
        if !self.focused || !window_focused {
            return false;
        }
        let elapsed = self.blink_phase_elapsed_ms();
        elapsed < Self::BLINK_ON_MS
    }

    pub fn next_blink_toggle_in(&self, window_focused: bool) -> Option<Duration> {
        if !self.focused || !window_focused {
            return None;
        }

        let elapsed = self.blink_phase_elapsed_ms();
        let remaining = if elapsed < Self::BLINK_ON_MS {
            Self::BLINK_ON_MS - elapsed
        } else {
            Self::BLINK_CYCLE_MS - elapsed
        };

        Some(Duration::from_millis(remaining.max(1) as u64))
    }

    fn blink_phase_elapsed_ms(&self) -> u128 {
        self.blink_reset.elapsed().as_millis() % Self::BLINK_CYCLE_MS
    }

    // ── Scroll ──────────────────────────────────────────────────────────

    pub fn update_scroll(&mut self, cursor_x: f32, visible_width: f32) {
        if visible_width <= 0.0 {
            return;
        }
        if cursor_x - self.scroll_offset < 0.0 {
            self.scroll_offset = cursor_x;
        } else if cursor_x - self.scroll_offset > visible_width {
            self.scroll_offset = cursor_x - visible_width;
        }
        if self.scroll_offset < 0.0 {
            self.scroll_offset = 0.0;
        }
    }

    pub fn update_scroll_y(&mut self, cursor_y: f32, line_height: f32, visible_height: f32) {
        if visible_height <= 0.0 {
            return;
        }
        let cursor_bottom = cursor_y + line_height;
        if cursor_y < self.scroll_offset_y {
            self.scroll_offset_y = cursor_y;
        } else if cursor_bottom > self.scroll_offset_y + visible_height {
            self.scroll_offset_y = cursor_bottom - visible_height;
        }
        if self.scroll_offset_y < 0.0 {
            self.scroll_offset_y = 0.0;
        }
    }

    // ── Keyboard dispatch ───────────────────────────────────────────────

    pub fn handle_key(
        &mut self,
        key: &Key,
        modifiers: u32,
        renderer: &mut TextRenderer,
    ) -> KeyResult {
        if self.disabled {
            return KeyResult::Ignored;
        }
        if !self.multiline && matches!(key, Key::Named(NamedKey::Enter)) {
            return KeyResult::Ignored;
        }

        let shift = modifiers & 4 != 0;
        let ctrl = modifiers & 1 != 0;

        match key {
            Key::Character(ch) => {
                if ctrl {
                    if ch.eq_ignore_ascii_case("a") {
                        self.select_all(renderer);
                        return KeyResult::Handled;
                    }
                    // Ctrl+Z → undo
                    if ch.eq_ignore_ascii_case("z") && !shift {
                        return self
                            .undo(renderer)
                            .map_or(KeyResult::Handled, KeyResult::Edit);
                    }
                    // Ctrl+Shift+Z or Ctrl+Y → redo
                    if (ch.eq_ignore_ascii_case("z") && shift) || ch.eq_ignore_ascii_case("y") {
                        return self
                            .redo(renderer)
                            .map_or(KeyResult::Handled, KeyResult::Edit);
                    }
                    return KeyResult::Ignored;
                }
                self.insert_text(ch, renderer)
                    .map_or(KeyResult::Handled, KeyResult::Edit)
            }
            Key::Named(named) => match named {
                NamedKey::Backspace => {
                    let result = if ctrl {
                        self.delete_word_backward(renderer)
                    } else {
                        self.delete_backward(renderer)
                    };
                    result.map_or(KeyResult::Handled, KeyResult::Edit)
                }
                NamedKey::Delete => {
                    let result = if ctrl {
                        self.delete_word_forward(renderer)
                    } else {
                        self.delete_forward(renderer)
                    };
                    result.map_or(KeyResult::Handled, KeyResult::Edit)
                }
                NamedKey::ArrowLeft => {
                    if ctrl {
                        self.move_word_left(shift, renderer);
                    } else {
                        self.move_left(shift, renderer);
                    }
                    KeyResult::Handled
                }
                NamedKey::ArrowRight => {
                    if ctrl {
                        self.move_word_right(shift, renderer);
                    } else {
                        self.move_right(shift, renderer);
                    }
                    KeyResult::Handled
                }
                NamedKey::Undo => self
                    .undo(renderer)
                    .map_or(KeyResult::Handled, KeyResult::Edit),
                NamedKey::Redo => self
                    .redo(renderer)
                    .map_or(KeyResult::Handled, KeyResult::Edit),
                NamedKey::ArrowUp => {
                    self.move_up(shift, renderer);
                    KeyResult::Handled
                }
                NamedKey::ArrowDown => {
                    self.move_down(shift, renderer);
                    KeyResult::Handled
                }
                NamedKey::Home => {
                    if ctrl {
                        self.move_absolute_home(shift, renderer);
                    } else {
                        self.move_home(shift, renderer);
                    }
                    KeyResult::Handled
                }
                NamedKey::End => {
                    if ctrl {
                        self.move_absolute_end(shift, renderer);
                    } else {
                        self.move_end(shift, renderer);
                    }
                    KeyResult::Handled
                }
                NamedKey::Space => self
                    .insert_text(" ", renderer)
                    .map_or(KeyResult::Handled, KeyResult::Edit),
                NamedKey::Escape => KeyResult::Blur,
                NamedKey::Enter => self
                    .insert_text("\n", renderer)
                    .map_or(KeyResult::Ignored, KeyResult::Edit),
                NamedKey::Tab => self
                    .insert_text("    ", renderer)
                    .map_or(KeyResult::Ignored, KeyResult::Edit),
                _ => KeyResult::Ignored,
            },
            _ => KeyResult::Ignored,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_renderer() -> TextRenderer {
        TextRenderer::new()
    }

    // ── Existing tests ──────────────────────────────────────────────────

    #[test]
    fn new_input_has_empty_text() {
        let is = InputState::new();
        assert_eq!(is.text(), "");
        assert!(!is.has_selection());
    }

    #[test]
    fn insert_text_updates_content() {
        let mut is = InputState::new();
        let mut r = make_renderer();
        is.insert_text("hello", &mut r);
        assert_eq!(is.text(), "hello");
    }

    #[test]
    fn insert_text_disabled_is_noop() {
        let mut is = InputState::new();
        is.disabled = true;
        let mut r = make_renderer();
        let result = is.insert_text("hello", &mut r);
        assert!(result.is_none());
        assert_eq!(is.text(), "");
    }

    #[test]
    fn single_line_strips_newlines() {
        let mut is = InputState::new_single_line();
        let mut r = make_renderer();
        is.insert_text("a\nb\nc", &mut r);
        assert_eq!(is.text(), "abc");
    }

    #[test]
    fn max_length_rejects_overflow() {
        let mut is = InputState::new();
        is.max_length = Some(3);
        let mut r = make_renderer();
        is.insert_text("ab", &mut r);
        let result = is.insert_text("cd", &mut r);
        assert!(result.is_none());
        assert_eq!(is.text(), "ab");
    }

    #[test]
    fn max_length_allows_exact_fit() {
        let mut is = InputState::new();
        is.max_length = Some(3);
        let mut r = make_renderer();
        is.insert_text("ab", &mut r);
        let result = is.insert_text("c", &mut r);
        assert!(result.is_some());
        assert_eq!(is.text(), "abc");
    }

    #[test]
    fn delete_backward_removes_char() {
        let mut is = InputState::new();
        let mut r = make_renderer();
        is.insert_text("abc", &mut r);
        is.delete_backward(&mut r);
        assert_eq!(is.text(), "ab");
    }

    #[test]
    fn delete_forward_at_end_is_noop() {
        let mut is = InputState::new();
        let mut r = make_renderer();
        is.insert_text("abc", &mut r);
        is.delete_forward(&mut r);
        assert_eq!(is.text(), "abc");
    }

    #[test]
    fn select_all_and_cut() {
        let mut is = InputState::new();
        let mut r = make_renderer();
        is.insert_text("hello", &mut r);
        is.select_all(&mut r);
        assert!(is.has_selection());
        let cut = is.cut_selected_text(&mut r);
        assert!(cut.is_some());
        let (text, _) = cut.unwrap();
        assert_eq!(text, "hello");
        assert_eq!(is.text(), "");
    }

    #[test]
    fn paste_text_inserts() {
        let mut is = InputState::new();
        let mut r = make_renderer();
        is.insert_text("ab", &mut r);
        is.paste_text("cd", &mut r);
        assert_eq!(is.text(), "abcd");
    }

    #[test]
    fn display_text_secure_masks() {
        let mut is = InputState::new();
        is.secure = true;
        let mut r = make_renderer();
        is.insert_text("pass", &mut r);
        assert_eq!(is.display_text(), "\u{2022}\u{2022}\u{2022}\u{2022}");
        assert_eq!(is.text(), "pass");
    }

    #[test]
    fn display_text_normal_shows_raw() {
        let mut is = InputState::new();
        let mut r = make_renderer();
        is.insert_text("hello", &mut r);
        assert_eq!(is.display_text(), "hello");
    }

    #[test]
    fn set_value_replaces_content() {
        let mut is = InputState::new();
        let mut r = make_renderer();
        is.insert_text("old", &mut r);
        is.set_value("new");
        assert_eq!(is.text(), "new");
    }

    #[test]
    fn update_scroll_scrolls_right() {
        let mut is = InputState::new();
        is.scroll_offset = 0.0;
        is.update_scroll(250.0, 200.0);
        assert_eq!(is.scroll_offset, 50.0);
    }

    #[test]
    fn update_scroll_scrolls_left() {
        let mut is = InputState::new();
        is.scroll_offset = 100.0;
        is.update_scroll(50.0, 200.0);
        assert_eq!(is.scroll_offset, 50.0);
    }

    #[test]
    fn update_scroll_no_negative() {
        let mut is = InputState::new();
        is.scroll_offset = -10.0;
        is.update_scroll(50.0, 200.0);
        assert!(is.scroll_offset >= 0.0);
    }

    #[test]
    fn update_scroll_y_scrolls_down() {
        let mut is = InputState::new();
        is.scroll_offset_y = 0.0;
        is.update_scroll_y(250.0, 20.0, 200.0);
        assert_eq!(is.scroll_offset_y, 70.0);
    }

    #[test]
    fn update_scroll_y_scrolls_up() {
        let mut is = InputState::new();
        is.scroll_offset_y = 100.0;
        is.update_scroll_y(50.0, 20.0, 200.0);
        assert_eq!(is.scroll_offset_y, 50.0);
    }

    #[test]
    fn blink_not_visible_unfocused() {
        let is = InputState::new();
        assert!(!is.blink_visible(true));
    }

    #[test]
    fn blink_not_visible_window_unfocused() {
        let mut is = InputState::new();
        is.focused = true;
        assert!(!is.blink_visible(false));
    }

    #[test]
    fn next_blink_toggle_is_absent_when_unfocused() {
        let is = InputState::new();
        assert!(is.next_blink_toggle_in(true).is_none());
    }

    #[test]
    fn next_blink_toggle_matches_visible_phase() {
        let mut is = InputState::new();
        is.focused = true;
        is.blink_reset = Instant::now() - Duration::from_millis(200);
        let next = is.next_blink_toggle_in(true).unwrap();
        assert!((329..=330).contains(&next.as_millis()));
    }

    #[test]
    fn next_blink_toggle_matches_hidden_phase() {
        let mut is = InputState::new();
        is.focused = true;
        is.blink_reset = Instant::now() - Duration::from_millis(700);
        let next = is.next_blink_toggle_in(true).unwrap();
        assert!((359..=360).contains(&next.as_millis()));
    }

    // ── Undo / Redo tests ───────────────────────────────────────────────

    #[test]
    fn undo_restores_previous_text() {
        let mut is = InputState::new();
        let mut r = make_renderer();
        is.insert_text("hello", &mut r);
        assert_eq!(is.text(), "hello");

        let result = is.undo(&mut r);
        assert!(result.is_some());
        assert_eq!(result.unwrap().kind, EditKind::HistoryUndo);
        assert_eq!(is.text(), "");
    }

    #[test]
    fn redo_after_undo() {
        let mut is = InputState::new();
        let mut r = make_renderer();
        is.insert_text("hello", &mut r);
        is.undo(&mut r);
        assert_eq!(is.text(), "");

        let result = is.redo(&mut r);
        assert!(result.is_some());
        assert_eq!(result.unwrap().kind, EditKind::HistoryRedo);
        assert_eq!(is.text(), "hello");
    }

    #[test]
    fn new_edit_clears_redo() {
        let mut is = InputState::new();
        let mut r = make_renderer();
        is.insert_text("a", &mut r);
        is.undo(&mut r);
        assert_eq!(is.text(), "");

        // New edit should clear redo stack
        is.insert_text("b", &mut r);
        assert_eq!(is.text(), "b");

        let result = is.redo(&mut r);
        assert!(result.is_none());
    }

    #[test]
    fn consecutive_inserts_grouped() {
        let mut is = InputState::new();
        let mut r = make_renderer();
        // Type characters rapidly (within GROUP_TIMEOUT_MS)
        is.insert_text("h", &mut r);
        is.insert_text("e", &mut r);
        is.insert_text("l", &mut r);
        is.insert_text("l", &mut r);
        is.insert_text("o", &mut r);
        assert_eq!(is.text(), "hello");

        // Single undo should remove the entire group
        is.undo(&mut r);
        assert_eq!(is.text(), "");
    }

    #[test]
    fn different_edit_kinds_break_group() {
        let mut is = InputState::new();
        let mut r = make_renderer();
        is.insert_text("ab", &mut r);
        // Switching from insert to delete starts a new group
        is.delete_backward(&mut r);
        assert_eq!(is.text(), "a");

        // Undo the delete → "ab"
        is.undo(&mut r);
        assert_eq!(is.text(), "ab");

        // Undo the insert → ""
        is.undo(&mut r);
        assert_eq!(is.text(), "");
    }

    #[test]
    fn paste_always_new_group() {
        let mut is = InputState::new();
        let mut r = make_renderer();
        is.insert_text("a", &mut r);
        is.paste_text("bc", &mut r);
        assert_eq!(is.text(), "abc");

        // Undo should remove only the paste
        is.undo(&mut r);
        assert_eq!(is.text(), "a");

        // Undo should remove the initial insert
        is.undo(&mut r);
        assert_eq!(is.text(), "");
    }

    #[test]
    fn typing_after_paste_starts_new_group() {
        let mut is = InputState::new();
        let mut r = make_renderer();
        is.paste_text("hello", &mut r);
        is.insert_text("!", &mut r);
        assert_eq!(is.text(), "hello!");

        // Undo should remove only the typed character, not the paste.
        is.undo(&mut r);
        assert_eq!(is.text(), "hello");

        // Undo again should remove the paste.
        is.undo(&mut r);
        assert_eq!(is.text(), "");
    }

    #[test]
    fn set_value_clears_history() {
        let mut is = InputState::new();
        let mut r = make_renderer();
        is.insert_text("hello", &mut r);
        is.set_value("world");
        assert_eq!(is.text(), "world");

        // Undo should be empty after set_value
        let result = is.undo(&mut r);
        assert!(result.is_none());
    }

    #[test]
    fn undo_on_empty_history_is_noop() {
        let mut is = InputState::new();
        let mut r = make_renderer();
        let result = is.undo(&mut r);
        assert!(result.is_none());
        assert_eq!(is.text(), "");
    }

    #[test]
    fn redo_on_empty_history_is_noop() {
        let mut is = InputState::new();
        let mut r = make_renderer();
        let result = is.redo(&mut r);
        assert!(result.is_none());
    }

    #[test]
    fn cursor_movement_breaks_group() {
        let mut is = InputState::new();
        let mut r = make_renderer();
        is.insert_text("a", &mut r);
        is.move_left(false, &mut r);
        is.insert_text("b", &mut r);
        assert_eq!(is.text(), "ba");

        // Undo should remove only the "b" (cursor movement broke the group)
        is.undo(&mut r);
        assert_eq!(is.text(), "a");
    }

    #[test]
    fn cut_creates_separate_undo_group() {
        let mut is = InputState::new();
        let mut r = make_renderer();
        is.insert_text("hello", &mut r);
        is.select_all(&mut r);
        is.cut_selected_text(&mut r);
        assert_eq!(is.text(), "");

        // Undo should restore the cut text
        is.undo(&mut r);
        assert_eq!(is.text(), "hello");
    }

    #[test]
    fn multiple_undo_redo_cycles() {
        let mut is = InputState::new();
        let mut r = make_renderer();
        is.insert_text("one", &mut r);
        is.break_undo_group();
        is.insert_text(" two", &mut r);
        assert_eq!(is.text(), "one two");

        is.undo(&mut r);
        assert_eq!(is.text(), "one");

        is.undo(&mut r);
        assert_eq!(is.text(), "");

        is.redo(&mut r);
        assert_eq!(is.text(), "one");

        is.redo(&mut r);
        assert_eq!(is.text(), "one two");
    }

    #[test]
    fn undo_disabled_input_is_noop() {
        let mut is = InputState::new();
        let mut r = make_renderer();
        is.insert_text("hello", &mut r);
        is.disabled = true;

        let result = is.undo(&mut r);
        assert!(result.is_none());
        assert_eq!(is.text(), "hello");
    }

    #[test]
    fn delete_backward_has_separate_group_from_insert() {
        let mut is = InputState::new();
        let mut r = make_renderer();
        is.insert_text("abc", &mut r);
        is.delete_backward(&mut r);
        is.delete_backward(&mut r);
        assert_eq!(is.text(), "a");

        // Undo the deletes (grouped together)
        is.undo(&mut r);
        assert_eq!(is.text(), "abc");
    }
}
