use crate::{
    compiler::{ir::Module, module::ModuleCompiler},
    error::Errors,
    parser::ast,
};
use alloc::vec::Vec;

pub mod ir;
pub mod module;

pub struct Compiler {
    modules: Vec<ModuleCompiler>,
}

impl Compiler {
    pub fn consume(mut self) -> Result<Vec<Module>, Vec<Errors>> {
        self.all_mods(ModuleCompiler::stage_1);
        self.finish()
    }

    fn all_mods(&mut self, mut cls: impl FnMut(&mut ModuleCompiler)) {
        for module in self.modules.iter_mut() {
            cls(module)
        }
    }

    fn finish(self) -> Result<Vec<Module>, Vec<Errors>> {
        let mut errors = Vec::new();
        let mods = self
            .modules
            .into_iter()
            .map(|m| {
                if !m.errors.is_empty() {
                    errors.push(m.errors);
                }
                m.module
            })
            .collect();

        if errors.is_empty() {
            Ok(mods)
        } else {
            Err(errors)
        }
    }

    pub fn new(modules: Vec<ast::Module>) -> Self {
        Self {
            modules: modules.into_iter().map(ModuleCompiler::new).collect(),
        }
    }
}
