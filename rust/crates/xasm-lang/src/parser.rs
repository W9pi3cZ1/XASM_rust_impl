//! Recursive-descent parser for XPL.

use crate::ast::*;
use crate::lexer::{Token, TokenKind};
use xasm_core::diag::Report;
use xasm_core::Span;

pub fn parse(tokens: &[Token], report: &mut Report) -> Option<Program> {
    Parser {
        tokens,
        pos: 0,
        report,
    }
    .program()
    .ok()
}

struct Parser<'a, 'r> {
    tokens: &'a [Token],
    pos: usize,
    report: &'r mut Report,
}

fn same_variant(a: &TokenKind, b: &TokenKind) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

impl<'a, 'r> Parser<'a, 'r> {
    fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn advance(&mut self) -> Token {
        let token = self.peek().clone();
        if !matches!(token.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        token
    }

    fn check(&self, kind: &TokenKind) -> bool {
        same_variant(&self.peek().kind, kind)
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind, context: &str) -> Result<Token, ()> {
        if self.check(&kind) {
            Ok(self.advance())
        } else {
            self.report.error(
                self.peek().span,
                format!(
                    "expected {} {}, found {}",
                    kind.name(),
                    context,
                    self.peek().kind.name()
                ),
            );
            Err(())
        }
    }

    fn program(&mut self) -> Result<Program, ()> {
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::Eof) {
            stmts.push(self.stmt()?);
        }
        Ok(Program { stmts })
    }

    fn stmt(&mut self) -> Result<Stmt, ()> {
        let start = self.peek().span.start;
        let kind = match self.peek().kind.clone() {
            TokenKind::Let => {
                self.advance();
                let name_token = self.expect(TokenKind::Ident(String::new()), "after `let`")?;
                let name = match name_token.kind {
                    TokenKind::Ident(name) => name,
                    _ => unreachable!(),
                };
                let init = if self.eat(&TokenKind::Assign) {
                    Some(self.expr()?)
                } else {
                    None
                };
                self.expect(TokenKind::Semi, "after `let` declaration")?;
                StmtKind::Let { name, init }
            }
            TokenKind::If => {
                self.advance();
                return self.if_stmt(start);
            }
            TokenKind::While => {
                self.advance();
                self.expect(TokenKind::LParen, "after `while`")?;
                let cond = self.expr()?;
                self.expect(TokenKind::RParen, "after while condition")?;
                let body = self.block()?;
                StmtKind::While { cond, body }
            }
            TokenKind::Print | TokenKind::Println => {
                let newline = matches!(&self.peek().kind, TokenKind::Println);
                self.advance();
                self.expect(TokenKind::LParen, "after print")?;
                let args = self.print_args()?;
                self.expect(TokenKind::RParen, "after print arguments")?;
                self.expect(TokenKind::Semi, "after print statement")?;
                StmtKind::Print { args, newline }
            }
            TokenKind::Putchar => {
                self.advance();
                self.expect(TokenKind::LParen, "after `putchar`")?;
                let value = self.expr()?;
                self.expect(TokenKind::RParen, "after putchar argument")?;
                self.expect(TokenKind::Semi, "after putchar statement")?;
                StmtKind::PutChar(value)
            }
            TokenKind::Puthex => {
                self.advance();
                self.expect(TokenKind::LParen, "after `puthex`")?;
                let value = self.expr()?;
                self.expect(TokenKind::RParen, "after puthex argument")?;
                self.expect(TokenKind::Semi, "after puthex statement")?;
                StmtKind::PutHex(value)
            }
            TokenKind::PuthexAt => {
                self.advance();
                self.expect(TokenKind::LParen, "after `puthex_at`")?;
                let addr = self.expr()?;
                self.expect(TokenKind::RParen, "after puthex_at argument")?;
                self.expect(TokenKind::Semi, "after puthex_at statement")?;
                StmtKind::PutHexAt(addr)
            }
            TokenKind::Poke => {
                self.advance();
                self.expect(TokenKind::LParen, "after `poke`")?;
                let addr = self.expr()?;
                self.expect(TokenKind::Comma, "after poke address")?;
                let value = self.expr()?;
                self.expect(TokenKind::RParen, "after poke arguments")?;
                self.expect(TokenKind::Semi, "after poke statement")?;
                StmtKind::Poke { addr, value }
            }
            TokenKind::LBrace => {
                let body = self.block()?;
                StmtKind::Block(body)
            }
            TokenKind::Ident(name) => {
                if matches!(
                    self.tokens.get(self.pos + 1).map(|token| &token.kind),
                    Some(TokenKind::Assign)
                ) {
                    self.advance();
                    self.advance();
                    let value = self.expr()?;
                    self.expect(TokenKind::Semi, "after assignment")?;
                    StmtKind::Assign { name, value }
                } else {
                    let value = self.expr()?;
                    self.expect(TokenKind::Semi, "after expression")?;
                    StmtKind::Expr(value)
                }
            }
            _ => {
                let value = self.expr()?;
                self.expect(TokenKind::Semi, "after expression")?;
                StmtKind::Expr(value)
            }
        };
        let end = self.tokens[self.pos.saturating_sub(1)].span.end;
        Ok(Stmt {
            kind,
            span: Span::new(start, end),
        })
    }

    fn if_stmt(&mut self, start: usize) -> Result<Stmt, ()> {
        self.expect(TokenKind::LParen, "after `if`")?;
        let cond = self.expr()?;
        self.expect(TokenKind::RParen, "after if condition")?;
        let then_body = self.block()?;
        let else_body = if self.eat(&TokenKind::Else) {
            if self.check(&TokenKind::If) {
                let nested_start = self.peek().span.start;
                self.advance();
                Some(vec![self.if_stmt(nested_start)?])
            } else {
                Some(self.block()?)
            }
        } else {
            None
        };
        let end = self.tokens[self.pos.saturating_sub(1)].span.end;
        Ok(Stmt {
            kind: StmtKind::If {
                cond,
                then_body,
                else_body,
            },
            span: Span::new(start, end),
        })
    }

