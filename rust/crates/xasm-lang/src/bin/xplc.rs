use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use xasm_lang::{compile, Error, Options, Source};
use xasm_vm::{Memory, Vm, VmOptions};

#[derive(Debug, Parser)]
#[command(
    name = "xplc",
    version,
    about = "XPL compiler: a small high-level language for XASM",
    after_help = "XPL programs compile to legacy XASM1 bytecode (23 original instructions + 0xff).\n\
        Example:\n  \
        xplc -i hello.xpl -o hello.bin --run"
)]
struct Cli {
    /// XPL source file
    #[arg(short, long, value_name = "FILE")]
    input: Option<PathBuf>,

    /// ROM output
    #[arg(short, long, value_name = "FILE", default_value = "out.bin")]
    output: PathBuf,

    /// Write the generated XASM assembly to a file
    #[arg(long, value_name = "FILE")]
    emit_asm: Option<PathBuf>,

    /// Print the generated XASM assembly to stdout
    #[arg(long)]
    print_asm: bool,

    /// Run the ROM on the Rust XASM VM after compiling
    #[arg(long)]
    run: bool,

    /// Highest VM address, inclusive
    #[arg(long, value_name = "ADDR", value_parser = parse_number, default_value = "0xffffe")]
    max_addr: u32,

    /// Print ROM and code-generation statistics
    #[arg(long)]
    stats: bool,

    /// Suppress progress messages
    #[arg(short, long)]
    quiet: bool,

    /// Print progress messages
    #[arg(short, long)]
    verbose: bool,

    /// XPL source file (positional form)
    #[arg(value_name = "INPUT")]
    positional: Option<PathBuf>,
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
    let input = cli
        .input
        .or(cli.positional)
        .ok_or_else(|| "no input file (use -i/--input or pass a path)".to_owned())?;
    let bytes = std::fs::read(&input)
        .map_err(|error| format!("cannot read `{}`: {error}", input.display()))?;
    let source = Source::new(input.display().to_string(), bytes);

    let options = Options {
        max_addr: cli.max_addr,
    };
    let generated = match compile(&source, &options) {
        Ok(generated) => generated,
        Err(Error::Semantic(report)) => {
            eprint!("{}", report.render(&source));
            eprintln!(
                "{} error(s), {} warning(s)",
                report.error_count(),
                report.warning_count()
            );
            return Ok(ExitCode::from(2));
        }
        Err(Error::Internal { report, asm }) => {
            eprintln!(
                "could not assemble the generated XASM program \
                 (usually the program is too large, or this is a compiler bug)."
            );
            let generated_source = Source::new("<xpl-codegen>", asm.as_bytes().to_vec());
            eprint!("{}", report.render(&generated_source));
            eprintln!("generated assembly:\n{asm}");
            return Ok(ExitCode::from(3));
        }
    };

    if cli.print_asm {
        print!("{}", generated.asm);
    }
    if let Some(path) = &cli.emit_asm {
        std::fs::write(path, &generated.asm)
            .map_err(|error| format!("cannot write `{}`: {error}", path.display()))?;
    }

    std::fs::write(&cli.output, &generated.rom)
        .map_err(|error| format!("cannot write `{}`: {error}", cli.output.display()))?;
    if !cli.quiet {
        if cli.verbose {
            eprintln!(
                "compiled `{}` -> `{}` ({} bytes, {} variables, {} temps)",
                input.display(),
                cli.output.display(),
                generated.stats.rom_size,
                generated.stats.variables,
                generated.stats.temps
            );
        } else {
            eprintln!(
                "compiled `{}` -> `{}` ({} bytes)",
                input.display(),
                cli.output.display(),
                generated.stats.rom_size
            );
        }
    }
    if cli.stats {
        let stats = &generated.stats;
        println!(
            "rom={} variables={} temps={} strings={} labels={} code_lines={}",
            stats.rom_size,
            stats.variables,
            stats.temps,
            stats.strings,
            stats.labels,
            stats.code_lines
        );
    }

    if cli.run {
        let mut vm = Vm::new(
            Memory::new_with_fill(cli.max_addr, -1),
            std::io::stdin(),
            std::io::stdout(),
            VmOptions::default(),
        );
        vm.load_rom(&generated.rom)
            .map_err(|error| error.to_string())?;
        vm.run().map_err(|error| error.to_string())?;
    }

    Ok(ExitCode::SUCCESS)
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
