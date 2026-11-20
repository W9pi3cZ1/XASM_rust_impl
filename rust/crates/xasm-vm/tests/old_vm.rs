//! Run a Rust-compiled ROM on the original XASM1 binary.
//!
//! This is the end-to-end compatibility check.  It is skipped when the
//! original binary (or the `timeout` utility) is unavailable.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use xasm_core::assembler::{AssembleOptions, Target, VerifyLevel};
use xasm_core::{compile, CompileOptions, Source};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn original_vm() -> Option<PathBuf> {
    let path = workspace_root()
        .parent()
        .unwrap()
        .join("XASM_Community_Rebuild/original/XASM_V1/XASM1");
    path.is_file().then_some(path)
}

#[test]
fn original_vm_runs_rust_compiled_rom() {
    let Some(vm) = original_vm() else {
        eprintln!("skipping: original XASM1 binary not found");
        return;
    };
    if Command::new("timeout").arg("--version").output().is_err() {
        eprintln!("skipping: `timeout` not available");
        return;
    }

    let source = "\
mov 0x00 0x00 0x01 0x00 0x00 0x00 0x00 0x48
mov 0x00 0x00 0x01 0x01 0x00 0x00 0x01 0x00
putc 0x00 0x00 0x01 0x01
exit
";
    let source = Source::new("<old-vm-test>", source.as_bytes().to_vec());
    let options = CompileOptions {
        raw_strings: true,
        entry: None,
        assemble: AssembleOptions {
            target: Target::Legacy,
            verify: VerifyLevel::Off,
            ..AssembleOptions::default()
        },
    };
    let artifact = compile(&source, &options).expect("compile");
    assert_eq!(artifact.bytes.len(), 24);

    let dir = std::env::temp_dir().join(format!("xasm-oldvm-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    std::fs::write(dir.join("disk.bin"), &artifact.bytes).expect("write disk.bin");

    let output = Command::new("timeout")
        .arg("30")
        .arg(&vm)
        .current_dir(&dir)
        .stdin(Stdio::null())
        .output()
        .expect("run original VM");

    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        output.status.success(),
        "original VM failed: status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"H");
}
