pub mod ast;

use crate::{
    error::{
        Error,
        ErrorKind::{E100, E101, E102},
        Errors, Res,
    },
    lexer::{Lexer, TKind, TKind::*, Token},
    parser::ast::{EExpr, Expr, Function, Literal, Member, Parameter, Type},
    smol_str::SmolStr,
};
use alloc::{boxed::Box, vec::Vec};
pub use ast::Module;
use core::{mem, str::FromStr};

pub struct Parser<'src> {
    lexer: Lexer<'src>,
    current: Token,
    errors: Errors,
}

impl<'src> Parser<'src> {
    pub fn parse(mut self, path: Vec<SmolStr>) -> Result<Module, Errors> {
        let mut functions = Vec::new();
        let mut classes = Vec::new();

        while !self.is_at_end() {
            match self.advance().kind {
                TKind::Class => self.make_cls(&mut classes),
                TKind::Fun => self.make_fn(&mut functions, false),
                TKind::Extern if self.matches(Fun) => self.make_fn(&mut functions, true),
                _ => {
                    self.errors.push(Error::new(self.current.start, E102));
                    self.synchronize()
                }
            }
        }
        if self.errors.is_empty() {
            Ok(Module {
                functions,
                classes,
                path,
            })
        } else {
            Err(self.errors)
        }
    }

    fn make_cls(&mut self, cls: &mut Vec<ast::Class>) {
        match self.class() {
            Ok(f) => cls.push(f),
            Err(e) => {
                self.errors.push(e);
                self.synchronize()
            }
        }
    }

    fn make_fn(&mut self, functions: &mut Vec<Function>, is_ext: bool) {
        match self.function(is_ext) {
            Ok(f) => functions.push(f),
            Err(e) => {
                self.errors.push(e);
                self.synchronize()
            }
        }
    }

    fn class(&mut self) -> Res<ast::Class> {
        let name = self.consume(Identifier)?;
        self.consume(LeftBrace)?;

        let mut members = Vec::new();
        let mut methods = Vec::new();
        let mut functions = Vec::new();
        while !self.check(RightBrace) {
            match self.advance().kind {
                Val => members.push(self.member(false)?),
                Var => members.push(self.member(true)?),
                Fun => methods.push(self.function(false)?),
                Static if self.matches(Fun) => functions.push(self.function(false)?),
                _ => return Err(Error::new(self.current.start, E102)),
            }
        }
        self.consume(RightBrace)?;

        Ok(ast::Class {
            name,
            members,
            methods,
            functions,
        })
    }

    fn member(&mut self, mutable: bool) -> Res<Member> {
        let name = self.consume(Identifier)?;
        self.consume(Colon)?;
        let ty = self.typ()?;
        Ok(Member { name, ty, mutable })
    }

    fn function(&mut self, is_ext: bool) -> Res<Function> {
        let name = self.consume(Identifier)?;

        self.consume(LeftParen)?;
        let mut params = Vec::new();
        if !self.check(RightParen) {
            loop {
                let name = self.consume(Identifier)?.lex;
                self.consume(Colon)?;
                let ty = self.typ()?;
                params.push(Parameter { name, ty });
                if !self.matches(Comma) {
                    break;
                }
            }
        }
        self.consume(RightParen)?;

        let ret_type = if self.matches(Arrow) {
            Some(self.typ()?)
        } else {
            None
        };

        let body = if !is_ext {
            Some(self.expression()?)
        } else {
            None
        };
        Ok(Function {
            name,
            params,
            ret_type,
            body,
        })
    }

    fn higher_expr(&mut self) -> Res<Expr> {
        if self.check_(&[Var, Val]) {
            self.var_decl()
        } else {
            self.expression()
        }
    }

    fn var_decl(&mut self) -> Res<Expr> {
        let final_ = self.advance().kind == Val;
        let name = self.consume(Identifier)?;
        self.consume(Equal)?;
        let value = self.expression()?;
        Ok(Expr {
            start: name.start,
            ty: Box::new(EExpr::Variable {
                final_,
                name,
                value,
            }),
        })
    }

    fn expression(&mut self) -> Res<Expr> {
        match self.current.kind {
            LeftBrace => self.block(),
            If => self.if_expr(),
            While => self.while_stmt(),
            _ => self.binary(0),
        }
    }

    fn block(&mut self) -> Res<Expr> {
        let brace = self.advance();
        let mut exprs = Vec::new();
        while !self.is_at_end() && !self.check(RightBrace) {
            exprs.push(self.higher_expr()?)
        }
        self.consume(RightBrace)?;
        Ok(Expr {
            ty: Box::new(EExpr::Block(exprs)),
            start: brace.start,
        })
    }

