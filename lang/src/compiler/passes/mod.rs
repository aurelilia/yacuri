use crate::{
    compiler::{expr_compiler::ExprCompiler, Compiler},
    error::Res,
    ir::{Expr, Function, FunctionBody, LocalVar, Type},
};
use alloc::{rc::Rc, vec::Vec};
use core::{cell::RefCell, mem};
use smallvec::SmallVec;

impl Compiler {
    pub fn run_passes(&mut self) {
        self.declare_functions().unwrap();
        self.generate_functions();
    }

    fn declare_functions(&mut self) -> Res<()> {
        let ast_fns = mem::replace(&mut self.module.ast.functions, Vec::new());
        for func in ast_fns {
            self.module
                .try_reserve_name(&func.name.lex, func.name.start);

            let params = func
                .params
                .iter()
                .map(|param| {
                    Ok(Rc::new(LocalVar {
                        ty: self.resolve_ty(&param.ty)?,
                        name: param.name.clone(),
                    }))
                })
                .collect::<Res<SmallVec<_>>>()?;
            let ret_type = func
                .ret_type
                .as_ref()
                .map(|t| self.resolve_ty(&t))
                .unwrap_or(Ok(Type::Void))?;

            self.module.funcs.push(Function {
                name: func.name.lex.clone(),
                body: RefCell::new(FunctionBody {
                    locals: SmallVec::from(params.as_slice()),
                    body: Expr::poison(),
                }),
                params,
                ret_type,
                ast: func,
            })
        }
        Ok(())
    }

    fn generate_functions(&self) -> Res<()> {
        for func in &self.module.funcs {
            let mut compiler = ExprCompiler::new(self, func);
            let body = compiler.expr(&func.ast.body);
            func.body.borrow_mut().body = body;
        }
        Ok(())
    }
}