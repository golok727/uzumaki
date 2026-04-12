use unicode_segmentation::UnicodeSegmentation;

use crate::text_buffer::TextBuffer;

// ── TextModel ────────────────────────────────────────────────────────
// Mid-level editing abstraction over TextBuffer.
// Provides position-based editing operations (delete_backward, delete_word, etc.)
// but knows nothing about selection — that lives in InputState.
// Future: undo/redo history lives here.

pub struct TextModel {
    pub buffer: TextBuffer,
    pub max_length: Option<usize>,
}

impl Default for TextModel {
    fn default() -> Self {
        Self::new()
    }
}

impl TextModel {
    pub fn new() -> Self {
        Self {
            buffer: TextBuffer::new(),
            max_length: None,
        }
    }

    // ── Delegated accessors ──────────────────────────────────────────

    pub fn text(&self) -> String {
        self.buffer.text()
    }

    pub fn line_count(&self) -> usize {
        self.buffer.line_count()
    }

    pub fn line(&self, row: usize) -> &str {
        self.buffer.line(row)
    }

    pub fn grapheme_count(&self) -> usize {
        self.buffer.grapheme_count()
    }

    pub fn line_grapheme_count(&self, row: usize) -> usize {
        self.buffer.line_grapheme_count(row)
    }

    pub fn flat_to_rowcol(&self, idx: usize) -> (usize, usize) {
        self.buffer.flat_to_rowcol(idx)
    }

    pub fn rowcol_to_flat(&self, row: usize, col: usize) -> usize {
        self.buffer.rowcol_to_flat(row, col)
    }

    pub fn text_in_range(&self, start: usize, end: usize) -> String {
        self.buffer.text_in_range(start, end)
    }

    // ── Editing operations ───────────────────────────────────────────
    // All take/return flat grapheme positions. No selection awareness.

    /// Insert `text` at `pos`. `selection_len` is subtracted from the current count
    /// for max_length checking (caller should delete selection first).
    /// Returns new cursor position, or None if blocked by max_length.
    pub fn insert(&mut self, pos: usize, text: &str, selection_len: usize) -> Option<usize> {
        if text.is_empty() {
            return None;
        }
        if let Some(max) = self.max_length {
            let current = self.buffer.grapheme_count() - selection_len;
            let insert_count = text.graphemes(true).count();
            if current + insert_count > max {
                return None;
            }
        }
        let inserted = self.buffer.insert(pos, text);
        Some(pos + inserted)
    }

    /// Delete one grapheme before `pos`. Returns new cursor position.
    pub fn delete_backward(&mut self, pos: usize) -> Option<usize> {
        if pos == 0 {
            return None;
        }
        self.buffer.delete(pos - 1, 1);
        Some(pos - 1)
    }

    /// Delete one grapheme at `pos`. Returns the same position, or None if at end.
    pub fn delete_forward(&mut self, pos: usize) -> Option<usize> {
        if pos >= self.buffer.grapheme_count() {
            return None;
        }
        self.buffer.delete(pos, 1);
        Some(pos)
    }

    /// Delete from word boundary backward to `pos`. Returns new cursor position.
    pub fn delete_word_backward(&mut self, pos: usize) -> Option<usize> {
        if pos == 0 {
            return None;
        }
        let word_start = self.find_word_start(pos);
        let len = pos - word_start;
        if len == 0 {
            return None;
        }
        self.buffer.delete(word_start, len);
        Some(word_start)
    }

    /// Delete from `pos` to next word boundary. Returns the same position.
    pub fn delete_word_forward(&mut self, pos: usize) -> Option<usize> {
        let count = self.buffer.grapheme_count();
        if pos >= count {
            return None;
        }
        let word_end = self.find_word_end(pos);
        let len = word_end - pos;
        if len == 0 {
            return None;
        }
        self.buffer.delete(pos, len);
        Some(pos)
    }

