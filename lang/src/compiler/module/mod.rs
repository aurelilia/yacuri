mod expr_compiler;
mod passes;
mod resolver;

use crate::{
    compiler::{ir::Module, MutRc},
    error::Errors,
};
use alloc::vec::Vec;

pub struct ModuleCompiler {
    pub(super) module: MutRc<Module>,
    pub(super) errors: Errors,
}

impl ModuleCompiler {
    pub fn consume(mut self) -> Result<MutRc<Module>, Errors> {
        self.run_all();
        if self.errors.is_empty() {
            Ok(self.module)
        } else {
            Err(self.errors)
        }
    }

    pub fn new(module: MutRc<Module>) -> Self {
        Self {
            module,
            errors: Vec::new(),
        }
    }
}
