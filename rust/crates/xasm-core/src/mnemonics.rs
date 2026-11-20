//! The XASM instruction table.
//!
//! The table is shared by the compiler and the VM/disassembler.  Mnemonics are
//! matched case-insensitively, exactly like the community rebuild does.

#[derive(Clone, Copy, Debug)]
pub struct Mnemonic {
    pub name: &'static str,
    pub opcode: u8,
    /// Number of 4-byte address operands encoded after the opcode.
    pub arity: u8,
    pub extension: bool,
    pub summary: &'static str,
}

pub const STO: u8 = 0xff;

pub const MNEMONICS: &[Mnemonic] = &[
    Mnemonic {
        name: "add",
        opcode: 0x01,
        arity: 2,
        extension: false,
        summary: "add",
    },
    Mnemonic {
        name: "sub",
        opcode: 0x02,
        arity: 2,
        extension: false,
        summary: "subtract",
    },
    Mnemonic {
        name: "xadd",
        opcode: 0x03,
        arity: 2,
        extension: false,
        summary: "multiply",
    },
    Mnemonic {
        name: "xsub",
        opcode: 0x04,
        arity: 2,
        extension: false,
        summary: "divide",
    },
    Mnemonic {
        name: "and",
        opcode: 0x11,
        arity: 3,
        extension: false,
        summary: "logical and",
    },
    Mnemonic {
        name: "or",
        opcode: 0x12,
        arity: 3,
        extension: false,
        summary: "logical or",
    },
    Mnemonic {
        name: "not",
        opcode: 0x13,
        arity: 2,
        extension: false,
        summary: "logical not",
    },
    Mnemonic {
        name: "mov",
        opcode: 0x21,
        arity: 2,
        extension: false,
        summary: "move immediate",
    },
    Mnemonic {
        name: "copy",
        opcode: 0x22,
        arity: 2,
        extension: false,
        summary: "copy cell",
    },
    Mnemonic {
        name: "goto",
        opcode: 0x23,
        arity: 1,
        extension: false,
        summary: "jump indirect",
    },
    Mnemonic {
        name: "geta",
        opcode: 0x24,
        arity: 1,
        extension: false,
        summary: "get address",
    },
    Mnemonic {
        name: ">",
        opcode: 0x25,
        arity: 3,
        extension: false,
        summary: "jump if greater",
    },
    Mnemonic {
        name: "<",
        opcode: 0x26,
        arity: 3,
        extension: false,
        summary: "jump if less",
    },
    Mnemonic {
        name: "=",
        opcode: 0x27,
        arity: 3,
        extension: false,
        summary: "jump if equal",
    },
    Mnemonic {
        name: "lm",
        opcode: 0x28,
        arity: 2,
        extension: false,
        summary: "move cell left",
    },
    Mnemonic {
        name: "rm",
        opcode: 0x29,
        arity: 2,
        extension: false,
        summary: "move cell right",
    },
    Mnemonic {
        name: "exit",
        opcode: 0x2a,
        arity: 0,
        extension: false,
        summary: "halt",
    },
    Mnemonic {
        name: "putc",
        opcode: 0x31,
        arity: 1,
        extension: false,
        summary: "put character",
    },
    Mnemonic {
        name: "putn",
        opcode: 0x32,
        arity: 1,
        extension: false,
        summary: "put decimal",
    },
    Mnemonic {
        name: "puth",
        opcode: 0x33,
        arity: 1,
        extension: false,
        summary: "put hex",
    },
    Mnemonic {
        name: "getc",
        opcode: 0x34,
        arity: 1,
        extension: false,
        summary: "get character",
    },
    Mnemonic {
        name: "getn",
        opcode: 0x35,
        arity: 1,
        extension: false,
        summary: "get decimal",
    },
    Mnemonic {
        name: "geth",
        opcode: 0x36,
        arity: 1,
        extension: false,
        summary: "get hex",
    },
    Mnemonic {
        name: "sto",
        opcode: 0xff,
        arity: 0,
        extension: false,
        summary: "data block marker",
    },
    Mnemonic {
        name: "&",
        opcode: 0x41,
        arity: 2,
        extension: true,
        summary: "bitwise and",
    },
    Mnemonic {
        name: "|",
        opcode: 0x42,
        arity: 2,
        extension: true,
        summary: "bitwise or",
    },
    Mnemonic {
        name: "^",
        opcode: 0x43,
        arity: 2,
        extension: true,
        summary: "bitwise xor",
    },
    Mnemonic {
        name: "~",
        opcode: 0x44,
        arity: 1,
        extension: true,
        summary: "bitwise not",
    },
    Mnemonic {
        name: "<<",
        opcode: 0x45,
        arity: 2,
        extension: true,
        summary: "shift left",
    },
    Mnemonic {
        name: ">>",
        opcode: 0x46,
        arity: 2,
        extension: true,
        summary: "shift right",
    },
];

pub fn lookup(name: &str) -> Option<&'static Mnemonic> {
    MNEMONICS
        .iter()
        .find(|mnemonic| mnemonic.name.eq_ignore_ascii_case(name))
}

pub fn by_opcode(opcode: u8) -> Option<&'static Mnemonic> {
    MNEMONICS.iter().find(|mnemonic| mnemonic.opcode == opcode)
}

pub fn is_extension(opcode: u8) -> bool {
    matches!(opcode, 0x41..=0x46)
}

pub fn name_for(opcode: u8) -> Option<&'static str> {
    by_opcode(opcode).map(|mnemonic| mnemonic.name)
}
