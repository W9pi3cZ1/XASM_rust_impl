//! The XASM CPU.
//!
//! Semantics are modelled on the original `XASM1` binary:
//!
//! * every memory cell is an `i32`;
//! * `*addr` operands are addresses of cells (one level of indirection);
//! * unknown opcodes are NOPs (the community VM halts on 0x00 instead);
//! * `0xff` skips bytes until the next `0xff`;
//! * all arithmetic wraps like the C `int` operations on x86.

use crate::error::VmError;
use crate::input::Input;
use crate::memory::{Memory, DEFAULT_MAX_ADDR};
use std::io::{Read, Write};
use std::time::Duration;

pub const STO: i32 = 0xff;

#[derive(Clone, Debug, Default)]
pub struct VmOptions {
    /// Reject unknown opcodes instead of treating them as NOPs.
    pub strict_unknown: bool,
    /// The community VM treats opcode 0x00 as a segfault/halt.
    pub zero_halts: bool,
    pub trace: bool,
    pub clock_hz: u64,
    pub max_steps: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitReason {
    /// `exit` (0x2a)
    Exit,
    /// The program counter moved past the last addressable cell.
    PcPastEnd,
    /// Opcode 0x00 with `zero_halts` enabled.
    ZeroHalt,
    /// `max_steps` was reached.
    StepLimit,
}

#[derive(Clone, Copy, Debug)]
pub struct RunOutcome {
    pub instructions: u64,
    pub pc: u32,
    pub reason: ExitReason,
}

pub struct Vm<R: Read, W: Write> {
    pub memory: Memory,
    pub pc: u32,
    pub halted: bool,
    pub instructions: u64,
    input: Input<R>,
    output: W,
    options: VmOptions,
    reason: ExitReason,
    last_trace: Option<(u32, i32, u32, u64)>,
}

impl<R: Read, W: Write> Vm<R, W> {
    pub fn new(memory: Memory, input: R, output: W, options: VmOptions) -> Self {
        Self {
            memory,
            pc: 0,
            halted: false,
            instructions: 0,
            input: Input::new(input),
            output,
            options,
            reason: ExitReason::PcPastEnd,
            last_trace: None,
        }
    }

    pub fn new_default(input: R, output: W) -> Self {
        Self::new(
            Memory::new_legacy(DEFAULT_MAX_ADDR),
            input,
            output,
            VmOptions::default(),
        )
    }

    pub fn load_rom(&mut self, rom: &[u8]) -> Result<(), VmError> {
        self.memory.load_rom(rom)
    }

    pub fn options_mut(&mut self) -> &mut VmOptions {
        &mut self.options
    }

    pub fn input_mut(&mut self) -> &mut Input<R> {
        &mut self.input
    }

    pub fn output_mut(&mut self) -> &mut W {
        &mut self.output
    }

    pub fn run(&mut self) -> Result<RunOutcome, VmError> {
        while !self.halted {
            if self.pc > self.memory.max_addr() {
                self.halted = true;
                self.reason = ExitReason::PcPastEnd;
                break;
            }
            if let Some(limit) = self.options.max_steps {
                if self.instructions >= limit {
                    self.halted = true;
                    self.reason = ExitReason::StepLimit;
                    break;
                }
            }
            self.step()?;
            self.instructions = self.instructions.saturating_add(1);
            if self.options.trace {
                if let Some((pc, op, next, instructions)) = self.last_trace.take() {
                    eprintln!("[{instructions:08}] pc=0x{pc:08x} op=0x{op:02x} -> 0x{next:08x}");
                }
            }
            if self.options.clock_hz > 0 {
                let micros = 1_000_000u64 / self.options.clock_hz.max(1);
                if micros > 0 {
                    std::thread::sleep(Duration::from_micros(micros));
                }
            }
        }
        self.output.flush()?;
        Ok(RunOutcome {
            instructions: self.instructions,
            pc: self.pc,
            reason: self.reason,
        })
    }

