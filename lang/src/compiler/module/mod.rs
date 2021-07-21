mod expr_compiler;
mod passes;
mod resolver;

use crate::{compiler::ir::Module, error::Errors, parser::ast};
use alloc::vec::Vec;
use hashbrown::HashSet;

pub struct ModuleCompiler {
    pub(super) module: Module,
    pub(super) errors: Errors,
}

impl ModuleCompiler {
    pub fn consume(mut self) -> Result<Module, Errors> {
        self.run_all();
        if self.errors.is_empty() {
            Ok(self.module)
        } else {
            Err(self.errors)
        }
    }

    pub fn new(ast: ast::Module) -> Self {
        Self {
            module: Module {
                funcs: Vec::with_capacity(ast.functions.len()),
                reserved_names: HashSet::with_capacity(ast.functions.len()),
                ast,
            },
            errors: Vec::new(),
        }
    }
}
