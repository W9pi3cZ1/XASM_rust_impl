//! Token stream to byte-emitting items.

use crate::diag::Report;
use crate::lexer::{Token, TokenKind};
use crate::mnemonics;
use crate::span::Span;

#[derive(Clone, Debug)]
pub enum ItemKind {
    /// A literal byte, either `0xHH` or a bare hexadecimal byte.
    Byte(u8),
    /// An instruction mnemonic.  It still emits exactly one byte.
    Op { opcode: u8, mnemonic: &'static str },
    /// Raw bytes produced by a string literal.
    Data(Vec<u8>),
    /// `[name]`
    Label(String),
    /// `@name`, encoded as four big-endian bytes.
    Ref(String),
}

#[derive(Clone, Debug)]
pub struct Item {
    pub kind: ItemKind,
    pub span: Span,
}

#[derive(Clone, Debug, Default)]
pub struct ParsedProgram {
    pub items: Vec<Item>,
    pub ops: usize,
}

impl ParsedProgram {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

pub fn parse(tokens: &[Token], report: &mut Report) -> ParsedProgram {
    let mut program = ParsedProgram::default();
    for token in tokens {
        match &token.kind {
            TokenKind::Eof => break,
            TokenKind::Word(word) => {
                if word.is_empty() {
                    continue;
                }
                if let Some(mnemonic) = mnemonics::lookup(word) {
                    program.ops += 1;
                    program.items.push(Item {
                        kind: ItemKind::Op {
                            opcode: mnemonic.opcode,
                            mnemonic: mnemonic.name,
                        },
                        span: token.span,
                    });
                    continue;
                }
                match parse_byte_word(word) {
                    Ok(byte) => program.items.push(Item {
                        kind: ItemKind::Byte(byte),
                        span: token.span,
                    }),
                    Err(message) => report.error(token.span, message),
                }
            }
            TokenKind::String(data) => program.items.push(Item {
                kind: ItemKind::Data(data.clone()),
                span: token.span,
            }),
            TokenKind::LabelDecl(name) => {
                if !name.is_empty() {
                    program.items.push(Item {
                        kind: ItemKind::Label(name.clone()),
                        span: token.span,
                    });
                }
            }
            TokenKind::LabelRef(name) => {
                if !name.is_empty() {
                    program.items.push(Item {
                        kind: ItemKind::Ref(name.clone()),
                        span: token.span,
                    });
                }
            }
        }
    }
    program
}

/// Parse one token as a single byte.
///
/// `0x` keeps the usual C-style meaning.  Bare hexadecimal tokens are accepted
/// for compatibility with the community compiler (`ff` == `0xff`).  Values
/// that do not fit in one byte are rejected instead of being silently
/// truncated.
fn parse_byte_word(word: &str) -> Result<u8, String> {
    if let Some(rest) = word.strip_prefix("0x").or_else(|| word.strip_prefix("0X")) {
        if rest.is_empty() {
            return Err("`0x` needs at least one hexadecimal digit".to_owned());
        }
        if !rest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!(
                "invalid byte literal `{word}`: non-hexadecimal character"
            ));
        }
        return parse_hex_byte(word, rest);
    }

    if !word.is_empty() && word.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return parse_hex_byte(word, word);
    }

    Err(format!(
        "unknown token `{word}`: expected an instruction, a 0xHH byte, [label] or @label"
    ))
}

fn parse_hex_byte(original: &str, digits: &str) -> Result<u8, String> {
    let value = u64::from_str_radix(digits, 16)
        .map_err(|_| format!("byte literal `{original}` is out of range"))?;
    if value > 0xff {
        return Err(format!(
            "byte literal `{original}` is 0x{value:x}, which does not fit in one byte; \
             XASM operands are written as four separate bytes, e.g. `0x00 0x00 0x01 0x00`"
        ));
    }
    Ok(value as u8)
}
