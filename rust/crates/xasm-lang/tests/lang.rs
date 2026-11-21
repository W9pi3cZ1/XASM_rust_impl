use std::io::Cursor;

use xasm_lang::{compile, Options, Source};
use xasm_vm::{ExitReason, Memory, RunOutcome, Vm, VmOptions};

fn compile_rom(source: &str) -> Result<Vec<u8>, String> {
    let source = Source::new("<test>", source.as_bytes().to_vec());
    compile(&source, &Options::default())
        .map(|generated| generated.rom)
        .map_err(|error| match error {
            xasm_lang::Error::Semantic(report) => report.render(&source),
            xasm_lang::Error::Internal { report, .. } => {
                let generated = Source::new("<xpl-codegen>", Vec::new());
                report.render(&generated)
            }
        })
}

fn run(source: &str, input: &[u8]) -> Result<(String, RunOutcome), String> {
    let rom = compile_rom(source)?;
    let mut output = Vec::new();
    let outcome = {
        let mut vm = Vm::new(
            Memory::new_with_fill(0xffffe, -1),
            Cursor::new(input.to_vec()),
            &mut output,
            VmOptions::default(),
        );
        vm.load_rom(&rom).map_err(|error| error.to_string())?;
        vm.run().map_err(|error| error.to_string())?
    };
    Ok((String::from_utf8_lossy(&output).into_owned(), outcome))
}

