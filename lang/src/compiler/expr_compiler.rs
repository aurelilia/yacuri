use crate::{
    compiler::{
        ir::{Expr, Function, LocalVar, Type},
        Compiler,
    },
    error::{
        ErrorKind,
        ErrorKind::{E500, E501, E502, E503, E504, E505},
    },
    lexer::TKind,
    parser::{ast, ast::EExpr},
    smol_str::SmolStr,
};
use alloc::{rc::Rc, string::ToString, vec, vec::Vec};
use hashbrown::HashMap;

type Environment<'e> = HashMap<SmolStr, &'e LocalVar>;

pub struct ExprCompiler<'e> {
    function: &'e Function,
    compiler: &'e Compiler,

    environments: Vec<Environment<'e>>,
}

impl<'e> ExprCompiler<'e> {
    pub fn expr(&mut self, expr: &ast::Expr) -> Expr {
        match &*expr.ty {
            EExpr::Literal(lit) => Expr::literal(lit.clone()),

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

            EExpr::Identifier(ident) => {
                let local = self.find_local(&ident.lex);
                if let Some(local) = local {
                    Expr::local(local)
                } else {
                    self.err(
                        ident.start,
                        E503 {
                            name: ident.lex.clone(),
                        },
                    );
                    Expr::poison()
                }
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

                let local = self.function.add_local(name.lex.clone(), ty);
                self.add_to_scope(local);
                Expr::assign_local(local, value)
            }

            /*
            EExpr::While { .. } => {}
            EExpr::Binary { .. } => {}
            EExpr::Unary { .. } => {}
            EExpr::Call { .. } => {}
            */
            _ => panic!("i can't compile this"),
        }
    }

    fn err(&self, _pos: usize, _err: ErrorKind) {
        // self.compiler.errors
    }

    fn find_local(&self, name: &str) -> Option<&LocalVar> {
        self.environments
            .iter()
            .rev()
            .filter_map(|env| env.get(name))
            .next()
            .copied()
    }

    fn add_to_scope(&mut self, var: &'e LocalVar) {
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

    pub fn new(compiler: &'e Compiler, function: &'e Function) -> Self {
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
