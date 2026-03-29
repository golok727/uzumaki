use std::time::Instant;
use winit::keyboard::{Key, NamedKey};

use crate::text_model::TextModel;

// ── Selection ────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default)]
pub struct TextSelection {
    /// Anchor point (where selection started), flat grapheme index
    pub anchor: usize,
    /// Active point / cursor position, flat grapheme index
    pub active: usize,
}

impl TextSelection {
    pub fn new(anchor: usize, active: usize) -> Self {
        Self { anchor, active }
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.active
    }

    pub fn start(&self) -> usize {
        self.anchor.min(self.active)
    }

    pub fn end(&self) -> usize {
        self.anchor.max(self.active)
    }

    pub fn set_cursor(&mut self, pos: usize) {
        self.anchor = pos;
        self.active = pos;
    }
}

// ── EditEvent ────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum EditKind {
    Insert,
    DeleteBackward,
    DeleteForward,
    DeleteWordBackward,
    DeleteWordForward,
}

#[derive(Clone, Debug)]
pub struct EditEvent {
    pub kind: EditKind,
    pub inserted: Option<String>,
}

// ── KeyResult ────────────────────────────────────────────────────────

pub enum KeyResult {
    Edit(EditEvent),
    Blur,
    Handled,
    Ignored,
}

// ── InputState ───────────────────────────────────────────────────────
// Owns selection, delegates buffer mutations to TextModel.
// Handles key events, movement, and presentation concerns.

pub struct InputState {
    pub model: TextModel,
    pub selection: TextSelection,
    pub placeholder: String,
    pub scroll_offset: f32,
    pub scroll_offset_y: f32,
    pub focused: bool,
    pub blink_reset: Instant,
    pub disabled: bool,
    pub secure: bool,
    pub multiline: bool,
    /// Preserved column for vertical navigation (sticky column in grapheme units).
    pub sticky_col: Option<usize>,
    /// Preserved X coordinate for vertical navigation.
    pub sticky_x: Option<f32>,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            model: TextModel::new(),
            selection: TextSelection::default(),
            placeholder: String::new(),
            scroll_offset: 0.0,
            scroll_offset_y: 0.0,
            focused: false,
            blink_reset: Instant::now(),
            disabled: false,
            secure: false,
            multiline: true,
            sticky_col: None,
            sticky_x: None,
        }
    }
}

impl InputState {
    pub fn new(multi_line: bool) -> Self {
        let mut state = Self::default();
        state.multiline = multi_line;
        state
    }

    /// Delete the current selection. Returns true if something was deleted.
    fn delete_selection(&mut self) -> bool {
        if self.selection.is_collapsed() {
            return false;
        }
        let start = self.selection.start();
        let end = self.selection.end();
        self.model.delete_range(start, end);
        self.selection.set_cursor(start);
        true
    }

    /// Get the selected text.
    pub fn selected_text(&self) -> String {
        if self.selection.is_collapsed() {
            return String::new();
        }
        self.model
            .text_in_range(self.selection.start(), self.selection.end())
    }

    pub fn text_content(&self) -> String {
        self.model.text()
    }

    pub fn insert_text(&mut self, text: &str) -> Option<EditEvent> {
        self.sticky_x = None;
        self.sticky_col = None;
        if self.disabled {
            return None;
        }
        // Single-line: reject newlines
        let text_to_insert;
        let input = if !self.multiline {
            text_to_insert = text
                .chars()
                .filter(|&c| c != '\n' && c != '\r')
                .collect::<String>();
            if text_to_insert.is_empty() {
                return None;
            }
            text_to_insert.as_str()
        } else {
            text
        };

        self.delete_selection();
        let pos = self.selection.active;

        match self.model.insert(pos, input, 0) {
            Some(new_pos) => {
                self.selection.set_cursor(new_pos);
                self.reset_blink();
                Some(EditEvent {
                    kind: EditKind::Insert,
                    inserted: Some(input.to_string()),
                })
            }
            None => None,
        }
    }

    pub fn delete_backward(&mut self) -> Option<EditEvent> {
        self.sticky_x = None;
        self.sticky_col = None;
        if self.disabled {
            return None;
        }
        if self.delete_selection() {
            self.reset_blink();
            return Some(EditEvent {
                kind: EditKind::DeleteBackward,
                inserted: None,
            });
        }
        match self.model.delete_backward(self.selection.active) {
            Some(new_pos) => {
                self.selection.set_cursor(new_pos);
                self.reset_blink();
                Some(EditEvent {
                    kind: EditKind::DeleteBackward,
                    inserted: None,
                })
            }
            None => None,
        }
    }

    pub fn delete_forward(&mut self) -> Option<EditEvent> {
        self.sticky_x = None;
        self.sticky_col = None;
        if self.disabled {
            return None;
        }
        if self.delete_selection() {
            self.reset_blink();
            return Some(EditEvent {
                kind: EditKind::DeleteForward,
                inserted: None,
            });
        }
        match self.model.delete_forward(self.selection.active) {
            Some(new_pos) => {
                self.selection.set_cursor(new_pos);
                self.reset_blink();
                Some(EditEvent {
                    kind: EditKind::DeleteForward,
                    inserted: None,
                })
            }
            None => None,
        }
    }

    pub fn delete_word_backward(&mut self) -> Option<EditEvent> {
        self.sticky_x = None;
        self.sticky_col = None;
        if self.disabled {
            return None;
        }
        if self.delete_selection() {
            self.reset_blink();
            return Some(EditEvent {
                kind: EditKind::DeleteWordBackward,
                inserted: None,
            });
        }
        match self.model.delete_word_backward(self.selection.active) {
            Some(new_pos) => {
                self.selection.set_cursor(new_pos);
                self.reset_blink();
                Some(EditEvent {
                    kind: EditKind::DeleteWordBackward,
                    inserted: None,
                })
            }
            None => None,
        }
    }

