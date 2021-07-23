mod function;
mod typesys;

use crate::{compiler::ir, vm::function::FnTranslator};
use core::mem;
use cranelift::{
    codegen::{
        binemit::{NullStackMapSink, NullTrapSink},
        ir as clif,
    },
    prelude::*,
};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{DataContext, FuncId, FuncOrDataId, Linkage, Module};

pub type SymbolTable<'t> = &'t [(&'t str, *const u8)];

#[allow(unused)]
pub struct JIT {
    builder_context: FunctionBuilderContext,
    ctx: codegen::Context,
    data_ctx: DataContext,
    module: JITModule,
}

impl JIT {
    pub(crate) fn jit_module(&mut self, module: &ir::Module) {
        for func in module.funcs.iter().filter(|f| f.ast.body.is_some()) {
            make_fn_sig(&mut self.ctx.func.signature, func);
            let id = declare_ir_function(&mut self.module, func, &self.ctx.func.signature);
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
        }

        self.module.finalize_definitions();
    }

    pub fn exec<T>(&mut self, name: &str) -> T {
        let id = self.module.get_name(name).unwrap();
        let id = if let FuncOrDataId::Func(id) = id {
            id
        } else {
            panic!()
        };

        let ptr = self.module.get_finalized_function(id);
        let func = unsafe { mem::transmute::<_, fn() -> T>(ptr) };
        func()
    }

    pub fn new(symbols: SymbolTable) -> Self {
        let mut builder = JITBuilder::new(cranelift_module::default_libcall_names());
        for (name, ptr) in symbols {
            builder.symbol(*name, *ptr);
        }

        let module = JITModule::new(builder);
        Self {
            builder_context: FunctionBuilderContext::new(),
            ctx: module.make_context(),
            data_ctx: DataContext::new(),
            module,
        }
    }
}

fn get_or_declare_ir_fn(module: &mut JITModule, func: &ir::Function) -> FuncId {
    let mut ir = func.ir.borrow_mut();
    if let Some(ir) = *ir {
        ir
    } else {
        let mut sig = module.make_signature();
        make_fn_sig(&mut sig, func);
        let id = module
            .declare_function(&func.name, get_linkage(func), &sig)
            .unwrap();
        *ir = Some(id);
        id
    }
}

fn declare_ir_function(
    module: &mut JITModule,
    func: &ir::Function,
    sig: &clif::Signature,
) -> FuncId {
    let mut ir = func.ir.borrow_mut();
    if let Some(ir) = *ir {
        ir
    } else {
        let id = module
            .declare_function(&func.name, get_linkage(func), &sig)
            .unwrap();
        *ir = Some(id);
        id
    }
}

fn get_linkage(func: &ir::Function) -> Linkage {
    if func.ast.body.is_none() {
        Linkage::Import
    } else {
        Linkage::Export
    }
}

fn make_fn_sig(sig: &mut clif::Signature, func: &ir::Function) {
    for p in &func.params {
        typesys::translate_type(&p.ty, |_, ty| sig.params.push(AbiParam::new(ty)));
    }
    typesys::translate_type(&func.ret_type, |_, ty| sig.returns.push(AbiParam::new(ty)));
}