    /// Delete a range [start, end).
    pub fn delete_range(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }
        self.buffer.delete(start, end - start);
    }

    pub fn set_value(&mut self, value: String) {
        let current = self.buffer.text();
        if current == value {
            return;
        }
        self.buffer.set_value(&value);
    }

    // ── Word boundary helpers ────────────────────────────────────────

    pub fn find_word_start(&self, pos: usize) -> usize {
        let text = self.buffer.text();
        let graphemes: Vec<&str> = text.graphemes(true).collect();
        let mut p = pos;
        while p > 0 && graphemes[p - 1].chars().all(char::is_whitespace) {
            p -= 1;
        }
        while p > 0 && !graphemes[p - 1].chars().all(char::is_whitespace) {
            p -= 1;
        }
        p
    }

    pub fn find_word_end(&self, pos: usize) -> usize {
        let text = self.buffer.text();
        let graphemes: Vec<&str> = text.graphemes(true).collect();
        let count = graphemes.len();
        let mut p = pos;
        while p < count && !graphemes[p].chars().all(char::is_whitespace) {
            p += 1;
        }
        while p < count && graphemes[p].chars().all(char::is_whitespace) {
            p += 1;
        }
        p
    }

    /// Find line boundaries around a flat grapheme position.
    /// Returns (start, end) where end is past the last grapheme of the line
    /// (not including the \n itself).
    pub fn line_at(&self, grapheme_idx: usize) -> (usize, usize) {
        let (row, _col) = self.buffer.flat_to_rowcol(grapheme_idx);
        let start = self.buffer.rowcol_to_flat(row, 0);
        let line_len = self.buffer.line_grapheme_count(row);
        let end = start + line_len;
        (start, end)
    }

    /// Find word boundaries around a position.
    pub fn word_at(&self, grapheme_idx: usize) -> (usize, usize) {
        let text = self.buffer.text();
        let graphemes: Vec<&str> = text.graphemes(true).collect();
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
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn model(text: &str) -> TextModel {
        let mut m = TextModel::new();
        m.set_value(text.to_string());
        m
    }

    #[test]
    fn insert_at_position() {
        let mut m = TextModel::new();
        let pos = m.insert(0, "hello", 0).unwrap();
        assert_eq!(pos, 5);
        assert_eq!(m.text(), "hello");

        let pos = m.insert(5, " world", 0).unwrap();
        assert_eq!(pos, 11);
        assert_eq!(m.text(), "hello world");
    }

    #[test]
    fn insert_with_newline() {
        let mut m = TextModel::new();
        let pos = m.insert(0, "hello\nworld", 0).unwrap();
        assert_eq!(pos, 11);
        assert_eq!(m.line_count(), 2);
        assert_eq!(m.line(0), "hello");
        assert_eq!(m.line(1), "world");
    }

    #[test]
    fn insert_respects_max_length() {
        let mut m = TextModel::new();
        m.max_length = Some(5);
        assert!(m.insert(0, "hello", 0).is_some());
        assert!(m.insert(5, "!", 0).is_none());
    }

    #[test]
    fn delete_backward_basic() {
        let mut m = model("hello");
        let pos = m.delete_backward(5).unwrap();
        assert_eq!(pos, 4);
        assert_eq!(m.text(), "hell");
    }

    #[test]
    fn delete_backward_at_start() {
        let mut m = model("hello");
        assert!(m.delete_backward(0).is_none());
        assert_eq!(m.text(), "hello");
    }

    #[test]
    fn delete_backward_joins_lines() {
        let mut m = model("hello\nworld");
        let pos = m.delete_backward(6).unwrap();
        assert_eq!(pos, 5);
        assert_eq!(m.text(), "helloworld");
    }

    #[test]
    fn delete_forward_basic() {
        let mut m = model("hello");
        let pos = m.delete_forward(0).unwrap();
        assert_eq!(pos, 0);
        assert_eq!(m.text(), "ello");
    }

    #[test]
    fn delete_forward_at_end() {
        let mut m = model("hello");
        assert!(m.delete_forward(5).is_none());
    }

    #[test]
    fn delete_forward_joins_lines() {
        let mut m = model("hello\nworld");
        let pos = m.delete_forward(5).unwrap();
        assert_eq!(pos, 5);
        assert_eq!(m.text(), "helloworld");
    }

    #[test]
    fn delete_word_backward_basic() {
        let mut m = model("hello world");
        let pos = m.delete_word_backward(11).unwrap();
        assert_eq!(pos, 6);
        assert_eq!(m.text(), "hello ");
    }

    #[test]
    fn delete_word_forward_basic() {
        let mut m = model("hello world");
        let pos = m.delete_word_forward(0).unwrap();
        assert_eq!(pos, 0);
        assert_eq!(m.text(), "world");
    }

    #[test]
    fn delete_range() {
        let mut m = model("hello world");
        m.delete_range(5, 11);
        assert_eq!(m.text(), "hello");
    }

    #[test]
    fn word_at_middle() {
        let m = model("hello world foo");
        assert_eq!(m.word_at(7), (6, 11));
    }

    #[test]
    fn word_at_start() {
        let m = model("hello world");
        assert_eq!(m.word_at(0), (0, 5));
    }

    #[test]
    fn find_word_boundaries() {
        let m = model("hello world foo");
        assert_eq!(m.find_word_start(8), 6);
        assert_eq!(m.find_word_end(6), 12);
    }
}