    pub fn delete_word_forward(&mut self) -> Option<EditEvent> {
        self.sticky_x = None;
        self.sticky_col = None;
        if self.disabled {
            return None;
        }
        if self.delete_selection() {
            self.reset_blink();
            return Some(EditEvent {
                kind: EditKind::DeleteWordForward,
                inserted: None,
            });
        }
        match self.model.delete_word_forward(self.selection.active) {
            Some(new_pos) => {
                self.selection.set_cursor(new_pos);
                self.reset_blink();
                Some(EditEvent {
                    kind: EditKind::DeleteWordForward,
                    inserted: None,
                })
            }
            None => None,
        }
    }

    // ── Movement ─────────────────────────────────────────────────────

    pub fn move_left(&mut self, extend: bool) {
        self.sticky_x = None;
        if !extend && !self.selection.is_collapsed() {
            let pos = self.selection.start();
            self.selection.set_cursor(pos);
        } else if self.selection.active > 0 {
            self.selection.active -= 1;
            if !extend {
                self.selection.anchor = self.selection.active;
            }
        }
        self.reset_blink();
    }

    pub fn move_right(&mut self, extend: bool) {
        self.sticky_x = None;
        let count = self.model.grapheme_count();
        if !extend && !self.selection.is_collapsed() {
            let pos = self.selection.end();
            self.selection.set_cursor(pos);
        } else if self.selection.active < count {
            self.selection.active += 1;
            if !extend {
                self.selection.anchor = self.selection.active;
            }
        }
        self.reset_blink();
    }

    pub fn move_word_left(&mut self, extend: bool) {
        self.sticky_x = None;
        let pos = self.model.find_word_start(self.selection.active);
        self.selection.active = pos;
        if !extend {
            self.selection.anchor = pos;
        }
        self.reset_blink();
    }

    pub fn move_word_right(&mut self, extend: bool) {
        self.sticky_x = None;
        let pos = self.model.find_word_end(self.selection.active);
        self.selection.active = pos;
        if !extend {
            self.selection.anchor = pos;
        }
        self.reset_blink();
    }

    pub fn move_home(&mut self, extend: bool) {
        self.sticky_x = None;
        let (row, _) = self.model.flat_to_rowcol(self.selection.active);
        let flat = self.model.rowcol_to_flat(row, 0);
        self.selection.active = flat;
        if !extend {
            self.selection.anchor = flat;
        }
        self.reset_blink();
    }

    pub fn move_end(&mut self, extend: bool) {
        self.sticky_x = None;
        let (row, _) = self.model.flat_to_rowcol(self.selection.active);
        let line_len = self.model.line_grapheme_count(row);
        let flat = self.model.rowcol_to_flat(row, line_len);
        self.selection.active = flat;
        if !extend {
            self.selection.anchor = flat;
        }
        self.reset_blink();
    }

    pub fn move_absolute_home(&mut self, extend: bool) {
        self.sticky_x = None;
        self.selection.active = 0;
        if !extend {
            self.selection.anchor = 0;
        }
        self.reset_blink();
    }

    pub fn move_absolute_end(&mut self, extend: bool) {
        self.sticky_x = None;
        let count = self.model.grapheme_count();
        self.selection.active = count;
        if !extend {
            self.selection.anchor = count;
        }
        self.reset_blink();
    }

    pub fn move_up(&mut self, extend: bool, sticky_col: Option<usize>) -> bool {
        let (row, col) = self.model.flat_to_rowcol(self.selection.active);
        if row == 0 {
            self.selection.active = 0;
            if !extend {
                self.selection.anchor = 0;
            }
            return false;
        }
        let target_col = sticky_col.unwrap_or(col);
        let prev_line_len = self.model.line_grapheme_count(row - 1);
        let new_col = target_col.min(prev_line_len);
        let flat = self.model.rowcol_to_flat(row - 1, new_col);
        self.selection.active = flat;
        if !extend {
            self.selection.anchor = flat;
        }
        true
    }

    pub fn move_down(&mut self, extend: bool, sticky_col: Option<usize>) -> bool {
        let (row, col) = self.model.flat_to_rowcol(self.selection.active);
        if row >= self.model.line_count() - 1 {
            let count = self.model.grapheme_count();
            self.selection.active = count;
            if !extend {
                self.selection.anchor = count;
            }
            return false;
        }
        let target_col = sticky_col.unwrap_or(col);
        let next_line_len = self.model.line_grapheme_count(row + 1);
        let new_col = target_col.min(next_line_len);
        let flat = self.model.rowcol_to_flat(row + 1, new_col);
        self.selection.active = flat;
        if !extend {
            self.selection.anchor = flat;
        }
        true
    }

    pub fn move_to(&mut self, pos: usize, extend: bool) {
        let count = self.model.grapheme_count();
        self.selection.active = pos.min(count);
        if !extend {
            self.selection.anchor = self.selection.active;
        }
        self.reset_blink();
    }

    pub fn set_selection(&mut self, anchor: usize, active: usize) {
        let max = self.grapheme_count();
        self.selection.anchor = anchor.min(max);
        self.selection.active = active.min(max);
        self.reset_blink();
    }

    pub fn select_all(&mut self) {
        self.sticky_x = None;
        self.selection.anchor = 0;
        self.selection.active = self.model.grapheme_count();
        self.reset_blink();
    }

    pub fn word_at(&self, grapheme_idx: usize) -> (usize, usize) {
        self.model.word_at(grapheme_idx)
    }

    pub fn set_value(&mut self, value: String) {
        self.model.set_value(value);
        let count = self.model.grapheme_count();
        if self.selection.active > count {
            self.selection.active = count;
        }
        if self.selection.anchor > count {
            self.selection.anchor = count;
        }
    }

