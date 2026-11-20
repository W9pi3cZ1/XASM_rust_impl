//! XPL to XASM assembly / bytecode code generation.
//!
//! The generated program uses a simple flat-memory ABI:
//!
//! * every variable and temporary is one i32 cell in the data segment;
//! * expressions write their result into a cell;
//! * comparisons and boolean operators materialise 0/1;
//! * `@__jmp` is a scratch cell used by every conditional/unconditional jump;
//! * strings are byte arrays printed by a length-counted loop.
//!
//! No stack or function calls are needed for the supported language.

use std::collections::HashMap;

use crate::ast::*;
use xasm_core::assembler::{AssembleOptions, DuplicateLabels, Target, VerifyLevel};
use xasm_core::diag::Report;
use xasm_core::{compile, CompileOptions, Source};

#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub rom_size: usize,
    pub variables: usize,
    pub temps: usize,
    pub strings: usize,
    pub labels: usize,
    pub code_lines: usize,
}

#[derive(Clone, Debug)]
pub struct Generated {
    pub asm: String,
    pub rom: Vec<u8>,
    pub stats: Stats,
}

#[derive(Debug)]
pub enum Error {
    /// A user-level error (undefined variable, duplicate let, ...).
    Semantic(Report),
    /// The generated assembly failed to assemble.  This is a compiler bug.
    Internal { report: Report, asm: String },
}

pub fn generate(program: &Program, memory_limit: u32) -> Result<Generated, Error> {
    let mut codegen = Codegen::new();
    codegen.program(program);
    if codegen.report.has_errors() {
        return Err(Error::Semantic(codegen.report));
    }
    let code_size = match codegen.code_size() {
        Ok(size) => size,
        Err(report) => {
            return Err(Error::Internal {
                report,
                asm: codegen.probe_asm(),
            })
        }
    };
    let asm = codegen.finish(code_size);
    let source = Source::new("<xpl-codegen>", asm.as_bytes().to_vec());
    let options = CompileOptions {
        raw_strings: true,
        entry: None,
        assemble: AssembleOptions {
            target: Target::Legacy,
            allow_community: false,
            memory_limit,
            verify: VerifyLevel::Off,
            duplicate_labels: DuplicateLabels::Error,
        },
    };
    match compile(&source, &options) {
        Ok(artifact) => {
            let stats = Stats {
                rom_size: artifact.bytes.len(),
                ..codegen.stats
            };
            Ok(Generated {
                asm,
                rom: artifact.bytes,
                stats,
            })
        }
        Err(report) => Err(Error::Internal { report, asm }),
    }
}

struct Codegen {
    out: String,
    /// Variable/temporary cells.  These are laid out on 256-byte boundaries.
    cells: Vec<String>,
    cell_labels: Vec<String>,
    strings: Vec<String>,
    string_labels: Vec<String>,
    label_id: usize,
    var_id: usize,
    temp_id: usize,
    str_id: usize,
    scopes: Vec<HashMap<String, String>>,
    report: Report,
    stats: Stats,
}

impl Codegen {
    fn new() -> Self {
        Self {
            out: String::new(),
            cells: Vec::new(),
            cell_labels: Vec::new(),
            strings: Vec::new(),
            string_labels: Vec::new(),
            label_id: 0,
            var_id: 0,
            temp_id: 0,
            str_id: 0,
            scopes: vec![HashMap::new()],
            report: Report::new(),
            stats: Stats::default(),
        }
    }

    fn program(&mut self, program: &Program) {
        // Constant used by `peek` to split a 32-bit address into bytes.
        self.emit("mov @__c256 0x00 0x00 0x01 0x00");
        for stmt in &program.stmts {
            if self.stmt(stmt).is_err() {
                return;
            }
        }
        self.emit("exit");
    }

    const CONST_LABELS: [&'static str; 7] = [
        "__zero",
        "__one",
        "__newline",
        "__jmp",
        "__ptr",
        "__cnt",
        "__c256",
    ];

    fn probe_asm(&self) -> String {
        let mut probe = self.out.clone();
        for label in Self::CONST_LABELS {
            probe.push('[');
            probe.push_str(label);
            probe.push_str("]\n");
        }
        for label in &self.cell_labels {
            probe.push('[');
            probe.push_str(label);
            probe.push_str("]\n");
        }
        for label in &self.string_labels {
            probe.push('[');
            probe.push_str(label);
            probe.push_str("]\n");
        }
        probe
    }

