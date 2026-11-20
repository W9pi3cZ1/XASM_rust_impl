//! VM errors.

use std::fmt;

#[derive(Debug, Clone)]
pub enum VmError {
    OutOfRange { addr: u32, max: u32 },
    DivideByZero { pc: u32 },
    UnknownOpcode { pc: u32, opcode: i32 },
    UnterminatedData { pc: u32 },
    RomTooLarge { size: usize, capacity: usize },
    Io(String),
    StepLimit { pc: u32, steps: u64 },
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VmError::OutOfRange { addr, max } => {
                write!(f, "memory address 0x{addr:08x} is outside 0..=0x{max:08x}")
            }
            VmError::DivideByZero { pc } => {
                write!(f, "division by zero at pc=0x{pc:08x}")
            }
            VmError::UnknownOpcode { pc, opcode } => {
                write!(f, "unknown opcode 0x{opcode:02x} at pc=0x{pc:08x}")
            }
            VmError::UnterminatedData { pc } => {
                write!(f, "unterminated 0xff data block at pc=0x{pc:08x}")
            }
            VmError::RomTooLarge { size, capacity } => {
                write!(
                    f,
                    "ROM is {size} bytes but VM memory only holds {capacity} bytes"
                )
            }
            VmError::Io(message) => write!(f, "I/O error: {message}"),
            VmError::StepLimit { pc, steps } => {
                write!(f, "step limit {steps} reached at pc=0x{pc:08x}")
            }
        }
    }
}

impl std::error::Error for VmError {}

impl From<std::io::Error> for VmError {
    fn from(error: std::io::Error) -> Self {
        VmError::Io(error.to_string())
    }
}
