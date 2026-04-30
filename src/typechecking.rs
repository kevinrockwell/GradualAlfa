use crate::ast::*;
use std::vec::Vec;

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

pub trait Consistency {
    fn consistent(&self, other: &impl KnownType) -> bool;
    fn most_typed_consistent(&self, other: &impl KnownType) -> AlfaType;
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

    fn most_typed_consistent(&self, other: &impl KnownType) -> AlfaType {
        use AlfaType::*;
        assert!(self.consistent(other));
        match (self.get_type(), other.get_type()) {
            (Dyn, _) => Dyn,
            (_, Dyn) => Dyn,
            (Num, Num) => Num,
            (Bool, Bool) => Bool,
            (Unit, Unit) => Unit,
            // ``Interesting'' composite types
            (Arrow(s1, s2), Arrow(o1, o2)) => arrow(
                s1.most_typed_consistent(o1.as_ref()),
                s2.most_typed_consistent(o2.as_ref()),
            ),
            (Product(s1, s2), Product(o1, o2)) => product(
                s1.most_typed_consistent(o1.as_ref()),
                s2.most_typed_consistent(o2.as_ref()),
            ),
            (Sum(s1, s2), Sum(o1, o2)) => sum(
                s1.most_typed_consistent(o1.as_ref()),
                s2.most_typed_consistent(o2.as_ref()),
            ),
            _ => unreachable!(),
        }
    }
}

impl Consistency for TypedExpr {
    fn consistent(&self, other: &impl KnownType) -> bool {
        self.get_type().consistent(other.get_type())
    }
    fn most_typed_consistent(&self, other: &impl KnownType) -> AlfaType {
        self.get_type().most_typed_consistent(other)
    }
}