    /// Determine the byte size of the code section by assembling it with all
    /// data labels defined as zero-size labels at the end.
    fn code_size(&self) -> Result<usize, xasm_core::Report> {
        let probe = self.probe_asm();
        let source = Source::new("<xpl-probe>", probe.as_bytes().to_vec());
        let options = CompileOptions {
            raw_strings: true,
            entry: None,
            assemble: AssembleOptions {
                target: Target::Legacy,
                allow_community: false,
                memory_limit: u32::MAX,
                verify: VerifyLevel::Off,
                duplicate_labels: DuplicateLabels::LastWins,
            },
        };
        compile(&source, &options).map(|artifact| artifact.bytes.len())
    }

    fn finish(&mut self, code_size: usize) -> String {
        let mut out = std::mem::take(&mut self.out);

        // Align the first variable/temp cell to a 256-byte boundary.  The
        // original XASM1 has a dispatch bug after `copy`: it reinterprets the
        // low byte of the source address as an opcode.  Placing every copy
        // source at ...00 makes that byte 0x00 (NOP), which is safe.
        let padding = (256 - code_size % 256) % 256;
        for _ in 0..padding {
            out.push_str("0x00\n");
        }
        for line in &self.cells {
            out.push_str(line);
            out.push('\n');
            for _ in 0..255 {
                out.push_str("0x00\n");
            }
        }

        out.push_str("[__zero] 0x00\n");
        out.push_str("[__one] 0x01\n");
        out.push_str("[__newline] 0x0a\n");
        out.push_str("[__jmp] 0x00\n");
        out.push_str("[__ptr] 0x00\n");
        out.push_str("[__cnt] 0x00\n");
        out.push_str("[__c256] 0x00\n");
        for line in &self.strings {
            out.push_str(line);
            out.push('\n');
        }
        out
    }

    fn emit(&mut self, line: &str) {
        self.out.push_str(line);
        self.out.push('\n');
        self.stats.code_lines += 1;
    }

    fn emit_label(&mut self, label: &str) {
        self.out.push('[');
        self.out.push_str(label);
        self.out.push_str("]\n");
    }

    /// Labels that can be the target of a conditional jump start with a NOP.
    ///
    /// The original XASM1 has a fall-through bug after `>`/`<`: it re-checks
    /// the opcode at the new PC against later opcodes in the dispatch chain.
    /// A NOP at every control-flow target makes those checks harmless.
    fn emit_control_label(&mut self, label: &str) {
        // The old VM checks `memory[local]` after a taken `>`/`<` with
        // local = target - 1.  Put a NOP both before and at the label.
        self.emit("0x00");
        self.emit_label(label);
        self.emit("0x00");
    }

    fn emit_cond(&mut self, op: &str, a: &str, b: &str, target: &str) {
        self.emit(&format!("{op} @{a} @{b} @{target}"));
        self.emit("0x00");
    }

    fn new_label(&mut self, prefix: &str) -> String {
        let label = format!("{prefix}_{}", self.label_id);
        self.label_id += 1;
        self.stats.labels += 1;
        label
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn alloc_var(&mut self, name: &str, span: xasm_core::Span) -> Result<String, ()> {
        if self.scopes.last().unwrap().contains_key(name) {
            self.report.error(
                span,
                format!("variable `{name}` is already declared in this scope"),
            );
            return Err(());
        }
        let label = format!("v{}", self.var_id);
        self.var_id += 1;
        self.stats.variables += 1;
        self.cells.push(format!("[{label}] 0x00"));
        self.cell_labels.push(label.clone());
        Ok(label)
    }

    fn bind(&mut self, name: &str, label: &str) {
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name.to_owned(), label.to_owned());
    }

    fn lookup(&mut self, name: &str, span: xasm_core::Span) -> Result<String, ()> {
        for scope in self.scopes.iter().rev() {
            if let Some(label) = scope.get(name) {
                return Ok(label.clone());
            }
        }
        self.report
            .error(span, format!("undefined variable `{name}`"));
        Err(())
    }

