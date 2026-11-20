//! Byte-oriented lexer.
//!
//! The lexer understands the union of the original and community syntaxes:
//!
//! * `//`, `#`, `--` line comments and `/* ... */` block comments
//! * `"..."` and `'...'` strings (optionally with community escape handling)
//! * `[label]` declarations and `@label` references
//! * bare words / `0xHH` byte literals

use crate::diag::Report;
use crate::source::Source;
use crate::span::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Word(String),
    String(Vec<u8>),
    LabelDecl(String),
    LabelRef(String),
    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LexOptions {
    /// Treat backslashes in strings as ordinary bytes instead of escapes.
    pub raw_strings: bool,
}

pub fn lex(source: &Source, options: LexOptions, report: &mut Report) -> Vec<Token> {
    Lexer {
        bytes: &source.bytes,
        pos: 0,
        options,
    }
    .tokenize(report)
}

struct Lexer<'a> {
    bytes: &'a [u8],
    pos: usize,
    options: LexOptions,
}

fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n' | 0x0b | 0x0c)
}

fn hex_val(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

impl<'a> Lexer<'a> {
    fn tokenize(mut self, report: &mut Report) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token(report);
            let is_eof = matches!(token.kind, TokenKind::Eof);
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
    }

    fn peek(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.pos + offset).copied()
    }

    fn starts_with(&self, needle: &[u8]) -> bool {
        self.bytes
            .get(self.pos..self.pos + needle.len())
            .map(|slice| slice == needle)
            .unwrap_or(false)
    }

    fn is_comment_start(&self) -> bool {
        self.starts_with(b"//")
            || self.starts_with(b"--")
            || self.starts_with(b"/*")
            || self.peek(0) == Some(b'#')
    }

    fn skip_line_comment(&mut self) {
        while let Some(byte) = self.peek(0) {
            self.pos += 1;
            if byte == b'\n' {
                break;
            }
        }
    }

    fn skip_trivia(&mut self, report: &mut Report) {
        loop {
            while self.peek(0).map(is_space).unwrap_or(false) {
                self.pos += 1;
            }
            if self.starts_with(b"//") || self.starts_with(b"--") {
                self.skip_line_comment();
                continue;
            }
            if self.peek(0) == Some(b'#') {
                self.skip_line_comment();
                continue;
            }
            if self.starts_with(b"/*") {
                let start = self.pos;
                self.pos += 2;
                let mut closed = false;
                while self.pos < self.bytes.len() {
                    if self.starts_with(b"*/") {
                        self.pos += 2;
                        closed = true;
                        break;
                    }
                    self.pos += 1;
                }
                if !closed {
                    report.error(
                        Span::new(start, self.bytes.len()),
                        "unterminated block comment",
                    );
                }
                continue;
            }
            break;
        }
    }

    fn next_token(&mut self, report: &mut Report) -> Token {
        self.skip_trivia(report);
        let start = self.pos;
        if self.pos >= self.bytes.len() {
            return Token {
                kind: TokenKind::Eof,
                span: Span::new(start, start),
            };
        }
        let byte = self.bytes[self.pos];
        match byte {
            b'"' | b'\'' => self.lex_string(byte, start, report),
            b'[' => self.lex_label(start, report),
            b'@' => self.lex_ref(start, report),
            _ => self.lex_word(start, report),
        }
    }

    fn lex_word(&mut self, start: usize, report: &mut Report) -> Token {
        while self.pos < self.bytes.len()
            && !is_space(self.bytes[self.pos])
            && !self.is_comment_start()
        {
            self.pos += 1;
        }
        if self.pos == start {
            // Defensive: never loop forever on an unexpected byte.
            self.pos += 1;
            report.error(
                Span::new(start, self.pos),
                format!("unexpected byte 0x{:02x}", self.bytes[start]),
            );
            return Token {
                kind: TokenKind::Word(String::new()),
                span: Span::new(start, self.pos),
            };
        }
        let raw = &self.bytes[start..self.pos];
        let word = match std::str::from_utf8(raw) {
            Ok(word) => word.to_owned(),
            Err(_) => {
                report.error(
                    Span::new(start, self.pos),
                    "token is not valid UTF-8 (only strings may contain arbitrary bytes)",
                );
                String::from_utf8_lossy(raw).into_owned()
            }
        };
        Token {
            kind: TokenKind::Word(word),
            span: Span::new(start, self.pos),
        }
    }

    fn lex_string(&mut self, quote: u8, start: usize, report: &mut Report) -> Token {
        self.pos += 1;
        let mut out = Vec::new();
        while self.pos < self.bytes.len() {
            let byte = self.bytes[self.pos];
            self.pos += 1;
            if byte == quote {
                return Token {
                    kind: TokenKind::String(out),
                    span: Span::new(start, self.pos),
                };
            }
            if byte == b'\\' && !self.options.raw_strings {
                if self.pos >= self.bytes.len() {
                    break;
                }
                let escape = self.bytes[self.pos];
                self.pos += 1;
                match escape {
                    b'n' => out.push(b'\n'),
                    b't' => out.push(b'\t'),
                    b'r' => out.push(b'\r'),
                    b'\\' => out.push(b'\\'),
                    b'"' => out.push(b'"'),
                    b'\'' => out.push(b'\''),
                    // The community compiler's default escape arm keeps the
                    // escaped character itself, so "\0" means ASCII '0'.
                    b'0' => out.push(b'0'),
                    b'x' => {
                        let hi = self.peek(0).and_then(hex_val);
                        let lo = self.peek(1).and_then(hex_val);
                        match (hi, lo) {
                            (Some(hi), Some(lo)) => {
                                self.pos += 2;
                                out.push((hi << 4) | lo);
                            }
                            _ => {
                                report.error(
                                    Span::new(self.pos.saturating_sub(2), self.pos),
                                    "`\\x` escape needs exactly two hexadecimal digits",
                                );
                            }
                        }
                    }
                    other => out.push(other),
                }
                continue;
            }
            out.push(byte);
        }
        report.error(Span::new(start, self.pos), "unterminated string literal");
        Token {
            kind: TokenKind::String(out),
            span: Span::new(start, self.pos),
        }
    }

    fn lex_label(&mut self, start: usize, report: &mut Report) -> Token {
        self.pos += 1; // '['
        let name_start = self.pos;
        while self.pos < self.bytes.len() {
            let byte = self.bytes[self.pos];
            if byte == b']' {
                let raw = &self.bytes[name_start..self.pos];
                self.pos += 1;
                return self.label_token(raw, start, report);
            }
            if is_space(byte) {
                break;
            }
            self.pos += 1;
        }
        let raw = &self.bytes[name_start..self.pos];
        report.error(
            Span::new(start, self.pos),
            "unterminated label declaration (expected `]` before whitespace)",
        );
        self.label_token(raw, start, report)
    }

    fn label_token(&self, raw: &[u8], start: usize, report: &mut Report) -> Token {
        let name = match std::str::from_utf8(raw) {
            Ok(name) if !name.is_empty() => name.to_owned(),
            Ok(_) => {
                report.error(Span::new(start, self.pos), "empty label name");
                String::new()
            }
            Err(_) => {
                report.error(Span::new(start, self.pos), "label name is not valid UTF-8");
                String::from_utf8_lossy(raw).into_owned()
            }
        };
        Token {
            kind: TokenKind::LabelDecl(name),
            span: Span::new(start, self.pos),
        }
    }

    fn lex_ref(&mut self, start: usize, report: &mut Report) -> Token {
        self.pos += 1; // '@'
        let name_start = self.pos;
        while self.pos < self.bytes.len()
            && !is_space(self.bytes[self.pos])
            && !self.is_comment_start()
        {
            self.pos += 1;
        }
        let raw = &self.bytes[name_start..self.pos];
        let name = match std::str::from_utf8(raw) {
            Ok(name) if !name.is_empty() => name.to_owned(),
            Ok(_) => {
                report.error(Span::new(start, self.pos), "empty label reference");
                String::new()
            }
            Err(_) => {
                report.error(
                    Span::new(start, self.pos),
                    "label reference is not valid UTF-8",
                );
                String::from_utf8_lossy(raw).into_owned()
            }
        };
        Token {
            kind: TokenKind::LabelRef(name),
            span: Span::new(start, self.pos),
        }
    }
}
