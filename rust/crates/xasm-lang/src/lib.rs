//! XPL: a small, modern-feeling language that compiles to XASM bytecode.
//!
//! Supported features:
//!
//! * `let` variables with block scoping
//! * arithmetic (`+ - * / %`), comparisons, `&&`, `||`, `!`
//! * `if`/`else`, `while`, blocks
//! * `print`, `println`, `putchar`
//! * `read_int()`, `read_char()`
//!
//! The generated code uses only the 23 original XASM1 instructions plus
//! `0xff`, so it runs on the original VM.

pub mod ast;
pub mod codegen;
pub mod lexer;
pub mod parser;

pub use codegen::{generate, Error, Generated, Stats};
pub use xasm_core::{Report, Source};

#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub max_addr: u32,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            max_addr: xasm_core::DEFAULT_MEMORY_LIMIT,
        }
    }
}

pub fn compile(source: &Source, options: &Options) -> Result<Generated, Error> {
    let mut report = Report::new();
    let tokens = lexer::lex(source, &mut report);
    if report.has_errors() {
        return Err(Error::Semantic(report));
    }
    let program = match parser::parse(&tokens, &mut report) {
        Some(program) => program,
        None => {
            if !report.has_errors() {
                report.error(None, "invalid program");
            }
            return Err(Error::Semantic(report));
        }
    };
    if report.has_errors() {
        return Err(Error::Semantic(report));
    }
    generate(&program, options.max_addr)
}
