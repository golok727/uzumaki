use unicode_segmentation::UnicodeSegmentation;

// todo we can use cosmic text TextBuffer instead ?
// Pure text storage. No selection, no editing policy — just insert/delete at positions.

pub struct TextBuffer {
    /// Line-based storage. Each entry is one line (no trailing \n).
    /// Always has at least one entry (empty string for empty document).
    lines: Vec<String>,
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
        }
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn line(&self, row: usize) -> &str {
        &self.lines[row]
    }

    pub fn line_grapheme_count(&self, row: usize) -> usize {
        self.lines[row].graphemes(true).count()
    }

    /// Total grapheme count across all lines (including \n separators).
    pub fn grapheme_count(&self) -> usize {
        let mut count = 0;
        for (i, line) in self.lines.iter().enumerate() {
            count += line.graphemes(true).count();
            if i < self.lines.len() - 1 {
                count += 1; // \n separator
            }
        }
        count
    }

    // ── Coordinate conversion ────────────────────────────────────────

    pub fn flat_to_rowcol(&self, idx: usize) -> (usize, usize) {
        let mut remaining = idx;
        for (row, line) in self.lines.iter().enumerate() {
            let line_len = line.graphemes(true).count();
            if remaining <= line_len && (row == self.lines.len() - 1 || remaining < line_len + 1) {
                return (row, remaining.min(line_len));
            }
            remaining -= line_len + 1;
        }
        let last = self.lines.len() - 1;
        (last, self.line_grapheme_count(last))
    }

    pub fn rowcol_to_flat(&self, row: usize, col: usize) -> usize {
        let mut flat = 0;
        for r in 0..row.min(self.lines.len()) {
            flat += self.line_grapheme_count(r) + 1;
        }
        let clamped_row = row.min(self.lines.len() - 1);
        flat + col.min(self.line_grapheme_count(clamped_row))
    }

    fn grapheme_to_byte_in_line(line: &str, grapheme_idx: usize) -> usize {
        line.grapheme_indices(true)
            .nth(grapheme_idx)
            .map(|(i, _)| i)
            .unwrap_or(line.len())
    }

    // ── Mutations ────────────────────────────────────────────────────

    /// Insert `text` at flat grapheme position `pos`.
    /// Returns the number of graphemes inserted.
    pub fn insert(&mut self, pos: usize, text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }
        let (row, col) = self.flat_to_rowcol(pos);
        let insert_lines: Vec<&str> = text.split('\n').collect();

        if insert_lines.len() == 1 {
            let line = &self.lines[row];
            let byte_pos = Self::grapheme_to_byte_in_line(line, col);
            let mut new_line = String::with_capacity(line.len() + insert_lines[0].len());
            new_line.push_str(&line[..byte_pos]);
            new_line.push_str(insert_lines[0]);
            new_line.push_str(&line[byte_pos..]);
            self.lines[row] = new_line;
        } else {
            let line = &self.lines[row];
            let byte_pos = Self::grapheme_to_byte_in_line(line, col);
            let before = line[..byte_pos].to_string();
            let after = line[byte_pos..].to_string();

            let mut first = before;
            first.push_str(insert_lines[0]);
            self.lines[row] = first;

            for (i, &ins_line) in insert_lines.iter().enumerate().skip(1) {
                if i < insert_lines.len() - 1 {
                    self.lines.insert(row + i, ins_line.to_string());
                } else {
                    let mut last = ins_line.to_string();
                    last.push_str(&after);
                    self.lines.insert(row + i, last);
                }
            }
        }

        text.graphemes(true).count()
    }

    /// Delete `len` graphemes starting at flat position `start`.
    pub fn delete(&mut self, start: usize, len: usize) {
        if len == 0 {
            return;
        }
        let end = start + len;
        let (sr, sc) = self.flat_to_rowcol(start);
        let (er, ec) = self.flat_to_rowcol(end);

        if sr == er {
            let line = &self.lines[sr];
            let byte_start = Self::grapheme_to_byte_in_line(line, sc);
            let byte_end = Self::grapheme_to_byte_in_line(line, ec);
            let mut new_line = String::with_capacity(line.len() - (byte_end - byte_start));
            new_line.push_str(&line[..byte_start]);
            new_line.push_str(&line[byte_end..]);
            self.lines[sr] = new_line;
        } else {
            let first = &self.lines[sr];
            let last = &self.lines[er];
            let byte_start = Self::grapheme_to_byte_in_line(first, sc);
            let byte_end = Self::grapheme_to_byte_in_line(last, ec);
            let mut merged = String::new();
            merged.push_str(&first[..byte_start]);
            merged.push_str(&last[byte_end..]);
            self.lines[sr] = merged;
            self.lines.drain((sr + 1)..=er);
        }
    }

    /// Replace entire buffer contents.
    pub fn set_value(&mut self, value: &str) {
        self.lines = value.split('\n').map(|s| s.to_string()).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
    }

    /// Extract text in the range [start, end) as flat grapheme indices.
    pub fn text_in_range(&self, start: usize, end: usize) -> String {
        if start >= end {
            return String::new();
        }
        let (sr, sc) = self.flat_to_rowcol(start);
        let (er, ec) = self.flat_to_rowcol(end);

        if sr == er {
            let line = &self.lines[sr];
            let byte_start = Self::grapheme_to_byte_in_line(line, sc);
            let byte_end = Self::grapheme_to_byte_in_line(line, ec);
            return line[byte_start..byte_end].to_string();
        }

        let mut result = String::new();
        let first = &self.lines[sr];
        let byte_start = Self::grapheme_to_byte_in_line(first, sc);
        result.push_str(&first[byte_start..]);
        result.push('\n');
        for r in (sr + 1)..er {
            result.push_str(&self.lines[r]);
            result.push('\n');
        }
        let last = &self.lines[er];
        let byte_end = Self::grapheme_to_byte_in_line(last, ec);
        result.push_str(&last[..byte_end]);
        result
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(text: &str) -> TextBuffer {
        let mut b = TextBuffer::new();
        b.set_value(text);
        b
    }

    // ── Coordinate conversion ────────────────────────────────────────

    #[test]
    fn flat_to_rowcol_single_line() {
        let b = buf("hello");
        assert_eq!(b.flat_to_rowcol(0), (0, 0));
        assert_eq!(b.flat_to_rowcol(3), (0, 3));
        assert_eq!(b.flat_to_rowcol(5), (0, 5));
    }

    #[test]
    fn flat_to_rowcol_multiline() {
        let b = buf("ab\ncd\nef");
        assert_eq!(b.flat_to_rowcol(0), (0, 0));
        assert_eq!(b.flat_to_rowcol(2), (0, 2));
        assert_eq!(b.flat_to_rowcol(3), (1, 0));
        assert_eq!(b.flat_to_rowcol(5), (1, 2));
        assert_eq!(b.flat_to_rowcol(6), (2, 0));
        assert_eq!(b.flat_to_rowcol(8), (2, 2));
    }

    #[test]
    fn rowcol_to_flat_roundtrip() {
        let b = buf("hello\nworld\nfoo");
        for i in 0..=b.grapheme_count() {
            let (r, c) = b.flat_to_rowcol(i);
            assert_eq!(
                b.rowcol_to_flat(r, c),
                i,
                "round-trip failed for flat index {i}"
            );
        }
    }

    // ── Insert ───────────────────────────────────────────────────────

    #[test]
    fn insert_at_start() {
        let mut b = buf("world");
        let n = b.insert(0, "hello ");
        assert_eq!(n, 6);
        assert_eq!(b.text(), "hello world");
    }

    #[test]
    fn insert_in_middle() {
        let mut b = buf("hllo");
        b.insert(1, "e");
        assert_eq!(b.text(), "hello");
    }

    #[test]
    fn insert_newline_splits_line() {
        let mut b = buf("helloworld");
        b.insert(5, "\n");
        assert_eq!(b.text(), "hello\nworld");
        assert_eq!(b.line_count(), 2);
    }

    #[test]
    fn insert_multiline_text() {
        let mut b = TextBuffer::new();
        b.insert(0, "line1\nline2\nline3");
        assert_eq!(b.text(), "line1\nline2\nline3");
        assert_eq!(b.line_count(), 3);
    }

    // ── Delete ───────────────────────────────────────────────────────

    #[test]
    fn delete_single_char() {
        let mut b = buf("hello");
        b.delete(4, 1);
        assert_eq!(b.text(), "hell");
    }

    #[test]
    fn delete_across_newline() {
        let mut b = buf("hello\nworld");
        b.delete(5, 1); // delete the \n
        assert_eq!(b.text(), "helloworld");
        assert_eq!(b.line_count(), 1);
    }

    #[test]
    fn delete_multiline_range() {
        let mut b = buf("abc\ndef\nghi");
        b.delete(2, 7); // "c\ndef\ng"
        assert_eq!(b.text(), "abhi");
    }

    #[test]
    fn delete_nothing() {
        let mut b = buf("hello");
        b.delete(2, 0);
        assert_eq!(b.text(), "hello");
    }

    // ── text_in_range ────────────────────────────────────────────────

    #[test]
    fn text_in_range_same_line() {
        let b = buf("hello world");
        assert_eq!(b.text_in_range(0, 5), "hello");
    }

    #[test]
    fn text_in_range_across_lines() {
        let b = buf("abc\ndef\nghi");
        assert_eq!(b.text_in_range(2, 6), "c\nde");
    }

    // ── set_value ────────────────────────────────────────────────────

    #[test]
    fn set_value_splits_lines() {
        let mut b = TextBuffer::new();
        b.set_value("line1\nline2\nline3");
        assert_eq!(b.line_count(), 3);
        assert_eq!(b.line(0), "line1");
        assert_eq!(b.line(1), "line2");
        assert_eq!(b.line(2), "line3");
    }

    // ── Empty buffer ─────────────────────────────────────────────────

    #[test]
    fn empty_buffer_state() {
        let b = TextBuffer::new();
        assert_eq!(b.text(), "");
        assert_eq!(b.grapheme_count(), 0);
        assert_eq!(b.line_count(), 1);
        assert_eq!(b.line(0), "");
        assert_eq!(b.flat_to_rowcol(0), (0, 0));
    }

    // ── Trailing newline ─────────────────────────────────────────────

    #[test]
    fn trailing_newline() {
        let mut b = TextBuffer::new();
        b.insert(0, "hello");
        b.insert(5, "\n");
        assert_eq!(b.text(), "hello\n");
        assert_eq!(b.line_count(), 2);
        assert_eq!(b.line(0), "hello");
        assert_eq!(b.line(1), "");
    }
}
