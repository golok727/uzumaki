use std::time::{Duration, Instant};

use parley::PlainEditor;
use winit::keyboard::{Key, NamedKey};

use crate::text::{TextBrush, TextRenderer};

#[derive(Clone, Debug)]
pub enum EditKind {
    Insert,
    InsertFromPaste,
    DeleteBackward,
    DeleteForward,
    DeleteWordBackward,
    DeleteWordForward,
    DeleteByCut,
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
    pub blink_reset: Instant,
    pub disabled: bool,
    pub secure: bool,
    pub multiline: bool,
    pub max_length: Option<usize>,
    pub preedit: Option<PreeditState>,
}

#[derive(Clone, Debug)]
pub struct PreeditState {
    pub text: String,
    pub cursor: Option<(usize, usize)>,
}

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
            blink_reset: Instant::now(),
            disabled: false,
            secure: false,
            multiline: true,
            max_length: None,
            preedit: None,
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

    pub fn insert_text(&mut self, text: &str, renderer: &mut TextRenderer) -> Option<EditEvent> {
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
            let insert_count = input.chars().count();
            if current + insert_count > max {
                return None;
            }
        }

        let generation = self.editor.generation();
        self.with_driver(renderer, |d| d.insert_or_replace_selection(&input));
        if self.text_changed(generation) {
            self.reset_blink();
            Some(EditEvent {
                kind: EditKind::Insert,
                inserted: Some(input),
            })
        } else {
            None
        }
    }

    pub fn paste_text(&mut self, text: &str, renderer: &mut TextRenderer) -> Option<EditEvent> {
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

        let generation = self.editor.generation();
        self.with_driver(renderer, |d| d.insert_or_replace_selection(&input));
        if self.text_changed(generation) {
            self.reset_blink();
            Some(EditEvent {
                kind: EditKind::InsertFromPaste,
                inserted: Some(input),
            })
        } else {
            None
        }
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
        self.with_driver(renderer, |d| d.delete_selection());
        self.reset_blink();
        Some((
            text,
            EditEvent {
                kind: EditKind::DeleteByCut,
                inserted: None,
            },
        ))
    }

    pub fn delete_backward(&mut self, renderer: &mut TextRenderer) -> Option<EditEvent> {
        if self.disabled {
            return None;
        }
        let generation = self.editor.generation();
        self.with_driver(renderer, |d| d.backdelete());
        if self.text_changed(generation) {
            self.reset_blink();
            Some(EditEvent {
                kind: EditKind::DeleteBackward,
                inserted: None,
            })
        } else {
            None
        }
    }

    pub fn delete_forward(&mut self, renderer: &mut TextRenderer) -> Option<EditEvent> {
        if self.disabled {
            return None;
        }
        let generation = self.editor.generation();
        self.with_driver(renderer, |d| d.delete());
        if self.text_changed(generation) {
            self.reset_blink();
            Some(EditEvent {
                kind: EditKind::DeleteForward,
                inserted: None,
            })
        } else {
            None
        }
    }

    pub fn delete_word_backward(&mut self, renderer: &mut TextRenderer) -> Option<EditEvent> {
        if self.disabled {
            return None;
        }
        let generation = self.editor.generation();
        self.with_driver(renderer, |d| d.backdelete_word());
        if self.text_changed(generation) {
            self.reset_blink();
            Some(EditEvent {
                kind: EditKind::DeleteWordBackward,
                inserted: None,
            })
        } else {
            None
        }
    }

    pub fn delete_word_forward(&mut self, renderer: &mut TextRenderer) -> Option<EditEvent> {
        if self.disabled {
            return None;
        }
        let generation = self.editor.generation();
        self.with_driver(renderer, |d| d.delete_word());
        if self.text_changed(generation) {
            self.reset_blink();
            Some(EditEvent {
                kind: EditKind::DeleteWordForward,
                inserted: None,
            })
        } else {
            None
        }
    }

    pub fn move_left(&mut self, extend: bool, renderer: &mut TextRenderer) {
        if extend {
            self.with_driver(renderer, |d| d.select_left());
        } else {
            self.with_driver(renderer, |d| d.move_left());
        }
        self.reset_blink();
    }

    pub fn move_right(&mut self, extend: bool, renderer: &mut TextRenderer) {
        if extend {
            self.with_driver(renderer, |d| d.select_right());
        } else {
            self.with_driver(renderer, |d| d.move_right());
        }
        self.reset_blink();
    }

    pub fn move_word_left(&mut self, extend: bool, renderer: &mut TextRenderer) {
        if extend {
            self.with_driver(renderer, |d| d.select_word_left());
        } else {
            self.with_driver(renderer, |d| d.move_word_left());
        }
        self.reset_blink();
    }

    pub fn move_word_right(&mut self, extend: bool, renderer: &mut TextRenderer) {
        if extend {
            self.with_driver(renderer, |d| d.select_word_right());
        } else {
            self.with_driver(renderer, |d| d.move_word_right());
        }
        self.reset_blink();
    }

    pub fn move_up(&mut self, extend: bool, renderer: &mut TextRenderer) {
        if extend {
            self.with_driver(renderer, |d| d.select_up());
        } else {
            self.with_driver(renderer, |d| d.move_up());
        }
    }

    pub fn move_down(&mut self, extend: bool, renderer: &mut TextRenderer) {
        if extend {
            self.with_driver(renderer, |d| d.select_down());
        } else {
            self.with_driver(renderer, |d| d.move_down());
        }
    }

    pub fn move_home(&mut self, extend: bool, renderer: &mut TextRenderer) {
        if extend {
            self.with_driver(renderer, |d| d.select_to_line_start());
        } else {
            self.with_driver(renderer, |d| d.move_to_line_start());
        }
        self.reset_blink();
    }

    pub fn move_end(&mut self, extend: bool, renderer: &mut TextRenderer) {
        if extend {
            self.with_driver(renderer, |d| d.select_to_line_end());
        } else {
            self.with_driver(renderer, |d| d.move_to_line_end());
        }
        self.reset_blink();
    }

    pub fn move_absolute_home(&mut self, extend: bool, renderer: &mut TextRenderer) {
        if extend {
            self.with_driver(renderer, |d| d.select_to_text_start());
        } else {
            self.with_driver(renderer, |d| d.move_to_text_start());
        }
        self.reset_blink();
    }

    pub fn move_absolute_end(&mut self, extend: bool, renderer: &mut TextRenderer) {
        if extend {
            self.with_driver(renderer, |d| d.select_to_text_end());
        } else {
            self.with_driver(renderer, |d| d.move_to_text_end());
        }
        self.reset_blink();
    }

    pub fn move_to_point(&mut self, x: f32, y: f32, renderer: &mut TextRenderer) {
        self.with_driver(renderer, |d| d.move_to_point(x, y));
        self.reset_blink();
    }

    pub fn extend_selection_to_point(&mut self, x: f32, y: f32, renderer: &mut TextRenderer) {
        self.with_driver(renderer, |d| d.extend_selection_to_point(x, y));
    }

    pub fn select_word_at_point(&mut self, x: f32, y: f32, renderer: &mut TextRenderer) {
        self.with_driver(renderer, |d| d.select_word_at_point(x, y));
        self.reset_blink();
    }

    pub fn select_line_at_point(&mut self, x: f32, y: f32, renderer: &mut TextRenderer) {
        self.with_driver(renderer, |d| d.select_line_at_point(x, y));
        self.reset_blink();
    }

    pub fn select_all(&mut self, renderer: &mut TextRenderer) {
        self.with_driver(renderer, |d| d.select_all());
        self.reset_blink();
    }

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
        self.insert_text(text, renderer)
    }

    pub fn set_value(&mut self, value: &str) {
        if self.editor.raw_text() != value {
            self.editor.set_text(value);
        }
    }

    pub fn set_width(&mut self, width: Option<f32>) {
        self.editor.set_width(width);
    }

    pub fn set_scale(&mut self, scale: f32) {
        self.editor.set_scale(scale);
    }

    pub fn reset_blink(&mut self) {
        self.blink_reset = Instant::now();
    }

    pub fn blink_visible(&self, focused: bool, window_focused: bool) -> bool {
        if !focused || !window_focused {
            return false;
        }
        let elapsed = self.blink_phase_elapsed_ms();
        elapsed < Self::BLINK_ON_MS
    }

    pub fn next_blink_toggle_in(&self, focused: bool, window_focused: bool) -> Option<Duration> {
        if !focused || !window_focused {
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
                    return KeyResult::Ignored;
                }
                match self.insert_text(ch, renderer) {
                    Some(edit) => KeyResult::Edit(edit),
                    None => KeyResult::Handled,
                }
            }
            Key::Named(named) => match named {
                NamedKey::Backspace => {
                    if ctrl {
                        match self.delete_word_backward(renderer) {
                            Some(edit) => KeyResult::Edit(edit),
                            None => KeyResult::Handled,
                        }
                    } else {
                        match self.delete_backward(renderer) {
                            Some(edit) => KeyResult::Edit(edit),
                            None => KeyResult::Handled,
                        }
                    }
                }
                NamedKey::Delete => {
                    if ctrl {
                        match self.delete_word_forward(renderer) {
                            Some(edit) => KeyResult::Edit(edit),
                            None => KeyResult::Handled,
                        }
                    } else {
                        match self.delete_forward(renderer) {
                            Some(edit) => KeyResult::Edit(edit),
                            None => KeyResult::Handled,
                        }
                    }
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
                NamedKey::Space => match self.insert_text(" ", renderer) {
                    Some(edit) => KeyResult::Edit(edit),
                    None => KeyResult::Handled,
                },
                NamedKey::Escape => KeyResult::Blur,
                NamedKey::Enter => match self.insert_text("\n", renderer) {
                    Some(edit) => KeyResult::Edit(edit),
                    None => KeyResult::Ignored,
                },
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
        assert!(!is.blink_visible(false, true));
    }

    #[test]
    fn blink_not_visible_window_unfocused() {
        let is = InputState::new();
        assert!(!is.blink_visible(true, false));
    }

    #[test]
    fn next_blink_toggle_is_absent_when_unfocused() {
        let is = InputState::new();
        assert!(is.next_blink_toggle_in(false, true).is_none());
    }

    #[test]
    fn next_blink_toggle_matches_visible_phase() {
        let mut is = InputState::new();
        is.blink_reset = Instant::now() - Duration::from_millis(200);
        let next = is.next_blink_toggle_in(true, true).unwrap();
        assert!((329..=330).contains(&next.as_millis()));
    }

    #[test]
    fn next_blink_toggle_matches_hidden_phase() {
        let mut is = InputState::new();
        is.blink_reset = Instant::now() - Duration::from_millis(700);
        let next = is.next_blink_toggle_in(true, true).unwrap();
        assert!((359..=360).contains(&next.as_millis()));
    }
}
