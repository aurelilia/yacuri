use crate::{lexer::Token, smol_str::SmolStr};
use alloc::{boxed::Box, vec::Vec};

#[derive(Debug)]
pub struct Module {
    pub functions: Vec<Function>,
}

#[derive(Debug)]
pub struct Function {
    pub name: SmolStr,
    pub params: Vec<Parameter>,
    pub ret_type: Option<AType>,
    pub body: AExpr,
}

#[derive(Debug)]
pub struct Parameter {
    pub name: SmolStr,
    pub ty: AType,
}

#[derive(Debug)]
pub struct AType {
    pub name: SmolStr,
}

#[derive(Debug)]
pub struct AExpr {
    pub ty: Box<EExpr>, // TODO use a bump allocator ideally
    pub start: usize,
}

#[derive(Debug)]
pub enum EExpr {
    Literal(Literal),

    Identifier(Token),

    Variable {
        final_: bool,
        name: SmolStr,
        value: AExpr,
    },

    Block(Vec<AExpr>),

    If {
        cond: AExpr,
        then: AExpr,
        els: Option<AExpr>,
    },

    While {
        cond: AExpr,
        body: AExpr,
    },

    Binary {
        left: AExpr,
        op: Token,
        right: AExpr,
    },

    Unary {
        op: Token,
        right: AExpr,
    },

    Call {
        callee: AExpr,
        args: Vec<AExpr>,
    },
}

#[derive(Debug)]
pub enum Literal {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(SmolStr),
}
