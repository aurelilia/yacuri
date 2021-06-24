use crate::{
    compiler::{ir, ir::IExpr},
    lexer::TKind,
    parser::ast::Literal,
    vm::{
        function::FnTranslator,
        typesys::{value, CValue},
    },
};
use cranelift::prelude::*;

impl<'b> FnTranslator<'b> {
    pub fn trans_expr(&mut self, expr: &ir::Expr) -> CValue {
        match &*expr.inner {
            IExpr::Binary { left, op, right } => value(self.binary(left, op.kind, right)),

            IExpr::Literal(literal) => value(self.literal(literal)),

            IExpr::Block(insts) => {
                let mut value = None;
                for inst in insts {
                    value = Some(self.trans_expr(inst));
                }
                value.unwrap()
            }

            IExpr::Poison => panic!("Cannot translate poison values!"),
        }
    }

    fn binary(&mut self, left: &ir::Expr, op: TKind, right: &ir::Expr) -> Value {
        let l = self.trans_expr(left)[0];
        let r = self.trans_expr(right)[0];

        if left.typ().is_int() {
            match op {
                TKind::Plus => self.cl.ins().iadd(l, r),
                TKind::Minus => self.cl.ins().isub(l, r),
                TKind::Star => self.cl.ins().imul(l, r),
                TKind::Slash => self.cl.ins().udiv(l, r),
                _ => self.cl.ins().icmp(intcmp(op), l, r),
            }
        } else {
            match op {
                TKind::Plus => self.cl.ins().fadd(l, r),
                TKind::Minus => self.cl.ins().fsub(l, r),
                TKind::Star => self.cl.ins().fmul(l, r),
                TKind::Slash => self.cl.ins().fdiv(l, r),
                _ => self.cl.ins().fcmp(floatcmp(op), l, r),
            }
        }
    }

    fn literal(&mut self, literal: &Literal) -> Value {
        match literal {
            Literal::Bool(val) => self.cl.ins().bconst(types::B1, *val),
            Literal::Int(int) => self.cl.ins().iconst(types::I64, *int),
            Literal::Float(float) => self.cl.ins().f64const(*float),
            Literal::String(_) => unimplemented!(),
        }
    }
}

fn intcmp(tok: TKind) -> IntCC {
    match tok {
        TKind::EqualEqual => IntCC::Equal,
        TKind::BangEqual => IntCC::NotEqual,
        TKind::Greater => IntCC::SignedGreaterThan,
        TKind::GreaterEqual => IntCC::SignedGreaterThanOrEqual,
        TKind::Less => IntCC::SignedLessThan,
        TKind::LessEqual => IntCC::SignedLessThanOrEqual,
        _ => panic!("unknown comparison operator"),
    }
}

fn floatcmp(tok: TKind) -> FloatCC {
    match tok {
        TKind::EqualEqual => FloatCC::Equal,
        TKind::BangEqual => FloatCC::NotEqual,
        TKind::Greater => FloatCC::GreaterThan,
        TKind::GreaterEqual => FloatCC::GreaterThanOrEqual,
        TKind::Less => FloatCC::LessThan,
        TKind::LessEqual => FloatCC::LessThanOrEqual,
        _ => panic!("unknown comparison operator"),
    }
}
