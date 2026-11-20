//! Human-readable ROM listing / disassembly.

use crate::assembler::Artifact;
use crate::mnemonics;
use std::collections::BTreeMap;
use std::fmt::Write as _;

pub fn render(artifact: &Artifact) -> String {
    let bytes = &artifact.bytes;
    let mut labels_at: BTreeMap<u32, Vec<&str>> = BTreeMap::new();
    for (name, address) in &artifact.labels {
        labels_at.entry(*address).or_default().push(name);
    }

    let mut out = String::new();
    let _ = writeln!(out, "; XASM ROM listing ({} bytes)", bytes.len());
    let mut pc = 0usize;
    while pc < bytes.len() {
        if let Some(names) = labels_at.get(&(pc as u32)) {
            for name in names {
                let _ = writeln!(out, "{:08x}  {name}:", pc);
            }
        }

        let byte = bytes[pc];
        if byte == mnemonics::STO {
            let mut end = pc + 1;
            while end < bytes.len() && bytes[end] != mnemonics::STO {
                end += 1;
            }
            let terminator = end < bytes.len();
            let hex = hex_bytes(&bytes[pc..(end + usize::from(terminator)).min(bytes.len())]);
            let _ = writeln!(
                out,
                "{:08x}  {:<48}  sto     ; data block{}",
                pc,
                hex,
                if terminator { "" } else { " (unterminated)" }
            );
            pc = if terminator { end + 1 } else { bytes.len() };
            continue;
        }

        if let Some(mnemonic) = mnemonics::by_opcode(byte) {
            let operand_start = pc + 1;
            let operand_end = operand_start + 4 * mnemonic.arity as usize;
            let shown_end = operand_end.min(bytes.len());
            let hex = hex_bytes(&bytes[pc..shown_end]);
            let mut operands = String::new();
            for index in 0..mnemonic.arity as usize {
                let start = operand_start + index * 4;
                if start + 4 > bytes.len() {
                    if !operands.is_empty() {
                        operands.push_str(", ");
                    }
                    operands.push_str("<?>");
                    continue;
                }
                let value = u32::from_be_bytes([
                    bytes[start],
                    bytes[start + 1],
                    bytes[start + 2],
                    bytes[start + 3],
                ]);
                if !operands.is_empty() {
                    operands.push_str(", ");
                }
                let _ = write!(operands, "0x{value:08x}");
            }
            let _ = writeln!(
                out,
                "{:08x}  {:<48}  {:<7} {}",
                pc, hex, mnemonic.name, operands
            );
            pc = shown_end.max(pc + 1);
        } else {
            let _ = writeln!(
                out,
                "{:08x}  {:<48}  db      0x{byte:02x}",
                pc,
                hex_bytes(&bytes[pc..pc + 1])
            );
            pc += 1;
        }
    }
    out
}

fn hex_bytes(bytes: &[u8]) -> String {
    let mut out = String::new();
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 {
            out.push(' ');
        }
        let _ = write!(out, "{byte:02x}");
    }
    if out.len() > 48 {
        out.truncate(45);
        out.push_str("...");
    }
    out
}