    fn temp(&mut self) -> String {
        let label = format!("t{}", self.temp_id);
        self.temp_id += 1;
        self.stats.temps += 1;
        self.cells.push(format!("[{label}] 0x00"));
        self.cell_labels.push(label.clone());
        label
    }

    fn string(&mut self, bytes: &[u8]) -> String {
        let label = format!("s{}", self.str_id);
        self.str_id += 1;
        self.stats.strings += 1;
        let mut line = format!("[{label}]");
        for byte in bytes {
            line.push_str(&format!(" 0x{byte:02x}"));
        }
        line.push_str(" 0x00");
        self.strings.push(line);
        self.string_labels.push(label.clone());
        label
    }

    fn imm(&mut self, dest: &str, value: i32) {
        let [a, b, c, d] = value.to_be_bytes();
        self.emit(&format!(
            "mov @{dest} 0x{a:02x} 0x{b:02x} 0x{c:02x} 0x{d:02x}"
        ));
    }

    fn stmt(&mut self, stmt: &Stmt) -> Result<(), ()> {
        match &stmt.kind {
            StmtKind::Let { name, init } => {
                let var = self.alloc_var(name, stmt.span)?;
                match init {
                    Some(expr) => {
                        // Resolve the initializer before binding the new name,
                        // so `let x = x + 1;` refers to an outer `x` (or is an
                        // undefined-variable error).
                        let tmp = self.temp();
                        self.expr(expr, &tmp)?;
                        self.bind(name, &var);
                        self.emit(&format!("copy @{var} @{tmp}"));
                    }
                    None => {
                        self.bind(name, &var);
                        self.imm(&var, 0);
                    }
                }
                Ok(())
            }
            StmtKind::Assign { name, value } => {
                let var = self.lookup(name, stmt.span)?;
                let tmp = self.temp();
                self.expr(value, &tmp)?;
                self.emit(&format!("copy @{var} @{tmp}"));
                Ok(())
            }
            StmtKind::If {
                cond,
                then_body,
                else_body,
            } => {
                let tmp = self.temp();
                self.expr(cond, &tmp)?;
                let else_label = self.new_label("else");
                let end_label = self.new_label("endif");
                self.emit(&format!("mov @__jmp @{else_label}"));
                self.emit_cond("=", &tmp, "__zero", "__jmp");

                self.push_scope();
                for stmt in then_body {
                    if self.stmt(stmt).is_err() {
                        self.pop_scope();
                        return Err(());
                    }
                }
                self.pop_scope();

                if let Some(else_body) = else_body {
                    self.emit(&format!("mov @__jmp @{end_label}"));
                    self.emit("goto @__jmp");
                    self.emit_control_label(&else_label);
                    self.push_scope();
                    for stmt in else_body {
                        if self.stmt(stmt).is_err() {
                            self.pop_scope();
                            return Err(());
                        }
                    }
                    self.pop_scope();
                    self.emit_control_label(&end_label);
                } else {
                    self.emit_control_label(&else_label);
                }
                Ok(())
            }
            StmtKind::While { cond, body } => {
                let start_label = self.new_label("while");
                let end_label = self.new_label("endwhile");
                self.emit_control_label(&start_label);
                let tmp = self.temp();
                self.expr(cond, &tmp)?;
                self.emit(&format!("mov @__jmp @{end_label}"));
                self.emit_cond("=", &tmp, "__zero", "__jmp");
                self.push_scope();
                for stmt in body {
                    if self.stmt(stmt).is_err() {
                        self.pop_scope();
                        return Err(());
                    }
                }
                self.pop_scope();
                self.emit(&format!("mov @__jmp @{start_label}"));
                self.emit("goto @__jmp");
                self.emit_control_label(&end_label);
                Ok(())
            }
            StmtKind::Print { args, newline } => {
                for arg in args {
                    match arg {
                        PrintArg::Str(bytes) => {
                            if !bytes.is_empty() {
                                self.print_string(bytes);
                            }
                        }
                        PrintArg::Expr(expr) => {
                            let tmp = self.temp();
                            self.expr(expr, &tmp)?;
                            self.emit(&format!("mov @__ptr @{tmp}"));
                            self.emit("putn @__ptr");
                        }
                    }
                }
                if *newline {
                    self.emit("mov @__ptr @__newline");
                    self.emit("putc @__ptr");
                }
                Ok(())
            }
            StmtKind::PutChar(expr) => {
                let tmp = self.temp();
                self.expr(expr, &tmp)?;
                self.emit(&format!("mov @__ptr @{tmp}"));
                self.emit("putc @__ptr");
                Ok(())
            }
            StmtKind::PutHex(expr) => {
                let tmp = self.temp();
                self.expr(expr, &tmp)?;
                self.emit(&format!("mov @__ptr @{tmp}"));
                self.emit("puth @__ptr");
                Ok(())
            }
            StmtKind::PutHexAt(addr) => {
                // `puth` takes the address of a pointer cell.  Copy the
                // runtime address into @__ptr, then print memory[address].
                let tmp = self.temp();
                self.expr(addr, &tmp)?;
                self.emit(&format!("copy @__ptr @{tmp}"));
                self.emit("puth @__ptr");
                Ok(())
            }
            StmtKind::Poke { addr, value } => {
                let a = self.temp();
                self.expr(addr, &a)?;
                let v = self.temp();
                self.expr(value, &v)?;
                // Original XASM1 `lm`: target = address_of_a - *b.
                // Set b = address_of_v - a, so target = a.
                let b = self.temp();
                self.emit(&format!("mov @{b} @{v}"));
                self.emit(&format!("sub @{b} @{a}"));
                self.emit(&format!("lm @{v} @{b}"));
                Ok(())
            }
            StmtKind::Expr(expr) => {
                let tmp = self.temp();
                self.expr(expr, &tmp)
            }
            StmtKind::Block(body) => {
                self.push_scope();
                for stmt in body {
                    if self.stmt(stmt).is_err() {
                        self.pop_scope();
                        return Err(());
                    }
                }
                self.pop_scope();
                Ok(())
            }
        }
    }

