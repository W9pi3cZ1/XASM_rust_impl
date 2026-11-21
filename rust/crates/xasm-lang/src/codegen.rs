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

#[derive(Clone, Debug)]
enum Symbol {
    Scalar(String),
    Array { base: String, len: usize },
}

#[derive(Clone, Debug)]
struct FunctionInfo {
    params: Vec<String>,
    param_labels: Vec<String>,
    label: String,
    retval_label: String,
    retparam_label: String,
}

struct Codegen {
    out: String,
    /// Variable/temporary cells.  These are laid out on 256-byte boundaries.
    cells: Vec<String>,
    cell_labels: Vec<String>,
    /// Array blocks are contiguous cells, placed after the scalar cells.
    arrays: Vec<(String, Vec<u8>)>,
    array_labels: Vec<String>,
    strings: Vec<String>,
    string_labels: Vec<String>,
    functions: Vec<FunctionInfo>,
    function_map: HashMap<String, usize>,
    current_function: Option<usize>,
    label_id: usize,
    var_id: usize,
    temp_id: usize,
    str_id: usize,
    array_id: usize,
    scopes: Vec<HashMap<String, Symbol>>,
    report: Report,
    stats: Stats,
}

impl Codegen {
    fn new() -> Self {
        Self {
            out: String::new(),
            cells: Vec::new(),
            cell_labels: Vec::new(),
            arrays: Vec::new(),
            array_labels: Vec::new(),
            strings: Vec::new(),
            string_labels: Vec::new(),
            functions: Vec::new(),
            function_map: HashMap::new(),
            current_function: None,
            label_id: 0,
            var_id: 0,
            temp_id: 0,
            str_id: 0,
            array_id: 0,
            scopes: vec![HashMap::new()],
            report: Report::new(),
            stats: Stats::default(),
        }
    }

    fn prepare_functions(&mut self, functions: &[Function]) -> bool {
        for (index, function) in functions.iter().enumerate() {
            if self.function_map.contains_key(&function.name) {
                self.report.error(
                    function.span,
                    format!("function `{}` is already defined", function.name),
                );
                return false;
            }
            for (position, param) in function.params.iter().enumerate() {
                if function.params[..position].contains(param) {
                    self.report.error(
                        function.span,
                        format!("duplicate parameter `{param}` in `{}`", function.name),
                    );
                    return false;
                }
            }
            let param_labels = function.params.iter().map(|_| self.temp()).collect();
            let retval_label = self.temp();
            let retparam_label = self.temp();
            let label = format!("fn_{}", function.name);
            self.function_map.insert(function.name.clone(), index);
            self.functions.push(FunctionInfo {
                params: function.params.clone(),
                param_labels,
                label,
                retval_label,
                retparam_label,
            });
        }
        true
    }

    fn program(&mut self, program: &Program) {
        if !self.prepare_functions(&program.functions) {
            return;
        }
        // Constant used by `peek` to split a 32-bit address into bytes.
        self.emit("mov @__c256 0x00 0x00 0x01 0x00");
        for stmt in &program.stmts {
            if self.stmt(stmt).is_err() {
                return;
            }
        }
        self.emit("exit");

        // Function bodies live after the top-level `exit`, so normal control
        // flow cannot fall into them.  Calls jump here with `goto`.
        for (index, function) in program.functions.iter().enumerate() {
            if self.function_body(index, function).is_err() {
                return;
            }
        }
    }

    fn function_body(&mut self, index: usize, function: &Function) -> Result<(), ()> {
        let info = self.functions[index].clone();
        self.emit_control_label(&info.label);
        self.push_scope();
        for (name, label) in info.params.iter().zip(info.param_labels.iter()) {
            self.bind_scalar(name, label);
        }
        self.current_function = Some(index);
        for stmt in &function.body {
            if self.stmt(stmt).is_err() {
                self.current_function = None;
                self.pop_scope();
                return Err(());
            }
        }
        self.current_function = None;
        // Falling off the end returns 0.
        self.imm(&info.retval_label, 0);
        self.emit(&format!("goto @{}", info.retparam_label));
        self.pop_scope();
        Ok(())
    }

