use crate::{
    compiler::{
        ir,
        ir::{Expr, IExpr},
    },
    lexer::TKind,
    parser::ast::Literal,
    vm::{
        function::FnTranslator,
        typesys,
        typesys::{value, values, CValue},
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

            IExpr::If {
                cond,
                then,
                els,
                phi,
            } => self.if_(cond, *phi, then, els),

            IExpr::Variable { index, typ } => self.variable_expr(*index, typ),

            IExpr::Assign { store, value } => match &*store.inner {
                IExpr::Variable { index, typ } => self.assign_var(*index, value, typ),
                _ => panic!("Unknown assignment target!"),
            },

            IExpr::Poison => panic!("Cannot translate poison values!"),
        }
    }

    fn variable(index: usize) -> Variable {
        Variable::with_u32(index as u32)
    }

    fn br(&mut self, cond: Value, then: Block, els: Block) {
        self.cl.ins().brz(cond, els, &[]);
        self.cl.ins().jump(then, &[]);
    }

    fn set_cont_params(&mut self, phi: bool, cont: Block, typ: &ir::Type) {
        if phi {
            typesys::translate_type(typ, |_, ty| {
                self.cl.append_block_param(cont, ty);
            });
        }
    }

    fn jump_cont(&mut self, cont: Block, phi: bool, value: CValue) {
        if phi {
            self.cl.ins().jump(cont, &value);
        } else {
            self.cl.ins().jump(cont, &[]);
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

    fn if_(&mut self, cond: &ir::Expr, phi: bool, then: &ir::Expr, els: &ir::Expr) -> CValue {
        let condition = self.trans_expr(cond);
        let then_b = self.new_block();
        let else_b = self.new_block();
        let cont_b = self.new_block();

        self.set_cont_params(phi, cont_b, &then.typ());
        self.br(condition[0], then_b, else_b);

        self.switch_block(then_b);
        self.cl.seal_block(then_b);
        let then_val = self.trans_expr(then);
        self.jump_cont(cont_b, phi, then_val);

        self.switch_block(else_b);
        self.cl.seal_block(else_b);
        let els_val = self.trans_expr(els);
        self.jump_cont(cont_b, phi, els_val);

        self.switch_block(cont_b);
        self.cl.seal_block(cont_b);
        values(self.cl.block_params(cont_b))
    }

    fn variable_expr(&mut self, index: usize, typ: &ir::Type) -> smallvec::SmallVec<[Value; 3]> {
        let offset = self.local_offsets[index];
        let mut vals = CValue::new();
        typesys::translate_type(typ, |i, _| {
            vals.push(self.cl.use_var(Self::variable(offset + i)))
        });
        vals
    }

    fn assign_var(
        &mut self,
        index: usize,
        value: &Expr,
        typ: &ir::Type,
    ) -> smallvec::SmallVec<[Value; 3]> {
        let offset = self.local_offsets[index];
        let value = self.trans_expr(value);
        typesys::translate_type(typ, |i, _| {
            self.cl.def_var(Self::variable(offset + i), value[i]);
        });
        value
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
