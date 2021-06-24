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
            self.build_function_signature(func);
            let mut translator =
                FnTranslator::new(func, &mut self.ctx.func, &mut self.builder_context);
            translator.build();

            let id = self
                .module
                .declare_function(&func.name, Linkage::Export, &self.ctx.func.signature)
                .unwrap();
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

    fn build_function_signature(&mut self, func: &ir::Function) {
        for param in &func.params {
            typesys::translate_type(&param.ty, |_, param_type| {
                self.ctx
                    .func
                    .signature
                    .params
                    .push(AbiParam::new(param_type));
            });
        }

        typesys::translate_type(&func.ret_type, |_, ret_type| {
            self.ctx
                .func
                .signature
                .returns
                .push(AbiParam::new(ret_type));
        });
    }
}