// This entire implementation is kinda gross, from the persistent cloning of
// boxed types to the context management. Sorry!
fn typecheck_helper(expr: Expr, ctx: Context) -> TypeCheckResult {
    match expr {
        Expr::Num(n) => Ok(TypedExpr::Num(n)),
        Expr::Bool(b) => Ok(TypedExpr::Bool(b)),
        Expr::Unit => Ok(TypedExpr::Unit),
        Expr::Var(id) => {
            let typ = match ctx.lookup(&id.id) {
                Some(t) => t.clone(),
                // TODO: make nicer error message
                None => return Err(format!("Unknown variable: ``{}''", id.id)),
            };
            Ok(TypedExpr::Var { id, typ })
        }
        Expr::Fun(id, arg_typ, body) => {
            // Get the type of the body, adding the variable to the context
            use AlfaType::Dyn;
            let arg_typ = arg_typ.unwrap_or(Dyn);
            let body = typecheck_helper(*body, ctx.update(&id.id, &arg_typ))?;
            let typ = arrow(arg_typ.clone(), body.get_type().clone());
            Ok(fun(&id.id, arg_typ, body, typ))
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
            let typ = if_body.most_typed_consistent(&else_body);
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
            let r_typ = if let Sum(_, r) = cond.get_type() {
                r.as_ref().clone()
            } else {
                Dyn
            };
            let r_body = typecheck_helper(*r_body, ctx.update(r_var.id.as_str(), &r_typ))?;
            if !l_body.consistent(&r_body) {
                return Err(format!(
                    "case branches must have consistent types. {:?} and {:?} are not consistent.",
                    l_body, r_body
                ));
            }
            let typ = l_body.most_typed_consistent(&r_body);
            Ok(case(cond, &l_var.id, l_body, &r_var.id, r_body, typ))
        }
        Expr::InjL(expr) => {
            let expr = typecheck_helper(*expr, ctx.clone())?;
            // This is actually easier to reason about than regular ALFA. We
            // know this must be part of a Sum type, and we cannot deduce from
            // this expression the other side of the Sum type, so we just let
            // it be ?.
            let typ = sum(expr.get_type().clone(), AlfaType::Dyn);
            Ok(injl(expr, typ))
        }
        Expr::InjR(expr) => {
            let expr = typecheck_helper(*expr, ctx.clone())?;
            // Same commentary as before regarding the type of the expression
            let typ = sum(AlfaType::Dyn, expr.get_type().clone());
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
        Expr::Let(id, arg_typ, def, body) => {
            let def = typecheck_helper(*def, ctx.clone())?;
            // If type annotation is given and consistent with the defintion
            // type, then use it. If inconsistent, type error.
            // If no type annotation, use what we know about the defintion
            let var_typ = if let Some(typ) = arg_typ {
                if !typ.consistent(&def) {
                    return Err(format!(
                        "Variable definition {:?} must be consistent with annotated type {:?}",
                        def, typ
                    ));
                }
                typ
            } else {
                def.get_type().clone()
            };
            let body = typecheck_helper(*body, ctx.update(&id.id, &var_typ))?;
            let typ = body.get_type().clone();
            Ok(let_expr(&id.id, var_typ, def, body, typ))
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
            // This is essentially the `fun` cast of Seik et al.
            let (arg_typ, ret_typ) = if let Arrow(arg, ret) = fun.get_type() {
                (arg.as_ref().clone(), ret.as_ref().clone())
            } else {
                (Dyn, Dyn)
            };
            let arg = typecheck_helper(*arg, ctx.clone())?;
            if !arg.consistent(&arg_typ) {
                return Err(format!(
                    "Argument {:?} to function {:?} inconsistent",
                    arg, fun
                ));
            }
            Ok(ap(fun, arg, ret_typ))
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
            if !expr.consistent(&typ) {
                return Err(format!(
                    "Unary Expression argument ({:?}) must be consistent with Num",
                    expr
                ));
            }
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
    use AlfaType::*;

    #[test]
    fn test_consistency() {
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

    #[test]
    fn test_most_typed_consistent() {
        let t1 = arrow(Num, Dyn);
        assert_eq!(t1.most_typed_consistent(&Dyn), Dyn);
        assert_eq!(t1.most_typed_consistent(&arrow(Num, Dyn)), t1);
        assert_eq!(t1.most_typed_consistent(&arrow(Dyn, Dyn)), arrow(Dyn, Dyn));
        let t1 = sum(Num, Dyn);
        assert_eq!(t1.most_typed_consistent(&Dyn), Dyn);
        assert_eq!(t1.most_typed_consistent(&sum(Num, Dyn)), t1);
        assert_eq!(t1.most_typed_consistent(&sum(Dyn, Dyn)), sum(Dyn, Dyn));
        let t1 = product(Num, Dyn);
        assert_eq!(t1.most_typed_consistent(&Dyn), Dyn);
        assert_eq!(t1.most_typed_consistent(&product(Num, Dyn)), t1);
        assert_eq!(
            t1.most_typed_consistent(&product(Dyn, Dyn)),
            product(Dyn, Dyn)
        );
    }

    #[test]
    fn test_basic_values() {
        let p = parse_alfa_program("x").unwrap();
        let typed = typecheck(p);
        assert_eq!(typed, Err("Unknown variable: ``x''".to_string()));
        let p = parse_alfa_program("1").unwrap();
        let typed = typecheck(p);
        assert_eq!(typed, Ok(num(1)));
        assert_eq!(typed.unwrap().get_type(), &AlfaType::Num);
        let p = parse_alfa_program("true").unwrap();
        let typed = typecheck(p);
        assert_eq!(typed, Ok(bool_lit(true)));
        assert_eq!(typed.unwrap().get_type(), &AlfaType::Bool);
        let p = parse_alfa_program("()").unwrap();
        let typed = typecheck(p);
        assert_eq!(typed, Ok(unit()));
        assert_eq!(typed.unwrap().get_type(), &AlfaType::Unit);
    }

    #[test]
    fn test_fun() {
        let p = parse_alfa_program("fun x -> x").unwrap();
        assert_eq!(
            typecheck(p).unwrap(),
            fun("x", Dyn, var("x", Dyn), arrow(Dyn, Dyn))
        );
        let p = parse_alfa_program("fun (x: Num) -> x").unwrap();
        assert_eq!(
            typecheck(p).unwrap(),
            fun("x", Num, var("x", Num), arrow(Num, Num))
        );
    }

    #[test]
    fn test_if() {
        let p = parse_alfa_program("if 1 < 2 then 3 else 4").unwrap();
        assert_eq!(
            typecheck(p).unwrap(),
            if_expr(
                binop(num(1), Infix::LessThan, num(2), Bool),
                num(3),
                num(4),
                Num
            )
        );
        let p = parse_alfa_program("fun x -> (if x > 0 then x else 1)").unwrap();
        assert_eq!(
            typecheck(p).unwrap(),
            fun(
                "x",
                Dyn,
                if_expr(
                    binop(var("x", Dyn), Infix::GreaterThan, num(0), Bool),
                    var("x", Dyn),
                    num(1),
                    Dyn,
                ),
                arrow(Dyn, Dyn)
            )
        );
        // Check that consistency is properly handled for both branches
        let ctx = Context::new().update("x", &Dyn);
        let p = parse_alfa_program("if x > 0 then fun x -> x else fun (x: Num) -> x").unwrap();
        assert_eq!(
            typecheck_helper(p, ctx.clone()),
            Ok(if_expr(
                binop(var("x", Dyn), Infix::GreaterThan, num(0), Bool),
                fun("x", Dyn, var("x", Dyn), arrow(Dyn, Dyn)),
                fun("x", Num, var("x", Num), arrow(Num, Num)),
                arrow(Dyn, Dyn)
            ))
        );
        let p = parse_alfa_program("if x > 0 then fun (x: Num) -> x else fun x -> x").unwrap();
        assert_eq!(
            typecheck_helper(p, ctx.clone()),
            Ok(if_expr(
                binop(var("x", Dyn), Infix::GreaterThan, num(0), Bool),
                fun("x", Num, var("x", Num), arrow(Num, Num)),
                fun("x", Dyn, var("x", Dyn), arrow(Dyn, Dyn)),
                arrow(Dyn, Dyn)
            ))
        );
        let p = parse_alfa_program("if 1 then 2 else 3").unwrap();
        assert_eq!(
            typecheck(p),
            Err("Condition for if (Num(1)) must be consistent with bool".to_string())
        );
        let p = parse_alfa_program("if true then (1,x) else (2,3)").unwrap();
        assert_eq!(
            typecheck_helper(p, ctx).unwrap(),
            if_expr(
                bool_lit(true),
                pair(num(1), var("x", Dyn), product(Num, Dyn)),
                pair(num(2), num(3), product(Num, Num)),
                product(Num, Dyn)
            )
        );
    }

    #[test]
    fn test_case() {
        let p = parse_alfa_program("case L () of L(x) -> x else R(y) -> y").unwrap();
        assert_eq!(
            typecheck(p).unwrap(),
            case(
                injl(unit(), sum(Unit, Dyn)),
                "x",
                var("x", Unit),
                "y",
                var("y", Dyn),
                Dyn,
            )
        );
        // Test
        let ctx = Context::new().update("x", &Dyn);
        let p = parse_alfa_program("case x of L(x) -> x else R(y) -> fun z -> y").unwrap();
        assert_eq!(
            typecheck_helper(p, ctx.clone()),
            Ok(case(
                var("x", Dyn),
                "x",
                var("x", Dyn),
                "y",
                fun("z", Dyn, var("y", Dyn), arrow(Dyn, Dyn)),
                Dyn
            ))
        );
        let ctx = Context::new().update("x", &sum(Num, Dyn));
        let p = parse_alfa_program("case x of L(x) -> x + 1 else R(y) -> y").unwrap();
        assert_eq!(
            typecheck_helper(p, ctx.clone()),
            Ok(case(
                var("x", sum(Num, Dyn)),
                "x",
                binop(var("x", Num), Infix::Plus, num(1), Num),
                "y",
                var("y", Dyn),
                Dyn
            ))
        );
        let p = parse_alfa_program("case 1 of L(x) -> x else R(y) -> y + 1").unwrap();
        assert_eq!(
            typecheck(p),
            Err("case condition must be consistent with Sum type, is Num(1)".to_string())
        );
        let p = parse_alfa_program("case x of L(x) -> 1 else R(y) -> false").unwrap();
        assert_eq!(
            typecheck_helper(p, ctx),
            Err("case branches must have consistent types. Num(1) and Bool(false) are not consistent.".to_string())
        );
    }

    #[test]
    fn test_inj() {
        let p = parse_alfa_program("L(1)").unwrap();
        assert_eq!(typecheck(p).unwrap(), injl(num(1), sum(Num, Dyn)));
        let p = parse_alfa_program("R(1)").unwrap();
        assert_eq!(typecheck(p).unwrap(), injr(num(1), sum(Dyn, Num)));
        let p = parse_alfa_program("L R L ()").unwrap();
        assert_eq!(
            typecheck(p).unwrap(),
            injl(
                injr(injl(unit(), sum(Unit, Dyn)), sum(Dyn, sum(Unit, Dyn))),
                sum(sum(Dyn, sum(Unit, Dyn)), Dyn)
            )
        );
    }

    #[test]
    fn test_prj() {
        let ctx = Context::new().update("x", &product(Bool, Num));
        let p = parse_alfa_program("x.fst").unwrap();
        assert_eq!(
            typecheck_helper(p, ctx.clone()).unwrap(),
            prjl(var("x", product(Bool, Num)), Bool)
        );
        let p = parse_alfa_program("x.snd").unwrap();
        assert_eq!(
            typecheck_helper(p, ctx.clone()).unwrap(),
            prjr(var("x", product(Bool, Num)), Num)
        );
        let p = parse_alfa_program("fun x -> x.fst").unwrap();
        assert_eq!(
            typecheck_helper(p, ctx.clone()).unwrap(),
            fun("x", Dyn, prjl(var("x", Dyn), Dyn), arrow(Dyn, Dyn))
        );
    }

    #[test]
    fn test_let() {
        let p = parse_alfa_program("let x be fun (y: Num) -> y * y in (x, 42)").unwrap();
        let x_typ = arrow(Num, Num);
        assert_eq!(
            typecheck(p).unwrap(),
            let_expr(
                "x",
                arrow(Num, Num),
                fun(
                    "y",
                    Num,
                    binop(var("y", Num), Infix::Times, var("y", Num), Num),
                    x_typ.clone()
                ),
                pair(
                    var("x", x_typ.clone()),
                    num(42),
                    product(x_typ.clone(), Num)
                ),
                product(x_typ.clone(), Num)
            )
        );
        let ctx = Context::new().update("x", &Dyn);
        let p = parse_alfa_program("let y be x.fst in y").unwrap();
        assert_eq!(
            typecheck_helper(p, ctx.clone()).unwrap(),
            let_expr("y", Dyn, prjl(var("x", Dyn), Dyn), var("y", Dyn), Dyn)
        );
        let p = parse_alfa_program("let (y: Bool) be 2 in y").unwrap();
        assert_eq!(
            typecheck(p),
            Err(
                "Variable definition Num(2) must be consistent with annotated type Bool"
                    .to_string()
            )
        );
        let p = parse_alfa_program("let (x: Bool) be x in if x then false else true").unwrap();
        assert_eq!(
            typecheck_helper(p, ctx).unwrap(),
            let_expr(
                "x",
                Bool,
                var("x", Dyn),
                if_expr(var("x", Bool), bool_lit(false), bool_lit(true), Bool),
                Bool
            )
        );
    }

    #[test]
    fn test_ap() {
        let ctx = Context::new()
            .update("x", &Dyn)
            .update("f", &arrow(Num, Dyn))
            .update("g", &arrow(Num, Num));
        let p = parse_alfa_program("let x be f(1) in x - 1").unwrap();
        assert_eq!(
            typecheck_helper(p, ctx.clone()).unwrap(),
            let_expr(
                "x",
                Dyn,
                ap(var("f", arrow(Num, Dyn)), num(1), Dyn),
                binop(var("x", Dyn), Infix::Minus, num(1), Num),
                Num
            )
        );
        let p = parse_alfa_program("f(g(x))").unwrap();
        assert_eq!(
            typecheck_helper(p, ctx.clone()).unwrap(),
            ap(
                var("f", arrow(Num, Dyn)),
                ap(var("g", arrow(Num, Num)), var("x", Dyn), Num),
                Dyn
            )
        );
        let p = parse_alfa_program("(fun (x: Num) -> x + 1)(false)").unwrap();
        if let Err(s) = typecheck(p) {
            assert!(s.contains("Argument Bool(false) to function"));
        } else {
            panic!("Inconsistent argument type with function should fail typechecking");
        }
        let p = parse_alfa_program("(fun x -> x + 1)(false)").unwrap();
        assert_eq!(
            typecheck(p).unwrap(),
            ap(
                fun(
                    "x",
                    Dyn,
                    binop(var("x", Dyn), Infix::Plus, num(1), Num),
                    arrow(Dyn, Num)
                ),
                bool_lit(false),
                Num
            )
        );
    }

    #[test]
    fn test_binexpr() {
        use Infix::*;
        let p = parse_alfa_program("1+2*3-4").unwrap();
        assert_eq!(
            typecheck(p).unwrap(),
            binop(
                binop(num(1), Plus, binop(num(2), Times, num(3), Num), Num),
                Minus,
                num(4),
                Num
            )
        );
        let p = parse_alfa_program("1+2 > 3-4").unwrap();
        assert_eq!(
            typecheck(p).unwrap(),
            binop(
                binop(num(1), Plus, num(2), Num),
                GreaterThan,
                binop(num(3), Minus, num(4), Num),
                Bool
            )
        );
        let p = parse_alfa_program("1+2 < 3-4").unwrap();
        assert_eq!(
            typecheck(p).unwrap(),
            binop(
                binop(num(1), Plus, num(2), Num),
                LessThan,
                binop(num(3), Minus, num(4), Num),
                Bool
            )
        );
        let p = parse_alfa_program("1+2 =? 3-4").unwrap();
        assert_eq!(
            typecheck(p).unwrap(),
            binop(
                binop(num(1), Plus, num(2), Num),
                EqualTo,
                binop(num(3), Minus, num(4), Num),
                Bool
            )
        );
        // Check all binary expressions on Nums reject non Num operands
        for op in [Plus, Minus, Times, GreaterThan, LessThan, EqualTo] {
            let p = parse_alfa_program(format!("1 {} false", op).as_str()).unwrap();
            assert_eq!(
                typecheck(p),
                Err(
                    "Binary Expression argument (Bool(false)) must be consistent with Num"
                        .to_string()
                )
            );
            let p = parse_alfa_program(format!("false {} 1", op).as_str()).unwrap();
            assert_eq!(
                typecheck(p),
                Err(
                    "Binary Expression argument (Bool(false)) must be consistent with Num"
                        .to_string()
                )
            );
        }
        // Check it allows Dyn as operand
        let ctx = Context::new().update("x", &Dyn);
        let p = parse_alfa_program("x + x").unwrap();
        assert_eq!(
            typecheck_helper(p, ctx).unwrap(),
            binop(var("x", Dyn), Plus, var("x", Dyn), Num)
        );
    }

    #[test]
    fn test_unop() {
        use Prefix::*;
        let p = parse_alfa_program("-1").unwrap();
        assert_eq!(typecheck(p).unwrap(), unop(Neg, num(1), Num));
        let p = parse_alfa_program("-false").unwrap();
        assert_eq!(
            typecheck(p),
            Err("Unary Expression argument (Bool(false)) must be consistent with Num".to_string())
        );
        let ctx = Context::new().update("x", &Dyn);
        let p = parse_alfa_program("-x").unwrap();
        assert_eq!(
            typecheck_helper(p, ctx).unwrap(),
            unop(Neg, var("x", Dyn), Num)
        );
    }

    #[test]
    fn test_pair() {
        let p = parse_alfa_program("(1,2)").unwrap();
        assert_eq!(
            typecheck(p).unwrap(),
            pair(num(1), num(2), product(Num, Num))
        );
        let ctx = Context::new().update("x", &Dyn);
        let p = parse_alfa_program("(1,x)").unwrap();
        assert_eq!(
            typecheck_helper(p, ctx).unwrap(),
            pair(num(1), var("x", Dyn), product(Num, Dyn))
        );
    }
}
