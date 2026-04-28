#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum AlfaType {
    Num,
    Bool,
    Arrow(Box<AlfaType>, Box<AlfaType>),
    Product(Box<AlfaType>, Box<AlfaType>),
    Sum(Box<AlfaType>, Box<AlfaType>),
}

#[derive(Debug, PartialEq)]
pub enum BinOp {
    Plus,
    Minus,
    Times,
    LessThan,
    GreaterThan,
    EqualTo,
}

#[derive(Debug, PartialEq)]
pub enum UnOp {
    Neg,
}

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
    Let(Id, Box<Expr>),
    Ap(Box<Expr>, Box<Expr>),
    BinaryExpr(Box<Expr>, BinOp, Box<Expr>),
    UnaryExpr(UnOp, Box<Expr>),
    Pair(Box<Expr>, Box<Expr>),
}
