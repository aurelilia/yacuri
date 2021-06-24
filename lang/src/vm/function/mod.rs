use super::clif;
use crate::{compiler::ir, vm::typesys};
use alloc::vec::Vec;
use cranelift::{
    frontend::{FunctionBuilder, FunctionBuilderContext},
    prelude::*,
};
use smallvec::SmallVec;

mod exprs;

pub struct FnTranslator<'b> {
    func: &'b ir::Function,
    cl: FunctionBuilder<'b>,
    local_offsets: SmallVec<[usize; 6]>,
    blocks: SmallVec<[Block; 5]>,
    current_block: Block,
}

impl<'b> FnTranslator<'b> {
    pub fn build(&mut self) {
        self.init();
        let ret = self.trans_expr(&self.func.body.borrow().body);
        self.cl.ins().return_(&ret);
    }

    fn init(&mut self) {
        let entry = self.switch_new_block();
        self.cl.append_block_params_for_function_params(entry);
        self.cl.seal_block(entry);
        self.declare_variables();
    }

    fn declare_variables(&mut self) {
        let entry_block = self.blocks[0];
        let params = self
            .cl
            .block_params(entry_block)
            .iter()
            .copied()
            .collect::<Vec<_>>();
        for var in self.func.params.iter() {
            self.declare_local(var);
            self.define_local(var, &params[self.local_offsets[var.index]..]);
        }
        for var in self.func.body.borrow().locals.iter() {
            self.declare_local(var);
        }
    }

    fn declare_local(&mut self, var: &ir::LocalVar) {
        let last_len = self.local_offsets[var.index];
        let start_offset = self.local_offsets[var.index - 1] + last_len;

        let len = typesys::translate_type(&var.ty, |i, local| {
            let var = Variable::new(start_offset + i);
            self.cl.declare_var(var, local);
        });

        self.local_offsets.push(last_len + len);
    }

    fn define_local(&mut self, var: &ir::LocalVar, with: &[Value]) {
        let offset = self.local_offsets[var.index];
        typesys::translate_type(&var.ty, |i, _| {
            self.cl.def_var(Variable::new(offset + i), with[offset + i]);
        });
    }

    fn new_block(&mut self) -> Block {
        let block = self.cl.create_block();
        self.blocks.push(block);
        block
    }

    fn switch_new_block(&mut self) -> Block {
        let block = self.new_block();
        self.switch_block(block);
        block
    }

    fn switch_block(&mut self, block: Block) {
        self.cl.switch_to_block(block);
        self.current_block = block;
    }

    pub fn new(
        func: &'b ir::Function,
        clif: &'b mut clif::Function,
        ctx: &'b mut FunctionBuilderContext,
    ) -> Self {
        Self {
            func,
            cl: FunctionBuilder::new(clif, ctx),
            local_offsets: SmallVec::from_slice(&[0]),
            blocks: SmallVec::new(),
            current_block: Block::with_number(0).unwrap(),
        }
    }
}
