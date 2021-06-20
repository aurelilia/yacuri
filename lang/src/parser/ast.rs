use crate::lexer::Token;
use alloc::{boxed::Box, vec::Vec};
use smol_str::SmolStr;

pub struct Module {
    pub functions: Vec<Function>,
}

pub struct Function {
    pub name: SmolStr,
    pub params: Vec<Parameter>,
    pub ret_type: Option<AType>,
    pub body: AExpr,
}

pub struct Parameter {
    pub name: SmolStr,
    pub ty: AType,
}

pub struct AType {
    pub name: SmolStr,
}

pub struct AExpr {
    pub ty: Box<EExpr>, // TODO use a bump allocator ideally
    pub start: usize,
}

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

pub enum Literal {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(SmolStr),
}