    fn step(&mut self) -> Result<(), VmError> {
        let pc = self.pc;
        let op = self.memory.read(pc)?;
        let next;

        match op {
            // ---------------------------------------------------------- data
            STO => {
                let mut cursor = pc.checked_add(1).ok_or(VmError::UnterminatedData { pc })?;
                loop {
                    if cursor > self.memory.max_addr() {
                        return Err(VmError::UnterminatedData { pc });
                    }
                    let byte = self.memory.read(cursor)?;
                    if byte == STO {
                        next = cursor + 1;
                        break;
                    }
                    cursor += 1;
                }
            }

            // -------------------------------------------------------- arithmetic
            0x01 => {
                let dest = self.addr(0)?;
                let src = self.addr(1)?;
                let value = self.memory.read(dest)?.wrapping_add(self.memory.read(src)?);
                self.memory.write(dest, value)?;
                next = pc + 9;
            }
            0x02 => {
                let dest = self.addr(0)?;
                let src = self.addr(1)?;
                let value = self.memory.read(dest)?.wrapping_sub(self.memory.read(src)?);
                self.memory.write(dest, value)?;
                next = pc + 9;
            }
            0x03 => {
                let dest = self.addr(0)?;
                let src = self.addr(1)?;
                let value = self.memory.read(dest)?.wrapping_mul(self.memory.read(src)?);
                self.memory.write(dest, value)?;
                next = pc + 9;
            }
            0x04 => {
                let dest = self.addr(0)?;
                let src = self.addr(1)?;
                let value = self
                    .memory
                    .read(dest)?
                    .checked_div(self.memory.read(src)?)
                    .ok_or(VmError::DivideByZero { pc })?;
                self.memory.write(dest, value)?;
                next = pc + 9;
            }

            // ----------------------------------------------------------- logic
            0x11 => {
                let a = self.addr(0)?;
                let b = self.addr(1)?;
                let dest = self.addr(2)?;
                let value = i32::from(self.memory.read(a)? != 0 && self.memory.read(b)? != 0);
                self.memory.write(dest, value)?;
                next = pc + 13;
            }
            0x12 => {
                let a = self.addr(0)?;
                let b = self.addr(1)?;
                let dest = self.addr(2)?;
                let value = i32::from(self.memory.read(a)? != 0 || self.memory.read(b)? != 0);
                self.memory.write(dest, value)?;
                next = pc + 13;
            }
            0x13 => {
                // Original XASM1 semantics: dest = (a != b), 3 operands.
                let a = self.addr(0)?;
                let b = self.addr(1)?;
                let dest = self.addr(2)?;
                let value = i32::from(self.memory.read(a)? != self.memory.read(b)?);
                self.memory.write(dest, value)?;
                next = pc + 13;
            }

            // --------------------------------------------------------- control
            0x21 => {
                let dest = self.addr(0)?;
                let immediate = self.addr(1)?;
                self.memory.write(dest, immediate as i32)?;
                next = pc + 9;
            }
            0x22 => {
                let dest = self.addr(0)?;
                let src = self.addr(1)?;
                let value = self.memory.read(src)?;
                self.memory.write(dest, value)?;
                next = pc + 9;
            }
            0x23 => {
                let target = self.addr(0)?;
                next = self.memory.read(target)? as u32;
            }
            0x24 => {
                let dest = self.addr(0)?;
                self.memory.write(dest, pc.wrapping_add(5) as i32)?;
                next = pc + 5;
            }
            0x25 => {
                let a = self.addr(0)?;
                let b = self.addr(1)?;
                let target = self.addr(2)?;
                if self.memory.read(a)? > self.memory.read(b)? {
                    next = self.memory.read(target)? as u32;
                } else {
                    next = pc + 13;
                }
            }
            0x26 => {
                let a = self.addr(0)?;
                let b = self.addr(1)?;
                let target = self.addr(2)?;
                if self.memory.read(a)? < self.memory.read(b)? {
                    next = self.memory.read(target)? as u32;
                } else {
                    next = pc + 13;
                }
            }
            0x27 => {
                let a = self.addr(0)?;
                let b = self.addr(1)?;
                let target = self.addr(2)?;
                if self.memory.read(a)? == self.memory.read(b)? {
                    next = self.memory.read(target)? as u32;
                } else {
                    next = pc + 13;
                }
            }
            0x28 => {
                let a = self.addr(0)?;
                let b = self.addr(1)?;
                let value = self.memory.read(a)?;
                let value2 = self.memory.read(b)?;
                let target = (a as i32).wrapping_sub(value2);
                self.memory.write(target as u32, value)?;
                self.memory.write(a, 0)?;
                next = pc + 9;
            }
            0x29 => {
                let a = self.addr(0)?;
                let b = self.addr(1)?;
                let value = self.memory.read(a)?;
                let value2 = self.memory.read(b)?;
                let target = (a as i32).wrapping_add(value2);
                self.memory.write(target as u32, value)?;
                self.memory.write(a, 0)?;
                next = pc + 9;
            }
            0x2a => {
                self.halted = true;
                self.reason = ExitReason::Exit;
                next = pc;
            }

            // ------------------------------------------------------------- I/O
            0x31 => {
                let pointer = self.addr(0)?;
                let char_addr = self.memory.read(pointer)? as u32;
                let byte = self.memory.read(char_addr)? as u8;
                self.output.write_all(&[byte])?;
                self.output.flush()?;
                next = pc + 5;
            }
            0x32 => {
                let pointer = self.addr(0)?;
                let value_addr = self.memory.read(pointer)? as u32;
                let value = self.memory.read(value_addr)?;
                self.output.write_all(value.to_string().as_bytes())?;
                self.output.flush()?;
                next = pc + 5;
            }
            0x33 => {
                let pointer = self.addr(0)?;
                let value_addr = self.memory.read(pointer)? as u32;
                let value = self.memory.read(value_addr)? as u32;
                self.output.write_all(format!("{value:x}").as_bytes())?;
                self.output.flush()?;
                next = pc + 5;
            }
            0x34 => {
                let dest = self.addr(0)?;
                let value = self.input.read_char()?.map(i32::from).unwrap_or(-1);
                self.memory.write(dest, value)?;
                next = pc + 5;
            }
            0x35 => {
                let dest = self.addr(0)?;
                if let Some(value) = self.input.read_decimal()? {
                    self.memory.write(dest, value)?;
                }
                next = pc + 5;
            }
            0x36 => {
                let dest = self.addr(0)?;
                if let Some(value) = self.input.read_hex()? {
                    self.memory.write(dest, value)?;
                }
                next = pc + 5;
            }

            // --------------------------------------------- community bit ops
            0x41 => {
                let dest = self.addr(0)?;
                let src = self.addr(1)?;
                let value = self.memory.read(dest)? & self.memory.read(src)?;
                self.memory.write(dest, value)?;
                next = pc + 9;
            }
            0x42 => {
                let dest = self.addr(0)?;
                let src = self.addr(1)?;
                let value = self.memory.read(dest)? | self.memory.read(src)?;
                self.memory.write(dest, value)?;
                next = pc + 9;
            }
            0x43 => {
                let dest = self.addr(0)?;
                let src = self.addr(1)?;
                let value = self.memory.read(dest)? ^ self.memory.read(src)?;
                self.memory.write(dest, value)?;
                next = pc + 9;
            }
            0x44 => {
                let dest = self.addr(0)?;
                let value = !self.memory.read(dest)?;
                self.memory.write(dest, value)?;
                next = pc + 5;
            }
            0x45 => {
                let dest = self.addr(0)?;
                let src = self.addr(1)?;
                let count = (self.memory.read(src)? as u32) & 31;
                let value = self.memory.read(dest)?.wrapping_shl(count);
                self.memory.write(dest, value)?;
                next = pc + 9;
            }
            0x46 => {
                let dest = self.addr(0)?;
                let src = self.addr(1)?;
                let count = (self.memory.read(src)? as u32) & 31;
                let value = self.memory.read(dest)? >> count;
                self.memory.write(dest, value)?;
                next = pc + 9;
            }

            0x00 if self.options.zero_halts => {
                self.halted = true;
                self.reason = ExitReason::ZeroHalt;
                next = pc;
            }

            _ => {
                if self.options.strict_unknown {
                    return Err(VmError::UnknownOpcode { pc, opcode: op });
                }
                next = pc + 1;
            }
        }

        self.last_trace = Some((pc, op, next, self.instructions));
        self.pc = next;
        Ok(())
    }

    /// Read the 4-byte big-endian address operand `index` of the current
    /// instruction.
    fn addr(&self, index: u8) -> Result<u32, VmError> {
        let start = self
            .pc
            .checked_add(1)
            .and_then(|value| value.checked_add(u32::from(index) * 4))
            .ok_or(VmError::OutOfRange {
                addr: self.pc,
                max: self.memory.max_addr(),
            })?;
        let mut value = 0u32;
        for offset in 0..4 {
            let cell = self.memory.read(start + offset)?;
            // Match the original `hexcat`: it shifts and adds full i32
            // values (with 32-bit wrap-around), it does not mask to bytes.
            value = value.wrapping_shl(8).wrapping_add(cell as u32);
        }
        Ok(value)
    }
}
