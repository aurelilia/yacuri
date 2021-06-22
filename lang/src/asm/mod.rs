use crate::ir::Expr;
use asm::x64::{AssemblerX64, RAX, Immediate};
use core::{mem, ptr};
use memmap::MmapOptions;

pub fn make_asm() -> usize { // TODO
    let mut asm = AssemblerX64::new();
    asm.movq_ri(RAX, Immediate(42));
    asm.retq();
    let code = asm.finalize();

    let mut map = MmapOptions::new().len(code.len()).map_anon().unwrap();
    unsafe {
        ptr::copy(code.as_ptr(), map.as_mut_ptr(), code.len());
        let ptr = map.make_exec().unwrap();
        let fnptr = mem::transmute::<_, FN>(ptr.as_ptr());
        fnptr()
    }
}

type FN = unsafe extern "C" fn() -> usize;
