//! XASM virtual machine.
//!
//! This is a development / verification VM.  It models the original XASM1
//! behavior by default and also implements the six community bit opcodes so
//! ROMs produced with `--target community` can be tested.

pub mod cpu;
pub mod error;
pub mod input;
pub mod memory;
pub mod terminal;

pub use cpu::{ExitReason, RunOutcome, Vm, VmOptions};
pub use error::VmError;
pub use input::Input;
pub use memory::{Memory, DEFAULT_MAX_ADDR, PAGE_BITS, PAGE_SIZE};

/// Load a ROM and run it to completion using the default VM configuration.
pub fn run_rom<R: std::io::Read, W: std::io::Write>(
    rom: &[u8],
    input: R,
    output: W,
) -> Result<RunOutcome, VmError> {
    let mut vm = Vm::new_default(input, output);
    vm.load_rom(rom)?;
    vm.run()
}
