use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use xasm_core::assembler::{AssembleOptions, DuplicateLabels, Target, VerifyLevel};
use xasm_core::{compile, CompileOptions, Source};

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum TargetArg {
    Legacy,
    Community,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum VerifyArg {
    Off,
    Warn,
    Error,
}

#[derive(Debug, Parser)]
#[command(
    name = "xasm",
    version,
    about = "XASM compiler for the original XASM1 VM",
    after_help = "COMPATIBILITY:\n  The original XASM1 VM implements opcodes 0x01..0x36 and 0xff.\n  The six community bit instructions (0x41..0x46) are rejected for\n  --target legacy. Use --target community only with the community VM.\n\n  With no arguments, xasm behaves like the original `compile` binary\n  when ./temp.asm exists (reads temp.asm, writes disk.bin)."
)]
struct Cli {
    /// Source file
    #[arg(short, long, value_name = "FILE")]
    input: Option<PathBuf>,

    /// ROM output (default: out.bin, or disk.bin with --legacy)
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Use the original fixed paths temp.asm -> disk.bin
    #[arg(long)]
    legacy: bool,

    /// Target VM
    #[arg(long, value_enum, default_value_t = TargetArg::Legacy)]
    target: TargetArg,

    /// Shortcut for --target community
    #[arg(long)]
    community: bool,

    /// Emit 0x41..0x46 for the legacy target (dangerous)
    #[arg(long)]
    allow_community: bool,

    /// Do not process \n, \t, \xHH, ... escapes in strings
    #[arg(long)]
    raw_strings: bool,

    /// Prepend a 14-byte jump to LABEL (for data-first programs)
    #[arg(long, value_name = "LABEL")]
    entry: Option<String>,

    /// Highest VM address, inclusive
    #[arg(
        long,
        value_name = "ADDR",
        value_parser = parse_number,
        default_value = "0xffffe"
    )]
    max_addr: u32,

    /// Verification level
    #[arg(long, value_enum, default_value_t = VerifyArg::Warn)]
    verify: VerifyArg,

    /// --verify error and reject duplicate labels
    #[arg(long)]
    strict: bool,

    /// Write a human-readable disassembly
    #[arg(long, value_name = "FILE")]
    listing: Option<PathBuf>,

    /// Print ROM statistics
    #[arg(long)]
    stats: bool,

    /// Compile and verify, do not write the ROM
    #[arg(long, visible_alias = "check-only")]
    check: bool,

    /// Suppress progress messages
    #[arg(short, long)]
    quiet: bool,

    /// Print progress messages
    #[arg(short, long)]
    verbose: bool,

    /// Source file (positional form)
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
    if let Some(entry) = &cli.entry {
        if entry.trim().is_empty() {
            return Err("--entry needs a non-empty label".to_owned());
        }
    }
    let mut legacy = cli.legacy;
    let mut input = cli.input.or(cli.positional);
    if input.is_none() && std::path::Path::new("temp.asm").is_file() {
        // Zero-argument drop-in compatibility with the original compiler.
        legacy = true;
        input = Some(PathBuf::from("temp.asm"));
    }
    let input = input.ok_or_else(|| "no input file (use -i/--input or pass a path)".to_owned())?;

    let output = cli.output.or_else(|| {
        if cli.check {
            None
        } else if legacy {
            Some(PathBuf::from("disk.bin"))
        } else {
            Some(PathBuf::from("out.bin"))
        }
    });

    let bytes = std::fs::read(&input)
        .map_err(|error| format!("cannot read `{}`: {error}", input.display()))?;
    let source = Source::new(input.display().to_string(), bytes);

    let target = if cli.community {
        Target::Community
    } else {
        match cli.target {
            TargetArg::Legacy => Target::Legacy,
            TargetArg::Community => Target::Community,
        }
    };
    let (verify, duplicate_labels) = if cli.strict {
        (VerifyLevel::Error, DuplicateLabels::Error)
    } else {
        (
            match cli.verify {
                VerifyArg::Off => VerifyLevel::Off,
                VerifyArg::Warn => VerifyLevel::Warn,
                VerifyArg::Error => VerifyLevel::Error,
            },
            DuplicateLabels::LastWins,
        )
    };

    let options = CompileOptions {
        raw_strings: cli.raw_strings,
        entry: cli.entry.clone(),
        assemble: AssembleOptions {
            target,
            allow_community: cli.allow_community,
            memory_limit: cli.max_addr,
            verify,
            duplicate_labels,
        },
    };

    let artifact = match compile(&source, &options) {
        Ok(artifact) => artifact,
        Err(report) => {
            eprint!("{}", report.render(&source));
            eprintln!(
                "{} error(s), {} warning(s)",
                report.error_count(),
                report.warning_count()
            );
            return Ok(ExitCode::from(2));
        }
    };

    if !artifact.report.is_empty() {
        eprint!("{}", artifact.report.render(&source));
    }

    if let Some(output) = &output {
        std::fs::write(output, &artifact.bytes)
            .map_err(|error| format!("cannot write `{}`: {error}", output.display()))?;
        if cli.verbose && !cli.quiet {
            eprintln!(
                "compiled `{}` -> `{}` ({} bytes, {} labels)",
                input.display(),
                output.display(),
                artifact.bytes.len(),
                artifact.labels.len()
            );
        }
    } else if cli.verbose && !cli.quiet {
        eprintln!(
            "checked `{}` ({} bytes, {} labels)",
            input.display(),
            artifact.bytes.len(),
            artifact.labels.len()
        );
    }

    if let Some(listing_path) = &cli.listing {
        let listing = xasm_core::listing::render(&artifact);
        std::fs::write(listing_path, listing)
            .map_err(|error| format!("cannot write `{}`: {error}", listing_path.display()))?;
    }

    if cli.stats {
        let stats = &artifact.stats;
        println!(
            "rom={} bytes labels={} source_ops={} instructions={} extension_ops={} unknown_bytes={} data_blocks={}",
            stats.rom_size,
            stats.labels,
            stats.source_ops,
            stats.decoded_instructions,
            stats.extension_ops,
            stats.unknown_bytes,
            stats.data_blocks
        );
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