    const CONST_LABELS: [&'static str; 9] = [
        "__zero",
        "__one",
        "__newline",
        "__jmp",
        "__ptr",
        "__cnt",
        "__c256",
        "__two",
        "__c32",
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
        for label in &self.array_labels {
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
        out.push_str("[__two] 0x02\n");
        out.push_str("[__c32] 0x20\n");
        for (label, bytes) in &self.arrays {
            out.push('[');
            out.push_str(label);
            out.push(']');
            for byte in bytes {
                out.push_str(&format!(" 0x{byte:02x}"));
            }
            out.push('\n');
        }
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

    fn bind_symbol(&mut self, name: &str, symbol: Symbol) {
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name.to_owned(), symbol);
    }

    fn bind_scalar(&mut self, name: &str, label: &str) {
        self.bind_symbol(name, Symbol::Scalar(label.to_owned()));
    }

    fn lookup_symbol(&mut self, name: &str, span: xasm_core::Span) -> Result<Symbol, ()> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return Ok(symbol.clone());
            }
        }
        self.report.error(span, format!("undefined name `{name}`"));
        Err(())
    }

    fn lookup(&mut self, name: &str, span: xasm_core::Span) -> Result<String, ()> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return match symbol {
                    Symbol::Scalar(label) => Ok(label.clone()),
                    Symbol::Array { .. } => {
                        self.report.error(
                            span,
                            format!("array `{name}` needs an index, e.g. `{name}[0]`"),
                        );
                        Err(())
                    }
                };
            }
        }
        self.report
            .error(span, format!("undefined variable `{name}`"));
        Err(())
    }

    fn lookup_array(&mut self, name: &str, span: xasm_core::Span) -> Result<(String, usize), ()> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return match symbol {
                    Symbol::Array { base, len } => Ok((base.clone(), *len)),
                    Symbol::Scalar(_) => {
                        self.report.error(span, format!("`{name}` is not an array"));
                        return Err(());
                    }
                };
            }
        }
        self.report.error(span, format!("undefined array `{name}`"));
        Err(())
    }

    fn alloc_array(
        &mut self,
        name: &str,
        len: usize,
        span: xasm_core::Span,
        init: Option<&ArrayInit>,
    ) -> Result<(), ()> {
        if self.scopes.last().unwrap().contains_key(name) {
            self.report.error(
                span,
                format!("name `{name}` is already declared in this scope"),
            );
            return Err(());
        }
        if len > 0x80000 {
            self.report.error(
                span,
                format!("array `{name}` is too large ({len} cells; max 524288)"),
            );
            return Err(());
        }

        let mut bytes = vec![0u8; len];
        match init {
            None => {}
            Some(ArrayInit::Values(values)) => {
                if values.len() > len {
                    self.report.error(
                        span,
                        format!(
                            "array `{name}` has {len} cells but {} initializer values",
                            values.len()
                        ),
                    );
                    return Err(());
                }
                bytes[..values.len()].copy_from_slice(values);
            }
            Some(ArrayInit::String(value)) => {
                if value.len() > len {
                    self.report.error(
                        span,
                        format!(
                            "array `{name}` has {len} cells but string initializer is {} bytes",
                            value.len()
                        ),
                    );
                    return Err(());
                }
                bytes[..value.len()].copy_from_slice(value);
            }
        }

        let label = format!("a{}", self.array_id);
        self.array_id += 1;
        self.arrays.push((label.clone(), bytes));
        self.array_labels.push(label.clone());
        self.bind_symbol(name, Symbol::Array { base: label, len });
        Ok(())
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
            StmtKind::Let {
                name,
                init,
                array_len,
                array_init,
            } => {
                if let Some(len) = array_len {
                    self.alloc_array(name, *len, stmt.span, array_init.as_ref())?;
                    return Ok(());
                }
                let var = self.alloc_var(name, stmt.span)?;
                match init {
                    Some(expr) => {
                        // Resolve the initializer before binding the new name,
                        // so `let x = x + 1;` refers to an outer `x` (or is an
                        // undefined-variable error).
                        let tmp = self.temp();
                        self.expr(expr, &tmp)?;
                        self.bind_scalar(name, &var);
                        self.emit(&format!("copy @{var} @{tmp}"));
                    }
                    None => {
                        self.bind_scalar(name, &var);
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
            StmtKind::AssignIndex { name, index, value } => {
                let (base, _) = self.lookup_array(name, stmt.span)?;
                let i = self.temp();
                self.expr(index, &i)?;
                let a = self.array_addr(&base, &i);
                let v = self.temp();
                self.expr(value, &v)?;
                self.poke_value(&a, &v);
                Ok(())
            }
            StmtKind::Return(value) => {
                let Some(index) = self.current_function else {
                    self.report
                        .error(stmt.span, "`return` is only allowed inside a function");
                    return Err(());
                };
                let info = self.functions[index].clone();
                match value {
                    Some(expr) => self.expr(expr, &info.retval_label)?,
                    None => self.imm(&info.retval_label, 0),
                }
                self.emit(&format!("goto @{}", info.retparam_label));
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
                self.poke_value(&a, &v);
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
                UnaryOp::Addr => self.address_of(inner, dest),
                UnaryOp::BitNot => {
                    if let ExprKind::Int(value) = &inner.kind {
                        self.imm(dest, !*value);
                        return Ok(());
                    }
                    self.expr(inner, dest)?;
                    let tmp = self.temp();
                    self.imm(&tmp, 0);
                    self.emit(&format!("sub @{tmp} @{dest}"));
                    self.emit(&format!("sub @{tmp} @__one"));
                    self.emit(&format!("copy @{dest} @{tmp}"));
                    Ok(())
                }
            },
            ExprKind::Binary { op, left, right } => {
                if let (ExprKind::Int(a), ExprKind::Int(b)) = (&left.kind, &right.kind) {
                    if let Some(value) = const_bitwise(*op, *a, *b) {
                        self.imm(dest, value);
                        return Ok(());
                    }
                }
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
            ExprKind::Index { name, index } => {
                let (base, _) = self.lookup_array(name, expr.span)?;
                let i = self.temp();
                self.expr(index, &i)?;
                let a = self.array_addr(&base, &i);
                self.peek(&a, dest);
                Ok(())
            }
            ExprKind::Call { name, args } => {
                let Some(index) = self.function_map.get(name).copied() else {
                    self.report
                        .error(expr.span, format!("undefined function `{name}`"));
                    return Err(());
                };
                let info = self.functions[index].clone();
                if self.current_function == Some(index) {
                    self.report.error(
                        expr.span,
                        format!("recursive call to `{name}` is not supported"),
                    );
                    return Err(());
                }
                if args.len() != info.params.len() {
                    self.report.error(
                        expr.span,
                        format!(
                            "function `{name}` expects {} argument(s), got {}",
                            info.params.len(),
                            args.len()
                        ),
                    );
                    return Err(());
                }
                // Evaluate every argument into a caller temp first.  Nested
                // calls may overwrite the callee's parameter cells, so the
                // parameters are only written after all arguments are ready.
                let mut arg_temps = Vec::with_capacity(args.len());
                for arg in args {
                    let temp = self.temp();
                    self.expr(arg, &temp)?;
                    arg_temps.push(temp);
                }
                for (temp, param) in arg_temps.iter().zip(info.param_labels.iter()) {
                    self.emit(&format!("copy @{param} @{temp}"));
                }
                let ret = self.new_label("call_return");
                self.emit(&format!("mov @{} @{}", info.retparam_label, ret));
                // XASM's `goto` is indirect: the operand is the address of a
                // cell that contains the target.  Use the shared jump cell.
                self.emit(&format!("mov @__jmp @{}", info.label));
                self.emit("goto @__jmp");
                self.emit_control_label(&ret);
                self.emit(&format!("copy @{dest} @{}", info.retval_label));
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

    /// Evaluate `&expr` into an i32 address cell.
    ///
    /// Supported forms are `&scalar`, `&array`, and `&array[index]`.  XPL has
    /// no reference type; the result is an ordinary i32 address that can be
    /// passed to `peek`/`poke` or to a function.
    fn address_of(&mut self, expr: &Expr, dest: &str) -> Result<(), ()> {
        match &expr.kind {
            ExprKind::Var(name) => {
                let symbol = self.lookup_symbol(name, expr.span)?;
                match symbol {
                    Symbol::Scalar(label) => {
                        self.emit(&format!("mov @{dest} @{label}"));
                    }
                    Symbol::Array { base, .. } => {
                        self.emit(&format!("mov @{dest} @{base}"));
                    }
                }
                Ok(())
            }
            ExprKind::Index { name, index } => {
                let (base, _) = self.lookup_array(name, expr.span)?;
                let i = self.temp();
                self.expr(index, &i)?;
                let addr = self.array_addr(&base, &i);
                self.emit(&format!("copy @{dest} @{addr}"));
                Ok(())
            }
            _ => {
                self.report.error(
                    expr.span,
                    "can only take the address of a variable, array, or array element",
                );
                Err(())
            }
        }
    }

    fn array_addr(&mut self, base: &str, index: &str) -> String {
        let addr = self.temp();
        self.emit(&format!("mov @{addr} @{base}"));
        self.emit(&format!("add @{addr} @{index}"));
        addr
    }

    fn poke_value(&mut self, addr: &str, value: &str) {
        // Original XASM1 `lm`: target = address_of_a - *b.  Set
        // b = address_of_value - addr, so target = addr.
        let b = self.temp();
        self.emit(&format!("mov @{b} @{value}"));
        self.emit(&format!("sub @{b} @{addr}"));
        self.emit(&format!("lm @{value} @{b}"));
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
            BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor => {
                self.bitwise(op, dest, rhs)?;
            }
            BinaryOp::Shl => self.shift_left(dest, rhs),
            BinaryOp::Shr => self.shift_right(dest, rhs),
            BinaryOp::And | BinaryOp::Or => unreachable!("handled in expr"),
        }
        Ok(())
    }

    /// `q = floor(value / 2)`, `bit = value mod 2` (0/1) for two's-complement
    /// bit extraction.  XASM division truncates toward zero, so negative odd
    /// values need the adjustment.
    fn floor_bit(&mut self, value: &str, quotient: &str, bit: &str) {
        let q = self.temp();
        let r = self.temp();
        let two = self.temp();
        self.emit(&format!("copy @{q} @{value}"));
        self.emit(&format!("xsub @{q} @__two"));
        self.emit(&format!("copy @{r} @{value}"));
        self.emit(&format!("copy @{two} @{q}"));
        self.emit(&format!("xadd @{two} @__two"));
        self.emit(&format!("sub @{r} @{two}"));

        let cond = self.temp();
        let skip = self.new_label("floor_bit_skip");
        self.bool_cmp(">", "__zero", &r, &cond);
        self.emit(&format!("mov @__jmp @{skip}"));
        self.emit_cond("=", &cond, "__zero", "__jmp");
        self.emit(&format!("sub @{q} @__one"));
        self.emit(&format!("add @{r} @__two"));
        self.emit_control_label(&skip);

        self.emit(&format!("copy @{quotient} @{q}"));
        self.emit(&format!("copy @{bit} @{r}"));
    }

    /// Lower `&`, `|`, `^` to 32 arithmetic iterations.  No community opcodes
    /// are used, so the result runs on the original XASM1.
    fn bitwise(&mut self, op: BinaryOp, dest: &str, rhs: &str) -> Result<(), ()> {
        let x = self.temp();
        let y = self.temp();
        self.emit(&format!("copy @{x} @{dest}"));
        self.emit(&format!("copy @{y} @{rhs}"));

        let res = self.temp();
        let bit = self.temp();
        let i = self.temp();
        self.imm(&res, 0);
        self.imm(&bit, 1);
        self.imm(&i, 0);

        let loop_label = self.new_label("bit_loop");
        let end_label = self.new_label("bit_end");
        self.emit_control_label(&loop_label);
        let cond = self.temp();
        self.bool_cmp(">", "__c32", &i, &cond);
        self.emit(&format!("mov @__jmp @{end_label}"));
        self.emit_cond("=", &cond, "__zero", "__jmp");

        let ax = self.temp();
        let abit = self.temp();
        self.floor_bit(&x, &ax, &abit);
        let ay = self.temp();
        let bbit = self.temp();
        self.floor_bit(&y, &ay, &bbit);
        self.emit(&format!("copy @{x} @{ax}"));
        self.emit(&format!("copy @{y} @{ay}"));

        let take = self.temp();
        match op {
            BinaryOp::BitAnd => {
                self.emit(&format!("copy @{take} @{abit}"));
                self.emit(&format!("xadd @{take} @{bbit}"));
            }
            BinaryOp::BitOr => {
                self.emit(&format!("copy @{take} @{abit}"));
                self.emit(&format!("add @{take} @{bbit}"));
            }
            BinaryOp::BitXor => {
                self.emit(&format!("copy @{take} @{abit}"));
                self.emit(&format!("sub @{take} @{bbit}"));
            }
            _ => unreachable!(),
        }

        let skip = self.new_label("bit_skip");
        self.emit(&format!("mov @__jmp @{skip}"));
        self.emit_cond("=", &take, "__zero", "__jmp");
        self.emit(&format!("add @{res} @{bit}"));
        self.emit_control_label(&skip);

        self.emit(&format!("add @{bit} @{bit}"));
        self.emit(&format!("add @{i} @__one"));
        self.emit(&format!("mov @__jmp @{loop_label}"));
        self.emit("goto @__jmp");
        self.emit_control_label(&end_label);
        self.emit(&format!("copy @{dest} @{res}"));
        Ok(())
    }

    fn shift_count(&mut self, rhs: &str) -> String {
        let n = self.temp();
        self.emit(&format!("copy @{n} @{rhs}"));
        let _ = self.binary(BinaryOp::Mod, &n, "__c32");
        let cond = self.temp();
        let skip = self.new_label("shift_norm");
        self.bool_cmp(">", "__zero", &n, &cond);
        self.emit(&format!("mov @__jmp @{skip}"));
        self.emit_cond("=", &cond, "__zero", "__jmp");
        self.emit(&format!("add @{n} @__c32"));
        self.emit_control_label(&skip);
        n
    }

    fn shift_left(&mut self, dest: &str, rhs: &str) {
        let n = self.shift_count(rhs);
        let loop_label = self.new_label("shl_loop");
        let end_label = self.new_label("shl_end");
        self.emit_control_label(&loop_label);
        let cond = self.temp();
        self.bool_cmp(">", &n, "__zero", &cond);
        self.emit(&format!("mov @__jmp @{end_label}"));
        self.emit_cond("=", &cond, "__zero", "__jmp");
        self.emit(&format!("add @{dest} @{dest}"));
        self.emit(&format!("sub @{n} @__one"));
        self.emit(&format!("mov @__jmp @{loop_label}"));
        self.emit("goto @__jmp");
        self.emit_control_label(&end_label);
    }

    fn shift_right(&mut self, dest: &str, rhs: &str) {
        let n = self.shift_count(rhs);
        let loop_label = self.new_label("shr_loop");
        let end_label = self.new_label("shr_end");
        self.emit_control_label(&loop_label);
        let cond = self.temp();
        self.bool_cmp(">", &n, "__zero", &cond);
        self.emit(&format!("mov @__jmp @{end_label}"));
        self.emit_cond("=", &cond, "__zero", "__jmp");
        let q = self.temp();
        let bit = self.temp();
        self.floor_bit(dest, &q, &bit);
        self.emit(&format!("copy @{dest} @{q}"));
        self.emit(&format!("sub @{n} @__one"));
        self.emit(&format!("mov @__jmp @{loop_label}"));
        self.emit("goto @__jmp");
        self.emit_control_label(&end_label);
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

fn const_bitwise(op: BinaryOp, a: i32, b: i32) -> Option<i32> {
    match op {
        BinaryOp::BitAnd => Some(a & b),
        BinaryOp::BitOr => Some(a | b),
        BinaryOp::BitXor => Some(a ^ b),
        BinaryOp::Shl => Some(a.wrapping_shl((b as u32) & 31)),
        BinaryOp::Shr => Some(a.wrapping_shr((b as u32) & 31)),
        _ => None,
    }
}
