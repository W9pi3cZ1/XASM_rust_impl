use std::process::{Command, Stdio};

#[test]
fn xasm_run_zero_args_reads_disk_bin() {
    let dir = std::env::temp_dir().join(format!("xasm-run-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // mov 0x100, 'H'; mov 0x101, 0x100; putc 0x101; exit
    let rom = [
        0x21, 0, 0, 1, 0, 0, 0, 0, 0x48, 0x21, 0, 0, 1, 1, 0, 0, 1, 0, 0x31, 0, 0, 1, 1, 0x2a,
    ];
    std::fs::write(dir.join("disk.bin"), rom).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_xasm-run"))
        .current_dir(&dir)
        .stdin(Stdio::null())
        .output()
        .expect("run xasm-run");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, b"H");
}
