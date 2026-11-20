//! Diagnostics with source snippets.

use crate::source::Source;
use crate::span::Span;
use std::fmt::Write as _;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Error,
    Warning,
    Note,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Error => "error",
            Level::Warning => "warning",
            Level::Note => "note",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub level: Level,
    pub span: Option<Span>,
    pub message: String,
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn new(level: Level, span: Option<Span>, message: impl Into<String>) -> Self {
        Self {
            level,
            span,
            message: message.into(),
            notes: Vec::new(),
        }
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}

#[derive(Clone, Debug, Default)]
pub struct Report {
    pub diagnostics: Vec<Diagnostic>,
}

impl Report {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn error(&mut self, span: impl Into<Option<Span>>, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::new(Level::Error, span.into(), message.into()));
    }

    pub fn warning(&mut self, span: impl Into<Option<Span>>, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::new(Level::Warning, span.into(), message.into()));
    }

    pub fn note(&mut self, span: impl Into<Option<Span>>, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::new(Level::Note, span.into(), message.into()));
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diag| diag.level == Level::Error)
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diag| diag.level == Level::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diag| diag.level == Level::Warning)
            .count()
    }

    pub fn extend(&mut self, other: Report) {
        self.diagnostics.extend(other.diagnostics);
    }

    /// Render every diagnostic against `source`.
    pub fn render(&self, source: &Source) -> String {
        let mut out = String::new();
        for diag in &self.diagnostics {
            let _ = writeln!(out, "{}: {}", diag.level.as_str(), diag.message);
            if let Some(span) = diag.span {
                let (line, col) = source.line_col(span.start);
                let _ = writeln!(out, "  --> {}:{}:{}", source.path, line, col);
                let text = source.line(line);
                let text_lossy = String::from_utf8_lossy(text);
                let line_no = line.to_string();
                let pad = " ".repeat(line_no.len());
                let _ = writeln!(out, "{} |", pad);
                let _ = writeln!(out, "{} | {}", line_no, text_lossy);
                let (_, end_col) = source.line_col(span.end.min(source.bytes.len()));
                let caret_len = if end_col > col {
                    (end_col - col).min(80)
                } else {
                    1
                };
                let _ = writeln!(
                    out,
                    "{} | {}{}",
                    pad,
                    " ".repeat(col.saturating_sub(1)),
                    "^".repeat(caret_len)
                );
            }
            for note in &diag.notes {
                let _ = writeln!(out, "  = note: {}", note);
            }
        }
        out
    }
}