    fn if_expr(&mut self) -> Res<Expr> {
        let start = self.advance().start;
        self.consume(LeftParen)?;
        let cond = self.expression()?;
        self.consume(RightParen)?;
        let then = self.expression()?;
        let els = if self.matches(Else) {
            Some(self.expression()?)
        } else {
            None
        };
        Ok(Expr {
            ty: Box::new(EExpr::If { cond, then, els }),
            start,
        })
    }

    fn while_stmt(&mut self) -> Res<Expr> {
        let start = self.advance().start;
        self.consume(LeftParen)?;
        let cond = self.expression()?;
        self.consume(RightParen)?;
        let body = self.expression()?;
        Ok(Expr {
            ty: Box::new(EExpr::While { cond, body }),
            start,
        })
    }

    fn binary(&mut self, minimum_binding_power: u8) -> Res<Expr> {
        let mut expr = self.unary()?;

        while let Some((lbp, rbp)) = self.current.kind.infix_binding_power() {
            if lbp < minimum_binding_power {
                return Ok(expr);
            }

            let op = self.advance();
            let right = self.binary(rbp)?;
            expr = Expr {
                start: expr.start,
                ty: Box::new(EExpr::Binary {
                    left: expr,
                    op,
                    right,
                }),
            };
        }

        Ok(expr)
    }

    fn unary(&mut self) -> Res<Expr> {
        if let Some(rbp) = self.current.kind.prefix_binding_power() {
            let op = self.advance();
            let right = self.binary(rbp)?;
            Ok(Expr {
                start: op.start,
                ty: Box::new(EExpr::Unary { op, right }),
            })
        } else {
            self.call()
        }
    }

    fn call(&mut self) -> Res<Expr> {
        let mut expr = self.primary()?;
        loop {
            match self.current.kind {
                LeftParen => {
                    self.advance();
                    let mut args = Vec::new();
                    if !self.check(RightParen) {
                        loop {
                            args.push(self.expression()?);
                            if !self.matches(Comma) {
                                break;
                            }
                        }
                    }
                    self.consume(RightParen)?;
                    expr = Expr {
                        start: expr.start,
                        ty: Box::new(EExpr::Call { callee: expr, args }),
                    }
                }

                _ => break,
            }
        }
        Ok(expr)
    }

    fn primary(&mut self) -> Res<Expr> {
        match self.current.kind {
            False => Ok(Expr {
                ty: Box::new(EExpr::Literal(Literal::Bool(false))),
                start: self.advance().start,
            }),
            True => Ok(Expr {
                ty: Box::new(EExpr::Literal(Literal::Bool(true))),
                start: self.advance().start,
            }),
            String => Ok(Expr {
                start: self.current.start,
                ty: Box::new(EExpr::Literal(Literal::String(self.advance().lex))),
            }),
            Int => Ok(Expr {
                ty: Box::new(EExpr::Literal(Literal::Int(
                    i64::from_str(&self.current.lex).unwrap(),
                ))),
                start: self.advance().start,
            }),
            Float => Ok(Expr {
                ty: Box::new(EExpr::Literal(Literal::Float(
                    f64::from_str(&self.current.lex).unwrap(),
                ))),
                start: self.advance().start,
            }),

            Identifier => Ok(Expr {
                start: self.current.start,
                ty: Box::new(EExpr::Identifier(self.advance())),
            }),
            LeftParen => {
                self.advance();
                let expr = self.expression()?;
                self.consume(RightParen)?;
                Ok(expr)
            }

            _ => Err(Error::new(self.current.start, E101)),
        }
    }

    fn typ(&mut self) -> Res<Type> {
        let name = self.consume(Identifier)?;
        Ok(Type { name })
    }

    fn matches(&mut self, kind: TKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume(&mut self, kind: TKind) -> Res<Token> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(Error::new(
                self.current.start,
                E100 {
                    expected: kind,
                    found: self.current.kind,
                },
            ))
        }
    }

    fn advance(&mut self) -> Token {
        let next = self.lexer.next().unwrap_or_else(|| Token {
            kind: TKind::Error,
            lex: SmolStr::new_inline("\0"),
            start: self.current.start + 1,
        });
        mem::replace(&mut self.current, next)
    }

    fn check(&mut self, kind: TKind) -> bool {
        self.current.kind == kind
    }

    fn check_(&mut self, kinds: &[TKind]) -> bool {
        for kind in kinds {
            if self.check(*kind) {
                return true;
            }
        }
        false
    }

    fn is_at_end(&self) -> bool {
        self.current.kind == TKind::Error
    }

    fn synchronize(&mut self) {
        while !self.is_at_end() {
            match self.advance().kind {
                Fun => return,
                _ => (),
            }
        }
    }

    pub fn new(src: &'src str) -> Self {
        let mut lexer = Lexer::new(src);
        let current = lexer.next().unwrap();
        Self {
            lexer,
            current,
            errors: Vec::new(),
        }
    }
}
