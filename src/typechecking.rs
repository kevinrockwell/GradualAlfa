use crate::ast::*;
use std::{fmt::format, vec::Vec};

type TypeCheckResult = Result<TypedExpr, String>;

mod context {
    use super::*;

    #[derive(Debug, Clone)]
    struct ContextEntry<'a> {
        id: &'a str,
        typ: AlfaType,
    }

    #[derive(Debug, Clone)]
    pub struct Context<'a> {
        vec: Vec<ContextEntry<'a>>,
    }

    impl<'a> Context<'a> {
        pub fn new() -> Self {
            return Context { vec: vec![] };
        }

        fn new_from(vec: Vec<ContextEntry>) -> Context {
            Context { vec }
        }

        pub fn lookup(&self, id: &Id) -> Option<&AlfaType> {
            for e in self.vec.iter().rev() {
                if id.id == e.id {
                    return Some(&e.typ);
                }
            }
            None
        }

        pub fn update(&self, id: &'a str, typ: &AlfaType) -> Self {
            let mut res = self.vec.clone();
            res.push(ContextEntry {
                id,
                typ: typ.clone(),
            });
            Context::new_from(res)
        }
    } // impl Context
} // mod context

use context::Context;

trait Consistency {
    fn consistent(&self, other: &impl KnownType) -> bool;
}

impl Consistency for AlfaType {
    fn consistent(&self, other: &impl KnownType) -> bool {
        use AlfaType::*;
        match (self, other.get_type()) {
            (Dyn, _) => true,
            (_, Dyn) => true,
            (Num, Num) => true,
            (Bool, Bool) => true,
            (Unit, Unit) => true,
            // ``Interesting'' composite types
            (Arrow(s1, s2), Arrow(o1, o2)) => {
                s1.consistent(o1.as_ref()) && s2.consistent(o2.as_ref())
            }
            (Product(s1, s2), Product(o1, o2)) => {
                s1.consistent(o1.as_ref()) && s2.consistent(o2.as_ref())
            }
            (Sum(s1, s2), Sum(o1, o2)) => s1.consistent(o1.as_ref()) && s2.consistent(o2.as_ref()),
            _ => false,
        }
    }
}

impl Consistency for TypedExpr {
    fn consistent(&self, other: &impl KnownType) -> bool {
        self.get_type().consistent(other.get_type())
    }
}

