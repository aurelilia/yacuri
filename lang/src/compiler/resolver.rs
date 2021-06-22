use crate::{
    compiler::Compiler,
    error::{Error, ErrorKind::E200, Res},
    ir::Type,
    parser::ast,
    smol_str::SmolStr,
};

impl Compiler {
    pub fn resolve_ty(&self, ty: &ast::Type) -> Res<Type> {
        self.resolve_ty_name(&ty.name.lex, ty.name.start)
    }

    fn resolve_ty_name(&self, name: &SmolStr, position: usize) -> Res<Type> {
        match &name[..] {
            "bool" => Ok(Type::Bool),
            "i64" => Ok(Type::I64),
            "f64" => Ok(Type::F64),
            _ => Err(Error::new(position, E200(name.clone()))),
        }
    }
}