#[test]
fn hello_world() {
    let (output, outcome) = run(r#"print("Hello, XPL!\n");"#, b"").unwrap();
    assert_eq!(output, "Hello, XPL!\n");
    assert_eq!(outcome.reason, ExitReason::Exit);
}

#[test]
fn arithmetic_and_precedence() {
    let (output, _) = run(
        r#"print(1 + 2 * 3, " ", (1 + 2) * 3, " ", 17 / 5, " ", 17 % 5, "\n");"#,
        b"",
    )
    .unwrap();
    assert_eq!(output, "7 9 3 2\n");

    let (output, _) = run(r#"print(-7 / 2, " ", -7 % 2, " ", 0xffffffff, "\n");"#, b"").unwrap();
    assert_eq!(output, "-3 -1 -1\n");
}

#[test]
fn comparisons_and_booleans() {
    let (output, _) = run(
        r#"print(1 < 2, 2 <= 2, 3 > 4, 4 >= 4, 5 == 5, 5 != 5, "\n");"#,
        b"",
    )
    .unwrap();
    assert_eq!(output, "110110\n");

    let (output, _) = run(
        r#"print(true && false, true || false, !false, !true, "\n");"#,
        b"",
    )
    .unwrap();
    assert_eq!(output, "0110\n");
}

#[test]
fn boolean_operators_short_circuit() {
    // If `&&` were eager, read_int() would consume 7 and y would be 0.
    let (output, _) = run(
        r#"let x = false && read_int();
           let y = read_int();
           print(x, " ", y, "\n");"#,
        b"7\n",
    )
    .unwrap();
    assert_eq!(output, "0 7\n");

    let (output, _) = run(
        r#"let x = true || read_int();
           let y = read_int();
           print(x, " ", y, "\n");"#,
        b"7\n",
    )
    .unwrap();
    assert_eq!(output, "1 7\n");
}

#[test]
fn if_else_and_while() {
    let (output, _) = run(
        r#"let x = 5;
           if (x > 3) { print("big"); } else { print("small"); }
           println();
           let i = 3;
           while (i > 0) { print(i, " "); i = i - 1; }
           println();"#,
        b"",
    )
    .unwrap();
    assert_eq!(output, "big\n3 2 1 \n");
}

#[test]
fn block_scopes_and_shadowing() {
    let (output, _) = run(
        r#"let x = 10;
           {
               let x = x + 1;
               print(x, " ");
           }
           print(x, "\n");"#,
        b"",
    )
    .unwrap();
    assert_eq!(output, "11 10\n");
}

#[test]
fn input_sum_and_char() {
    let (output, _) = run(
        r#"let a = read_int();
           let b = read_int();
           print(a + b, "\n");
           let c = read_char();
           putchar(c);
           println();"#,
        b"20 22H",
    )
    .unwrap();
    assert_eq!(output, "42\nH\n");
}

#[test]
fn strings_and_escapes() {
    let (output, _) = run(r#"print("a\tb\\c\"d\n"); putchar(65); println();"#, b"").unwrap();
    assert_eq!(output, "a\tb\\c\"d\nA\n");
}

#[test]
fn nested_loops_and_primes() {
    let (output, _) = run(
        r#"let n = 2;
           while (n <= 20) {
               let d = 2;
               let prime = true;
               while (d * d <= n) {
                   if (n % d == 0) { prime = false; }
                   d = d + 1;
               }
               if (prime) { print(n, " "); }
               n = n + 1;
           }
           println();"#,
        b"",
    )
    .unwrap();
    assert_eq!(output, "2 3 5 7 11 13 17 19 \n");
}

#[test]
fn errors_are_reported() {
    let error = compile_rom("let x = y;").unwrap_err();
    assert!(error.contains("undefined variable `y`"), "{error}");

    let error = compile_rom("let x = 1; let x = 2;").unwrap_err();
    assert!(error.contains("already declared"), "{error}");

    let error = compile_rom("print(1 / 0);").unwrap_err();
    assert!(error.contains("division by zero"), "{error}");

    let error = compile_rom("if (1) { print(1); ").unwrap_err();
    assert!(error.contains("expected"), "{error}");
}

#[test]
fn poke_and_puthex_at() {
    let (output, _) = run(
        r#"poke(0x8000, 42);
           puthex_at(0x8000);
           print(" ");
           poke(0x8001, 7);
           puthex_at(0x8001);
           println();
           puthex(0x2a);
           print(" ");
           puthex(255);
           println();"#,
        b"",
    )
    .unwrap();
    assert_eq!(output, "2a 7\n2a ff\n");
}

#[test]
fn read_hex_input() {
    let (output, _) = run(
        r#"let x = read_hex();
           print(x, "\n");
           puthex(x);
           println();"#,
        b"ff\n",
    )
    .unwrap();
    assert_eq!(output, "255\nff\n");
}

#[test]
fn memory_editor_compiles_and_quits() {
    let source = include_str!("../../../examples/xpl/memedit.xpl");
    let (output, outcome) = run(source, b"q").unwrap();
    assert_eq!(outcome.reason, ExitReason::Exit);
    assert!(output.contains("XASM memory editor"), "{output}");
}

#[test]
fn peek_and_padded_hex() {
    let (output, _) = run(
        r#"poke(0x8000, 5);
           let b = peek(0x8000) % 256;
           puthex(b / 16);
           puthex(b % 16);
           println();"#,
        b"",
    )
    .unwrap();
    assert_eq!(output, "05\n");
}

#[test]
fn functions_and_arrays() {
    let (output, _) = run(
        r#"fn add(a, b) { return a + b; }
           fn twice(x) { return add(x, x); }
           let a[5];
           let i = 0;
           while (i < 5) {
               a[i] = i * i;
               i = i + 1;
           }
           print(add(a[3], twice(a[2])), "\n");"#,
        b"",
    )
    .unwrap();
    assert_eq!(output, "17\n");
}

#[test]
fn function_default_return_and_local_array() {
    let (output, _) = run(
        r#"fn zero() { }
           fn sum() {
               let a[3];
               a[0] = 2;
               a[1] = 3;
               a[2] = 4;
               return a[0] + a[1] + a[2];
           }
           print(zero(), " ", sum(), "\n");"#,
        b"",
    )
    .unwrap();
    assert_eq!(output, "0 9\n");
}

#[test]
fn recursive_function_is_rejected() {
    let error = compile_rom(
        r#"fn f(n) { return f(n - 1); }
           print(f(3), "\n");"#,
    )
    .unwrap_err();
    assert!(error.contains("recursive call"), "{error}");
}

#[test]
fn pi_example_compiles() {
    let source = include_str!("../../../examples/xpl/pi.xpl");
    let rom = compile_rom(source).unwrap();
    assert!(!rom.is_empty());
}

#[test]
fn readline_function() {
    let source = include_str!("../../../examples/xpl/readline.xpl");
    let (output, _) = run(source, b"hello\n").unwrap();
    assert_eq!(output, "> hello\nhello\n");
}

#[test]
fn address_of_variables_and_arrays() {
    let (output, _) = run(
        r#"let x = 7;
           let p = &x;
           print(peek(p), "\n");
           poke(p, 9);
           print(x, "\n");

           let a[3];
           a[1] = 42;
           let q = &a[1];
           print(peek(q), "\n");
           print(peek(&a), "\n");"#,
        b"",
    )
    .unwrap();
    assert_eq!(output, "7\n9\n42\n0\n");
}

#[test]
fn address_of_temporary_is_rejected() {
    let error = compile_rom(r#"let p = &(1 + 2);"#).unwrap_err();
    assert!(error.contains("take the address"), "{error}");
}