    fn block(&mut self) -> Result<Vec<Stmt>, ()> {
        self.expect(TokenKind::LBrace, "to start a block")?;
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            stmts.push(self.stmt()?);
        }
        self.expect(TokenKind::RBrace, "to end a block")?;
        Ok(stmts)
    }

    fn print_args(&mut self) -> Result<Vec<PrintArg>, ()> {
        let mut args = Vec::new();
        if self.check(&TokenKind::RParen) {
            return Ok(args);
        }
        loop {
            if let TokenKind::Str(bytes) = &self.peek().kind {
                let bytes = bytes.clone();
                self.advance();
                args.push(PrintArg::Str(bytes));
            } else {
                args.push(PrintArg::Expr(self.expr()?));
            }
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        Ok(args)
    }

    fn expr(&mut self) -> Result<Expr, ()> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ()> {
        let mut left = self.parse_and()?;
        while self.check(&TokenKind::OrOr) {
            self.advance();
            let right = self.parse_and()?;
            left = binary(BinaryOp::Or, left, right);
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ()> {
        let mut left = self.parse_cmp()?;
        while self.check(&TokenKind::AndAnd) {
            self.advance();
            let right = self.parse_cmp()?;
            left = binary(BinaryOp::And, left, right);
        }
        Ok(left)
    }

    fn parse_cmp(&mut self) -> Result<Expr, ()> {
        let mut left = self.parse_add()?;
        loop {
            let op = match &self.peek().kind {
                TokenKind::EqEq => BinaryOp::Eq,
                TokenKind::NotEq => BinaryOp::Ne,
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::Le => BinaryOp::Le,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::Ge => BinaryOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.parse_add()?;
            left = binary(op, left, right);
        }
        Ok(left)
    }

    fn parse_add(&mut self) -> Result<Expr, ()> {
        let mut left = self.parse_mul()?;
        loop {
            let op = match &self.peek().kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_mul()?;
            left = binary(op, left, right);
        }
        Ok(left)
    }

    fn parse_mul(&mut self) -> Result<Expr, ()> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match &self.peek().kind {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = binary(op, left, right);
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ()> {
        let start = self.peek().span.start;
        let op = match &self.peek().kind {
            TokenKind::Minus => Some(UnaryOp::Neg),
            TokenKind::Plus => Some(UnaryOp::Plus),
            TokenKind::Bang => Some(UnaryOp::Not),
            _ => None,
        };
        if let Some(op) = op {
            self.advance();
            let expr = self.parse_unary()?;
            let span = Span::new(start, expr.span.end);
            return Ok(Expr {
                kind: ExprKind::Unary {
                    op,
                    expr: Box::new(expr),
                },
                span,
            });
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, ()> {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::Int { value, hex } => {
                self.advance();
                let value = int_value(value, hex, token.span, self.report)?;
                Ok(Expr {
                    kind: ExprKind::Int(value),
                    span: token.span,
                })
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Bool(true),
                    span: token.span,
                })
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Bool(false),
                    span: token.span,
                })
            }
            TokenKind::Peek => {
                self.advance();
                self.expect(TokenKind::LParen, "after `peek`")?;
                let addr = self.expr()?;
                let close = self.expect(TokenKind::RParen, "after peek argument")?;
                Ok(Expr {
                    kind: ExprKind::Peek(Box::new(addr)),
                    span: Span::new(token.span.start, close.span.end),
                })
            }
            TokenKind::ReadInt | TokenKind::ReadChar | TokenKind::ReadHex => {
                self.advance();
                self.expect(TokenKind::LParen, "after input builtin")?;
                self.expect(TokenKind::RParen, "after input builtin")?;
                let end = self.tokens[self.pos.saturating_sub(1)].span.end;
                let kind = match &token.kind {
                    TokenKind::ReadInt => ExprKind::ReadInt,
                    TokenKind::ReadChar => ExprKind::ReadChar,
                    TokenKind::ReadHex => ExprKind::ReadHex,
                    _ => unreachable!(),
                };
                Ok(Expr {
                    kind,
                    span: Span::new(token.span.start, end),
                })
            }
            TokenKind::Ident(name) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Var(name),
                    span: token.span,
                })
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.expr()?;
                let close = self.expect(TokenKind::RParen, "after expression")?;
                Ok(Expr {
                    kind: expr.kind,
                    span: Span::new(token.span.start, close.span.end),
                })
            }
            TokenKind::Str(_) => {
                self.report.error(
                    token.span,
                    "string literals are only allowed as print/println arguments",
                );
                Err(())
            }
            _ => {
                self.report.error(
                    token.span,
                    format!("expected an expression, found {}", token.kind.name()),
                );
                Err(())
            }
        }
    }
}

fn binary(op: BinaryOp, left: Expr, right: Expr) -> Expr {
    let span = Span::new(left.span.start, right.span.end);
    Expr {
        kind: ExprKind::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
        span,
    }
}

fn int_value(value: u64, hex: bool, span: Span, report: &mut Report) -> Result<i32, ()> {
    if value <= i32::MAX as u64 {
        return Ok(value as i32);
    }
    if hex && value <= u32::MAX as u64 {
        return Ok(value as u32 as i32);
    }
    report.error(
        span,
        format!(
            "integer literal `{value}` does not fit in 32 bits; use 0x{:08x} for a bit pattern",
            value as u32
        ),
    );
    Err(())
}
