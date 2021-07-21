use crate::{
    compiler::{
        ir::{Expr, Function, LocalVar, Type},
        module::{expr_compiler::ExprCompiler, ModuleCompiler},
    },
    error::Res,
};
use alloc::vec::Vec;
use core::{cell::RefCell, mem};
use smallvec::SmallVec;

impl ModuleCompiler {
    pub fn run_all(&mut self) {
        self.stage_1();
    }

    pub fn stage_1(&mut self) {
        self.declare_functions().unwrap();
        self.generate_functions().unwrap();
    }

    fn declare_functions(&mut self) -> Res<()> {
        let ast_fns = mem::replace(&mut self.module.ast.functions, Vec::new());
        for func in ast_fns {
            self.module
                .try_reserve_name(&func.name.lex, func.name.start)?;

            let params = func
                .params
                .iter()
                .enumerate()
                .map(|(index, param)| {
                    Ok(LocalVar {
                        ty: self.resolve_ty(&param.ty)?,
                        name: param.name.clone(),
                        index,
                        mutable: false,
                    })
                })
                .collect::<Res<SmallVec<_>>>()?;
            let ret_type = func
                .ret_type
                .as_ref()
                .map(|t| self.resolve_ty(&t))
                .unwrap_or(Ok(Type::Void))?;

            self.module.funcs.push(Function {
                name: func.name.lex.clone(),
                body: RefCell::new(Expr::poison()),
                params,
                locals: SmallVec::new(),
                ret_type,
                ir: RefCell::new(None),
                ast: func,
            })
        }
        Ok(())
    }

    fn generate_functions(&self) -> Res<()> {
        for func in &self.module.funcs {
            let mut compiler = ExprCompiler::new(self, func);
            let body = compiler.expr(&func.ast.body);
            *func.body.borrow_mut() = body;
        }
        Ok(())
    }
}
