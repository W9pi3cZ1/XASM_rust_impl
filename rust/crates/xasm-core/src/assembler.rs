//! Layout, label resolution, ROM emission and legacy-ROM verification.

use crate::diag::Report;
use crate::mnemonics;
use crate::parser::{ItemKind, ParsedProgram};
use crate::span::Span;
use std::collections::BTreeMap;

pub const DEFAULT_MEMORY_LIMIT: u32 = 0x000f_fffe;
pub const DEFAULT_MAX_ROM: usize = DEFAULT_MEMORY_LIMIT as usize + 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// The original XASM1 VM.  Community-only opcodes are rejected.
    Legacy,
    /// The community rebuild VM.  Community opcodes are emitted natively.
    Community,
}

impl Target {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "legacy" | "original" | "old" => Some(Target::Legacy),
            "community" | "new" => Some(Target::Community),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerifyLevel {
    Off,
    Warn,
    Error,
}

impl VerifyLevel {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "off" | "none" => Some(VerifyLevel::Off),
            "warn" | "warning" | "on" => Some(VerifyLevel::Warn),
            "error" | "strict" => Some(VerifyLevel::Error),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DuplicateLabels {
    Error,
    LastWins,
}

#[derive(Clone, Debug)]
pub struct AssembleOptions {
    pub target: Target,
    /// Emit community opcodes even with `Target::Legacy` (dangerous).
    pub allow_community: bool,
    /// Highest address the VM may access, inclusive.
    pub memory_limit: u32,
    pub verify: VerifyLevel,
    pub duplicate_labels: DuplicateLabels,
}

impl Default for AssembleOptions {
    fn default() -> Self {
        Self {
            target: Target::Legacy,
            allow_community: false,
            memory_limit: DEFAULT_MEMORY_LIMIT,
            verify: VerifyLevel::Warn,
            duplicate_labels: DuplicateLabels::LastWins,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub rom_size: usize,
    pub items: usize,
    pub source_ops: usize,
    pub labels: usize,
    pub decoded_instructions: usize,
    pub extension_ops: usize,
    pub unknown_bytes: usize,
    pub data_blocks: usize,
}

#[derive(Clone, Debug)]
pub struct Artifact {
    pub bytes: Vec<u8>,
    pub labels: BTreeMap<String, u32>,
    pub item_offsets: Vec<u32>,
    pub stats: Stats,
    pub report: Report,
}

impl Artifact {
    pub fn label_offset(&self, name: &str) -> Option<u32> {
        self.labels.get(name).copied()
    }
}

pub fn assemble(program: &ParsedProgram, options: &AssembleOptions) -> Result<Artifact, Report> {
    let mut report = Report::new();
    let max_rom = options.memory_limit as usize + 1;

    // Pass 1: assign an offset to every item and collect labels.
    let mut item_offsets = Vec::with_capacity(program.items.len());
    let mut labels: BTreeMap<String, u32> = BTreeMap::new();
    let mut offset: usize = 0;

    for item in &program.items {
        match &item.kind {
            ItemKind::Label(name) => {
                if let Some(previous) = labels.get(name) {
                    match options.duplicate_labels {
                        DuplicateLabels::Error => {
                            report.error(item.span, format!("duplicate label `{name}`"));
                            if let Some(last) = report.diagnostics.last_mut() {
                                last.notes
                                    .push(format!("previous definition at 0x{previous:08x}"));
                            }
                        }
                        DuplicateLabels::LastWins => {
                            // The community compiler prepends labels to a
                            // linked list, so the last definition wins.
                        }
                    }
                }
                labels.insert(name.clone(), offset as u32);
                item_offsets.push(offset as u32);
            }
            ItemKind::Byte(_) | ItemKind::Op { .. } => {
                item_offsets.push(offset as u32);
                offset += 1;
            }
            ItemKind::Data(data) => {
                item_offsets.push(offset as u32);
                offset += data.len();
            }
            ItemKind::Ref(_) => {
                item_offsets.push(offset as u32);
                offset += 4;
            }
        }
    }

    if offset > u32::MAX as usize {
        report.error(
            None,
            format!("program is larger than 4 GiB (0x{offset:x} bytes)"),
        );
        return Err(report);
    }
    if offset > max_rom {
        let target_name = match options.target {
            Target::Legacy => "the original XASM1 VM",
            Target::Community => "the configured VM memory",
        };
        report.error(
            None,
            format!(
                "ROM is {offset} bytes (0x{offset:x}), but {target_name} can load at most \
                 {max_rom} bytes (0x{max_rom:x}) with the current memory limit 0x{:x}. \
                 Reduce the program size or raise --max-addr.",
                options.memory_limit
            ),
        );
        return Err(report);
    }

    // Source-level community-opcode check.  This is exact: it is based on the
    // mnemonic the user wrote, not on a linear disassembly of the byte stream.
    for item in &program.items {
        if let ItemKind::Op { opcode, mnemonic } = &item.kind {
            if !mnemonics::is_extension(*opcode) {
                continue;
            }
            match options.target {
                Target::Legacy if !options.allow_community => {
                    report.error(
                        item.span,
                        format!(
                            "community-only instruction `{mnemonic}` (opcode 0x{opcode:02x}) \
                             cannot run on the original XASM1 VM"
                        ),
                    );
                    if let Some(last) = report.diagnostics.last_mut() {
                        last.notes.push(
                            "the original VM has no opcodes 0x41..0x46; unknown bytes are \
                             treated as NOPs, so the operands would be executed as instructions"
                                .to_owned(),
                        );
                        last.notes.push(
                            "use --target community for the community VM, or rewrite the \
                             operation with the 23 original instructions"
                                .to_owned(),
                        );
                    }
                }
                Target::Legacy => {
                    report.warning(
                        item.span,
                        format!(
                            "community opcode 0x{opcode:02x} (`{mnemonic}`) will be treated as \
                             a NOP by the original XASM1 VM"
                        ),
                    );
                }
                Target::Community => {}
            }
        }
    }

    // Pass 2: emit bytes, resolving @labels.
    let mut bytes = Vec::with_capacity(offset);
    for item in &program.items {
        match &item.kind {
            ItemKind::Byte(byte) => bytes.push(*byte),
            ItemKind::Op { opcode, .. } => bytes.push(*opcode),
            ItemKind::Data(data) => bytes.extend_from_slice(data),
            ItemKind::Label(_) => {}
            ItemKind::Ref(name) => match labels.get(name) {
                Some(address) => bytes.extend_from_slice(&address.to_be_bytes()),
                None => {
                    report.error(item.span, format!("undefined label `{name}`"));
                    bytes.extend_from_slice(&[0, 0, 0, 0]);
                }
            },
        }
    }

    debug_assert_eq!(bytes.len(), offset);

    let ranges = build_ranges(program, &item_offsets);
    let mut stats = Stats {
        rom_size: bytes.len(),
        items: program.items.len(),
        source_ops: program.ops,
        labels: labels.len(),
        ..Stats::default()
    };
    verify_rom(&bytes, &ranges, options, &mut stats, &mut report);

    if report.has_errors() {
        return Err(report);
    }

    Ok(Artifact {
        bytes,
        labels,
        item_offsets,
        stats,
        report,
    })
}

struct ItemRange {
    start: u32,
    end: u32,
    span: Span,
}

fn build_ranges(program: &ParsedProgram, offsets: &[u32]) -> Vec<ItemRange> {
    let mut ranges = Vec::new();
    for (item, start) in program.items.iter().zip(offsets.iter()) {
        let size = match &item.kind {
            ItemKind::Label(_) => 0,
            ItemKind::Byte(_) | ItemKind::Op { .. } => 1,
            ItemKind::Data(data) => data.len(),
            ItemKind::Ref(_) => 4,
        };
        if size > 0 {
            ranges.push(ItemRange {
                start: *start,
                end: start.saturating_add(size as u32),
                span: item.span,
            });
        }
    }
    ranges
}

fn span_at(ranges: &[ItemRange], offset: usize) -> Option<Span> {
    let offset = u32::try_from(offset).ok()?;
    let idx = ranges.partition_point(|range| range.start <= offset);
    if idx == 0 {
        return None;
    }
    let range = &ranges[idx - 1];
    if offset < range.end {
        Some(range.span)
    } else {
        None
    }
}

fn verify_rom(
    bytes: &[u8],
    ranges: &[ItemRange],
    options: &AssembleOptions,
    stats: &mut Stats,
    report: &mut Report,
) {
    let strict = options.verify == VerifyLevel::Error;
    let verbose = options.verify != VerifyLevel::Off;

    if bytes.is_empty() {
        if verbose {
            emit_warn(
                report,
                strict,
                None,
                "the ROM is empty; the VM will start executing memory past the ROM",
            );
        }
        return;
    }

    // This is a linear scan, not a control-flow analysis.  It is used for
    // statistics and for the "entry byte" sanity check; it deliberately does
    // not report unknown/truncated bytes, because XASM programs may jump into
    // data or keep data after an `exit`.  Community mnemonics are checked at
    // the source level in `assemble`.
    let mut pc = 0usize;
    while pc < bytes.len() {
        let byte = bytes[pc];
        if byte == mnemonics::STO {
            stats.data_blocks += 1;
            match bytes[pc + 1..]
                .iter()
                .position(|candidate| *candidate == mnemonics::STO)
            {
                Some(relative) => {
                    pc = pc + 1 + relative + 1;
                }
                None => break,
            }
            continue;
        }

        match mnemonics::by_opcode(byte) {
            Some(mnemonic) => {
                stats.decoded_instructions += 1;
                if mnemonic.extension {
                    stats.extension_ops += 1;
                }
                let end = pc + 1 + 4 * mnemonic.arity as usize;
                if end > bytes.len() {
                    break;
                }
                pc = end;
            }
            None => {
                stats.unknown_bytes += 1;
                if verbose && pc == 0 {
                    emit_warn(
                        report,
                        strict,
                        span_at(ranges, pc),
                        format!(
                            "execution starts at 0x00000000 with 0x{byte:02x}, which is not a \
                             known instruction.  The VM starts at address 0, so a string or \
                             data block here is executed as opcodes.  Put executable code \
                             first, guard data with `0xff ... 0xff`, or use --entry <label>."
                        ),
                    );
                }
                pc += 1;
            }
        }
    }
}

fn emit_warn(report: &mut Report, strict: bool, span: Option<Span>, message: impl Into<String>) {
    if strict {
        report.error(span, message);
    } else {
        report.warning(span, message);
    }
}
