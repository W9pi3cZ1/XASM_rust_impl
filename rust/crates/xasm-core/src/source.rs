//! Source files and line/column lookup.
//!
//! XASM source is byte oriented. Strings may contain arbitrary bytes and UTF-8
//! text, so the compiler never assumes that the whole file is valid UTF-8.

use crate::span::Span;

#[derive(Clone, Debug)]
pub struct Source {
    pub path: String,
    pub bytes: Vec<u8>,
    line_starts: Vec<usize>,
}

impl Source {
    pub fn new(path: impl Into<String>, bytes: Vec<u8>) -> Self {
        let mut line_starts = Vec::with_capacity(64);
        line_starts.push(0);
        for (idx, byte) in bytes.iter().enumerate() {
            if *byte == b'\n' {
                line_starts.push(idx + 1);
            }
        }
        Self {
            path: path.into(),
            bytes,
            line_starts,
        }
    }

    /// Returns the 1-based line and column for a byte offset.
    ///
    /// Columns count Unicode scalar values so that diagnostics line up for
    /// UTF-8 source. Invalid UTF-8 is counted lossily.
    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.bytes.len());
        let line_idx = match self.line_starts.binary_search(&offset) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };
        let line_start = self.line_starts[line_idx];
        let col = String::from_utf8_lossy(&self.bytes[line_start..offset])
            .chars()
            .count()
            + 1;
        (line_idx + 1, col)
    }

    /// Returns the bytes of a 1-based line without the trailing newline.
    pub fn line(&self, line: usize) -> &[u8] {
        if line == 0 || line > self.line_starts.len() {
            return &[];
        }
        let start = self.line_starts[line - 1];
        let end = self
            .line_starts
            .get(line)
            .copied()
            .unwrap_or(self.bytes.len());
        let mut slice = &self.bytes[start..end];
        while let [rest @ .., b'\n' | b'\r'] = slice {
            slice = rest;
        }
        slice
    }

    pub fn span_line_col(&self, span: Span) -> (usize, usize) {
        self.line_col(span.start)
    }
}
