use super::clif;
use crate::compiler::ir;
use cranelift::prelude::*;
use smallvec::SmallVec;

pub type CValue = SmallVec<[Value; 3]>;
pub const CLIF_PTR: clif::Type = types::I64;

pub fn value(clif: Value) -> CValue {
    SmallVec::from_slice(&[clif])
}
pub fn values(clif: &[Value]) -> CValue {
    SmallVec::from_slice(clif)
}

pub fn translate_type<T: FnMut(usize, clif::Type)>(typ: &ir::Type, mut adder: T) -> usize {
    translate_type_ref(typ, &mut adder)
}

fn translate_type_ref<T: FnMut(usize, clif::Type)>(typ: &ir::Type, adder: &mut T) -> usize {
    match typ {
        ir::Type::Void | ir::Type::Poison => return 0,
        ir::Type::Bool => adder(0, types::B1),
        ir::Type::F64 => adder(0, types::F64),
        ir::Type::I64 => adder(0, types::I64),
        ir::Type::Function(_) => adder(0, CLIF_PTR),
        ir::Type::Class(cls_ref) => {
            let mut count = 0;
            let cls = cls_ref.resolve();
            for mem in &cls.members {
                count += translate_type_ref(&mem.ty, adder);
            }
            return count;
        }
    }
    1
}
