//! XPL abstract syntax tree.

use xasm_core::Span;

#[derive(Debug)]
pub struct Program {
    pub functions: Vec<Function>,
    pub stmts: Vec<Stmt>,
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum StmtKind {
    Let {
        name: String,
        init: Option<Expr>,
        array_len: Option<usize>,
    },
    Assign {
        name: String,
        value: Expr,
    },
    AssignIndex {
        name: String,
        index: Expr,
        value: Expr,
    },
    Return(Option<Expr>),
    If {
        cond: Expr,
        then_body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    Print {
        args: Vec<PrintArg>,
        newline: bool,
    },
    PutChar(Expr),
    PutHex(Expr),
    PutHexAt(Expr),
    Poke {
        addr: Expr,
        value: Expr,
    },
    Expr(Expr),
    Block(Vec<Stmt>),
}

#[derive(Debug)]
pub enum PrintArg {
    Str(Vec<u8>),
    Expr(Expr),
}

#[derive(Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum ExprKind {
    Int(i32),
    Bool(bool),
    Var(String),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    ReadInt,
    ReadChar,
    ReadHex,
    Peek(Box<Expr>),
    Call {
        name: String,
        args: Vec<Expr>,
    },
    Index {
        name: String,
        index: Box<Expr>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    Plus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl BinaryOp {
    pub fn is_comparison(self) -> bool {
        matches!(
            self,
            BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
        )
    }
}
