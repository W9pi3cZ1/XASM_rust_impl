//! XASM compiler core.
//!
//! The compiler is deliberately byte oriented: every token either emits zero,
//! one, or four bytes.  This mirrors both the original `XASM_IDE_V1.c`
//! compiler and the community rebuild, and keeps the generated ROM layout
//! identical for the shared instruction subset.

pub mod assembler;
pub mod diag;
pub mod lexer;
pub mod listing;
pub mod mnemonics;
pub mod parser;
pub mod source;
pub mod span;

pub use assembler::{
    assemble, Artifact, AssembleOptions, DuplicateLabels, Stats, Target, VerifyLevel,
    DEFAULT_MAX_ROM, DEFAULT_MEMORY_LIMIT,
};
pub use diag::{Diagnostic, Level, Report};
pub use lexer::LexOptions;
pub use parser::{Item, ItemKind, ParsedProgram};
pub use source::Source;
pub use span::Span;

#[derive(Clone, Debug, Default)]
pub struct CompileOptions {
    pub raw_strings: bool,
    /// Optional entry label.  When set, a 14-byte trampoline is prepended so
    /// the VM (which always starts at address 0) jumps to this label.  This
    /// is useful when a program puts data/strings before its code.
    pub entry: Option<String>,
    pub assemble: AssembleOptions,
}

/// Lex, parse and assemble a source buffer.
pub fn compile(source: &Source, options: &CompileOptions) -> Result<Artifact, Report> {
    let mut report = Report::new();
    let tokens = lexer::lex(
        source,
        LexOptions {
            raw_strings: options.raw_strings,
        },
        &mut report,
    );
    let mut program = parser::parse(&tokens, &mut report);
    if let Some(entry) = &options.entry {
        prepend_entry_trampoline(&mut program, entry);
    }
    let mut artifact = match assemble(&program, &options.assemble) {
        Ok(artifact) => artifact,
        Err(mut assemble_report) => {
            report.extend(std::mem::take(&mut assemble_report));
            return Err(report);
        }
    };
    let mut all = Report::new();
    all.extend(report);
    all.extend(std::mem::take(&mut artifact.report));
    artifact.report = all;
    if artifact.report.has_errors() {
        return Err(artifact.report);
    }
    Ok(artifact)
}

/// Prepend:
///
/// ```text
/// mov 0x00000000 <entry>   ; overwrite cell 0 with the entry address
/// goto 0x00000000          ; jump through cell 0
/// ```
///
/// The trampoline is only 14 bytes and does not need a reserved scratch cell.
/// It is opt-in via `--entry`.
fn prepend_entry_trampoline(program: &mut ParsedProgram, entry: &str) {
    let span = Span::default();
    let mut items = Vec::with_capacity(program.items.len() + 11);
    items.push(Item {
        kind: ItemKind::Op {
            opcode: 0x21,
            mnemonic: "mov",
        },
        span,
    });
    for _ in 0..4 {
        items.push(Item {
            kind: ItemKind::Byte(0),
            span,
        });
    }
    items.push(Item {
        kind: ItemKind::Ref(entry.to_owned()),
        span,
    });
    items.push(Item {
        kind: ItemKind::Op {
            opcode: 0x23,
            mnemonic: "goto",
        },
        span,
    });
    for _ in 0..4 {
        items.push(Item {
            kind: ItemKind::Byte(0),
            span,
        });
    }
    items.append(&mut program.items);
    program.items = items;
    program.ops += 2;
}
