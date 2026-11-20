//! Differential tests against the original compiler binary and the community
//! rebuild.  They are skipped automatically when those binaries are absent.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use xasm_core::assembler::{AssembleOptions, Target, VerifyLevel};
use xasm_core::{compile, CompileOptions, Source};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn temp_dir(tag: &str) -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("xasm-diff-{}-{}-{}", std::process::id(), tag, id));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn ours(source: &[u8], raw_strings: bool, target: Target) -> Vec<u8> {
    let source = Source::new("<diff>", source.to_vec());
    let options = CompileOptions {
        raw_strings,
        entry: None,
        assemble: AssembleOptions {
            target,
            verify: VerifyLevel::Off,
            ..AssembleOptions::default()
        },
    };
    compile(&source, &options)
        .unwrap_or_else(|report| panic!("our compiler failed:\n{}", report.render(&source)))
        .bytes
}

fn original_compiler() -> Option<PathBuf> {
    let path = workspace_root()
        .parent()
        .unwrap()
        .join("XASM_Community_Rebuild/original/XASM_V1/compile");
    path.is_file().then_some(path)
}

fn run_original(source: &[u8]) -> Option<Vec<u8>> {
    let compiler = original_compiler()?;
    let dir = temp_dir("orig");
    std::fs::write(dir.join("temp.asm"), source).expect("write temp.asm");
    let output = Command::new(&compiler)
        .current_dir(&dir)
        .output()
        .expect("run original compiler");
    let rom = std::fs::read(dir.join("disk.bin")).unwrap_or_default();
    if !output.status.success() {
        eprintln!(
            "original compiler failed: status={:?} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
    Some(rom)
}

fn compare_case(name: &str, source: &[u8], raw_strings: bool) {
    let Some(expected) = run_original(source) else {
        return;
    };
    let actual = ours(source, raw_strings, Target::Legacy);
    assert_eq!(
        actual,
        expected,
        "case `{name}` differs\nsource:\n{}",
        String::from_utf8_lossy(source)
    );
}

#[test]
fn original_temp_program_is_byte_identical() {
    let path = workspace_root().join("tests/fixtures/temp.asm");
    let source = std::fs::read(path).expect("fixture");
    compare_case("temp.asm", &source, true);
}

#[test]
fn all_original_instructions_and_comments_match() {
    let mut source = String::new();
    source.push_str("// generated conformance program\n");
    for mnemonic in xasm_core::mnemonics::MNEMONICS {
        if mnemonic.extension || mnemonic.name == "sto" {
            continue;
        }
        source.push_str(mnemonic.name);
        for index in 0..mnemonic.arity as usize {
            source.push_str(&format!(
                " 0x{:02x} 0x{:02x} 0x{:02x} 0x{:02x}",
                index,
                index + 1,
                index + 2,
                index + 3
            ));
        }
        source.push('\n');
    }
    source.push_str("0xff 0x01 0x02 0xff\n");
    source.push_str("\"A raw string without escapes\"\n");
    source.push_str("exit\n");
    compare_case("all instructions", source.as_bytes(), true);
}

#[test]
fn randomized_original_programs_match() {
    let mut state = 0x1234_5678_9abc_def0u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let instructions: Vec<_> = xasm_core::mnemonics::MNEMONICS
        .iter()
        .filter(|m| !m.extension && m.name != "sto")
        .collect();

    for case in 0..40 {
        let mut source = String::new();
        source.push_str("// random case\n");
        for _ in 0..12 {
            let mnemonic = instructions[(next() as usize) % instructions.len()];
            source.push_str(mnemonic.name);
            for _ in 0..mnemonic.arity {
                for _ in 0..4 {
                    source.push_str(&format!(" 0x{:02x}", (next() & 0xff) as u8));
                }
            }
            source.push('\n');
            if next() % 7 == 0 {
                source.push_str("\"random string\"\n");
            }
            if next() % 9 == 0 {
                source.push_str("0xff 0x00 0x11 0xff\n");
            }
        }
        source.push_str("exit\n");
        compare_case(&format!("random {case}"), source.as_bytes(), true);
    }
}

fn community_compiler() -> Option<PathBuf> {
    let path = workspace_root()
        .parent()
        .unwrap()
        .join("XASM_Community_Rebuild/build/compiler");
    path.is_file().then_some(path)
}

fn collect_xasm(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_xasm(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("xasm") {
            out.push(path);
        }
    }
}

#[test]
fn community_examples_match_the_community_compiler() {
    let Some(compiler) = community_compiler() else {
        return;
    };
    let mut examples = Vec::new();
    collect_xasm(
        &workspace_root()
            .parent()
            .unwrap()
            .join("XASM_Community_Rebuild/examples"),
        &mut examples,
    );
    examples.sort();

    for path in examples {
        let dir = temp_dir("community");
        let output_path = dir.join("out.bin");
        // Some example programs hang the community compiler; never let the
        // test suite block on an external binary.
        let status = Command::new("timeout")
            .arg("3")
            .arg(&compiler)
            .arg("-i")
            .arg(&path)
            .arg("-o")
            .arg(&output_path)
            .output()
            .expect("run community compiler");
        if !status.status.success() {
            let _ = std::fs::remove_dir_all(&dir);
            continue;
        }
        let expected = std::fs::read(&output_path).unwrap_or_default();
        let source = std::fs::read(&path).expect("read example");
        let actual = ours(&source, false, Target::Community);
        assert_eq!(
            actual,
            expected,
            "community example `{}` differs",
            path.display()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
