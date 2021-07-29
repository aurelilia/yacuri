use crate::{
    compiler::{ir::Module, module::ModuleCompiler},
    error::Errors,
    parser::ast,
};
use alloc::{rc::Rc, vec::Vec};
use core::cell::RefCell;

pub mod ir;
pub mod module;

pub type MutRc<T> = Rc<RefCell<T>>;

fn mutrc_new<T>(inner: T) -> MutRc<T> {
    Rc::new(RefCell::new(inner))
}

pub struct Compiler {
    modules: Vec<MutRc<Module>>,
    compilers: Vec<ModuleCompiler>,
}

impl Compiler {
    pub fn consume(mut self) -> Result<Vec<MutRc<Module>>, Vec<Errors>> {
        self.all_mods(ModuleCompiler::stage_1);
        self.finish()
    }

    fn all_mods(&mut self, mut cls: impl FnMut(&mut ModuleCompiler)) {
        for compiler in self.compilers.iter_mut() {
            cls(compiler)
        }
    }

    fn finish(self) -> Result<Vec<MutRc<Module>>, Vec<Errors>> {
        let mut errors = Vec::new();
        for comp in self.compilers {
            if !comp.errors.is_empty() {
                errors.push(comp.errors);
            }
        }

        if errors.is_empty() {
            Ok(self.modules)
        } else {
            Err(errors)
        }
    }

    pub fn new(modules: Vec<ast::Module>) -> Self {
        let modules: Vec<_> = modules.into_iter().map(Module::from_ast).collect();
        Self {
            compilers: modules.iter().cloned().map(ModuleCompiler::new).collect(),
            modules,
        }
    }
}
