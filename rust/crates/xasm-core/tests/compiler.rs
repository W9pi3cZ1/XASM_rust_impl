use xasm_core::assembler::{AssembleOptions, DuplicateLabels, Target, VerifyLevel};
use xasm_core::{compile, CompileOptions, Source};

fn options(target: Target, verify: VerifyLevel) -> CompileOptions {
    CompileOptions {
        raw_strings: false,
        entry: None,
        assemble: AssembleOptions {
            target,
            verify,
            ..AssembleOptions::default()
        },
    }
}

fn compile_str(source: &str, options: &CompileOptions) -> Result<xasm_core::Artifact, String> {
    let source = Source::new("<test>", source.as_bytes().to_vec());
    compile(&source, options).map_err(|report| report.render(&source))
}

#[test]
fn every_mnemonic_encodes_to_its_opcode() {
    for mnemonic in xasm_core::mnemonics::MNEMONICS {
        let mut source = String::new();
        source.push_str(mnemonic.name);
        for _ in 0..mnemonic.arity {
            source.push_str(" 0x00 0x00 0x00 0x00");
        }
        source.push('\n');
        let target = if mnemonic.extension {
            Target::Community
        } else {
            Target::Legacy
        };
        let artifact = compile_str(&source, &options(target, VerifyLevel::Off))
            .unwrap_or_else(|error| panic!("{}: {error}", mnemonic.name));
        assert_eq!(
            artifact.bytes.first().copied(),
            Some(mnemonic.opcode),
            "mnemonic {}",
            mnemonic.name
        );
        assert_eq!(artifact.bytes.len(), 1 + 4 * mnemonic.arity as usize);
    }
}

#[test]
fn labels_and_refs_are_big_endian_absolute_offsets() {
    let source = "\
[msg] \"Hi\"
[ptr] 0x00 0x00 0x00 0x00
mov @ptr @msg
";
    let artifact = compile_str(source, &options(Target::Legacy, VerifyLevel::Off)).unwrap();
    assert_eq!(artifact.labels["msg"], 0);
    assert_eq!(artifact.labels["ptr"], 2);
    assert_eq!(
        artifact.bytes,
        vec![
            b'H', b'i', 0, 0, 0, 0, // msg + ptr cell
            0x21, 0, 0, 0, 2, // mov @ptr
            0, 0, 0, 0, // @msg
        ]
    );
}

#[test]
fn duplicate_labels_use_the_last_definition_like_the_community_compiler() {
    let source = "\
[a] 0x01
[a] 0x02
mov @a 0x00 0x00 0x00 0x00
";
    let artifact = compile_str(source, &options(Target::Legacy, VerifyLevel::Off)).unwrap();
    assert_eq!(artifact.labels["a"], 1);
    assert_eq!(artifact.bytes, vec![1, 2, 0x21, 0, 0, 0, 1, 0, 0, 0, 0]);
}

#[test]
fn community_string_escapes() {
    let source = r#""A\n\x41\0\\\"""#;
    let artifact = compile_str(source, &options(Target::Community, VerifyLevel::Off)).unwrap();
    assert_eq!(artifact.bytes, b"A\nA0\\\"");
}

