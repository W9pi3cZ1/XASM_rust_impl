use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_xasm")
}

fn temp_dir(tag: &str) -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("xasm-cli-{}-{}-{}", std::process::id(), tag, id));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn run(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(bin())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run xasm")
}

const HELLO: &str = "\
mov 0x00 0x00 0x01 0x00 0x00 0x00 0x00 0x48
mov 0x00 0x00 0x01 0x01 0x00 0x00 0x01 0x00
putc 0x00 0x00 0x01 0x01
exit
";

#[test]
fn legacy_mode_uses_temp_asm_and_disk_bin() {
    let dir = temp_dir("legacy");
    std::fs::write(dir.join("temp.asm"), HELLO).unwrap();
    let output = run(&dir, &["--legacy"]);
    assert!(output.status.success(), "{:?}", output);
    let rom = std::fs::read(dir.join("disk.bin")).unwrap();
    assert_eq!(rom.len(), 24);
    assert_eq!(rom[0], 0x21);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn community_opcode_is_rejected_without_target_community() {
    let dir = temp_dir("ext");
    std::fs::write(
        dir.join("ext.xasm"),
        "& 0x00 0x00 0x00 0x01 0x00 0x00 0x00 0x01\n",
    )
    .unwrap();
    let output = run(&dir, &["-i", "ext.xasm", "-o", "out.bin"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(!dir.join("out.bin").exists());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("community-only instruction"), "{stderr}");

    let output = run(
        &dir,
        &["-i", "ext.xasm", "-o", "out.bin", "--target", "community"],
    );
    assert!(output.status.success(), "{:?}", output);
    let rom = std::fs::read(dir.join("out.bin")).unwrap();
    assert_eq!(rom[0], 0x41);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn listing_and_stats_are_written() {
    let dir = temp_dir("listing");
    std::fs::write(dir.join("hello.xasm"), HELLO).unwrap();
    let output = run(
        &dir,
        &[
            "-i",
            "hello.xasm",
            "-o",
            "hello.bin",
            "--listing",
            "hello.lst",
            "--stats",
        ],
    );
    assert!(output.status.success(), "{:?}", output);
    let listing = std::fs::read_to_string(dir.join("hello.lst")).unwrap();
    assert!(listing.contains("putc"), "{listing}");
    assert!(listing.contains("exit"), "{listing}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rom=24 bytes"), "{stdout}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn check_only_does_not_write_a_rom() {
    let dir = temp_dir("check");
    std::fs::write(dir.join("hello.xasm"), HELLO).unwrap();
    let output = run(&dir, &["-i", "hello.xasm", "--check"]);
    assert!(output.status.success(), "{:?}", output);
    assert!(!dir.join("out.bin").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn raw_strings_option() {
    let dir = temp_dir("raw");
    std::fs::write(dir.join("raw.xasm"), "\"A\\n\"\n").unwrap();
    let output = run(&dir, &["-i", "raw.xasm", "-o", "raw.bin", "--raw-strings"]);
    assert!(output.status.success(), "{:?}", output);
    assert_eq!(std::fs::read(dir.join("raw.bin")).unwrap(), b"A\\n");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn zero_arguments_use_legacy_paths_when_temp_asm_exists() {
    let dir = temp_dir("zero");
    std::fs::write(dir.join("temp.asm"), HELLO).unwrap();
    let output = run(&dir, &[]);
    assert!(output.status.success(), "{:?}", output);
    assert!(dir.join("disk.bin").is_file());
    let _ = std::fs::remove_dir_all(&dir);
}
