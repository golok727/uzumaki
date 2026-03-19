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
}

// ── InputState ───────────────────────────────────────────────────────

pub struct InputState {
    pub text: String,
    pub placeholder: String,
    pub selection: Selection,
    pub scroll_offset: f32,
    pub focused: bool,
    pub blink_reset: Instant,
    pub disabled: bool,
    pub max_length: Option<usize>,
    pub multiline: bool,
    pub secure: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            placeholder: String::new(),
            selection: Selection::new(),
            scroll_offset: 0.0,
            focused: false,
            blink_reset: Instant::now(),
            disabled: false,
            max_length: None,
            multiline: false,
            secure: false,
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
        self.text = value;
        let count = self.grapheme_count();
        self.selection.set_cursor(count);
        self.scroll_offset = 0.0;
    }

    pub fn insert_text(&mut self, ch: &str) -> Option<InputEdit> {
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
        self.selection.active = 0;
        if !extend {
            self.selection.anchor = 0;
        }
        self.reset_blink();
    }

    pub fn move_end(&mut self, extend: bool) {
        let count = self.grapheme_count();
        self.selection.active = count;
        if !extend {
            self.selection.anchor = count;
        }
        self.reset_blink();
    }

    pub fn select_all(&mut self) {
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
                NamedKey::Backspace => match self.backspace() {
                    Some(edit) => KeyResult::Edit(edit),
                    None => KeyResult::Handled,
                },
                NamedKey::Delete => match self.delete() {
                    Some(edit) => KeyResult::Edit(edit),
                    None => KeyResult::Handled,
                },
                NamedKey::ArrowLeft => {
                    self.move_left(shift);
                    KeyResult::Handled
                }
                NamedKey::ArrowRight => {
                    self.move_right(shift);
                    KeyResult::Handled
                }
                NamedKey::Home => {
                    self.move_home(shift);
                    KeyResult::Handled
                }
                NamedKey::End => {
                    self.move_end(shift);
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
                _ => KeyResult::Ignored,
            },
            _ => KeyResult::Ignored,
        }
    }
}
