use crate::{
    compiler::{mutrc_new, MutRc},
    error::{Error, ErrorKind::E201, Res},
    lexer::Token,
    parser::{ast, ast::Literal},
    smol_str::SmolStr,
};
use alloc::{boxed::Box, rc::Rc};
use core::{
    cell::{Ref, RefCell},
    fmt,
    fmt::Display,
};
use cranelift_module::FuncId;
use hashbrown::HashSet;
use indexmap::map::IndexMap;
use smallvec::{
    alloc::{fmt::Formatter, vec::Vec},
    SmallVec,
};

#[derive(Debug)]
pub struct Module {
    pub funcs: Vec<Function>,
    pub classes: Vec<Class>,
    pub reserved_names: HashSet<SmolStr>,
    pub ast: ast::Module,
}

impl Module {
    pub fn try_reserve_name(&mut self, name: &SmolStr, pos: usize) -> Res<()> {
        if !self.reserved_names.insert(name.clone()) {
            Err(Error::new(pos, E201(name.clone())))
        } else {
            Ok(())
        }
    }

    pub fn from_ast(ast: ast::Module) -> MutRc<Module> {
        mutrc_new(Self {
            funcs: Vec::with_capacity(ast.functions.len()),
            classes: Vec::with_capacity(ast.classes.len()),
            reserved_names: HashSet::with_capacity(ast.functions.len()),
            ast,
        })
    }
}

#[derive(Debug)]
pub struct Class {
    pub name: SmolStr,
    pub content: RefCell<IndexMap<SmolStr, ClassContent>>,
    pub ast: RefCell<ast::Class>,
}

#[derive(Debug)]
pub enum ClassContent {
    Member(VarStore),
    Method(FuncRef),
    Function(FuncRef),
}

#[derive(Debug)]
pub struct Function {
    pub name: SmolStr,
    pub params: SmallVec<[VarStore; 4]>,
    pub ret_type: Type,
    pub locals: SmallVec<[VarStore; 6]>,
    pub body: RefCell<Expr>,
    pub ir: RefCell<Option<FuncId>>,
    pub ast: ast::Function,
}

impl Function {
    pub fn add_local(&self, name: SmolStr, ty: Type, mutable: bool) -> &VarStore {
        let local = VarStore {
            ty,
            name,
            index: self.locals.len(),
            mutable,
        };
        unsafe {
            self.unsafe_mut().locals.push(local);
        }
        self.locals.last().unwrap()
    }

    /// # Safety
    /// This method allows getting a mutable reference from a immutable one.
    /// Very unsafe!
    /// The main usage of this method is `add_local`, where it is used
    /// to append to the list of locals.
    /// This is required to allow borrowing locals (see `src/compiler/expr_compiler.rs`) of the
    /// function immutably (which a RefCell, for example, would make impossible).
    ///
    /// TODO: Is this even safe?! references are probably going to be invalid
    /// if the vector has to reallocate since their memory location moves!!!
    unsafe fn unsafe_mut(&self) -> &mut Self {
        let ptr = self as *const Function;
        let mutptr = ptr as *mut Function;
        mutptr.as_mut().unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct FuncRef {
    pub module: MutRc<Module>,
    pub index: usize,
}

impl FuncRef {
    pub fn resolve<'t>(&self) -> Ref<Function> {
        Ref::map(self.module.borrow(), |module| &module.funcs[self.index])
    }

    pub fn new_last(module: &MutRc<Module>) -> Self {
        Self {
            module: module.clone(),
            index: module.borrow().funcs.len() - 1,
        }
    }
}

impl PartialEq for FuncRef {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && Rc::ptr_eq(&self.module, &other.module)
    }
}

#[derive(Clone, Debug)]
pub struct ClassRef {
    pub module: MutRc<Module>,
    pub index: usize,
}

impl ClassRef {
    pub fn resolve<'t>(&self) -> Ref<Class> {
        Ref::map(self.module.borrow(), |module| &module.classes[self.index])
    }
}

impl PartialEq for ClassRef {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && Rc::ptr_eq(&self.module, &other.module)
    }
}

#[derive(Debug, Clone)]
pub struct VarStore {
    pub ty: Type,
    pub name: SmolStr,
    pub index: usize,
    pub mutable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Void,
    Poison,
    Bool,
    I64,
    F64,

    Function(FuncRef),
    Class(ClassRef),
}

impl Type {
    pub fn is_int(&self) -> bool {
        *self == Type::I64 || *self == Type::Poison
    }

    pub fn allow_math(&self) -> bool {
        *self == Type::I64 || *self == Type::F64 || *self == Type::Poison
    }

    pub fn allow_logic(&self) -> bool {
        *self == Type::Bool || *self == Type::Poison
    }

    pub fn allow_assignment(&self) -> bool {
        *self != Type::Void
    }

