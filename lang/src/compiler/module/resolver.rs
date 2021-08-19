use crate::{
    compiler::{
        ir::{ClassRef, Type},
        module::ModuleCompiler,
    },
    error::{Error, ErrorKind::E200, Res},
    parser::ast,
    smol_str::SmolStr,
};

impl ModuleCompiler {
    pub fn resolve_ty(&self, ty: &ast::Type) -> Res<Type> {
        self.resolve_ty_name(&ty.name.lex, ty.name.start)
    }

    fn resolve_ty_name(&self, name: &SmolStr, position: usize) -> Res<Type> {
        match &name[..] {
            "bool" => Ok(Type::Bool),
            "i64" => Ok(Type::I64),
            "f64" => Ok(Type::F64),
            _ => self
                .module
                .borrow_mut()
                .classes
                .iter()
                .position(|cls| cls.name == *name)
                .map(|index| {
                    Type::Class(ClassRef {
                        module: self.module.clone(),
                        index,
                    })
                })
                .ok_or_else(|| Error::new(position, E200(name.clone()))),
        }
    }
}
