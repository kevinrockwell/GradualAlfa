#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum AlfaType {
    Num,
    Bool,
    Unit,
    Arrow(Box<AlfaType>, Box<AlfaType>),
    Product(Box<AlfaType>, Box<AlfaType>),
    Sum(Box<AlfaType>, Box<AlfaType>),
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
#[derive(Debug)]
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
