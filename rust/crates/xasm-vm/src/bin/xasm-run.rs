use std::io::Write;
use std::process::ExitCode;

use clap::Parser;
use xasm_vm::{Memory, Vm, VmOptions};

#[derive(Debug, Parser)]
#[command(
    name = "xasm-run",
    version,
    about = "XASM virtual machine (original XASM1 semantics + community opcodes)",
    after_help = "The default behavior follows the original XASM1 VM:\n  unknown opcodes are NOPs, and memory past the ROM is filled with -1.\n  Use --community for the community VM behavior (0x00 halt, zero fill, strict unknown).\n\n  With no arguments, xasm-run behaves like the original `XASM1` binary\n  when ./disk.bin exists."
)]
struct Cli {
    /// ROM image (disk.bin)
    #[arg(short, long, value_name = "ROM")]
    input: Option<String>,

    /// Highest address, inclusive
    #[arg(short, long, value_name = "N", value_parser = parse_number, default_value = "0xffffe")]
    memory: u32,

    /// Instruction rate (0 = unlimited)
    #[arg(short, long, value_name = "HZ", value_parser = parse_u64, default_value_t = 0)]
    clock: u64,

    /// Error on unknown opcodes instead of treating them as NOPs
    #[arg(long)]
    strict: bool,

    /// Community VM behavior (0x00 halt, zero fill, strict unknown)
    #[arg(long)]
    community: bool,

    /// Treat 0x00 as a halt
    #[arg(long)]
    zero_halts: bool,

    /// Leave memory past the ROM at 0 (community behavior)
    #[arg(long)]
    zero_fill: bool,

    /// Print every executed instruction to stderr
    #[arg(long)]
    trace: bool,

    /// Stop after N instructions
    #[arg(long, value_name = "N", value_parser = parse_u64)]
    max_steps: Option<u64>,

    /// Dump memory to stderr after the run
    #[arg(long)]
    dump_memory: bool,

    /// Dump start address
    #[arg(long, value_name = "ADDR", value_parser = parse_number, default_value_t = 0)]
    dump_start: u32,

    /// Dump end address
    #[arg(long, value_name = "ADDR", value_parser = parse_number, default_value = "0x1000")]
    dump_end: u32,

    /// Print instruction count and exit reason
    #[arg(long)]
    stats: bool,

    /// ROM image (positional form)
    #[arg(value_name = "ROM")]
    positional: Option<String>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode, String> {
    if cli.input.is_some() && cli.positional.is_some() {
        return Err("input was given both positionally and with -i/--input".to_owned());
    }
    let mut input = cli.input.or(cli.positional);
    if input.is_none() && std::path::Path::new("disk.bin").is_file() {
        // Zero-argument drop-in compatibility with the original XASM1 binary.
        input = Some("disk.bin".to_owned());
    }
    let input = input.ok_or_else(|| "no input ROM (use -i/--input)".to_owned())?;

    let rom = std::fs::read(&input).map_err(|error| format!("cannot read `{input}`: {error}"))?;

    let zero_fill = cli.zero_fill || cli.community;
    let zero_halts = cli.zero_halts || cli.community;
    // The original VM fills every cell past EOF with -1 (fgetc's EOF).
    // The community VM leaves them at zero.
    let memory = Memory::new_with_fill(cli.memory, if zero_fill { 0 } else { -1 });
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let options = VmOptions {
        zero_halts,
        strict_unknown: cli.strict || cli.community,
        trace: cli.trace,
        clock_hz: cli.clock,
        max_steps: cli.max_steps,
    };
    let mut vm = Vm::new(memory, stdin.lock(), stdout.lock(), options);
    vm.load_rom(&rom).map_err(|error| error.to_string())?;

    match vm.run() {
        Ok(outcome) => {
            if cli.stats {
                eprintln!(
                    "instructions={} pc=0x{:08x} exit={:?}",
                    outcome.instructions, outcome.pc, outcome.reason
                );
            }
            if cli.dump_memory {
                let start = cli.dump_start.min(cli.memory);
                let end = cli.dump_end.min(cli.memory).max(start);
                let dump = vm
                    .memory
                    .dump(start, end)
                    .map_err(|error| error.to_string())?;
                let _ = std::io::stderr().write_all(dump.as_bytes());
            }
            let _ = std::io::stderr().flush();
            Ok(ExitCode::SUCCESS)
        }
        Err(error) => {
            eprintln!("vm error: {error}");
            Ok(ExitCode::from(1))
        }
    }
}

fn parse_number(value: &str) -> Result<u32, String> {
    let value = value.trim().replace('_', "");
    let parsed = if let Some(rest) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u32::from_str_radix(rest, 16)
    } else {
        value.parse::<u32>()
    };
    parsed.map_err(|_| format!("invalid address `{value}` (use decimal or 0xHHHH)"))
}

fn parse_u64(value: &str) -> Result<u64, String> {
    let value = value.trim().replace('_', "");
    let parsed = if let Some(rest) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u64::from_str_radix(rest, 16)
    } else {
        value.parse::<u64>()
    };
    parsed.map_err(|_| format!("invalid number `{value}`"))
}
