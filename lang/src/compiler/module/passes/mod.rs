use crate::{
    compiler::{
        ir::{Class, ClassContent, Expr, FuncRef, Function, Type, VarStore},
        module::{expr_compiler::ExprCompiler, ModuleCompiler},
    },
    error::Res,
    parser::ast,
};
use alloc::vec::Vec;
use core::{cell::RefCell, mem};
use indexmap::IndexMap;
use smallvec::SmallVec;

impl ModuleCompiler {
    pub fn run_all(&mut self) {
        self.stage_1();
    }

    pub fn stage_1(&mut self) {
        self.declare_classes().unwrap();
        self.declare_functions().unwrap();
        self.generate_classes().unwrap();
        self.generate_functions().unwrap();
    }

    fn declare_classes(&mut self) -> Res<()> {
        let ast_cls = mem::replace(&mut self.module.borrow_mut().ast.classes, Vec::new());
        for cls in ast_cls {
            self.module
                .borrow_mut()
                .try_reserve_name(&cls.name.lex, cls.name.start)?;

            self.module.borrow_mut().classes.push(Class {
                name: cls.name.lex.clone(),
                content: RefCell::new(IndexMap::with_capacity(
                    cls.methods.len() + cls.members.len() + cls.functions.len() + 2,
                )),
                ast: RefCell::new(cls),
            })
        }
        Ok(())
    }

    fn declare_functions(&mut self) -> Res<()> {
        let ast_fns = mem::replace(&mut self.module.borrow_mut().ast.functions, Vec::new());
        for func in ast_fns {
            self.module
                .borrow_mut()
                .try_reserve_name(&func.name.lex, func.name.start)?;

            self.declare_function(func)?;
        }
        Ok(())
    }

    fn declare_function(&mut self, func: ast::Function) -> Res<FuncRef> {
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
        });

        Ok(FuncRef::new_last(&self.module))
    }

    fn generate_classes(&mut self) -> Res<()> {
        let module = self.module.clone();
        for cls in module.borrow().classes.iter() {
            let mut ast = cls.ast.borrow_mut();
            for (index, member) in ast.members.iter().enumerate() {
                let store = VarStore {
                    ty: self.resolve_ty(&member.ty)?,
                    name: member.name.lex.clone(),
                    index,
                    mutable: member.mutable,
                };
                cls.content
                    .borrow_mut()
                    .insert(member.name.lex.clone(), ClassContent::Member(store.clone()));
            }

            for method in ast.methods.drain(..) {
                let name = method.name.lex.clone();
                let fun = self.declare_function(method)?;
                cls.content
                    .borrow_mut()
                    .insert(name, ClassContent::Method(fun));
            }

            for function in ast.functions.drain(..) {
                let name = function.name.lex.clone();
                let fun = self.declare_function(function)?;
                cls.content
                    .borrow_mut()
                    .insert(name, ClassContent::Function(fun));
            }
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