    pub fn into_fn(self) -> FuncRef {
        match self {
            Self::Function(r) => r,
            _ => panic!(),
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
pub struct Expr {
    pub inner: Box<IExpr>, // todo bump allocation
    ty: RefCell<Option<Type>>,
}

impl Expr {
    pub fn zero() -> Expr {
        Self::new(IExpr::Constant(Constant::Int(0)))
    }

    pub fn poison() -> Expr {
        Self::new(IExpr::Poison)
    }

    pub fn binary(left: Expr, op: Token, right: Expr) -> Expr {
        Self::new(IExpr::Binary { left, op, right })
    }

    pub fn constant(lit: Constant) -> Expr {
        Self::new(IExpr::Constant(lit))
    }

    pub fn block(exprs: Vec<Expr>) -> Expr {
        Self::new(IExpr::Block(exprs))
    }

    pub fn if_(cond: Expr, then: Expr, els: Option<Expr>) -> Expr {
        Self::new(IExpr::If {
            phi: els.is_some() && then.typ() == els.as_ref().unwrap().typ(),
            cond,
            then,
            els: els.unwrap_or_else(|| Self::zero()),
        })
    }

    pub fn while_(cond: Expr, body: Expr) -> Expr {
        Self::new(IExpr::While { cond, body })
    }

    pub fn local(variable: &VarStore) -> Expr {
        Self::new(IExpr::Variable {
            index: variable.index,
            typ: variable.ty.clone(),
        })
    }

    pub fn assign(store: Expr, value: Expr) -> Expr {
        Self::new(IExpr::Assign { store, value })
    }

    pub fn assign_local(variable: &VarStore, value: Expr) -> Expr {
        Self::assign(Self::local(variable), value)
    }

    pub fn call(callee: Expr, args: SmallVec<[Expr; 4]>, ret_type: Type) -> Expr {
        Self::with_typ(IExpr::Call { callee, args }, ret_type)
    }

    pub fn typ(&self) -> Type {
        let mut cached = self.ty.borrow_mut();
        if let Some(ty) = &*cached {
            ty.clone()
        } else {
            let ty = self.get_type();
            *cached = Some(ty.clone());
            ty
        }
    }

    pub fn assignable(&self) -> bool {
        match &*self.inner {
            IExpr::Variable { .. } => true,
            _ => false,
        }
    }

    fn get_type(&self) -> Type {
        match &*self.inner {
            IExpr::Poison => Type::Poison,

            IExpr::Binary { op, .. } if op.kind.is_binary_logic() => Type::Bool,
            IExpr::Binary { left, .. } => left.typ(),

            IExpr::Constant(Constant::Bool(_)) => Type::Bool,
            IExpr::Constant(Constant::Int(_)) => Type::I64,
            IExpr::Constant(Constant::Float(_)) => Type::F64,
            IExpr::Constant(Constant::String(_)) => unimplemented!(),
            IExpr::Constant(Constant::Function(f)) => Type::Function(f.clone()),
            IExpr::Constant(Constant::Class(c)) => Type::Class(c.clone()),

            IExpr::Block(expr) => expr.last().map(|e| e.typ()).unwrap_or(Type::Void),

            IExpr::If { phi, .. } if !phi => Type::Void,
            IExpr::If { then, .. } => then.typ(),

            IExpr::While { .. } => Type::Void,

            IExpr::Variable { typ, .. } => typ.clone(),

            IExpr::Assign { value, .. } => value.typ(),

            IExpr::Call { .. } => panic!(),
        }
    }

    fn new(inner: IExpr) -> Expr {
        Expr {
            inner: Box::new(inner),
            ty: RefCell::new(None),
        }
    }

    fn with_typ(inner: IExpr, typ: Type) -> Expr {
        Expr {
            inner: Box::new(inner),
            ty: RefCell::new(Some(typ)),
        }
    }
}

#[derive(Debug)]
pub enum IExpr {
    Poison,

    Binary {
        left: Expr,
        op: Token,
        right: Expr,
    },

    Constant(Constant),

    Block(Vec<Expr>),

    If {
        cond: Expr,
        then: Expr,
        els: Expr,
        phi: bool,
    },

    While {
        cond: Expr,
        body: Expr,
    },

    Variable {
        index: usize,
        typ: Type,
    },

    Assign {
        store: Expr,
        value: Expr,
    },

    Call {
        callee: Expr,
        args: SmallVec<[Expr; 4]>,
    },
}

#[derive(Debug, Clone)]
pub enum Constant {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(SmolStr),
    Function(FuncRef),
    Class(ClassRef),
}

impl Constant {
    pub fn from_literal(lit: &ast::Literal) -> Constant {
        match lit {
            Literal::Bool(b) => Self::Bool(*b),
            Literal::Int(i) => Self::Int(*i),
            Literal::Float(f) => Self::Float(*f),
            Literal::String(s) => Self::String(s.clone()),
        }
    }
}