// This entire implementation is kinda gross, from the persistent cloning of
// boxed types to the context management. Sorry!
fn typecheck_helper(expr: Expr, ctx: Context) -> TypeCheckResult {
    match expr {
        Expr::Num(n) => Ok(TypedExpr::Num(n)),
        Expr::Bool(b) => Ok(TypedExpr::Bool(b)),
        Expr::Unit => Ok(TypedExpr::Unit),
        // TODO: why did i put the types in the IDs like this aghghghghghghghhg
        // it makes no sense frfr.
        Expr::Var(id) => {
            let typ = match ctx.lookup(&id) {
                Some(t) => t.clone(),
                // TODO: make nicer error message
                None => return Err(format!("Unknown variable: ``{}''", id.id)),
            };
            Ok(TypedExpr::Var { id, typ })
        }
        Expr::Fun(mut id, body) => {
            // Get the type of the body, adding the variable to the context
            let typed_body = typecheck_helper(
                *body,
                // TODO: this seems unnecessary: ctx should be able to accept &str
                ctx.update(id.id.as_str(), id.typ.as_ref().unwrap_or(&AlfaType::Dyn)),
            )?;
            let fun_typ = AlfaType::Arrow(
                Box::new(id.typ.take().unwrap_or(AlfaType::Dyn)),
                Box::new(typed_body.get_type().clone()),
            );
            Ok(TypedExpr::Fun {
                id,
                body: Box::new(typed_body),
                typ: fun_typ,
            })
        }
        Expr::If(cond, if_body, else_body) => {
            let cond = typecheck_helper(*cond, ctx.clone())?;
            if !cond.consistent(&AlfaType::Bool) {
                return Err(format!(
                    "Condition for if ({:?}) must be consistent with bool",
                    cond
                ));
            }
            let if_body = typecheck_helper(*if_body, ctx.clone())?;
            let else_body = typecheck_helper(*else_body, ctx.clone())?;
            if !if_body.consistent(&else_body) {
                return Err(format!(
                    "if branches must have consistent types. {:?} and {:?} are not consistent.",
                    if_body, else_body
                ));
            }
            // TODO this should be the most general type we can get from the two!!!
            let typ = if_body.get_type().clone();
            Ok(TypedExpr::If {
                cond: Box::new(cond),
                if_body: Box::new(if_body),
                else_body: Box::new(else_body),
                typ,
            })
        }
        Expr::Case(cond, l_var, l_body, r_var, r_body) => {
            let cond = typecheck_helper(*cond, ctx.clone())?;
            if !cond.consistent(&AlfaType::Sum(
                Box::new(AlfaType::Dyn),
                Box::new(AlfaType::Dyn),
            )) {
                return Err(format!(
                    "case condition must be consistent with Sum type, is {:?}",
                    cond
                ));
            }
            // If cond is a sum type, the type of x in L(x) is the first entry
            // of the sum. Otherwise, cond is ? and so x should also be type ?
            let l_typ = if let AlfaType::Sum(l, _) = cond.get_type() {
                l.as_ref().clone()
            } else {
                AlfaType::Dyn
            };
            let l_body = typecheck_helper(*l_body, ctx.update(l_var.id.as_str(), &l_typ))?;
            // Same thing for R(x)
            let r_typ = if let AlfaType::Sum(l, _) = cond.get_type() {
                l.as_ref().clone()
            } else {
                AlfaType::Dyn
            };
            let r_body = typecheck_helper(*r_body, ctx.update(r_var.id.as_str(), &r_typ))?;
            if !l_body.consistent(&r_body) {
                return Err(format!(
                    "if branches must have consistent types. {:?} and {:?} are not consistent.",
                    l_body, r_body
                ));
            }
            // TODO this should be the most general type we can get from the two!!!
            let typ = l_body.get_type().clone();
            Ok(TypedExpr::Case {
                cond: Box::new(cond),
                l_var,
                l_body: Box::new(l_body),
                r_var,
                r_body: Box::new(r_body),
                typ,
            })
        }
        Expr::InjL(expr) => {
            let expr = typecheck_helper(*expr, ctx.clone())?;
            // This is actually easier to reason about than regular ALFA. We
            // know this must be part of a Sum type, and we cannot deduce from
            // this expression the other side of the Sum type, so we just let
            // it be ?.
            let typ = AlfaType::Sum(Box::new(expr.get_type().clone()), Box::new(AlfaType::Dyn));
            Ok(TypedExpr::InjL {
                expr: Box::new(expr),
                typ,
            })
        }
        Expr::InjR(expr) => {
            let expr = typecheck_helper(*expr, ctx.clone())?;
            // Same commentary as before regarding the type of the expression
            let typ = AlfaType::Sum(Box::new(expr.get_type().clone()), Box::new(AlfaType::Dyn));
            Ok(TypedExpr::InjR {
                expr: Box::new(expr),
                typ,
            })
        }
        Expr::PrjL(expr) => {
            let expr = typecheck_helper(*expr, ctx.clone())?;
            if !expr.consistent(&AlfaType::Product(
                Box::new(AlfaType::Dyn),
                Box::new(AlfaType::Dyn),
            )) {
                return Err(format!(
                    ".fst must operate on something consistent with Product typ, instead has {:?}",
                    expr
                ));
            }
            let typ = if let AlfaType::Product(fst, _) = expr.get_type() {
                fst.as_ref().clone()
            } else {
                AlfaType::Dyn
            };
            Ok(TypedExpr::PrjL {
                expr: Box::new(expr),
                typ,
            })
        }
        Expr::PrjR(expr) => {
            let expr = typecheck_helper(*expr, ctx.clone())?;
            if !expr.consistent(&AlfaType::Product(
                Box::new(AlfaType::Dyn),
                Box::new(AlfaType::Dyn),
            )) {
                return Err(format!(
                    ".snd must operate on something consistent with Product typ, instead has {:?}",
                    expr
                ));
            }
            let typ = if let AlfaType::Product(_, snd) = expr.get_type() {
                snd.as_ref().clone()
            } else {
                AlfaType::Dyn
            };
            Ok(TypedExpr::PrjR {
                expr: Box::new(expr),
                typ,
            })
        }
        // Its astounding that I thought putting variable declaration types
        // in the Id struct was a good idea. It causes me pain each time
        Expr::Let(mut id, def, body) => {
            let def = typecheck_helper(*def, ctx.clone())?;
            let var_typ = if let Some(typ) = id.typ.take() {
                if !typ.consistent(&def) {
                    return Err(format!(
                        "Variable definition {:?} must be consistent with annotated type {:?}",
                        def, typ
                    ));
                }
                // TODO this should be the most general type we can get from the two!!!
                def.get_type().clone()
            } else {
                def.get_type().clone()
            };
            let body = typecheck_helper(*body, ctx.update(&id.id, &var_typ))?;
            let typ = body.get_type().clone();
            Ok(TypedExpr::Let {
                id,
                def: Box::new(def),
                body: Box::new(body),
                typ,
            })
        }
        Expr::Ap(fun, arg) => {
            use AlfaType::{Arrow, Dyn};
            let fun = typecheck_helper(*fun, ctx.clone())?;
            if !fun.consistent(&Arrow(Box::new(Dyn), Box::new(Dyn))) {
                return Err(format!(
                    "LHS of Application ({:?}) must be consistent with function type",
                    fun
                ));
            }
            let typ = if let Arrow(_, ret) = fun.get_type() {
                ret.as_ref().clone()
            } else {
                Dyn
            };
            let arg = typecheck_helper(*arg, ctx.clone())?;
            Ok(TypedExpr::Ap {
                fun: Box::new(fun),
                arg: Box::new(arg),
                typ,
            })
        }
        Expr::BinOp(lhs, op, rhs) => {
            use Infix::*;
            let typ = match op {
                Plus | Minus | Times => AlfaType::Num,
                LessThan | GreaterThan | EqualTo => AlfaType::Bool,
            };
            let lhs = typecheck_helper(*lhs, ctx.clone())?;
            let rhs = typecheck_helper(*rhs, ctx.clone())?;
            if !lhs.consistent(&AlfaType::Num) {
                return Err(format!(
                    "Binary Expression argument ({:?}) must be consistent with Num",
                    lhs
                ));
            } else if !rhs.consistent(&AlfaType::Num) {
                return Err(format!(
                    "Binary Expression argument ({:?}) must be consistent with Num",
                    rhs
                ));
            }
            Ok(TypedExpr::BinOp {
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
                typ,
            })
        }
        Expr::UnOp(op, expr) => {
            use Prefix::*;
            let typ = match op {
                Neg => AlfaType::Num,
            };
            let expr = typecheck_helper(*expr, ctx.clone())?;
            Ok(TypedExpr::UnOp {
                op,
                expr: Box::new(expr),
                typ,
            })
        }
        Expr::Pair(fst, snd) => {
            let fst = typecheck_helper(*fst, ctx.clone())?;
            let snd = typecheck_helper(*snd, ctx.clone())?;
            let typ = AlfaType::Product(
                Box::new(fst.get_type().clone()),
                Box::new(snd.get_type().clone()),
            );
            Ok(TypedExpr::Pair {
                fst: Box::new(fst),
                snd: Box::new(snd),
                typ,
            })
        }
    }
}

pub fn typecheck(expr: Expr) -> TypeCheckResult {
    typecheck_helper(expr, Context::new())
}
