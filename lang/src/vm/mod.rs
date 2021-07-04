mod function;
mod typesys;

use crate::{compiler::ir, vm::function::FnTranslator};
use alloc::vec::Vec;
use core::mem;
use cranelift::{
    codegen::{
        binemit::{NullStackMapSink, NullTrapSink},
        ir as clif,
    },
    prelude::*,
};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{DataContext, FuncId, Linkage, Module};

pub struct JIT {
    builder_context: FunctionBuilderContext,
    ctx: codegen::Context,
    data_ctx: DataContext,
    module: JITModule,
}

impl Default for JIT {
    fn default() -> Self {
        let builder = JITBuilder::new(cranelift_module::default_libcall_names());
        let module = JITModule::new(builder);
        Self {
            builder_context: FunctionBuilderContext::new(),
            ctx: module.make_context(),
            data_ctx: DataContext::new(),
            module,
        }
    }
}

impl JIT {
    pub fn compile_mod(&mut self, module: &ir::Module) -> FuncId {
        let mut ids = Vec::with_capacity(module.funcs.len());
        for func in &module.funcs {
            make_fn_sig(&mut self.ctx.func.signature, func);
            let id = get_func_id_with_sig(&mut self.module, func, &self.ctx.func.signature);
            let mut translator = FnTranslator::new(
                func,
                &mut self.ctx.func,
                &mut self.builder_context,
                &mut self.module,
                &module,
            );
            translator.build();

            self.module
                .define_function(
                    id,
                    &mut self.ctx,
                    &mut NullTrapSink {},
                    &mut NullStackMapSink {},
                )
                .unwrap();
            self.module.clear_context(&mut self.ctx);
            ids.push(id);
        }

        self.module.finalize_definitions();
        ids[0]
    }

    pub fn exec<T>(&mut self, id: FuncId) -> T {
        let ptr = self.module.get_finalized_function(id);
        let func = unsafe { mem::transmute::<_, fn() -> T>(ptr) };
        func()
    }
}

fn get_func_id(module: &mut JITModule, func: &ir::Function) -> FuncId {
    let mut ir = func.ir.borrow_mut();
    if let Some(ir) = *ir {
        ir
    } else {
        let mut sig = module.make_signature();
        make_fn_sig(&mut sig, func);
        let id = module
            .declare_function(&func.name, Linkage::Export, &sig)
            .unwrap();
        *ir = Some(id);
        id
    }
}

fn get_func_id_with_sig(
    module: &mut JITModule,
    func: &ir::Function,
    sig: &clif::Signature,
) -> FuncId {
    let mut ir = func.ir.borrow_mut();
    if let Some(ir) = *ir {
        ir
    } else {
        let id = module
            .declare_function(&func.name, Linkage::Export, &sig)
            .unwrap();
        *ir = Some(id);
        id
    }
}

fn make_fn_sig(sig: &mut clif::Signature, func: &ir::Function) {
    for p in &func.params {
        typesys::translate_type(&p.ty, |_, ty| sig.params.push(AbiParam::new(ty)));
    }
    typesys::translate_type(&func.ret_type, |_, ty| sig.returns.push(AbiParam::new(ty)));
}