    fn print_string(&mut self, bytes: &[u8]) {
        let label = self.string(bytes);
        self.emit(&format!("mov @__ptr @{label}"));
        self.imm("__cnt", bytes.len() as i32);
        let loop_label = self.new_label("strloop");
        self.emit_control_label(&loop_label);
        self.emit("putc @__ptr");
        self.emit("add @__ptr @__one");
        self.emit("sub @__cnt @__one");
        self.emit(&format!("mov @__jmp @{loop_label}"));
        self.emit_cond(">", "__cnt", "__zero", "__jmp");
    }

    fn expr(&mut self, expr: &Expr, dest: &str) -> Result<(), ()> {
        match &expr.kind {
            ExprKind::Int(value) => {
                self.imm(dest, *value);
                Ok(())
            }
            ExprKind::Bool(value) => {
                self.imm(dest, i32::from(*value));
                Ok(())
            }
            ExprKind::Var(name) => {
                let var = self.lookup(name, expr.span)?;
                self.emit(&format!("copy @{dest} @{var}"));
                Ok(())
            }
            ExprKind::Unary { op, expr: inner } => match op {
                UnaryOp::Plus => self.expr(inner, dest),
                UnaryOp::Not => {
                    self.expr(inner, dest)?;
                    self.logical_not(dest);
                    Ok(())
                }
                UnaryOp::Neg => {
                    let tmp = self.temp();
                    self.expr(inner, &tmp)?;
                    self.imm(dest, 0);
                    self.emit(&format!("sub @{dest} @{tmp}"));
                    Ok(())
                }
            },
            ExprKind::Binary { op, left, right } => {
                if matches!(op, BinaryOp::Div | BinaryOp::Mod)
                    && matches!(&right.kind, ExprKind::Int(0))
                {
                    self.report.error(expr.span, "division by zero");
                    return Err(());
                }
                self.expr(left, dest)?;
                match op {
                    BinaryOp::And => self.logical_and(right, dest),
                    BinaryOp::Or => self.logical_or(right, dest),
                    _ => {
                        let rhs = self.temp();
                        self.expr(right, &rhs)?;
                        self.binary(*op, dest, &rhs)
                    }
                }
            }
            ExprKind::Peek(addr) => {
                let a = self.temp();
                self.expr(addr, &a)?;
                self.peek(&a, dest);
                Ok(())
            }
            ExprKind::ReadInt => {
                self.imm(dest, 0);
                self.emit(&format!("getn @{dest}"));
                Ok(())
            }
            ExprKind::ReadChar => {
                self.imm(dest, 0);
                self.emit(&format!("getc @{dest}"));
                Ok(())
            }
            ExprKind::ReadHex => {
                self.imm(dest, 0);
                self.emit(&format!("geth @{dest}"));
                Ok(())
            }
        }
    }

