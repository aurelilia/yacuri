use crate::{
    compiler::{
        ir::{Expr, Function, Type, VarStore},
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
        let ast_fns = mem::replace(&mut self.module.borrow_mut().ast.functions, Vec::new());
        for func in ast_fns {
            self.module
                .borrow_mut()
                .try_reserve_name(&func.name.lex, func.name.start)?;

            let params = func
                .params
                .iter()
                .enumerate()
                .map(|(index, param)| {
                    Ok(VarStore {
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

            self.module.borrow_mut().funcs.push(Function {
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
        for func in self
            .module
            .borrow()
            .funcs
            .iter()
            .filter(|f| f.ast.body.is_some())
        {
            let mut compiler = ExprCompiler::new(self, func);
            let body = compiler.expr(&func.ast.body.as_ref().unwrap());
            *func.body.borrow_mut() = body;
        }
        Ok(())
    }
}
