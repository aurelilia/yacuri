#![no_std]

extern crate alloc;

use crate::{
    compiler::Compiler,
    error::{Error, Errors},
    ir::Module,
    parser::Parser,
};
use alloc::vec::Vec;

#[cfg(feature = "core")]
pub use cranelift_jit::{MemoryManager, set_manager};

mod compiler;
mod error;
mod ir;
mod lexer;
mod parser;
mod smol_str;
mod vm;

pub fn execute_program(program: &str) -> Result<Module, Errors> {
    let parse = Parser::new(program).parse()?;
    Compiler::new(parse).consume()
}