    /// Split a 32-bit address into four big-endian byte cells.
    fn address_bytes(&mut self, a: &str) -> [String; 4] {
        let q0 = self.temp();
        self.emit(&format!("copy @{q0} @{a}"));

        let b3 = self.temp();
        let q1 = self.temp();
        self.split_byte(&q0, &q1, &b3);

        let b2 = self.temp();
        let q2 = self.temp();
        self.split_byte(&q1, &q2, &b2);

        let b1 = self.temp();
        let q3 = self.temp();
        self.split_byte(&q2, &q3, &b1);

        [q3, b1, b2, b3]
    }

    /// `dest = memory[a]`, using a self-modified `lm` plus a restore.
    ///
    /// The original VM's `copy` dispatch bug keys off the last operand byte.
    /// `lm`'s last operand here is an aligned cell (`...00`), and its first
    /// operand is patched with the target address.  `lm` reads the target,
    /// zeroes it, and writes the value to `dest`; a second `lm` writes it
    /// back, so the read is non-destructive from the program's point of view.
    fn peek(&mut self, a: &str, dest: &str) {
        let bytes = self.address_bytes(a);

        // b = target - address_of_dest, so `lm target,b` writes to dest.
        let b = self.temp();
        let d = self.temp();
        self.emit(&format!("copy @{b} @{a}"));
        self.emit(&format!("mov @{d} @{dest}"));
        self.emit(&format!("sub @{b} @{d}"));

        let s0 = self.new_label("peek_a0");
        let s1 = self.new_label("peek_a1");
        let s2 = self.new_label("peek_a2");
        let s3 = self.new_label("peek_a3");
        let patch = self.new_label("peek_lm");

        self.emit(&format!("copy @{s0} @{}", bytes[0]));
        self.emit(&format!("copy @{s1} @{}", bytes[1]));
        self.emit(&format!("copy @{s2} @{}", bytes[2]));
        self.emit(&format!("copy @{s3} @{}", bytes[3]));

        self.emit_label(&patch);
        self.emit("lm");
        self.emit_label(&s0);
        self.emit("0x00");
        self.emit_label(&s1);
        self.emit("0x00");
        self.emit_label(&s2);
        self.emit("0x00");
        self.emit_label(&s3);
        self.emit("0x00");
        self.emit(&format!("@{b}"));

        // Restore target = value, so peek does not destroy memory.
        let v = self.temp();
        let b2 = self.temp();
        self.emit(&format!("copy @{v} @{dest}"));
        self.emit(&format!("mov @{b2} @{v}"));
        self.emit(&format!("sub @{b2} @{a}"));
        self.emit(&format!("lm @{v} @{b2}"));
    }

    fn split_byte(&mut self, value: &str, quotient: &str, byte: &str) {
        let div = self.temp();
        let mul = self.temp();
        self.emit(&format!("copy @{div} @{value}"));
        self.emit(&format!("xsub @{div} @__c256"));
        self.emit(&format!("copy @{mul} @{div}"));
        self.emit(&format!("xadd @{mul} @__c256"));
        self.emit(&format!("copy @{byte} @{value}"));
        self.emit(&format!("sub @{byte} @{mul}"));
        self.emit(&format!("copy @{quotient} @{div}"));
    }