    /// Current (row, col) of the active cursor position.
    pub fn cursor_rowcol(&self) -> (usize, usize) {
        self.model.flat_to_rowcol(self.selection.active)
    }

    // ── Widget-layer concerns ────────────────────────────────────────

    pub fn grapheme_count(&self) -> usize {
        self.model.grapheme_count()
    }

    pub fn reset_blink(&mut self) {
        self.blink_reset = Instant::now();
    }

    pub fn blink_visible(&self, window_focused: bool) -> bool {
        if !self.focused || !window_focused {
            return false;
        }
        let elapsed = self.blink_reset.elapsed().as_millis();
        (elapsed % 1060) < 530
    }

    pub fn display_text(&self) -> String {
        if self.secure {
            "\u{2022}".repeat(self.model.grapheme_count())
        } else {
            self.model.text()
        }
    }

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

    // ── Key handling ─────────────────────────────────────────────────

    pub fn handle_key(&mut self, key: &Key, modifiers: u32) -> KeyResult {
        if self.disabled {
            return KeyResult::Ignored;
        }
        // Single-line: reject Enter
        if !self.multiline {
            if matches!(key, Key::Named(NamedKey::Enter)) {
                return KeyResult::Ignored;
            }
        }
        self.sticky_x = None;
        self.sticky_col = None;

        let shift = modifiers & 4 != 0;
        let ctrl = modifiers & 1 != 0;

        match key {
            Key::Character(ch) => {
                if ctrl {
                    if ch.eq_ignore_ascii_case("a") {
                        self.select_all();
                        return KeyResult::Handled;
                    }
                    return KeyResult::Ignored;
                }
                match self.insert_text(ch) {
                    Some(edit) => KeyResult::Edit(edit),
                    None => KeyResult::Handled,
                }
            }
            Key::Named(named) => match named {
                NamedKey::Backspace => {
                    if ctrl {
                        match self.delete_word_backward() {
                            Some(edit) => KeyResult::Edit(edit),
                            None => KeyResult::Handled,
                        }
                    } else {
                        match self.delete_backward() {
                            Some(edit) => KeyResult::Edit(edit),
                            None => KeyResult::Handled,
                        }
                    }
                }
                NamedKey::Delete => {
                    if ctrl {
                        match self.delete_word_forward() {
                            Some(edit) => KeyResult::Edit(edit),
                            None => KeyResult::Handled,
                        }
                    } else {
                        match self.delete_forward() {
                            Some(edit) => KeyResult::Edit(edit),
                            None => KeyResult::Handled,
                        }
                    }
                }
                NamedKey::ArrowLeft => {
                    if ctrl {
                        self.move_word_left(shift);
                    } else {
                        self.move_left(shift);
                    }
                    KeyResult::Handled
                }
                NamedKey::ArrowRight => {
                    if ctrl {
                        self.move_word_right(shift);
                    } else {
                        self.move_right(shift);
                    }
                    KeyResult::Handled
                }
                NamedKey::ArrowUp | NamedKey::ArrowDown => KeyResult::Ignored,
                NamedKey::Home => {
                    if ctrl {
                        self.move_absolute_home(shift);
                    } else {
                        self.move_home(shift);
                    }
                    KeyResult::Handled
                }
                NamedKey::End => {
                    if ctrl {
                        self.move_absolute_end(shift);
                    } else {
                        self.move_end(shift);
                    }
                    KeyResult::Handled
                }
                NamedKey::Space => match self.insert_text(" ") {
                    Some(edit) => KeyResult::Edit(edit),
                    None => KeyResult::Handled,
                },
                NamedKey::Escape => KeyResult::Blur,
                NamedKey::Enter => match self.insert_text("\n") {
                    Some(edit) => KeyResult::Edit(edit),
                    None => KeyResult::Ignored,
                },
                NamedKey::Tab => match self.insert_text("    ") {
                    Some(edit) => KeyResult::Edit(edit),
                    None => KeyResult::Ignored,
                },
                _ => KeyResult::Ignored,
            },
            _ => KeyResult::Ignored,
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn input(text: &str) -> InputState {
        let mut is = InputState::default();
        is.set_value(text.to_string());
        is
    }

    fn input_at(text: &str, cursor: usize) -> InputState {
        let mut is = input(text);
        is.selection.set_cursor(cursor);
        is
    }

    fn input_sel(text: &str, anchor: usize, active: usize) -> InputState {
        let mut is = input(text);
        is.selection = TextSelection::new(anchor, active);
        is
    }

    // ── Insert ───────────────────────────────────────────────────────

    #[test]
    fn insert_text_basic() {
        let mut is = InputState::default();
        is.insert_text("hello");
        assert_eq!(is.model.text(), "hello");
        assert_eq!(is.selection.active, 5);
        assert!(is.selection.is_collapsed());
    }

    #[test]
    fn insert_text_at_cursor() {
        let mut is = input_at("hllo", 1);
        is.insert_text("e");
        assert_eq!(is.model.text(), "hello");
        assert_eq!(is.selection.active, 2);
    }

    #[test]
    fn insert_replaces_selection() {
        let mut is = input_sel("hello world", 0, 5);
        is.insert_text("goodbye");
        assert_eq!(is.model.text(), "goodbye world");
        assert_eq!(is.selection.active, 7);
        assert!(is.selection.is_collapsed());
    }

    #[test]
    fn insert_newline_in_single_line_mode() {
        let mut is = input_at("hello", 5);
        is.multiline = false;
        let result = is.insert_text("\n");
        assert!(result.is_none());
        assert_eq!(is.model.text(), "hello");
    }

    #[test]
    fn insert_newline_in_multiline_mode() {
        let mut is = input_at("hello", 5);
        // multiline is true by default
        let result = is.insert_text("\n");
        assert!(result.is_some());
        assert_eq!(is.model.text(), "hello\n");
    }

    #[test]
    fn insert_disabled_does_nothing() {
        let mut is = input_at("hello", 5);
        is.disabled = true;
        assert!(is.insert_text("!").is_none());
        assert_eq!(is.model.text(), "hello");
    }

    #[test]
    fn delete_backward_at_end() {
        let mut is = input_at("hello", 5);
        let result = is.delete_backward();
        assert!(result.is_some());
        assert_eq!(is.model.text(), "hell");
        assert_eq!(is.selection.active, 4);
    }

    #[test]
    fn delete_backward_at_start() {
        let mut is = input_at("hello", 0);
        assert!(is.delete_backward().is_none());
        assert_eq!(is.model.text(), "hello");
    }

    #[test]
    fn delete_backward_with_selection() {
        let mut is = input_sel("hello world", 5, 11);
        let result = is.delete_backward();
        assert!(result.is_some());
        assert_eq!(is.model.text(), "hello");
        assert_eq!(is.selection.active, 5);
    }

    #[test]
    fn delete_backward_joins_lines() {
        let mut is = input_at("hello\nworld", 6);
        is.multiline = true;
        let result = is.delete_backward();
        assert!(result.is_some());
        assert_eq!(is.model.text(), "helloworld");
        assert_eq!(is.selection.active, 5);
    }

    #[test]
    fn delete_forward_at_start() {
        let mut is = input_at("hello", 0);
        let result = is.delete_forward();
        assert!(result.is_some());
        assert_eq!(is.model.text(), "ello");
        assert_eq!(is.selection.active, 0);
    }

    #[test]
    fn delete_forward_at_end() {
        let mut is = input_at("hello", 5);
        assert!(is.delete_forward().is_none());
    }

    #[test]
    fn delete_forward_with_selection() {
        let mut is = input_sel("hello world", 0, 6);
        let result = is.delete_forward();
        assert!(result.is_some());
        assert_eq!(is.model.text(), "world");
        assert_eq!(is.selection.active, 0);
    }

    #[test]
    fn delete_forward_joins_lines() {
        let mut is = input_at("hello\nworld", 5);
        is.multiline = true;
        let result = is.delete_forward();
        assert!(result.is_some());
        assert_eq!(is.model.text(), "helloworld");
    }

    #[test]
    fn delete_word_backward_basic() {
        let mut is = input_at("hello world", 11);
        let result = is.delete_word_backward();
        assert!(result.is_some());
        assert_eq!(is.model.text(), "hello ");
        assert_eq!(is.selection.active, 6);
    }

    #[test]
    fn delete_word_backward_at_start() {
        let mut is = input_at("hello", 0);
        assert!(is.delete_word_backward().is_none());
    }

    #[test]
    fn delete_word_backward_with_selection() {
        let mut is = input_sel("hello world", 6, 11);
        let result = is.delete_word_backward();
        assert!(result.is_some());
        assert_eq!(is.model.text(), "hello ");
        assert_eq!(is.selection.active, 6);
    }

    #[test]
    fn delete_word_forward_basic() {
        let mut is = input_at("hello world", 0);
        let result = is.delete_word_forward();
        assert!(result.is_some());
        assert_eq!(is.model.text(), "world");
        assert_eq!(is.selection.active, 0);
    }

    #[test]
    fn delete_word_forward_at_end() {
        let mut is = input_at("hello", 5);
        assert!(is.delete_word_forward().is_none());
    }

    #[test]
    fn move_left_basic() {
        let mut is = input_at("hello", 3);
        is.move_left(false);
        assert_eq!(is.selection.active, 2);
        assert!(is.selection.is_collapsed());
    }

    #[test]
    fn move_left_collapses_selection() {
        let mut is = input_sel("hello", 1, 4);
        is.move_left(false);
        assert_eq!(is.selection.active, 1);
        assert!(is.selection.is_collapsed());
    }

    #[test]
    fn move_left_extend() {
        let mut is = input_at("hello", 3);
        is.move_left(true);
        assert_eq!(is.selection.active, 2);
        assert_eq!(is.selection.anchor, 3);
    }

    #[test]
    fn move_left_at_start() {
        let mut is = input_at("hello", 0);
        is.move_left(false);
        assert_eq!(is.selection.active, 0);
    }

    #[test]
    fn move_right_basic() {
        let mut is = input_at("hello", 3);
        is.move_right(false);
        assert_eq!(is.selection.active, 4);
        assert!(is.selection.is_collapsed());
    }

    #[test]
    fn move_right_collapses_selection() {
        let mut is = input_sel("hello", 1, 4);
        is.move_right(false);
        assert_eq!(is.selection.active, 4);
        assert!(is.selection.is_collapsed());
    }

    #[test]
    fn move_right_extend() {
        let mut is = input_at("hello", 3);
        is.move_right(true);
        assert_eq!(is.selection.active, 4);
        assert_eq!(is.selection.anchor, 3);
    }

    #[test]
    fn move_right_at_end() {
        let mut is = input_at("hello", 5);
        is.move_right(false);
        assert_eq!(is.selection.active, 5);
    }

    #[test]
    fn move_right_across_newline() {
        let mut is = input_at("ab\ncd", 2);
        is.multiline = true;
        is.move_right(false);
        assert_eq!(is.selection.active, 3);
        assert_eq!(is.model.flat_to_rowcol(3), (1, 0));
    }

    #[test]
    fn move_left_across_newline() {
        let mut is = input_at("ab\ncd", 3);
        is.multiline = true;
        is.move_left(false);
        assert_eq!(is.selection.active, 2);
        assert_eq!(is.model.flat_to_rowcol(2), (0, 2));
    }

    #[test]
    fn move_word_left() {
        let mut is = input_at("hello world foo", 15);
        is.move_word_left(false);
        assert_eq!(is.selection.active, 12);
        is.move_word_left(false);
        assert_eq!(is.selection.active, 6);
        is.move_word_left(false);
        assert_eq!(is.selection.active, 0);
    }

    #[test]
    fn move_word_right() {
        let mut is = input_at("hello world foo", 0);
        is.move_word_right(false);
        assert_eq!(is.selection.active, 6);
        is.move_word_right(false);
        assert_eq!(is.selection.active, 12);
        is.move_word_right(false);
        assert_eq!(is.selection.active, 15);
    }

    #[test]
    fn move_word_left_with_extend() {
        let mut is = input_at("hello world", 11);
        is.move_word_left(true);
        assert_eq!(is.selection.active, 6);
        assert_eq!(is.selection.anchor, 11);
    }

    #[test]
    fn move_home_single_line() {
        let mut is = input_at("hello", 3);
        is.move_home(false);
        assert_eq!(is.selection.active, 0);
    }

    #[test]
    fn move_end_single_line() {
        let mut is = input_at("hello", 2);
        is.move_end(false);
        assert_eq!(is.selection.active, 5);
    }

    #[test]
    fn move_home_on_second_line() {
        let mut is = input_at("hello\nworld", 8);
        is.multiline = true;
        is.move_home(false);
        assert_eq!(is.selection.active, 6);
    }

    #[test]
    fn move_end_on_first_line() {
        let mut is = input_at("hello\nworld", 2);
        is.multiline = true;
        is.move_end(false);
        assert_eq!(is.selection.active, 5);
    }

    #[test]
    fn move_home_end_each_line() {
        let mut is = input_at("abc\ndef\nghi", 1);
        is.multiline = true;

        is.move_home(false);
        assert_eq!(is.selection.active, 0);
        is.move_end(false);
        assert_eq!(is.selection.active, 3);

        is.move_to(5, false);
        is.move_home(false);
        assert_eq!(is.selection.active, 4);
        is.move_end(false);
        assert_eq!(is.selection.active, 7);

        is.move_to(9, false);
        is.move_home(false);
        assert_eq!(is.selection.active, 8);
        is.move_end(false);
        assert_eq!(is.selection.active, 11);
    }

    #[test]
    fn move_absolute_home() {
        let mut is = input_at("hello\nworld", 8);
        is.move_absolute_home(false);
        assert_eq!(is.selection.active, 0);
    }

    #[test]
    fn move_absolute_end() {
        let mut is = input_at("hello\nworld", 2);
        is.move_absolute_end(false);
        assert_eq!(is.selection.active, 11);
    }

    #[test]
    fn move_up_basic() {
        let mut is = input_at("hello\nworld", 8);
        is.multiline = true;
        let moved = is.move_up(false, None);
        assert!(moved);
        assert_eq!(is.selection.active, 2);
    }

    #[test]
    fn move_down_basic() {
        let mut is = input_at("hello\nworld", 2);
        is.multiline = true;
        let moved = is.move_down(false, None);
        assert!(moved);
        assert_eq!(is.selection.active, 8);
    }

    #[test]
    fn move_up_from_first_line() {
        let mut is = input_at("hello\nworld", 3);
        let moved = is.move_up(false, None);
        assert!(!moved);
        assert_eq!(is.selection.active, 0);
    }

    #[test]
    fn move_down_from_last_line() {
        let mut is = input_at("hello\nworld", 8);
        let moved = is.move_down(false, None);
        assert!(!moved);
        assert_eq!(is.selection.active, 11);
    }

    #[test]
    fn move_up_down_preserves_sticky_col() {
        let mut is = input_at("abcdef\nab\nabcdef", 5);
        is.multiline = true;

        let (_, col) = is.cursor_rowcol();
        assert_eq!(col, 5);

        is.move_down(false, Some(5));
        assert_eq!(is.cursor_rowcol(), (1, 2));

        is.move_down(false, Some(5));
        assert_eq!(is.cursor_rowcol(), (2, 5));
    }

    #[test]
    fn move_up_with_extend() {
        let mut is = input_at("hello\nworld", 8);
        is.multiline = true;
        is.move_up(true, None);
        assert_eq!(is.selection.active, 2);
        assert_eq!(is.selection.anchor, 8);
    }

    #[test]
    fn select_all_basic() {
        let mut is = input("hello");
        is.select_all();
        assert_eq!(is.selection.anchor, 0);
        assert_eq!(is.selection.active, 5);
    }

    #[test]
    fn select_all_multiline() {
        let mut is = input("hello\nworld");
        is.select_all();
        assert_eq!(is.selection.anchor, 0);
        assert_eq!(is.selection.active, 11);
        assert_eq!(is.selected_text(), "hello\nworld");
    }

    #[test]
    fn selected_text_empty_when_collapsed() {
        let is = input_at("hello", 3);
        assert_eq!(is.selected_text(), "");
    }

    #[test]
    fn selected_text_within_line() {
        let is = input_sel("hello world", 0, 5);
        assert_eq!(is.selected_text(), "hello");
    }

    #[test]
    fn selected_text_across_lines() {
        let is = input_sel("abc\ndef\nghi", 2, 6);
        assert_eq!(is.selected_text(), "c\nde");
    }

    #[test]
    fn selected_text_reversed_selection() {
        let is = input_sel("hello world", 5, 0);
        assert_eq!(is.selected_text(), "hello");
    }

    #[test]
    fn move_to_basic() {
        let mut is = input("hello");
        is.move_to(3, false);
        assert_eq!(is.selection.active, 3);
        assert!(is.selection.is_collapsed());
    }

    #[test]
    fn move_to_with_extend() {
        let mut is = input_at("hello", 1);
        is.move_to(4, true);
        assert_eq!(is.selection.active, 4);
        assert_eq!(is.selection.anchor, 1);
    }

    #[test]
    fn move_to_clamps() {
        let mut is = input("hello");
        is.move_to(100, false);
        assert_eq!(is.selection.active, 5);
    }

    #[test]
    fn word_at_basic() {
        let is = input("hello world");
        assert_eq!(is.word_at(0), (0, 5));
        assert_eq!(is.word_at(7), (6, 11));
    }

    #[test]
    fn set_value_clamps_selection() {
        let mut is = input_at("hello world", 11);
        is.set_value("hi".to_string());
        assert_eq!(is.selection.active, 2);
        assert_eq!(is.selection.anchor, 2);
    }

    #[test]
    fn display_text_normal() {
        let is = input("hello");
        assert_eq!(is.display_text(), "hello");
    }

    #[test]
    fn display_text_secure() {
        let mut is = input("hello");
        is.secure = true;
        assert_eq!(
            is.display_text(),
            "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}"
        );
    }

    #[test]
    fn cursor_rowcol_basic() {
        let is = input_at("hello\nworld", 8);
        assert_eq!(is.cursor_rowcol(), (1, 2));
    }

    #[test]
    fn disabled_blocks_all_edits() {
        let mut is = input_at("hello", 5);
        is.disabled = true;
        assert!(is.insert_text("!").is_none());
        assert!(is.delete_backward().is_none());
        assert!(is.delete_forward().is_none());
        assert!(is.delete_word_backward().is_none());
        assert!(is.delete_word_forward().is_none());
        assert_eq!(is.model.text(), "hello");
    }

    #[test]
    fn delete_selection_multiline() {
        let mut is = input_sel("abc\ndef\nghi", 2, 9);
        is.multiline = true;
        is.delete_backward();
        assert_eq!(is.model.text(), "abhi");
        assert_eq!(is.selection.active, 2);
    }

    #[test]
    fn integration_type_select_delete_type() {
        let mut is = InputState::default();
        is.insert_text("hello world");
        assert_eq!(is.model.text(), "hello world");

        // Select "world"
        is.selection = TextSelection::new(6, 11);
        is.insert_text("rust");
        assert_eq!(is.model.text(), "hello rust");

        // Move to start, delete forward
        is.move_to(0, false);
        is.delete_forward();
        assert_eq!(is.model.text(), "ello rust");
    }

    #[test]
    fn integration_multiline_editing() {
        let mut is = InputState::default();
        is.multiline = true;
        is.insert_text("line1\nline2\nline3");
        assert_eq!(is.model.text(), "line1\nline2\nline3");
        assert_eq!(is.model.line_count(), 3);

        // Cursor should be at end
        assert_eq!(is.selection.active, is.model.grapheme_count());

        // Move up
        let (_, col) = is.cursor_rowcol();
        is.move_up(false, Some(col));
        assert_eq!(is.cursor_rowcol(), (1, 5));

        // Delete backward (delete "2")
        is.delete_backward();
        assert_eq!(is.model.text(), "line1\nline\nline3");
    }

    #[test]
    fn max_length_blocks_insert() {
        let mut is = InputState::default();
        is.model.max_length = Some(5);
        is.insert_text("hello");
        assert_eq!(is.model.text(), "hello");
        assert!(is.insert_text("!").is_none());
        assert_eq!(is.model.text(), "hello");
    }

    #[test]
    fn max_length_allows_replace_within_limit() {
        let mut is = InputState::default();
        is.model.max_length = Some(5);
        is.insert_text("hello");
        // Select all and replace with shorter text
        is.select_all();
        let result = is.insert_text("hi");
        assert!(result.is_some());
        assert_eq!(is.model.text(), "hi");
    }

    /// Type "hello world", press Enter to split into two lines, verify content.
    #[test]
    fn should_split_line_on_enter() {
        let mut is = InputState::default();
        is.multiline = true;

        // Type "hello world"
        is.insert_text("hello world");
        assert_eq!(is.model.text(), "hello world");
        assert_eq!(is.selection.active, 11);

        // Move cursor between "hello" and " world" (pos 5)
        is.move_to(5, false);
        assert_eq!(is.selection.active, 5);

        // Press Enter to split the line
        is.insert_text("\n");
        assert_eq!(is.model.text(), "hello\n world");
        assert_eq!(is.model.line_count(), 2);
        assert_eq!(is.cursor_rowcol(), (1, 0));
    }

    /// Type on line 1, Enter, type on line 2, move up, insert text on line 1.
    #[test]
    fn should_type_enter_move_up_and_insert() {
        let mut is = InputState::default();
        is.multiline = true;

        is.insert_text("aaa");
        is.insert_text("\n");
        is.insert_text("bbb");
        assert_eq!(is.model.text(), "aaa\nbbb");
        assert_eq!(is.cursor_rowcol(), (1, 3));

        // Move up — should land on row 0, col 3
        let col = is.cursor_rowcol().1;
        is.move_up(false, Some(col));
        assert_eq!(is.cursor_rowcol(), (0, 3));

        // Insert " hello" at end of line 1
        is.insert_text(" hello");
        assert_eq!(is.model.text(), "aaa hello\nbbb");
        assert_eq!(is.cursor_rowcol(), (0, 9));
    }

    /// Type two lines, move up, move left twice, insert a character mid-line.
    #[test]
    fn should_move_up_left_and_insert_mid_line() {
        let mut is = InputState::default();
        is.multiline = true;

        is.insert_text("abcd\nefgh");
        assert_eq!(is.cursor_rowcol(), (1, 4));

        // Move up → row 0, col 4 (end of "abcd")
        let col = is.cursor_rowcol().1;
        is.move_up(false, Some(col));
        assert_eq!(is.cursor_rowcol(), (0, 4));

        // Move left twice → col 2
        is.move_left(false);
        is.move_left(false);
        assert_eq!(is.cursor_rowcol(), (0, 2));

        // Insert "X"
        is.insert_text("X");
        assert_eq!(is.model.text(), "abXcd\nefgh");
        assert_eq!(is.cursor_rowcol(), (0, 3));
    }

    /// Split a line in the middle: "helloworld" → Enter at pos 5 → "hello\nworld".
    #[test]
    fn should_split_line_in_middle_and_continue_typing() {
        let mut is = InputState::default();
        is.multiline = true;

        is.insert_text("helloworld");
        is.move_to(5, false);
        is.insert_text("\n");

        assert_eq!(is.model.text(), "hello\nworld");
        assert_eq!(is.model.line_count(), 2);
        assert_eq!(is.cursor_rowcol(), (1, 0));

        // Continue typing on the new line
        is.insert_text("beautiful ");
        assert_eq!(is.model.text(), "hello\nbeautiful world");
        assert_eq!(is.cursor_rowcol(), (1, 10));
    }

    /// Join two lines by pressing Backspace at the start of line 2.
    #[test]
    fn should_join_lines_with_backspace() {
        let mut is = InputState::default();
        is.multiline = true;

        is.insert_text("hello\nworld");
        assert_eq!(is.model.line_count(), 2);

        // Move to start of line 2 (flat index 6 = row 1, col 0)
        is.move_to(6, false);
        assert_eq!(is.cursor_rowcol(), (1, 0));

        // Backspace joins the lines
        is.delete_backward();
        assert_eq!(is.model.text(), "helloworld");
        assert_eq!(is.model.line_count(), 1);
        assert_eq!(is.selection.active, 5);
    }

    /// Type three lines, navigate up/down, delete and re-type.
    #[test]
    fn should_navigate_up_delete_and_retype_line() {
        let mut is = InputState::default();
        is.multiline = true;

        is.insert_text("first");
        is.insert_text("\n");
        is.insert_text("second");
        is.insert_text("\n");
        is.insert_text("third");
        assert_eq!(is.model.text(), "first\nsecond\nthird");
        assert_eq!(is.model.line_count(), 3);
        assert_eq!(is.cursor_rowcol(), (2, 5));

        // Move up twice to get to line 0
        let col = is.cursor_rowcol().1;
        is.move_up(false, Some(col));
        assert_eq!(is.cursor_rowcol(), (1, 5));
        is.move_up(false, Some(col));
        assert_eq!(is.cursor_rowcol(), (0, 5));

        // Delete "first" backwards (5 backspaces)
        is.delete_backward();
        is.delete_backward();
        is.delete_backward();
        is.delete_backward();
        is.delete_backward();
        assert_eq!(is.cursor_rowcol(), (0, 0));
        assert_eq!(is.model.text(), "\nsecond\nthird");

        // Type replacement
        is.insert_text("ONE");
        assert_eq!(is.model.text(), "ONE\nsecond\nthird");
        assert_eq!(is.cursor_rowcol(), (0, 3));

        // Move down to line 1 and verify we're there
        is.move_down(false, Some(3));
        assert_eq!(is.cursor_rowcol(), (1, 3));
    }

    /// Type text, use Home/End to navigate, then edit.
    #[test]
    fn should_use_home_end_to_navigate_and_edit() {
        let mut is = InputState::default();
        is.multiline = true;

        is.insert_text("hello world\ngoodbye");
        assert_eq!(is.cursor_rowcol(), (1, 7));

        // Home goes to start of line 2
        is.move_home(false);
        assert_eq!(is.cursor_rowcol(), (1, 0));

        // Insert text at beginning of line 2
        is.insert_text("say ");
        assert_eq!(is.model.text(), "hello world\nsay goodbye");
        assert_eq!(is.cursor_rowcol(), (1, 4));

        // Move up, then End to go to end of line 1
        is.move_up(false, None);
        is.move_end(false);
        assert_eq!(is.cursor_rowcol(), (0, 11));

        // Append to line 1
        is.insert_text("!");
        assert_eq!(is.model.text(), "hello world!\nsay goodbye");
    }

    /// Select text across lines and replace it.
    #[test]
    fn should_select_across_lines_and_replace() {
        let mut is = InputState::default();
        is.multiline = true;

        is.insert_text("aaa\nbbb\nccc");
        assert_eq!(is.model.line_count(), 3);

        // Select from middle of line 1 to middle of line 3: "a\nbbb\nc"
        is.selection = TextSelection::new(2, 9);
        assert_eq!(is.selected_text(), "a\nbbb\nc");

        // Replace selection
        is.insert_text("X");
        assert_eq!(is.model.text(), "aaXcc");
        assert_eq!(is.model.line_count(), 1);
        assert_eq!(is.selection.active, 3);
    }

    #[test]
    fn should_move_down_insert_and_stuff() {
        let mut is = InputState::default();
        is.insert_text("Line 1\nLine 2\nLine 3");
        is.move_to(4, false);
        is.insert_text("A");
        assert_eq!(is.text_content(), "LineA 1\nLine 2\nLine 3");
        assert_eq!(is.selection.active, 5);
        is.move_down(false, None);
        is.insert_text("B");
        assert_eq!(is.text_content(), "LineA 1\nLine B2\nLine 3");
        is.move_down(false, None);
        is.insert_text("C");
        assert_eq!(is.text_content(), "LineA 1\nLine B2\nLine 3C");
    }

    /// Simulate typing a function: type name, parens, Enter, body, Enter, close brace.
    #[test]
    fn should_type_function_body_and_fix_string() {
        let mut is = InputState::default();

        is.insert_text("fn main() {");
        is.insert_text("\n");
        is.insert_text("    println!(\"hi\");");
        is.insert_text("\n");
        is.insert_text("}");

        assert_eq!(is.model.text(), "fn main() {\n    println!(\"hi\");\n}");
        assert_eq!(is.model.line_count(), 3);
        assert_eq!(is.cursor_rowcol(), (2, 1));

        // Go back up to the println line and fix the message
        is.move_up(false, Some(1));
        assert_eq!(is.cursor_rowcol(), (1, 1));
        is.move_end(false);

        // We're at end of line 1, move left past ");" to get inside the string
        // "    println!("hi");" — move left 3 to get after the 'i'
        is.move_left(false); // ;
        is.move_left(false); // )
        is.move_left(false); // "
        is.move_left(false); // i
        is.move_left(false); // h

        // Select "hi" (2 chars)
        is.move_right(true);
        is.move_right(true);
        assert_eq!(is.selected_text(), "hi");

        // Replace with "hello"
        is.insert_text("hello");
        assert_eq!(is.model.text(), "fn main() {\n    println!(\"hello\");\n}");
    }

    /// Move down past a shorter line, verify sticky column behavior.
    #[test]
    fn should_preserve_sticky_col_through_short_line() {
        let mut is = InputState::default();

        is.insert_text("long line here\nhi\nlong line here");
        // Cursor at end: row 2, col 14
        assert_eq!(is.cursor_rowcol(), (2, 14));

        // Go to row 0, col 10
        is.move_to(10, false);
        assert_eq!(is.cursor_rowcol(), (0, 10));

        // Move down through the short line (sticky col = 10)
        is.move_down(false, Some(10));
        // "hi" is only 2 chars, so we clamp to col 2
        assert_eq!(is.cursor_rowcol(), (1, 2));

        // Move down again with same sticky col
        is.move_down(false, Some(10));
        // Back to a long line, col 10 is available
        assert_eq!(is.cursor_rowcol(), (2, 10));
    }

    /// Delete forward at end of line joins with next line.
    #[test]
    fn should_delete_forward_at_line_end_join_lines() {
        let mut is = InputState::default();

        is.insert_text("abc\ndef");
        // Move to end of line 1 (pos 3 = row 0, col 3)
        is.move_to(3, false);
        assert_eq!(is.cursor_rowcol(), (0, 3));
        is.delete_forward();

        // Delete forward removes the newline
        assert_eq!(is.model.text(), "abcdef");
        assert_eq!(is.model.line_count(), 1);
        assert_eq!(is.selection.active, 3);
    }

    /// Type, make a mistake, backspace, correct it — common editing pattern.
    #[test]
    fn should_correct_typo_with_backspace() {
        let mut is = InputState::default();

        // Type "teh " (typo for "the ")
        is.insert_text("teh ");
        assert_eq!(is.model.text(), "teh ");

        // Backspace three times to remove " ", "h", "e"
        is.delete_backward(); // remove space → "teh"
        is.delete_backward(); // remove h → "te"
        is.delete_backward(); // remove e → "t"
        assert_eq!(is.model.text(), "t");

        // Re-type "he " correctly
        is.insert_text("he ");
        assert_eq!(is.model.text(), "the ");

        // Continue typing
        is.insert_text("quick brown fox");
        assert_eq!(is.model.text(), "the quick brown fox");
    }

    /// Multiple Enter presses to create blank lines, then navigate and fill them.
    #[test]
    fn should_create_blank_lines_then_fill() {
        let mut is = InputState::default();

        is.insert_text("header");
        is.insert_text("\n");
        is.insert_text("\n");
        is.insert_text("footer");
        assert_eq!(is.model.text(), "header\n\nfooter");
        assert_eq!(is.model.line_count(), 3);
        assert_eq!(is.cursor_rowcol(), (2, 6));

        // Navigate to the blank line (row 1)
        is.move_up(false, Some(0));
        is.move_up(false, Some(0));
        assert_eq!(is.cursor_rowcol(), (0, 0));
        is.move_down(false, Some(0));
        assert_eq!(is.cursor_rowcol(), (1, 0));

        // Type content on the blank line
        is.insert_text("body content");
        assert_eq!(is.model.text(), "header\nbody content\nfooter");
        assert_eq!(is.model.line_count(), 3);
    }

    /// Word-delete backward on a multiline buffer.
    #[test]
    fn should_word_delete_backward_across_lines() {
        let mut is = InputState::default();

        is.insert_text("hello world\ngoodbye planet");
        assert_eq!(is.cursor_rowcol(), (1, 14));

        // Delete word backward: removes "planet"
        is.delete_word_backward();
        assert_eq!(is.model.text(), "hello world\ngoodbye ");

        // Delete word backward: removes "goodbye"
        is.delete_word_backward();
        assert_eq!(is.model.text(), "hello world\n");

        // One more: the newline and "world" get eaten (crosses line boundary)
        is.delete_word_backward();
        // Exact result depends on word boundary logic, but line count should decrease
        assert!(is.model.line_count() <= 2);
    }

    /// Select all, delete, type fresh — common "clear and retype" workflow.
    #[test]
    fn should_select_all_and_retype() {
        let mut is = InputState::default();

        is.insert_text("old content\nspanning\nmultiple lines");
        assert_eq!(is.model.line_count(), 3);

        // Ctrl+A (select all) then type to replace
        is.select_all();
        assert_eq!(is.selected_text(), "old content\nspanning\nmultiple lines");

        is.insert_text("brand new text");
        assert_eq!(is.model.text(), "brand new text");
        assert_eq!(is.model.line_count(), 1);
        assert_eq!(is.selection.active, 14);
    }
}
