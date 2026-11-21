//! XPL lexer.

use xasm_core::diag::Report;
use xasm_core::{Source, Span};

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Int { value: u64, hex: bool },
    Str(Vec<u8>),

    Let,
    Fn,
    Return,
    If,
    Else,
    While,
    Print,
    Println,
    Putchar,
    Puthex,
    PuthexAt,
    ReadInt,
    ReadChar,
    ReadHex,
    Peek,
    Poke,
    True,
    False,

    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semi,
    Comma,

    Assign,
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    Amp,
    OrOr,
    Bang,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,

    Eof,
}

impl TokenKind {
    pub fn name(&self) -> &'static str {
        match self {
            TokenKind::Ident(_) => "identifier",
            TokenKind::Int { .. } => "integer",
            TokenKind::Str(_) => "string",
            TokenKind::Let => "`let`",
            TokenKind::Fn => "`fn`",
            TokenKind::Return => "`return`",
            TokenKind::If => "`if`",
            TokenKind::Else => "`else`",
            TokenKind::While => "`while`",
            TokenKind::Print => "`print`",
            TokenKind::Println => "`println`",
            TokenKind::Putchar => "`putchar`",
            TokenKind::Puthex => "`puthex`",
            TokenKind::PuthexAt => "`puthex_at`",
            TokenKind::ReadInt => "`read_int`",
            TokenKind::ReadChar => "`read_char`",
            TokenKind::ReadHex => "`read_hex`",
            TokenKind::Peek => "`peek`",
            TokenKind::Poke => "`poke`",
            TokenKind::True => "`true`",
            TokenKind::False => "`false`",
            TokenKind::LParen => "`(`",
            TokenKind::RParen => "`)`",
            TokenKind::LBrace => "`{`",
            TokenKind::RBrace => "`}`",
            TokenKind::LBracket => "`[`",
            TokenKind::RBracket => "`]`",
            TokenKind::Semi => "`;`",
            TokenKind::Comma => "`,`",
            TokenKind::Assign => "`=`",
            TokenKind::EqEq => "`==`",
            TokenKind::NotEq => "`!=`",
            TokenKind::Lt => "`<`",
            TokenKind::Le => "`<=`",
            TokenKind::Gt => "`>`",
            TokenKind::Ge => "`>=`",
            TokenKind::AndAnd => "`&&`",
            TokenKind::Amp => "`&`",
            TokenKind::OrOr => "`||`",
            TokenKind::Bang => "`!`",
            TokenKind::Plus => "`+`",
            TokenKind::Minus => "`-`",
            TokenKind::Star => "`*`",
            TokenKind::Slash => "`/`",
            TokenKind::Percent => "`%`",
            TokenKind::Eof => "end of file",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub fn lex(source: &Source, report: &mut Report) -> Vec<Token> {
    Lexer {
        bytes: &source.bytes,
        pos: 0,
    }
    .tokenize(report)
}

struct Lexer<'a> {
    bytes: &'a [u8],
    pos: usize,
}

fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n' | 0x0b | 0x0c)
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
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
            let eof = matches!(token.kind, TokenKind::Eof);
            tokens.push(token);
            if eof {
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

    fn skip_trivia(&mut self, report: &mut Report) {
        loop {
            while self.peek(0).map(is_space).unwrap_or(false) {
                self.pos += 1;
            }
            if self.starts_with(b"//") {
                while let Some(byte) = self.peek(0) {
                    self.pos += 1;
                    if byte == b'\n' {
                        break;
                    }
                }
                continue;
            }
            if self.peek(0) == Some(b'#') {
                while let Some(byte) = self.peek(0) {
                    self.pos += 1;
                    if byte == b'\n' {
                        break;
                    }
                }
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
        if is_ident_start(byte) {
            return self.lex_ident(start);
        }
        if byte.is_ascii_digit() {
            return self.lex_number(start, report);
        }
        match byte {
            b'"' | b'\'' => self.lex_string(byte, start, report),
            b'(' => self.single(TokenKind::LParen, start),
            b')' => self.single(TokenKind::RParen, start),
            b'{' => self.single(TokenKind::LBrace, start),
            b'}' => self.single(TokenKind::RBrace, start),
            b'[' => self.single(TokenKind::LBracket, start),
            b']' => self.single(TokenKind::RBracket, start),
            b';' => self.single(TokenKind::Semi, start),
            b',' => self.single(TokenKind::Comma, start),
            b'+' => self.single(TokenKind::Plus, start),
            b'-' => self.single(TokenKind::Minus, start),
            b'*' => self.single(TokenKind::Star, start),
            b'%' => self.single(TokenKind::Percent, start),
            b'/' => self.single(TokenKind::Slash, start),
            b'!' => {
                self.pos += 1;
                if self.peek(0) == Some(b'=') {
                    self.pos += 1;
                    self.token(TokenKind::NotEq, start)
                } else {
                    self.token(TokenKind::Bang, start)
                }
            }
            b'=' => {
                self.pos += 1;
                if self.peek(0) == Some(b'=') {
                    self.pos += 1;
                    self.token(TokenKind::EqEq, start)
                } else {
                    self.token(TokenKind::Assign, start)
                }
            }
            b'<' => {
                self.pos += 1;
                if self.peek(0) == Some(b'=') {
                    self.pos += 1;
                    self.token(TokenKind::Le, start)
                } else {
                    self.token(TokenKind::Lt, start)
                }
            }
            b'>' => {
                self.pos += 1;
                if self.peek(0) == Some(b'=') {
                    self.pos += 1;
                    self.token(TokenKind::Ge, start)
                } else {
                    self.token(TokenKind::Gt, start)
                }
            }
            b'&' => {
                self.pos += 1;
                if self.peek(0) == Some(b'&') {
                    self.pos += 1;
                    self.token(TokenKind::AndAnd, start)
                } else {
                    self.token(TokenKind::Amp, start)
                }
            }
            b'|' => {
                self.pos += 1;
                if self.peek(0) == Some(b'|') {
                    self.pos += 1;
                    self.token(TokenKind::OrOr, start)
                } else {
                    report.error(
                        Span::new(start, self.pos),
                        "unknown operator `|`; did you mean `||`?",
                    );
                    self.next_token(report)
                }
            }
            _ => {
                self.pos += 1;
                report.error(
                    Span::new(start, self.pos),
                    format!("unexpected character `{}`", char::from(byte)),
                );
                self.next_token(report)
            }
        }
    }

    fn single(&mut self, kind: TokenKind, start: usize) -> Token {
        self.pos += 1;
        self.token(kind, start)
    }

    fn token(&self, kind: TokenKind, start: usize) -> Token {
        Token {
            kind,
            span: Span::new(start, self.pos),
        }
    }

    fn lex_ident(&mut self, start: usize) -> Token {
        self.pos += 1;
        while self.peek(0).map(is_ident_continue).unwrap_or(false) {
            self.pos += 1;
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).unwrap_or("");
        let kind = match text {
            "let" => TokenKind::Let,
            "fn" => TokenKind::Fn,
            "return" => TokenKind::Return,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "print" => TokenKind::Print,
            "println" => TokenKind::Println,
            "putchar" => TokenKind::Putchar,
            "puthex" => TokenKind::Puthex,
            "puthex_at" => TokenKind::PuthexAt,
            "read_int" => TokenKind::ReadInt,
            "read_char" => TokenKind::ReadChar,
            "read_hex" => TokenKind::ReadHex,
            "peek" => TokenKind::Peek,
            "poke" => TokenKind::Poke,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            _ => TokenKind::Ident(text.to_owned()),
        };
        self.token(kind, start)
    }

    fn lex_number(&mut self, start: usize, report: &mut Report) -> Token {
        let mut hex = false;
        let mut value: u64 = 0;
        let mut any = false;
        if self.peek(0) == Some(b'0') && matches!(self.peek(1), Some(b'x') | Some(b'X')) {
            hex = true;
            self.pos += 2;
            while let Some(byte) = self.peek(0) {
                if byte == b'_' {
                    self.pos += 1;
                    continue;
                }
                let Some(digit) = hex_val(byte) else {
                    break;
                };
                any = true;
                value = value.saturating_mul(16).saturating_add(u64::from(digit));
                self.pos += 1;
            }
            if !any {
                report.error(
                    Span::new(start, self.pos),
                    "hexadecimal literal needs at least one digit",
                );
            }
        } else {
            while let Some(byte) = self.peek(0) {
                if byte == b'_' {
                    self.pos += 1;
                    continue;
                }
                if !byte.is_ascii_digit() {
                    break;
                }
                any = true;
                value = value
                    .saturating_mul(10)
                    .saturating_add(u64::from(byte - b'0'));
                self.pos += 1;
            }
            if !any {
                report.error(Span::new(start, self.pos), "invalid number");
            }
        }
        self.token(TokenKind::Int { value, hex }, start)
    }

    fn lex_string(&mut self, quote: u8, start: usize, report: &mut Report) -> Token {
        self.pos += 1;
        let mut out = Vec::new();
        while self.pos < self.bytes.len() {
            let byte = self.bytes[self.pos];
            self.pos += 1;
            if byte == quote {
                return self.token(TokenKind::Str(out), start);
            }
            if byte == b'\\' {
                if self.pos >= self.bytes.len() {
                    break;
                }
                let escape = self.bytes[self.pos];
                self.pos += 1;
                match escape {
                    b'n' => out.push(b'\n'),
                    b't' => out.push(b'\t'),
                    b'r' => out.push(b'\r'),
                    b'0' => out.push(0),
                    b'\\' => out.push(b'\\'),
                    b'"' => out.push(b'"'),
                    b'\'' => out.push(b'\''),
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
        self.token(TokenKind::Str(out), start)
    }
}