    fn binary(&mut self, op: BinaryOp, dest: &str, rhs: &str) -> Result<(), ()> {
        match op {
            BinaryOp::Add => self.emit(&format!("add @{dest} @{rhs}")),
            BinaryOp::Sub => self.emit(&format!("sub @{dest} @{rhs}")),
            BinaryOp::Mul => self.emit(&format!("xadd @{dest} @{rhs}")),
            BinaryOp::Div => self.emit(&format!("xsub @{dest} @{rhs}")),
            BinaryOp::Mod => {
                let tmp = self.temp();
                self.emit(&format!("copy @{tmp} @{dest}"));
                self.emit(&format!("xsub @{tmp} @{rhs}"));
                self.emit(&format!("xadd @{tmp} @{rhs}"));
                self.emit(&format!("sub @{dest} @{tmp}"));
            }
            BinaryOp::Eq => self.bool_cmp("=", dest, rhs, dest),
            BinaryOp::Ne => {
                // Original XASM1 `not a b dest` computes dest = (a != b).
                self.emit(&format!("not @{dest} @{rhs} @{dest}"));
            }
            BinaryOp::Gt => self.bool_cmp(">", dest, rhs, dest),
            BinaryOp::Ge => {
                self.bool_cmp(">", rhs, dest, rhs);
                self.emit(&format!("copy @{dest} @{rhs}"));
                self.invert_bool(dest);
            }
            BinaryOp::Lt => {
                self.bool_cmp(">", rhs, dest, rhs);
                self.emit(&format!("copy @{dest} @{rhs}"));
            }
            BinaryOp::Le => {
                self.bool_cmp(">", dest, rhs, dest);
                self.invert_bool(dest);
            }
            BinaryOp::And | BinaryOp::Or => unreachable!("handled in expr"),
        }
        Ok(())
    }

    /// Flip a boolean cell (0 -> 1, 1 -> 0) using original `not dest,1,dest`.
    fn invert_bool(&mut self, dest: &str) {
        self.emit(&format!("not @{dest} @__one @{dest}"));
    }

    /// Logical not for an arbitrary integer cell: dest = (dest == 0).
    fn logical_not(&mut self, dest: &str) {
        self.bool_cmp("=", dest, "__zero", dest);
    }

    /// Emit `res = (a op b)` where `op` is one of `>`, `<`, `=`.
    fn bool_cmp(&mut self, op: &str, a: &str, b: &str, res: &str) {
        let true_label = self.new_label("cmp_true");
        let end_label = self.new_label("cmp_end");
        self.emit(&format!("mov @__jmp @{true_label}"));
        self.emit_cond(op, a, b, "__jmp");
        self.imm(res, 0);
        self.emit(&format!("mov @__jmp @{end_label}"));
        self.emit("goto @__jmp");
        self.emit_control_label(&true_label);
        self.imm(res, 1);
        self.emit_control_label(&end_label);
    }

    fn logical_and(&mut self, right: &Expr, dest: &str) -> Result<(), ()> {
        let false_label = self.new_label("and_false");
        let end_label = self.new_label("and_end");
        self.emit(&format!("mov @__jmp @{false_label}"));
        self.emit_cond("=", dest, "__zero", "__jmp");
        let rhs = self.temp();
        self.expr(right, &rhs)?;
        self.emit(&format!("mov @__jmp @{false_label}"));
        self.emit_cond("=", &rhs, "__zero", "__jmp");
        self.imm(dest, 1);
        self.emit(&format!("mov @__jmp @{end_label}"));
        self.emit("goto @__jmp");
        self.emit_control_label(&false_label);
        self.imm(dest, 0);
        self.emit_control_label(&end_label);
        Ok(())
    }

    fn logical_or(&mut self, right: &Expr, dest: &str) -> Result<(), ()> {
        let right_label = self.new_label("or_right");
        let false_label = self.new_label("or_false");
        let end_label = self.new_label("or_end");
        self.emit(&format!("mov @__jmp @{right_label}"));
        self.emit_cond("=", dest, "__zero", "__jmp");
        self.imm(dest, 1);
        self.emit(&format!("mov @__jmp @{end_label}"));
        self.emit("goto @__jmp");
        self.emit_control_label(&right_label);
        let rhs = self.temp();
        self.expr(right, &rhs)?;
        self.emit(&format!("mov @__jmp @{false_label}"));
        self.emit_cond("=", &rhs, "__zero", "__jmp");
        self.imm(dest, 1);
        self.emit(&format!("mov @__jmp @{end_label}"));
        self.emit("goto @__jmp");
        self.emit_control_label(&false_label);
        self.imm(dest, 0);
        self.emit_control_label(&end_label);
        Ok(())
    }
}
