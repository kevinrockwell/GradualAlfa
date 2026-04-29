pub trait KnownType {
    fn get_type(&self) -> &AlfaType;
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Clone)]
pub enum AlfaType {
    Num,
    Bool,
    Unit,
    Arrow(Box<AlfaType>, Box<AlfaType>),
    Product(Box<AlfaType>, Box<AlfaType>),
    Sum(Box<AlfaType>, Box<AlfaType>),
    Dyn,
}
// Helper functions for constructing composite types
pub fn arrow(t1: AlfaType, t2: AlfaType) -> AlfaType {
    AlfaType::Arrow(Box::new(t1), Box::new(t2))
}
pub fn product(t1: AlfaType, t2: AlfaType) -> AlfaType {
    AlfaType::Product(Box::new(t1), Box::new(t2))
}
pub fn sum(t1: AlfaType, t2: AlfaType) -> AlfaType {
    AlfaType::Sum(Box::new(t1), Box::new(t2))
}

impl KnownType for AlfaType {
    fn get_type(&self) -> &AlfaType {
        &self
    }
}

#[derive(Debug, PartialEq)]
pub enum Infix {
    Plus,
    Minus,
    Times,
    LessThan,
    GreaterThan,
    EqualTo,
}

#[derive(Debug, PartialEq)]
pub enum Prefix {
    Neg,
}

// TODO: typ should probably be refactored out to be with the variable decl
// instead of the ID? And then type checking will propagate, along with
// (eventually) transforming AST to include casts/RTTI in some way.
#[derive(Debug, Clone)]
pub struct Id {
    pub id: String,
    pub typ: Option<AlfaType>,
}

impl PartialEq for Id {
    fn eq(&self, other: &Self) -> bool {
        if self.id != other.id {
            return false;
        }
        self.typ.as_ref() == other.typ.as_ref()
    }
}

