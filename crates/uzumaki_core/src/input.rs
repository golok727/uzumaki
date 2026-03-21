use std::time::Instant;
use unicode_segmentation::UnicodeSegmentation;
use winit::keyboard::{Key, NamedKey};

// ── Selection ────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Selection {
    /// Anchor point (where selection started)
    pub anchor: usize,
    /// Active point / cursor position
    pub active: usize,
}

impl Selection {
    pub fn new() -> Self {
        Self {
            anchor: 0,
            active: 0,
        }
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

// ── Edit result ──────────────────────────────────────────────────────

pub struct InputEdit {
    pub input_type: &'static str,
    pub data: Option<String>,
}

pub enum KeyResult {
    Edit(InputEdit),
    Blur,
    Handled,
    Ignored,
    /// Multiline vertical navigation: direction -1=up, +1=down; extend=shift held.
    /// The caller must resolve the target grapheme using the text renderer and call `move_to`.
    VerticalNav { direction: i32, extend: bool },
}

// ── InputState ───────────────────────────────────────────────────────

pub struct InputState {
    pub text: String,
    pub placeholder: String,
    pub selection: Selection,
    pub scroll_offset: f32,
    pub scroll_offset_y: f32,
    pub focused: bool,
    pub blink_reset: Instant,
    pub disabled: bool,
    pub max_length: Option<usize>,
    pub multiline: bool,
    pub secure: bool,
    /// Preserved X coordinate for vertical navigation (sticky column).
    /// Set on first Up/Down press, cleared on any horizontal/edit action.
    pub sticky_x: Option<f32>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            placeholder: String::new(),
            selection: Selection::new(),
            scroll_offset: 0.0,
            scroll_offset_y: 0.0,
            focused: false,
            blink_reset: Instant::now(),
            disabled: false,
            max_length: None,
            multiline: false,
            secure: false,
            sticky_x: None,
        }
    }

    pub fn grapheme_count(&self) -> usize {
        self.text.graphemes(true).count()
    }

    fn grapheme_to_byte(&self, idx: usize) -> usize {
        self.text
            .grapheme_indices(true)
            .nth(idx)
            .map(|(i, _)| i)
            .unwrap_or(self.text.len())
    }

