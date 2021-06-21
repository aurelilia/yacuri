use crate::smol_str::SmolStr;
use smallvec::alloc::vec::Vec;

pub struct Function {
    pub name: SmolStr,
    pub params: Vec<LocalVar>,
}

pub struct LocalVar {
    pub ty: Type,
    pub name: SmolStr,
}

pub enum Type {}
