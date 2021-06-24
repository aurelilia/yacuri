use crate::{
    compiler::{
        ir::{Expr, Function, LocalVar, Type},
        Compiler,
    },
    error::{
        ErrorKind,
        ErrorKind::{E500, E501, E502},
    },
    parser::{ast, ast::EExpr},
    smol_str::SmolStr,
};
use alloc::{rc::Rc, string::ToString, vec, vec::Vec};
use hashbrown::HashMap;

type Environment = HashMap<SmolStr, Rc<LocalVar>>;

pub struct ExprCompiler<'e> {
    function: &'e Function,
    compiler: &'e Compiler,

    environments: Vec<Environment>,
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
                let exprs = exprs.iter().map(|e| self.expr(e)).collect();
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

            /*
            EExpr::Identifier(_) => {}
            EExpr::Variable { .. } => {}
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

    pub fn new(compiler: &'e Compiler, function: &'e Function) -> Self {
        ExprCompiler {
            function,
            compiler,
            environments: vec![function
                .params
                .iter()
                .map(|p| (p.name.clone(), p.clone()))
                .collect()],
        }
    }
}
