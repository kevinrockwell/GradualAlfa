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

        pub fn lookup(&self, id: &str) -> Option<&AlfaType> {
            for e in self.vec.iter().rev() {
                if id == e.id {
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
    mod tests {
        use super::*;
        #[test]
        fn test_lookup() {
            use AlfaType::*;
            assert_eq!(Context::new().lookup("k"), None);
            let ctx = Context::new_from(vec![
                ContextEntry { id: "k", typ: Bool },
                ContextEntry { id: "t", typ: Unit },
                ContextEntry { id: "k", typ: Num },
            ]);
            assert_eq!(ctx.lookup("k"), Some(&Num));
            assert_eq!(ctx.lookup("t"), Some(&Unit));
            assert_eq!(ctx.lookup("z"), None);
        }

        #[test]
        fn test_update() {
            use AlfaType::*;
            let ctx = Context::new_from(vec![
                ContextEntry { id: "k", typ: Bool },
                ContextEntry { id: "t", typ: Unit },
            ]);
            assert_eq!(ctx.update("k", &Num).lookup("k"), Some(&Num));
            assert_eq!(ctx.update("t", &Bool).lookup("k"), Some(&Bool));
            assert_eq!(ctx.update("j", &Bool).lookup("j"), Some(&Bool));
            assert_eq!(ctx.update("j", &Bool).lookup("j"), Some(&Bool));
        }
    } // mod test
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
            let typ = match ctx.lookup(&id.id) {
                Some(t) => t.clone(),
                // TODO: make nicer error message
                None => return Err(format!("Unknown variable: ``{}''", id.id)),
            };
            Ok(TypedExpr::Var { id, typ })
        }
        Expr::Fun(mut id, body) => {
            // Get the type of the body, adding the variable to the context
            use AlfaType::Dyn;
            let body =
                typecheck_helper(*body, ctx.update(&id.id, id.typ.as_ref().unwrap_or(&Dyn)))?;
            let typ = arrow(id.typ.take().unwrap_or(Dyn), body.get_type().clone());
            Ok(fun(id, body, typ))
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
            Ok(if_expr(cond, if_body, else_body, typ))
        }
        Expr::Case(cond, l_var, l_body, r_var, r_body) => {
            use AlfaType::{Dyn, Sum};
            let cond = typecheck_helper(*cond, ctx.clone())?;
            if !cond.consistent(&sum(Dyn, Dyn)) {
                return Err(format!(
                    "case condition must be consistent with Sum type, is {:?}",
                    cond
                ));
            }
            // If cond is a sum type, the type of x in L(x) is the first entry
            // of the sum. Otherwise, cond is ? and so x should also be type ?
            let l_typ = if let Sum(l, _) = cond.get_type() {
                l.as_ref().clone()
            } else {
                Dyn
            };
            let l_body = typecheck_helper(*l_body, ctx.update(l_var.id.as_str(), &l_typ))?;
            // Same thing for R(x)
            let r_typ = if let Sum(l, _) = cond.get_type() {
                l.as_ref().clone()
            } else {
                Dyn
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
            Ok(case(cond, l_var, l_body, r_var, r_body, typ))
        }
        Expr::InjL(expr) => {
            let expr = typecheck_helper(*expr, ctx.clone())?;
            // This is actually easier to reason about than regular ALFA. We
            // know this must be part of a Sum type, and we cannot deduce from
            // this expression the other side of the Sum type, so we just let
            // it be ?.
            let typ = AlfaType::Sum(Box::new(expr.get_type().clone()), Box::new(AlfaType::Dyn));
            Ok(injl(expr, typ))
        }
        Expr::InjR(expr) => {
            let expr = typecheck_helper(*expr, ctx.clone())?;
            // Same commentary as before regarding the type of the expression
            let typ = AlfaType::Sum(Box::new(expr.get_type().clone()), Box::new(AlfaType::Dyn));
            Ok(injr(expr, typ))
        }
        Expr::PrjL(expr) => {
            use AlfaType::{Dyn, Product};
            let expr = typecheck_helper(*expr, ctx.clone())?;
            if !expr.consistent(&product(Dyn, Dyn)) {
                return Err(format!(
                    ".fst must operate on something consistent with Product typ, instead has {:?}",
                    expr
                ));
            }
            let typ = if let Product(fst, _) = expr.get_type() {
                fst.as_ref().clone()
            } else {
                AlfaType::Dyn
            };
            Ok(prjl(expr, typ))
        }
        Expr::PrjR(expr) => {
            use AlfaType::{Dyn, Product};
            let expr = typecheck_helper(*expr, ctx.clone())?;
            if !expr.consistent(&product(Dyn, Dyn)) {
                return Err(format!(
                    ".snd must operate on something consistent with Product typ, instead has {:?}",
                    expr
                ));
            }
            let typ = if let Product(_, snd) = expr.get_type() {
                snd.as_ref().clone()
            } else {
                AlfaType::Dyn
            };
            Ok(prjr(expr, typ))
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
            Ok(let_expr(id, def, body, typ))
        }
        Expr::Ap(fun, arg) => {
            use AlfaType::{Arrow, Dyn};
            let fun = typecheck_helper(*fun, ctx.clone())?;
            if !fun.consistent(&arrow(Dyn, Dyn)) {
                return Err(format!(
                    "LHS of Application ({:?}) must be consistent with function type",
                    fun
                ));
            }
            // If `fun` is not an Arrow type, it is Dyn, so its return type
            // will also be Dyn
            let typ = if let Arrow(_, ret) = fun.get_type() {
                ret.as_ref().clone()
            } else {
                Dyn
            };
            let arg = typecheck_helper(*arg, ctx.clone())?;
            Ok(ap(fun, arg, typ))
        }
        Expr::BinOp(lhs, op, rhs) => {
            use Infix::*;
            let typ = match op {
                Plus | Minus | Times => AlfaType::Num,
                LessThan | GreaterThan | EqualTo => AlfaType::Bool,
            };
            let lhs = typecheck_helper(*lhs, ctx.clone())?;
            let rhs = typecheck_helper(*rhs, ctx)?;
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
            Ok(binop(lhs, op, rhs, typ))
        }
        Expr::UnOp(op, expr) => {
            use Prefix::*;
            let typ = match op {
                Neg => AlfaType::Num,
            };
            let expr = typecheck_helper(*expr, ctx)?;
            Ok(unop(op, expr, typ))
        }
        Expr::Pair(fst, snd) => {
            let fst = typecheck_helper(*fst, ctx.clone())?;
            let snd = typecheck_helper(*snd, ctx)?;
            let typ = product(fst.get_type().clone(), snd.get_type().clone());
            Ok(pair(fst, snd, typ))
        }
    }
}

pub fn typecheck(expr: Expr) -> TypeCheckResult {
    typecheck_helper(expr, Context::new())
}

mod tests {
    use super::*;
    use crate::parser::parse_alfa_program;

    #[test]
    fn test_consistency() {
        use AlfaType::*;
        let t = arrow(Num, Unit);
        assert!(t.consistent(&Dyn));
        assert!(t.consistent(&arrow(Dyn, Dyn)));
        assert!(t.consistent(&arrow(Num, Dyn)));
        assert!(t.consistent(&arrow(Dyn, Unit)));
        assert!(t.consistent(&arrow(Num, Unit)));
        assert!(!t.consistent(&Num));
        assert!(!t.consistent(&arrow(Num, Num)));
        assert!(!t.consistent(&sum(Num, Unit)));
    }

    fn empty_id(name: &str) -> Id {
        Id {
            id: name.to_string(),
            typ: None,
        }
    }

    #[test]
    fn test_fun() {
        use AlfaType::*;
        let p = parse_alfa_program("fun x -> x").unwrap();
        assert_eq!(
            typecheck(p).unwrap(),
            fun(empty_id("x"), var(empty_id("x"), Dyn), arrow(Dyn, Dyn))
        );
    }
}
