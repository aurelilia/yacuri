use crate::{
    compiler::{
        ir::{Constant, Expr, FuncRef, Function, Type, VarStore},
        module::ModuleCompiler,
    },
    error::{ErrorKind, ErrorKind::*},
    lexer::TKind,
    parser::{ast, ast::EExpr},
    smol_str::SmolStr,
};
use alloc::{string::ToString, vec, vec::Vec};
use hashbrown::HashMap;
use smallvec::SmallVec;

type Environment<'e> = HashMap<SmolStr, &'e VarStore>;

pub struct ExprCompiler<'e> {
    function: &'e Function,
    compiler: &'e ModuleCompiler,
    environments: Vec<Environment<'e>>,
}

impl<'e> ExprCompiler<'e> {
    pub fn expr(&mut self, expr: &ast::Expr) -> Expr {
        match &*expr.ty {
            EExpr::Literal(lit) => Expr::constant(Constant::from_literal(lit)),

            EExpr::Binary { left, op, right } => {
                let left = self.expr(left);
                let right = self.expr(right);
                let lty = left.typ();
                let rty = right.typ();
                let logic = op.kind.is_binary_logic();

                match () {
                    _ if lty != rty => self.err(
                        op.start,
                        E500 {
                            left: lty.to_string(),
                            right: rty.to_string(),
                        },
                    ),

                    _ if op.kind == TKind::Equal => {
                        // Assignment
                        if !left.assignable() {
                            self.err(op.start, E505)
                        }
                        return Expr::assign(left, right);
                    }

                    _ if (logic && !lty.allow_logic()) || !logic && !lty.allow_math() => self.err(
                        op.start,
                        E501 {
                            op: op.lex.clone(),
                            ty: lty.to_string(),
                        },
                    ),

                    _ => (),
                }

                Expr::binary(left, op.clone(), right)
            }

            EExpr::Block(exprs) => {
                self.begin_scope();
                let exprs = exprs.iter().map(|e| self.expr(e)).collect();
                self.end_scope();
                Expr::block(exprs)
            }

            EExpr::If { cond, then, els } => {
                let condition = self.expr(cond);
                if condition.typ() != Type::Bool {
                    self.err(cond.start, E502);
                }

                let then = self.expr(then);
                let els = els.as_ref().map(|e| self.expr(e));
                Expr::if_(condition, then, els)
            }

            EExpr::While { cond, body } => {
                let condition = self.expr(cond);
                if condition.typ() != Type::Bool {
                    self.err(cond.start, E502);
                }
                let body = self.expr(body);
                Expr::while_(condition, body)
            }

            EExpr::Identifier(ident) => {
                let local = self.find_local(&ident.lex);
                if let Some(local) = local {
                    return Expr::local(local);
                }
                let func = self.find_function(&ident.lex);
                if let Some(func) = func {
                    return Expr::constant(Constant::Function(func));
                }

                self.err(
                    ident.start,
                    E503 {
                        name: ident.lex.clone(),
                    },
                );
                Expr::poison()
            }

            EExpr::Variable {
                final_,
                name,
                value,
            } => {
                let value = self.expr(value);
                let ty = value.typ();
                if !ty.allow_assignment() {
                    self.err(name.start, E504 { ty: ty.to_string() })
                }

                let local = self.function.add_local(name.lex.clone(), ty, !*final_);
                self.add_to_scope(local);
                Expr::assign_local(local, value)
            }

            EExpr::Call { callee, args } => {
                let start = callee.start;
                let callee = self.expr(callee);
                let fn_ref = if let Type::Function(fn_ref) = callee.typ() {
                    fn_ref
                } else {
                    self.err(
                        start,
                        E506 {
                            ty: callee.typ().to_string(),
                        },
                    );
                    return Expr::poison();
                };
                let func = fn_ref.resolve();

                let args = args
                    .iter()
                    .map(|a| self.expr(a))
                    .collect::<SmallVec<[Expr; 4]>>();
                if args.len() != func.params.len() {
                    self.err(
                        start,
                        E507 {
                            expected: func.params.len(),
                            found: args.len(),
                        },
                    );
                }
                for (i, (arg, param)) in args.iter().zip(func.params.iter()).enumerate() {
                    if arg.typ() != param.ty {
                        self.err(
                            start,
                            E508 {
                                expected: param.ty.to_string(),
                                found: arg.typ().to_string(),
                                pos: i,
                            },
                        );
                    }
                }

                Expr::call(callee, args, func.ret_type.clone())
            }

            /*
            EExpr::Unary { .. } => {}
            */
            _ => panic!("i can't compile this"),
        }
    }

    fn err(&self, _pos: usize, _err: ErrorKind) {
        // self.compiler.errors
    }

    fn find_local(&self, name: &str) -> Option<&VarStore> {
        self.environments
            .iter()
            .rev()
            .filter_map(|env| env.get(name))
            .next()
            .copied()
    }

    fn find_function(&self, name: &str) -> Option<FuncRef> {
        self.compiler
            .module
            .borrow()
            .funcs
            .iter()
            .position(|func| func.name == name)
            .map(|index| FuncRef {
                module: self.compiler.module.clone(),
                index,
            })
    }

    fn add_to_scope(&mut self, var: &'e VarStore) {
        self.environments
            .last_mut()
            .unwrap()
            .insert(var.name.clone(), var);
    }

    fn begin_scope(&mut self) {
        self.environments.push(HashMap::new());
    }

    fn end_scope(&mut self) {
        self.environments.pop();
    }

    pub fn new(compiler: &'e ModuleCompiler, function: &'e Function) -> Self {
        ExprCompiler {
            function,
            compiler,
            environments: vec![function
                .params
                .iter()
                .map(|p| (p.name.clone(), p))
                .collect()],
        }
    }
}
