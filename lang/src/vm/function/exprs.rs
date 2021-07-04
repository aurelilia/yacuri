use crate::{
    compiler::{
        ir,
        ir::{Constant, Expr, IExpr},
    },
    lexer::TKind,
    vm::{
        function::FnTranslator,
        get_func_id, typesys,
        typesys::{value, values, CValue},
    },
};
use alloc::vec::Vec;
use cranelift::prelude::*;
use cranelift_module::{Linkage, Module};
use smallvec::SmallVec;

impl<'b> FnTranslator<'b> {
    pub fn trans_expr(&mut self, expr: &ir::Expr) -> CValue {
        match &*expr.inner {
            IExpr::Binary { left, op, right } => value(self.binary(left, op.kind, right)),

            IExpr::Constant(constant) => value(self.constant(constant)),

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

            IExpr::While { cond, body } => self.while_expr(cond, body),

            IExpr::Variable { index, typ } => self.variable_expr(*index, typ),

            IExpr::Assign { store, value } => match &*store.inner {
                IExpr::Variable { index, typ } => self.assign_var(*index, value, typ),
                _ => panic!("Unknown assignment target!"),
            },

            IExpr::Call { callee, args } => self.call(callee, args),

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

    fn constant(&mut self, constant: &Constant) -> Value {
        match constant {
            Constant::Bool(val) => self.cl.ins().bconst(types::B1, *val),
            Constant::Int(int) => self.cl.ins().iconst(types::I64, *int),
            Constant::Float(float) => self.cl.ins().f64const(*float),
            Constant::String(_) => unimplemented!(),

            // Functions are always their own types, so their values are essentially zero-sized.
            // However, cranelift of course does not have zero-sized values,
            // so we just return whatever.
            // TODO is this fine? might be reasonable to return the pointer to the function instead for FFI or something in the future?
            Constant::Function(_) => self.cl.ins().iconst(types::I64, 0),
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

    fn while_expr(&mut self, cond: &Expr, body: &Expr) -> CValue {
        let head_b = self.new_block();
        let body_b = self.new_block();
        let cont_b = self.new_block();
        self.cl.ins().jump(head_b, &[]);
        self.switch_block(head_b);
        let condition_value = self.trans_expr(cond)[0];
        self.cl.ins().brz(condition_value, cont_b, &[]);
        self.cl.ins().jump(body_b, &[]);
        self.switch_block(body_b);
        self.cl.seal_block(body_b);
        self.trans_expr(body);
        self.cl.ins().jump(head_b, &[]);
        self.cl.switch_to_block(cont_b);
        self.cl.seal_block(head_b);
        self.cl.seal_block(cont_b);
        value(self.cl.ins().iconst(types::I64, 0))
    }

    fn variable_expr(&mut self, index: usize, typ: &ir::Type) -> CValue {
        let offset = self.local_offsets[index];
        let mut vals = CValue::new();
        typesys::translate_type(typ, |i, _| {
            vals.push(self.cl.use_var(Self::variable(offset + i)))
        });
        vals
    }

    fn assign_var(&mut self, index: usize, value: &Expr, typ: &ir::Type) -> CValue {
        let offset = self.local_offsets[index];
        let value = self.trans_expr(value);
        typesys::translate_type(typ, |i, _| {
            self.cl.def_var(Self::variable(offset + i), value[i]);
        });
        value
    }

    fn call(&mut self, callee: &Expr, args: &SmallVec<[Expr; 4]>) -> CValue {
        let ir_callee = self.trans_expr(callee);
        let func_id = {
            let func = callee.typ().into_fn().resolve(self.ya_module);
            get_func_id(&mut self.ir_module, func)
        };

        let local_callee = self
            .ir_module
            .declare_func_in_func(func_id, &mut self.cl.func);

        let mut call_args = Vec::new();
        for arg in args {
            let res = self.trans_expr(arg);
            for val in res {
                call_args.push(val);
            }
        }
        let call = self.cl.ins().call(local_callee, &call_args);
        values(self.cl.inst_results(call))
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
