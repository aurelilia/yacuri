mod expr_compiler;
pub mod ir;
mod passes;
mod resolver;

use crate::{error::Errors, parser::ast};
use alloc::vec::Vec;
use hashbrown::HashSet;
use ir::Module;

pub struct Compiler {
    module: Module,
    errors: Errors,
}

impl Compiler {
    pub fn consume(mut self) -> Result<Module, Errors> {
        self.run_passes();
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
