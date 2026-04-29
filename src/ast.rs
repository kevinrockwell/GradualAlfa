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

