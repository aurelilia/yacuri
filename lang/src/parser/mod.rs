pub mod ast;

use crate::{
    error::{Error, Error::E101, Res},
    lexer::{Lexer, TKind, TKind::*, Token},
    parser::ast::{AExpr, AType, EExpr, Function, Literal, Parameter},
    smol_str::SmolStr,
};
use alloc::vec::Vec;
pub use ast::Module;
use core::{mem, str::FromStr};
use alloc::boxed::Box;

pub struct Parser<'src> {
    lexer: Lexer<'src>,
    current: Token,
    errors: Vec<Error>,
}

impl<'src> Parser<'src> {
    pub fn parse(mut self) -> Result<Module, Vec<Error>> {
        let mut functions = Vec::new();
        while !self.is_at_end() {
            self.advance(); // consume 'fun' for now
            match self.function() {
                Ok(f) => functions.push(f),
                Err(e) => {
                    self.errors.push(e);
                    self.synchronize()
                }
            }
        }
        if self.errors.is_empty() {
            Ok(Module { functions })
        } else {
            Err(self.errors)
        }
    }

    fn function(&mut self) -> Res<Function> {
        let name = self.consume(Identifier)?.lex;

        self.consume(LeftParen)?;
        let mut params = Vec::new();
        if !self.check(RightParen) {
            loop {
                let name = self.consume(Identifier)?.lex;
                self.consume(Colon)?;
                let ty = self.typ()?;
                params.push(Parameter { name, ty })
            }
        }
        self.consume(RightParen)?;

        let ret_type = if self.matches(Arrow) {
            Some(self.typ()?)
        } else {
            None
        };

        let body = self.expression()?;
        Ok(Function {
            name,
            params,
            ret_type,
            body,
        })
    }

    fn higher_expr(&mut self) -> Res<AExpr> {
        if self.matches_(&[Var, Val]) {
            self.var_decl()
        } else {
            self.expression()
        }
    }

    fn var_decl(&mut self) -> Res<AExpr> {
        let final_ = self.advance().kind == Val;
        let name = self.consume(Identifier)?;
        self.consume(Equal)?;
        let value = self.expression()?;
        Ok(AExpr {
            ty: Box::new(EExpr::Variable {
                final_,
                name: name.lex,
                value,
            }),
            start: name.start,
        })
    }

    fn expression(&mut self) -> Res<AExpr> {
        match self.current.kind {
            LeftBrace => self.block(),
            If => self.if_expr(),
            _ => self.binary(0),
        }
    }

    fn block(&mut self) -> Res<AExpr> {
        let brace = self.advance();
        let mut exprs = Vec::new();
        while !self.is_at_end() && !self.check(RightBrace) {
            exprs.push(self.higher_expr()?)
        }
        self.consume(RightBrace)?;
        Ok(AExpr {
            ty: Box::new(EExpr::Block(exprs)),
            start: brace.start,
        })
    }

    fn if_expr(&mut self) -> Res<AExpr> {
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
        Ok(AExpr {
            ty: Box::new(EExpr::If { cond, then, els }),
            start,
        })
    }

    fn binary(&mut self, minimum_binding_power: u8) -> Res<AExpr> {
        let mut expr = self.unary()?;

        while let Some((lbp, rbp)) = self.current.kind.infix_binding_power() {
            if lbp < minimum_binding_power {
                return Ok(expr);
            }

            let op = self.advance();
            let right = self.binary(rbp)?;
            expr = AExpr {
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

    fn unary(&mut self) -> Res<AExpr> {
        if let Some(rbp) = self.current.kind.prefix_binding_power() {
            let op = self.advance();
            let right = self.binary(rbp)?;
            Ok(AExpr {
                start: op.start,
                ty: Box::new(EExpr::Unary { op, right }),
            })
        } else {
            self.call()
        }
    }

    fn call(&mut self) -> Res<AExpr> {
        let mut expr = self.primary()?;
        loop {
            match self.current.kind {
                LeftParen => {
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
                    expr = AExpr {
                        start: expr.start,
                        ty: Box::new(EExpr::Call { callee: expr, args }),
                    }
                }

                _ => break,
            }
        }
        Ok(expr)
    }

    fn primary(&mut self) -> Res<AExpr> {
        match self.current.kind {
            False => Ok(AExpr {
                ty: Box::new(EExpr::Literal(Literal::Bool(false))),
                start: self.advance().start,
            }),
            True => Ok(AExpr {
                ty: Box::new(EExpr::Literal(Literal::Bool(true))),
                start: self.advance().start,
            }),
            String => Ok(AExpr {
                start: self.current.start,
                ty: Box::new(EExpr::Literal(Literal::String(self.advance().lex))),
            }),
            Int => Ok(AExpr {
                ty: Box::new(EExpr::Literal(Literal::Int(i64::from_str(&self.current.lex).unwrap()))),
                start: self.advance().start,
            }),
            Float => Ok(AExpr {
                ty: Box::new(EExpr::Literal(Literal::Float(f64::from_str(&self.current.lex).unwrap()))),
                start: self.advance().start,
            }),

            Identifier => Ok(AExpr {
                start: self.current.start,
                ty: Box::new(EExpr::Identifier(self.advance())),
            }),
            LeftParen => {
                self.advance();
                let expr = self.expression()?;
                self.consume(RightParen)?;
                Ok(expr)
            }

            _ => Err(E101),
        }
    }

    fn typ(&mut self) -> Res<AType> {
        let ident = self.consume(Identifier)?;
        Ok(AType { name: ident.lex })
    }

    fn matches(&mut self, kind: TKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn matches_(&mut self, kinds: &[TKind]) -> bool {
        for kind in kinds {
            if self.check(*kind) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn consume(&mut self, kind: TKind) -> Res<Token> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(Error::E100 {
                expected: kind,
                found: self.current.kind,
            })
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
