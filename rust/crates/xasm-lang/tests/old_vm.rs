//! End-to-end: compile XPL and run the ROM on the original XASM1 binary.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use xasm_lang::{compile, Options, Source};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("."))
}

#[test]
fn original_vm_runs_xpl_program() {
    let vm = workspace_root()
        .parent()
        .unwrap()
        .join("XASM_Community_Rebuild/original/XASM_V1/XASM1");
    if !vm.is_file() {
        eprintln!("skipping: original XASM1 binary not found");
        return;
    }
    if Command::new("timeout").arg("--version").output().is_err() {
        eprintln!("skipping: `timeout` not available");
        return;
    }

    let source = Source::new(
        "<xpl-old-vm>",
        br#"print("Hello from XPL on the original VM!\n");
            let i = 3;
            while (i > 0) { print(i, " "); i = i - 1; }
            println();
            poke(0x8000, 65);
            puthex_at(0x8000);
            println();

            fn add(a, b) { return a + b; }
            let a[3];
            a[0] = 7;
            a[1] = 8;
            a[2] = 9;
            print(add(a[0], add(a[1], a[2])), "\n");

            let bx = 6;
            let by = 3;
            print(bx & by, " ", bx << 2, "\n");"#
            .to_vec(),
    );
    let generated = compile(&source, &Options::default()).expect("compile XPL");
    assert!(!generated.rom.is_empty());

    let dir = std::env::temp_dir().join(format!("xpl-oldvm-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    std::fs::write(dir.join("disk.bin"), &generated.rom).expect("write disk.bin");

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
        "original VM failed: {:?} {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Hello from XPL on the original VM!\n3 2 1 \n41\n24\n2 24\n"
    );
}
