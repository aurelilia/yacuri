use crate::lexer::TKind;
use alloc::{format, string::String};

pub type Res<T> = Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    E100 { expected: TKind, found: TKind },
    E101,
}

impl Error {
    pub fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}
