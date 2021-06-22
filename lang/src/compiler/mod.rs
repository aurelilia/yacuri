mod expr_compiler;
mod passes;
mod resolver;

use crate::{
    compiler::expr_compiler::ExprCompiler,
    error::Errors,
    ir::{LocalVar, Module},
    parser::ast,
    smol_str::SmolStr,
};
use alloc::{rc::Rc, vec::Vec};
use core::cell::RefCell;
use hashbrown::{HashMap, HashSet};

type MutRc<T> = Rc<RefCell<T>>;

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