#[test]
fn raw_strings_keep_backslashes() {
    let mut opts = options(Target::Legacy, VerifyLevel::Off);
    opts.raw_strings = true;
    let artifact = compile_str(r#""A\n""#, &opts).unwrap();
    assert_eq!(artifact.bytes, b"A\\n");
}

#[test]
fn all_comment_styles_are_ignored() {
    let source = "\
// line
# hash line
-- dash line
/* block
   comment */
add 0x00 0x00 0x00 0x01 0x00 0x00 0x00 0x02
";
    let artifact = compile_str(source, &options(Target::Legacy, VerifyLevel::Off)).unwrap();
    assert_eq!(artifact.bytes, vec![0x01, 0, 0, 0, 1, 0, 0, 0, 2]);
}

#[test]
fn community_opcodes_are_rejected_for_the_legacy_target() {
    let source = "& 0x00 0x00 0x00 0x01 0x00 0x00 0x00 0x01\n";
    let error = compile_str(source, &options(Target::Legacy, VerifyLevel::Warn)).unwrap_err();
    assert!(error.contains("community-only instruction"), "{error}");
    assert!(error.contains("0x41"), "{error}");

    let artifact = compile_str(source, &options(Target::Community, VerifyLevel::Off)).unwrap();
    assert_eq!(artifact.bytes[0], 0x41);
}

#[test]
fn legacy_target_accepts_sto_data_blocks() {
    let source = "sto 0x01 0x02 sto exit\n";
    let artifact = compile_str(source, &options(Target::Legacy, VerifyLevel::Warn)).unwrap();
    assert_eq!(artifact.bytes, vec![0xff, 0x01, 0x02, 0xff, 0x2a]);
}

#[test]
fn undefined_label_is_an_error() {
    let error = compile_str(
        "mov @missing 0x00 0x00 0x00 0x00\n",
        &options(Target::Legacy, VerifyLevel::Off),
    )
    .unwrap_err();
    assert!(error.contains("undefined label `missing`"), "{error}");
}

#[test]
fn rom_size_is_limited_by_the_configured_memory() {
    let mut opts = options(Target::Legacy, VerifyLevel::Off);
    opts.assemble.memory_limit = 0x10;
    let source = "0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00\n";
    let error = compile_str(source, &opts).unwrap_err();
    assert!(error.contains("can load at most"), "{error}");
}

#[test]
fn warns_when_execution_starts_with_data() {
    let source = "\"Hello\"\n";
    let source = Source::new("<test>", source.as_bytes().to_vec());
    let artifact = compile(&source, &options(Target::Legacy, VerifyLevel::Warn)).unwrap();
    let rendered = artifact.report.render(&source);
    assert!(
        rendered.contains("execution starts at 0x00000000"),
        "{rendered}"
    );
    assert_eq!(artifact.bytes, b"Hello");
}

#[test]
fn original_temp_fixture_is_byte_identical() {
    let mut opts = options(Target::Legacy, VerifyLevel::Off);
    opts.raw_strings = true;
    let source = Source::new(
        "tests/fixtures/temp.asm",
        include_bytes!("../../../tests/fixtures/temp.asm").to_vec(),
    );
    let artifact = compile(&source, &opts).unwrap();
    assert_eq!(
        artifact.bytes.as_slice(),
        include_bytes!("../../../tests/fixtures/temp.disk.bin")
    );
}

#[test]
fn strict_duplicate_labels_are_an_error() {
    let mut opts = options(Target::Legacy, VerifyLevel::Off);
    opts.assemble.duplicate_labels = DuplicateLabels::Error;
    let error = compile_str("[a] 0x00\n[a] 0x01\n", &opts).unwrap_err();
    assert!(error.contains("duplicate label `a`"), "{error}");
}

#[test]
fn bare_hex_bytes_and_out_of_range_values() {
    let artifact = compile_str("ff 10\n", &options(Target::Legacy, VerifyLevel::Off)).unwrap();
    assert_eq!(artifact.bytes, vec![0xff, 0x10]);

    let error = compile_str("0x100\n", &options(Target::Legacy, VerifyLevel::Off)).unwrap_err();
    assert!(error.contains("does not fit in one byte"), "{error}");
}

#[test]
fn entry_trampoline_jumps_to_a_label() {
    let mut opts = options(Target::Legacy, VerifyLevel::Off);
    opts.entry = Some("main".to_owned());
    let source = "\"data\"\n[main]\nmov 0x00 0x00 0x01 0x00 0x00 0x00 0x00 0x48\nputc 0x00 0x00 0x01 0x00\nexit\n";
    let artifact = compile_str(source, &opts).unwrap();
    assert_eq!(artifact.bytes[0], 0x21);
    assert_eq!(artifact.bytes[9], 0x23);
    assert_eq!(artifact.labels["main"], 14 + 4);
    assert_eq!(
        &artifact.bytes[..14],
        &[0x21, 0, 0, 0, 0, 0, 0, 0, 18, 0x23, 0, 0, 0, 0]
    );
}
