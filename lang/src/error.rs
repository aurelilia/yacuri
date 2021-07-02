use crate::{lexer::TKind, smol_str::SmolStr};
use alloc::{string::String, vec::Vec};
use core::fmt::Display;

pub type Res<T> = Result<T, Error>;
pub type Errors = Vec<Error>;

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    start: usize,
}

impl Error {
    pub fn new(start: usize, kind: ErrorKind) -> Self {
        Self { start, kind }
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    // Expected '{}', found '{}'.
    E100 { expected: TKind, found: TKind },
    // Expected expression.
    E101,

    // Cannot find type '{}'.
    E200(SmolStr),
    // Name '{}' already used.
    E201(SmolStr),

    // L/R side of binary expression must have same type (left is '{}', right is '{}').
    E500 { left: String, right: String },
    // Operator '{}' not applicable to type '{}'.
    E501 { op: SmolStr, ty: String },
    // Condition must be of type bool.
    E502,
    // Unknown variable '{}'.
    E503 { name: SmolStr },
    // Cannot assign type '{}' to a variable.
    E504 { ty: String },
    // Cannot assign to this.
    E505,
}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}