    fn delete_selection(&mut self) {
        if self.selection.is_collapsed() {
            return;
        }
        let start = self.selection.start();
        let end = self.selection.end();
        let byte_start = self.grapheme_to_byte(start);
        let byte_end = self.grapheme_to_byte(end);
        self.text.replace_range(byte_start..byte_end, "");
        self.selection.set_cursor(start);
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
            "\u{2022}".repeat(self.grapheme_count())
        } else {
            self.text.clone()
        }
    }

    pub fn set_value(&mut self, value: String) {
        // Same value — no-op, preserves cursor and scroll (controlled input echo-back)
        if self.text == value {
            return;
        }
        self.text = value;
        let count = self.grapheme_count();
        // Clamp cursor instead of resetting — keeps position stable for small edits
        if self.selection.active > count {
            self.selection.active = count;
        }
        if self.selection.anchor > count {
            self.selection.anchor = count;
        }
    }

    pub fn insert_text(&mut self, ch: &str) -> Option<InputEdit> {
        self.sticky_x = None;
        if self.disabled {
            return None;
        }
        if let Some(max) = self.max_length {
            let current = self.grapheme_count()
                - (self.selection.end() - self.selection.start());
            let insert = ch.graphemes(true).count();
            if current + insert > max {
                return None;
            }
        }
        self.delete_selection();
        let byte_pos = self.grapheme_to_byte(self.selection.active);
        self.text.insert_str(byte_pos, ch);
        let inserted = ch.graphemes(true).count();
        self.selection.active += inserted;
        self.selection.anchor = self.selection.active;
        self.reset_blink();
        Some(InputEdit {
            input_type: "insertText",
            data: Some(ch.to_string()),
        })
    }

    pub fn backspace(&mut self) -> Option<InputEdit> {
        self.sticky_x = None;
        if self.disabled {
            return None;
        }
        if !self.selection.is_collapsed() {
            self.delete_selection();
            self.reset_blink();
            return Some(InputEdit {
                input_type: "deleteContentBackward",
                data: None,
            });
        }
        if self.selection.active == 0 {
            return None;
        }
        let end_byte = self.grapheme_to_byte(self.selection.active);
        self.selection.active -= 1;
        self.selection.anchor = self.selection.active;
        let start_byte = self.grapheme_to_byte(self.selection.active);
        self.text.replace_range(start_byte..end_byte, "");
        self.reset_blink();
        Some(InputEdit {
            input_type: "deleteContentBackward",
            data: None,
        })
    }

    pub fn delete(&mut self) -> Option<InputEdit> {
        self.sticky_x = None;
        if self.disabled {
            return None;
        }
        if !self.selection.is_collapsed() {
            self.delete_selection();
            self.reset_blink();
            return Some(InputEdit {
                input_type: "deleteContentForward",
                data: None,
            });
        }
        let count = self.grapheme_count();
        if self.selection.active >= count {
            return None;
        }
        let start_byte = self.grapheme_to_byte(self.selection.active);
        let end_byte = self.grapheme_to_byte(self.selection.active + 1);
        self.text.replace_range(start_byte..end_byte, "");
        self.reset_blink();
        Some(InputEdit {
            input_type: "deleteContentForward",
            data: None,
        })
    }

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
        let count = self.grapheme_count();
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

    pub fn move_home(&mut self, extend: bool) {
        self.sticky_x = None;
        if self.multiline {
            self.move_line_start(extend);
        } else {
            self.selection.active = 0;
            if !extend {
                self.selection.anchor = 0;
            }
            self.reset_blink();
        }
    }

    pub fn move_end(&mut self, extend: bool) {
        self.sticky_x = None;
        if self.multiline {
            self.move_line_end(extend);
        } else {
            let count = self.grapheme_count();
            self.selection.active = count;
            if !extend {
                self.selection.anchor = count;
            }
            self.reset_blink();
        }
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
        let count = self.grapheme_count();
        self.selection.active = count;
        if !extend {
            self.selection.anchor = count;
        }
        self.reset_blink();
    }

    /// Move cursor to start of current line (bounded by \n or start of text).
    pub fn move_line_start(&mut self, extend: bool) {
        self.sticky_x = None;
        let graphemes: Vec<&str> = self.text.graphemes(true).collect();
        let mut pos = self.selection.active;
        while pos > 0 && graphemes[pos - 1] != "\n" {
            pos -= 1;
        }
        self.selection.active = pos;
        if !extend {
            self.selection.anchor = pos;
        }
        self.reset_blink();
    }

    /// Move cursor to end of current line (bounded by \n or end of text).
    pub fn move_line_end(&mut self, extend: bool) {
        self.sticky_x = None;
        let graphemes: Vec<&str> = self.text.graphemes(true).collect();
        let count = graphemes.len();
        let mut pos = self.selection.active;
        while pos < count && graphemes[pos] != "\n" {
            pos += 1;
        }
        self.selection.active = pos;
        if !extend {
            self.selection.anchor = pos;
        }
        self.reset_blink();
    }

    /// Move cursor to a specific grapheme index (used by caller for vertical nav and mouse clicks).
    pub fn move_to(&mut self, pos: usize, extend: bool) {
        self.sticky_x = None;
        let count = self.grapheme_count();
        self.selection.active = pos.min(count);
        if !extend {
            self.selection.anchor = self.selection.active;
        }
        self.reset_blink();
    }

    pub fn move_word_left(&mut self, extend: bool) {
        self.sticky_x = None;
        let graphemes: Vec<&str> = self.text.graphemes(true).collect();
        let mut pos = self.selection.active;
        // Skip whitespace
        while pos > 0 && graphemes[pos - 1].chars().all(char::is_whitespace) {
            pos -= 1;
        }
        // Skip word chars
        while pos > 0 && !graphemes[pos - 1].chars().all(char::is_whitespace) {
            pos -= 1;
        }
        self.selection.active = pos;
        if !extend {
            self.selection.anchor = pos;
        }
        self.reset_blink();
    }

    pub fn move_word_right(&mut self, extend: bool) {
        self.sticky_x = None;
        let graphemes: Vec<&str> = self.text.graphemes(true).collect();
        let count = graphemes.len();
        let mut pos = self.selection.active;
        // Skip word chars
        while pos < count && !graphemes[pos].chars().all(char::is_whitespace) {
            pos += 1;
        }
        // Skip whitespace
        while pos < count && graphemes[pos].chars().all(char::is_whitespace) {
            pos += 1;
        }
        self.selection.active = pos;
        if !extend {
            self.selection.anchor = pos;
        }
        self.reset_blink();
    }

    pub fn delete_word_backward(&mut self) -> Option<InputEdit> {
        self.sticky_x = None;
        if self.disabled {
            return None;
        }
        if !self.selection.is_collapsed() {
            self.delete_selection();
            self.reset_blink();
            return Some(InputEdit {
                input_type: "deleteWordBackward",
                data: None,
            });
        }
        if self.selection.active == 0 {
            return None;
        }
        let end = self.selection.active;
        let graphemes: Vec<&str> = self.text.graphemes(true).collect();
        let mut pos = end;
        // Skip whitespace
        while pos > 0 && graphemes[pos - 1].chars().all(char::is_whitespace) {
            pos -= 1;
        }
        // Skip word chars
        while pos > 0 && !graphemes[pos - 1].chars().all(char::is_whitespace) {
            pos -= 1;
        }
        let byte_start = self.grapheme_to_byte(pos);
        let byte_end = self.grapheme_to_byte(end);
        self.text.replace_range(byte_start..byte_end, "");
        self.selection.set_cursor(pos);
        self.reset_blink();
        Some(InputEdit {
            input_type: "deleteWordBackward",
            data: None,
        })
    }

    pub fn delete_word_forward(&mut self) -> Option<InputEdit> {
        self.sticky_x = None;
        if self.disabled {
            return None;
        }
        if !self.selection.is_collapsed() {
            self.delete_selection();
            self.reset_blink();
            return Some(InputEdit {
                input_type: "deleteWordForward",
                data: None,
            });
        }
        let count = self.grapheme_count();
        if self.selection.active >= count {
            return None;
        }
        let start = self.selection.active;
        let graphemes: Vec<&str> = self.text.graphemes(true).collect();
        let mut pos = start;
        // Skip word chars
        while pos < count && !graphemes[pos].chars().all(char::is_whitespace) {
            pos += 1;
        }
        // Skip whitespace
        while pos < count && graphemes[pos].chars().all(char::is_whitespace) {
            pos += 1;
        }
        let byte_start = self.grapheme_to_byte(start);
        let byte_end = self.grapheme_to_byte(pos);
        self.text.replace_range(byte_start..byte_end, "");
        self.reset_blink();
        Some(InputEdit {
            input_type: "deleteWordForward",
            data: None,
        })
    }

    pub fn select_all(&mut self) {
        self.sticky_x = None;
        self.selection.anchor = 0;
        self.selection.active = self.grapheme_count();
        self.reset_blink();
    }

    pub fn word_at(&self, grapheme_idx: usize) -> (usize, usize) {
        let graphemes: Vec<&str> = self.text.graphemes(true).collect();
        if graphemes.is_empty() {
            return (0, 0);
        }
        let idx = grapheme_idx.min(graphemes.len().saturating_sub(1));

        let mut start = idx;
        while start > 0 && !graphemes[start - 1].chars().all(char::is_whitespace) {
            start -= 1;
        }

        let mut end = idx;
        while end < graphemes.len() && !graphemes[end].chars().all(char::is_whitespace) {
            end += 1;
        }

        (start, end)
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

    pub fn handle_key(&mut self, key: &Key, modifiers: u32) -> KeyResult {
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
                        match self.backspace() {
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
                        match self.delete() {
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
                NamedKey::ArrowUp => {
                    if self.multiline {
                        KeyResult::VerticalNav { direction: -1, extend: shift }
                    } else {
                        // Single-line: up goes to start
                        self.move_home(shift);
                        KeyResult::Handled
                    }
                }
                NamedKey::ArrowDown => {
                    if self.multiline {
                        KeyResult::VerticalNav { direction: 1, extend: shift }
                    } else {
                        // Single-line: down goes to end
                        self.move_end(shift);
                        KeyResult::Handled
                    }
                }
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
                NamedKey::Enter => {
                    if self.multiline {
                        match self.insert_text("\n") {
                            Some(edit) => KeyResult::Edit(edit),
                            None => KeyResult::Handled,
                        }
                    } else {
                        KeyResult::Ignored
                    }
                }
                NamedKey::Tab => {
                    if self.multiline {
                        match self.insert_text("    ") {
                            Some(edit) => KeyResult::Edit(edit),
                            None => KeyResult::Handled,
                        }
                    } else {
                        KeyResult::Ignored
                    }
                }
                _ => KeyResult::Ignored,
            },
            _ => KeyResult::Ignored,
        }
    }
}