#[derive(Debug, PartialEq)]
pub enum Expr {
    Num(i32),
    Bool(bool),
    Unit,
    Var(Id),
    Fun(Id, Box<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    Case(Box<Expr>, Id, Box<Expr>, Id, Box<Expr>),
    InjL(Box<Expr>),
    InjR(Box<Expr>),
    PrjL(Box<Expr>),
    PrjR(Box<Expr>),
    Let(Id, Box<Expr>, Box<Expr>),
    Ap(Box<Expr>, Box<Expr>),
    BinOp(Box<Expr>, Infix, Box<Expr>),
    UnOp(Prefix, Box<Expr>),
    Pair(Box<Expr>, Box<Expr>),
}

#[derive(Debug, PartialEq)]
pub enum TypedExpr {
    Num(i32),
    Bool(bool),
    Unit,
    Var {
        id: Id,
        typ: AlfaType,
    },
    Fun {
        id: Id,
        body: Box<TypedExpr>,
        typ: AlfaType,
    },
    If {
        cond: Box<TypedExpr>,
        if_body: Box<TypedExpr>,
        else_body: Box<TypedExpr>,
        typ: AlfaType,
    },
    Case {
        cond: Box<TypedExpr>,
        l_var: Id,
        l_body: Box<TypedExpr>,
        r_var: Id,
        r_body: Box<TypedExpr>,
        typ: AlfaType,
    },
    InjL {
        expr: Box<TypedExpr>,
        typ: AlfaType,
    },
    InjR {
        expr: Box<TypedExpr>,
        typ: AlfaType,
    },
    PrjL {
        expr: Box<TypedExpr>,
        typ: AlfaType,
    },
    PrjR {
        expr: Box<TypedExpr>,
        typ: AlfaType,
    },
    Let {
        id: Id,
        def: Box<TypedExpr>,
        body: Box<TypedExpr>,
        typ: AlfaType,
    },
    Ap {
        fun: Box<TypedExpr>,
        arg: Box<TypedExpr>,
        typ: AlfaType,
    },
    BinOp {
        lhs: Box<TypedExpr>,
        op: Infix,
        rhs: Box<TypedExpr>,
        typ: AlfaType,
    },
    UnOp {
        op: Prefix,
        expr: Box<TypedExpr>,
        typ: AlfaType,
    },
    Pair {
        fst: Box<TypedExpr>,
        snd: Box<TypedExpr>,
        typ: AlfaType,
    },
}

// Helper functions for constructing these things nicely
// Don't worry, not written by hand :)
pub fn num(n: i32) -> TypedExpr {
    TypedExpr::Num(n)
}

pub fn bool(b: bool) -> TypedExpr {
    TypedExpr::Bool(b)
}

pub fn unit() -> TypedExpr {
    TypedExpr::Unit
}

pub fn var(id: Id, typ: AlfaType) -> TypedExpr {
    TypedExpr::Var { id, typ }
}

pub fn fun(id: Id, body: TypedExpr, typ: AlfaType) -> TypedExpr {
    TypedExpr::Fun {
        id,
        body: Box::new(body),
        typ,
    }
}

pub fn if_expr(
    cond: TypedExpr,
    if_body: TypedExpr,
    else_body: TypedExpr,
    typ: AlfaType,
) -> TypedExpr {
    TypedExpr::If {
        cond: Box::new(cond),
        if_body: Box::new(if_body),
        else_body: Box::new(else_body),
        typ,
    }
}

pub fn case(
    cond: TypedExpr,
    l_var: Id,
    l_body: TypedExpr,
    r_var: Id,
    r_body: TypedExpr,
    typ: AlfaType,
) -> TypedExpr {
    TypedExpr::Case {
        cond: Box::new(cond),
        l_var,
        l_body: Box::new(l_body),
        r_var,
        r_body: Box::new(r_body),
        typ,
    }
}

pub fn injl(expr: TypedExpr, typ: AlfaType) -> TypedExpr {
    TypedExpr::InjL {
        expr: Box::new(expr),
        typ,
    }
}

pub fn injr(expr: TypedExpr, typ: AlfaType) -> TypedExpr {
    TypedExpr::InjR {
        expr: Box::new(expr),
        typ,
    }
}

pub fn prjl(expr: TypedExpr, typ: AlfaType) -> TypedExpr {
    TypedExpr::PrjL {
        expr: Box::new(expr),
        typ,
    }
}

pub fn prjr(expr: TypedExpr, typ: AlfaType) -> TypedExpr {
    TypedExpr::PrjR {
        expr: Box::new(expr),
        typ,
    }
}

pub fn let_expr(id: Id, def: TypedExpr, body: TypedExpr, typ: AlfaType) -> TypedExpr {
    TypedExpr::Let {
        id,
        def: Box::new(def),
        body: Box::new(body),
        typ,
    }
}

pub fn ap(fun: TypedExpr, arg: TypedExpr, typ: AlfaType) -> TypedExpr {
    TypedExpr::Ap {
        fun: Box::new(fun),
        arg: Box::new(arg),
        typ,
    }
}

pub fn binop(lhs: TypedExpr, op: Infix, rhs: TypedExpr, typ: AlfaType) -> TypedExpr {
    TypedExpr::BinOp {
        lhs: Box::new(lhs),
        op,
        rhs: Box::new(rhs),
        typ,
    }
}

pub fn unop(op: Prefix, expr: TypedExpr, typ: AlfaType) -> TypedExpr {
    TypedExpr::UnOp {
        op,
        expr: Box::new(expr),
        typ,
    }
}

pub fn pair(fst: TypedExpr, snd: TypedExpr, typ: AlfaType) -> TypedExpr {
    TypedExpr::Pair {
        fst: Box::new(fst),
        snd: Box::new(snd),
        typ,
    }
}

impl KnownType for TypedExpr {
    fn get_type(&self) -> &AlfaType {
        use TypedExpr::*;
        match self {
            Num(_) => &AlfaType::Num,
            Bool(_) => &AlfaType::Bool,
            Unit => &AlfaType::Unit,
            Var { typ: t, .. } => t,
            Fun { typ: t, .. } => t,
            If { typ: t, .. } => t,
            Case { typ: t, .. } => t,
            InjL { typ: t, .. } => t,
            InjR { typ: t, .. } => t,
            PrjL { typ: t, .. } => t,
            PrjR { typ: t, .. } => t,
            Let { typ: t, .. } => t,
            Ap { typ: t, .. } => t,
            BinOp { typ: t, .. } => t,
            UnOp { typ: t, .. } => t,
            Pair { typ: t, .. } => t,
        }
    }
}
