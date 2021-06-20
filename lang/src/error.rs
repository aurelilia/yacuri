use crate::lexer::TKind;
use core::fmt::Display;

pub type Res<T> = Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    E100 { expected: TKind, found: TKind },
    E101,
}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}
