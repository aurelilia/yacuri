use crate::{
    error::{Error, ErrorKind::E201, Res},
    lexer::Token,
    parser::{ast, ast::Literal},
    smol_str::SmolStr,
};
use alloc::{boxed::Box, rc::Rc};
use core::{cell::RefCell, fmt, fmt::Display};
use hashbrown::HashSet;
use smallvec::{
    alloc::{fmt::Formatter, vec::Vec},
    SmallVec,
};

#[derive(Debug)]
pub struct Module {
    pub funcs: Vec<Function>,
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
}

#[derive(Debug)]
pub struct Function {
    pub name: SmolStr,
    pub params: SmallVec<[Rc<LocalVar>; 4]>,
    pub ret_type: Type,
    pub body: RefCell<FunctionBody>,
    pub ast: ast::Function,
}

#[derive(Debug)]
pub struct FunctionBody {
    pub locals: SmallVec<[Rc<LocalVar>; 6]>,
    pub body: Expr,
}

#[derive(Debug)]
pub struct LocalVar {
    pub ty: Type,
    pub name: SmolStr,
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Void,
    Poison,
    Bool,
    I64,
    F64,
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
        Self::new(IExpr::Literal(Literal::Int(0)))
    }

    pub fn poison() -> Expr {
        Self::new(IExpr::Poison)
    }

    pub fn binary(left: Expr, op: Token, right: Expr) -> Expr {
        Self::new(IExpr::Binary { left, op, right })
    }

    pub fn literal(lit: Literal) -> Expr {
        Self::new(IExpr::Literal(lit))
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

    fn get_type(&self) -> Type {
        match &*self.inner {
            IExpr::Poison => Type::Poison,

            IExpr::Binary { op, .. } if op.kind.is_binary_logic() => Type::Bool,
            IExpr::Binary { left, .. } => left.get_type(),

            IExpr::Literal(Literal::Bool(_)) => Type::Bool,
            IExpr::Literal(Literal::Int(_)) => Type::I64,
            IExpr::Literal(Literal::Float(_)) => Type::F64,
            IExpr::Literal(_) => unimplemented!(),

            IExpr::Block(expr) => expr.last().map(|e| e.typ()).unwrap_or(Type::Void),

            IExpr::If { phi, .. } if !phi => Type::Void,
            IExpr::If { then, .. } => then.get_type(),
        }
    }

    fn new(inner: IExpr) -> Expr {
        Expr {
            inner: Box::new(inner),
            ty: RefCell::new(None),
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

    Literal(Literal),

    Block(Vec<Expr>),

    If {
        cond: Expr,
        then: Expr,
        els: Expr,
        phi: bool,
    },
}
