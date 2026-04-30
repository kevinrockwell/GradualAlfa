use crate::ast::*;

type EvalResult = Result<AlfaValue, String>;
type CastResult<T> = Result<T, String>;

mod cast {
    use super::CastResult;
    use crate::ast::{AlfaType, AlfaValue, KnownType};
    use AlfaType::Dyn;
    use AlfaType::*;
    use AlfaValue::*;
    // Mirrors the `fun` call in Siek et al.
    // In particular, fun(?) => ? -> ?
    pub fn fun_cast(typ: &AlfaType) -> CastResult<(AlfaType, AlfaType)> {
        match typ {
            Dyn => Ok((Dyn, Dyn)),
            Arrow(arg, ret) => Ok((*arg.clone(), *ret.clone())),
            _ => Err(format!("Cannot cast {:?} to Arrow type", typ)),
        }
    }

    pub fn sum_cast(typ: &AlfaType) -> CastResult<(AlfaType, AlfaType)> {
        match typ {
            Dyn => Ok((Dyn, Dyn)),
            Sum(l, r) => Ok((*l.clone(), *r.clone())),
            _ => Err(format!("Cannot cast {:?} to Sum type", typ)),
        }
    }

    pub fn prod_cast(typ: &AlfaType) -> CastResult<(AlfaType, AlfaType)> {
        match typ {
            Dyn => Ok((Dyn, Dyn)),
            Product(fst, snd) => Ok((*fst.clone(), *snd.clone())),
            _ => Err(format!("Cannot cast {:?} to Product type", typ)),
        }
    }

    // TODO: it feels like there should be a way to encode this better into the
    // type system, but it's not obvious how so I spam unreachable later
    pub fn cast(val: AlfaValue, typ: AlfaType) -> CastResult<AlfaValue> {
        // IdBase and IdStar, as well as any other identical types
        // Of note: the "other identical types" is not present in Siek et al., and perhaps
        // structuring things differently is not necessary. However, for already fully typed
        // functions (which I'm not sure if they consider at all), it seems to be necessary to allow
        // equal types to be matched.
        if *val.get_type() == typ {
            return Ok(val);
        }
        // Injection
        if val.get_type().is_ground() && typ == Dyn {
            return Ok(Cast(Box::new(val), Dyn));
        }
        // Succeed / Fail
        if let Cast(val, Dyn) = val {
            return if *val.get_type() == typ {
                Ok(*val)
            } else {
                Err(format!("Cannot cast {:?} to {:?}", val.get_type(), typ))
            };
        }
        // Ground --- we already know val does not have type `?`
        if typ == Dyn {
            let ground_type = val.get_type().get_ground();
            // First cast to ground type, then cast to Dyn
            let grounded = Cast(Box::new(val), ground_type);
            return Ok(Cast(Box::new(grounded), Dyn));
        }
        // Expand --- we already know typ is not ?
        if *val.get_type() == Dyn {
            let ground_type = typ.get_ground();
            let grounded = Cast(Box::new(val), ground_type);
            return Ok(Cast(Box::new(grounded), typ));
        }
        unreachable!("Casting {:?} to {:?}", val.get_type(), typ)
    }
}

mod subst {
    use crate::ast::*;
    use TypedExpr::*;
    pub fn subst(sub_id: &Id, val: &AlfaValue, expr: TypedExpr) -> TypedExpr {
        let dosub = |e: Box<TypedExpr>| subst(sub_id, val, *e);
        match expr {
            Num(n) => Num(n),
            Bool(b) => Bool(b),
            Unit => Unit,
            Value { val } => Value { val },
            Pair { fst, snd, typ } => pair(dosub(fst), dosub(snd), typ),
            Fun {
                id,
                var_typ,
                body,
                typ,
            } => {
                if id == *sub_id {
                    Fun {
                        id,
                        var_typ,
                        body,
                        typ,
                    }
                } else {
                    fun(&id.id, var_typ, dosub(body), typ)
                }
            }
            Var { id, typ } => {
                if id == *sub_id {
                    Value {
                        val: Box::new(val.clone()),
                    }
                } else {
                    Var { id, typ }
                }
            }
            If {
                cond,
                if_body,
                else_body,
                typ,
            } => if_expr(
                dosub(cond),
                subst(sub_id, val, *if_body),
                subst(sub_id, val, *else_body),
                typ,
            ),
            Case {
                cond,
                l_var,
                l_body,
                r_var,
                r_body,
                typ,
            } => {
                let l_body = if l_var == *sub_id {
                    *l_body
                } else {
                    dosub(l_body)
                };
                let r_body = if r_var == *sub_id {
                    *r_body
                } else {
                    dosub(r_body)
                };
                case(dosub(cond), &l_var.id, l_body, &r_var.id, r_body, typ)
            }
            InjL { expr, typ } => injl(dosub(expr), typ),
            InjR { expr, typ } => injr(dosub(expr), typ),
            PrjL { expr, typ } => prjl(dosub(expr), typ),
            PrjR { expr, typ } => prjr(dosub(expr), typ),
            Let {
                id,
                var_typ,
                def,
                body,
                typ,
            } => {
                let def = dosub(def);
                let body = if id == *sub_id { *body } else { dosub(body) };
                let_expr(&id.id, var_typ, def, body, typ)
            }
            Ap { fun, arg, typ } => ap(dosub(fun), dosub(arg), typ),
            BinOp { lhs, op, rhs, typ } => binop(dosub(lhs), op, dosub(rhs), typ),
            UnOp { op, expr, typ } => unop(op, dosub(expr), typ),
        }
    }
}

use cast::*;
use subst::*;

pub fn expecting_num(expr: TypedExpr) -> CastResult<i32> {
    let val = cast(eval(expr)?, AlfaType::Num);
    if let AlfaValue::Num(n) = val?.inner() {
        Ok(n)
    } else {
        unreachable!("Cast to Num should value is Num")
    }
}

// Note: the "interesting" dynamic semantics all involve the insertion of casts. These happen in
// several places, but the "canonical" one is function application --- the ideas are largely the
// same throughout, but the dynamic semantics function application come directly from "Refined
// Criteria for Gradual Typing" by Siek et al. The remaining places extend these ideas in a natural
// way, hardcoding types where appropriate (e.g. the condition of an `if` expression is cast
// directly to a boolean instead of using a cast like the `fun` of Siek et al.)
pub fn eval(expr: TypedExpr) -> EvalResult {
    use AlfaType::Dyn;
    use AlfaValue::*;
    match expr {
        TypedExpr::Num(n) => Ok(Num(n)),
        TypedExpr::Bool(b) => Ok(Bool(b)),
        TypedExpr::Unit => Ok(Unit),
        TypedExpr::Pair { fst, snd, typ } => {
            let fst = eval(*fst)?;
            let snd = eval(*snd)?;
            let typ = product(fst.get_type().clone(), snd.get_type().clone());
            Ok(Pair(Box::new(fst), Box::new(snd), typ))
        }
        TypedExpr::Value { val } => Ok(*val),
        TypedExpr::Fun {
            id,
            var_typ,
            body,
            typ,
        } => Ok(Fun(id, var_typ, *body, typ)),
        TypedExpr::Var { id: _, typ: _ } => {
            unreachable!("Substitution Failure on Well Typed Program")
        }
        TypedExpr::Ap { fun, arg, typ } => {
            let (arg_typ, ret_typ) = fun_cast(fun.get_type())?;
            // First, for soundness, cast to the derived function type
            let fun = cast(eval(*fun)?, arrow(arg_typ.clone(), ret_typ.clone()))?;
            let arg = cast(eval(*arg)?, arg_typ)?;
            // Once casts are inserted and are allowed, we know we can safely
            // consider the ``inner'' function value
            if let Fun(id, _, body, _) = fun.inner() {
                Ok(eval(subst(&id, &arg, body))?)
            } else {
                unreachable!("Arrow type must correspond to Function value")
            }
        }
        TypedExpr::If {
            cond,
            if_body,
            else_body,
            typ,
        } => {
            let cond = cast(eval(*cond)?, AlfaType::Bool)?;
            if let Bool(b) = cond {
                let res = if b {
                    eval(*if_body)?
                } else {
                    eval(*else_body)?
                };
                // Ensure return type matches what we have promised during typechecking
                cast(res, typ)
            } else {
                unreachable!("Cast to Bool should ensure value is Bool")
            }
        }
        TypedExpr::Case {
            cond,
            l_var,
            l_body,
            r_var,
            r_body,
            typ,
        } => {
            let (l_typ, r_typ) = sum_cast(cond.get_type())?;
            // For soundness, cast cond to derived Sum type after evaluation
            let cond = cast(eval(*cond)?, sum(l_typ.clone(), r_typ.clone()))?;
            let res = match cond.inner() {
                InjL(val, _) => {
                    let val = cast(*val, l_typ)?;
                    eval(subst(&l_var, &val, *l_body))
                }
                InjR(val, _) => {
                    let val = cast(*val, r_typ)?;
                    eval(subst(&r_var, &val, *r_body))
                }
                _ => unreachable!("Cast to Sum should ensure value is Sum"),
            };
            // Cast result to desired output type
            cast(res?, typ)
        }
        TypedExpr::InjL { expr, typ } => Ok(InjL(Box::new(eval(*expr)?), typ)),
        TypedExpr::InjR { expr, typ } => Ok(InjR(Box::new(eval(*expr)?), typ)),
        TypedExpr::PrjL { expr, typ } => {
            let (fst_t, snd_t) = prod_cast(expr.get_type())?;
            let pair = cast(eval(*expr)?, product(fst_t.clone(), snd_t))?;
            if let Pair(fst, _, _) = pair.inner() {
                cast(*fst, fst_t)
            } else {
                unreachable!("Cast to Product type should ensure value is Pair")
            }
        }
        TypedExpr::PrjR { expr, typ } => {
            let (fst_t, snd_t) = prod_cast(expr.get_type())?;
            let pair = cast(eval(*expr)?, product(fst_t, snd_t.clone()))?;
            if let Pair(_, snd, _) = pair.inner() {
                cast(*snd, snd_t)
            } else {
                unreachable!("Cast to Product type should ensure value is Pair")
            }
        }
        TypedExpr::Let {
            id,
            var_typ,
            def,
            body,
            typ,
        } => {
            // Cast definition to expected type
            let def = cast(eval(*def)?, var_typ)?;
            let result = eval(subst(&id, &def, *body));
            // Cast result to expected type
            cast(result?, typ)
        }
        TypedExpr::BinOp { lhs, op, rhs, typ } => {
            let lhs = expecting_num(*lhs)?;
            let rhs = expecting_num(*rhs)?;
            let result = match op {
                Infix::Plus => Num(lhs + rhs),
                Infix::Minus => Num(lhs - rhs),
                Infix::Times => Num(lhs * rhs),
                Infix::LessThan => Bool(lhs < rhs),
                Infix::GreaterThan => Bool(lhs > rhs),
                Infix::EqualTo => Bool(lhs == rhs),
            };
            cast(result, typ)
        }
        TypedExpr::UnOp { op, expr, typ } => {
            let val = expecting_num(*expr)?;
            let result = match op {
                Prefix::Neg => Num(-val),
            };
            cast(result, typ)
        }
    }
}

mod test {
    use super::*;
    use crate::parser::parse_alfa_program;
    mod subst {
        use super::*;
        use AlfaType::{Dyn, Num};

        fn run_subst_test(sub_id: &str, val: &AlfaValue, start: &TypedExpr, goal: TypedExpr) {
            let res = subst(&id(sub_id), &val, start.clone());
            assert_eq!(res, goal);
        }

        fn val(v: AlfaValue) -> TypedExpr {
            TypedExpr::Value { val: Box::new(v) }
        }

        fn numval(n: i32) -> TypedExpr {
            TypedExpr::Value {
                val: Box::new(AlfaValue::Num(n)),
            }
        }

        #[test]
        fn test_subst_let() {
            // let x be x + y in x + y
            let let_test = let_expr(
                "x",
                Dyn,
                binop(var("x", Dyn), Infix::Plus, var("y", Dyn), Num),
                binop(var("x", Dyn), Infix::Plus, var("y", Dyn), Num),
                Num,
            );
            let sub_x = let_expr(
                "x",
                Dyn,
                binop(numval(1), Infix::Plus, var("y", Dyn), Num),
                binop(var("x", Dyn), Infix::Plus, var("y", Dyn), Num),
                Num,
            );
            let sub_y = let_expr(
                "x",
                Dyn,
                binop(var("x", Dyn), Infix::Plus, numval(1), Num),
                binop(var("x", Dyn), Infix::Plus, numval(1), Num),
                Num,
            );
            run_subst_test("x", &AlfaValue::Num(1), &let_test, sub_x);
            run_subst_test("y", &AlfaValue::Num(1), &let_test, sub_y);
        }

        #[test]
        fn test_subst_fun() {
            // fun x -> x + y
            let fun_test = fun(
                "x",
                Dyn,
                binop(var("x", Dyn), Infix::Plus, var("y", Dyn), Num),
                arrow(Dyn, Num),
            );
            let sub_x = fun(
                "x",
                Dyn,
                binop(var("x", Dyn), Infix::Plus, var("y", Dyn), Num),
                arrow(Dyn, Num),
            );
            let sub_y = fun(
                "x",
                Dyn,
                binop(var("x", Dyn), Infix::Plus, numval(1), Num),
                arrow(Dyn, Num),
            );
            run_subst_test("x", &AlfaValue::Num(1), &fun_test, sub_x);
            run_subst_test("y", &AlfaValue::Num(1), &fun_test, sub_y);
        }

        #[test]
        fn test_subst_case() {
            let case_test = case(
                var("z", Dyn),
                "x",
                binop(var("x", Dyn), Infix::Plus, var("y", Dyn), Num),
                "y",
                binop(var("x", Dyn), Infix::Plus, var("y", Dyn), Num),
                Num,
            );
            let sub_x = case(
                var("z", Dyn),
                "x",
                binop(var("x", Dyn), Infix::Plus, var("y", Dyn), Num),
                "y",
                binop(numval(1), Infix::Plus, var("y", Dyn), Num),
                Num,
            );
            let sub_y = case(
                var("z", Dyn),
                "x",
                binop(var("x", Dyn), Infix::Plus, numval(1), Num),
                "y",
                binop(var("x", Dyn), Infix::Plus, var("y", Dyn), Num),
                Num,
            );
            let sub_z = case(
                numval(1), // Look it doesn't type check but its easy to setup so lol
                "x",
                binop(var("x", Dyn), Infix::Plus, var("y", Dyn), Num),
                "y",
                binop(var("x", Dyn), Infix::Plus, var("y", Dyn), Num),
                Num,
            );
            run_subst_test("x", &AlfaValue::Num(1), &case_test, sub_x);
            run_subst_test("y", &AlfaValue::Num(1), &case_test, sub_y);
            run_subst_test("z", &AlfaValue::Num(1), &case_test, sub_z);
        }

        #[test]
        fn test_subst_other() {
            let val = AlfaValue::Num(2);
            // Basic types
            run_subst_test("x", &val, &num(3), num(3));
            run_subst_test("x", &val, &bool_lit(true), bool_lit(true));
            run_subst_test("x", &val, &unit(), unit());
            run_subst_test("x", &val, &var("x", Dyn), numval(2));
            run_subst_test("x", &val, &var("y", Dyn), var("y", Dyn));
            // If
            let if_test = if_expr(
                binop(
                    var("x", Dyn),
                    Infix::GreaterThan,
                    var("y", Dyn),
                    AlfaType::Bool,
                ),
                binop(var("x", Dyn), Infix::Plus, var("y", Dyn), Num),
                binop(var("x", Dyn), Infix::Plus, var("y", Dyn), Num),
                Num,
            );
            let sub_x = if_expr(
                binop(numval(2), Infix::GreaterThan, var("y", Dyn), AlfaType::Bool),
                binop(numval(2), Infix::Plus, var("y", Dyn), Num),
                binop(numval(2), Infix::Plus, var("y", Dyn), Num),
                Num,
            );
            let sub_y = if_expr(
                binop(var("x", Dyn), Infix::GreaterThan, numval(4), AlfaType::Bool),
                binop(var("x", Dyn), Infix::Plus, numval(4), Num),
                binop(var("x", Dyn), Infix::Plus, numval(4), Num),
                Num,
            );
            run_subst_test("x", &AlfaValue::Num(2), &if_test, sub_x);
            run_subst_test("y", &AlfaValue::Num(4), &if_test, sub_y);
            // Injections
            run_subst_test(
                "x",
                &AlfaValue::Num(2),
                &injl(var("x", Dyn), sum(Dyn, Dyn)),
                injl(numval(2), sum(Dyn, Dyn)),
            );
            run_subst_test(
                "x",
                &AlfaValue::Num(2),
                &injr(var("x", Dyn), sum(Dyn, Dyn)),
                injr(numval(2), sum(Dyn, Dyn)),
            );
            // Projections
            run_subst_test(
                "x",
                &AlfaValue::Num(2),
                &prjl(var("x", Dyn), sum(Dyn, Dyn)),
                prjl(numval(2), sum(Dyn, Dyn)),
            );
            run_subst_test(
                "x",
                &AlfaValue::Num(2),
                &prjr(var("x", Dyn), sum(Dyn, Dyn)),
                prjr(numval(2), sum(Dyn, Dyn)),
            );
            // Application -- not the right types, but no matter
            run_subst_test(
                "x",
                &AlfaValue::Num(2),
                &ap(var("x", Dyn), var("x", Dyn), Dyn),
                ap(numval(2), numval(2), Dyn),
            );
            // Pair
            run_subst_test(
                "x",
                &AlfaValue::Num(42),
                &pair(var("x", Dyn), var("y", Dyn), Dyn),
                pair(numval(42), var("y", Dyn), Dyn),
            );
            run_subst_test(
                "y",
                &AlfaValue::Num(42),
                &pair(var("x", Dyn), var("y", Dyn), Dyn),
                pair(var("x", Dyn), numval(42), Dyn),
            );
            // Value
            run_subst_test("x", &AlfaValue::Unit, &numval(2), numval(2));
        }
    }
    mod cast {
        use super::*;
        use AlfaType::Dyn;

        #[test]
        fn test_fun_cast() {
            use AlfaType::Num;
            assert_eq!(fun_cast(&Dyn), Ok((Dyn, Dyn)));
            assert_eq!(fun_cast(&arrow(Num, Num)), Ok((Num, Num)));
            assert_eq!(fun_cast(&arrow(Num, Dyn)), Ok((Num, Dyn)));
            assert_eq!(
                fun_cast(&arrow(arrow(Num, Num), Dyn)),
                Ok((arrow(Num, Num), Dyn))
            );
        }

        #[test]
        fn test_cast() {
            use AlfaValue::*;
            // Base
            assert_eq!(cast(Num(2), AlfaType::Num), Ok(Num(2)));
            // Star
            assert_eq!(
                cast(Cast(Box::new(Num(1)), Dyn), AlfaType::Dyn),
                Ok(Cast(Box::new(Num(1)), Dyn))
            );
            // Succeed
            assert_eq!(cast(Cast(Box::new(Num(1)), Dyn), AlfaType::Num), Ok(Num(1)));
            // Fail
            assert_eq!(
                cast(Cast(Box::new(Num(1)), Dyn), AlfaType::Bool),
                Err("Cannot cast Num to Bool".to_string())
            );
            // Ground
            let p = Pair(
                Box::new(Num(1)),
                Box::new(Bool(true)),
                product(AlfaType::Num, AlfaType::Bool),
            );
            assert_eq!(
                cast(p.clone(), Dyn),
                Ok(Cast(Box::new(Cast(Box::new(p), product(Dyn, Dyn))), Dyn))
            );
            // Expand
            let p = Pair(Box::new(Num(1)), Box::new(Bool(true)), Dyn);
            assert_eq!(
                cast(p.clone(), product(AlfaType::Num, AlfaType::Bool)),
                Ok(Cast(
                    Box::new(Cast(Box::new(p), product(Dyn, Dyn))),
                    product(AlfaType::Num, AlfaType::Bool)
                ))
            );
            // Cast failures manifest going T1 -> Dyn -> T2
            // Otherwise it will just be unreachable: we only use cast rules from Siek et al.
            let v = Cast(Box::new(Num(1)), Dyn);
            assert_eq!(
                cast(v, AlfaType::Bool),
                Err("Cannot cast Num to Bool".to_string())
            );
        }
    }

    fn typecheck(prog: &str) -> TypedExpr {
        use crate::typechecking::typecheck;
        let p = parse_alfa_program(prog).unwrap();
        typecheck(p).unwrap()
    }

    #[test]
    fn test_eval_basic() {
        use AlfaType::Dyn;
        use AlfaValue::*;
        assert_eq!(eval(num(42)), Ok(Num(42)));
        assert_eq!(eval(num(0)), Ok(Num(0)));
        assert_eq!(eval(bool_lit(true)), Ok(Bool(true)));
        assert_eq!(eval(bool_lit(false)), Ok(Bool(false)));
        assert_eq!(eval(unit()), Ok(Unit));
        assert_eq!(
            eval(pair(
                num(1),
                bool_lit(false),
                product(AlfaType::Num, AlfaType::Bool)
            )),
            Ok(Pair(
                Box::new(Num(1)),
                Box::new(Bool(false)),
                product(AlfaType::Num, AlfaType::Bool)
            ))
        );
        let f = typecheck("fun (x: Num) -> x");
        assert_eq!(
            eval(f),
            Ok(Fun(
                id("x"),
                AlfaType::Num,
                var("x", AlfaType::Num),
                arrow(AlfaType::Num, AlfaType::Num)
            ))
        );
        let f = typecheck("fun x -> x");
        assert_eq!(
            eval(f),
            Ok(Fun(id("x"), Dyn, var("x", Dyn), arrow(Dyn, Dyn)))
        );
    }

    #[test]
    fn test_ap() {
        use AlfaValue::*;
        let f = typecheck("(fun (x: Num) -> x)(1)");
        assert_eq!(eval(f), Ok(Num(1)));
        let f = typecheck("(fun x -> x)(1)");
        assert_eq!(eval(f), Ok(Cast(Box::new(Num(1)), AlfaType::Dyn)));
        // Pro tip: one more layer of indirection just one more just one more
        let f = typecheck("(fun x -> (fun (x: Num) -> x)(x))(true)");
        assert_eq!(eval(f), Err("Cannot cast Bool to Num".to_string()));
    }

    #[test]
    fn test_if() {
        use AlfaValue::*;
        let e = typecheck("if true then 1 else 2");
        assert_eq!(eval(e), Ok(Num(1)));
        let e = typecheck("(fun x -> if x then false else true)(true)");
        assert_eq!(eval(e), Ok(Bool(false)));
        let e = typecheck("(fun x -> if x then false else true)(false)");
        assert_eq!(eval(e), Ok(Bool(true)));
        let e = typecheck("(fun x -> if x then false else true)(1)");
        assert_eq!(eval(e), Err("Cannot cast Num to Bool".to_string()));
    }

    #[test]
    fn test_case() {
        use AlfaValue::*;
        let e = typecheck("case L () of L(x) -> 1 else R(y) -> 2");
        assert_eq!(eval(e), Ok(Num(1)));
        let e = typecheck("case L (1) of L(x) -> x else R(y) -> 2");
        assert_eq!(eval(e), Ok(Num(1)));
        let e = typecheck("case R (2) of L(x) -> 1 else R(y) -> y");
        assert_eq!(eval(e), Ok(Num(2)));
        let e = typecheck("(fun x -> case x of L(x) -> x else R(y) -> y)(L(3))");
        assert_eq!(eval(e), Ok(Cast(Box::new(Num(3)), AlfaType::Dyn)));
        let e = typecheck("(fun x -> case x of L(x) -> x else R(y) -> y)(R(2))");
        assert_eq!(eval(e), Ok(Cast(Box::new(Num(2)), AlfaType::Dyn)));
        let e = typecheck("(fun x -> case x of L(x) -> 1 else R(y) -> y)(22)");
        assert_eq!(eval(e), Err("Cannot cast Num to Sum(Dyn, Dyn)".to_string()));
    }

    #[test]
    fn test_let() {
        use AlfaValue::*;
        let e = typecheck("let x be (fun x -> x)(1) in x");
        assert_eq!(eval(e), Ok(Cast(Box::new(Num(1)), AlfaType::Dyn)));
        let e = typecheck("let x be (fun x -> x)(false) in x");
        assert_eq!(eval(e), Ok(Cast(Box::new(Bool(false)), AlfaType::Dyn)));
        let e = typecheck("let (x: Num) be (fun x -> x)(false) in x");
        assert_eq!(eval(e), Err("Cannot cast Bool to Num".to_string()));
    }

    #[test]
    fn test_prj() {
        use AlfaValue::*;
        let e = typecheck("(1, false).fst");
        assert_eq!(eval(e), Ok(Num(1)));
        let e = typecheck("(1, false).snd");
        assert_eq!(eval(e), Ok(Bool(false)));
        // TODO: this panics, need to get inner probably.
        let e = typecheck("(fun x -> x.fst)((1, false))");
        assert_eq!(eval(e), Ok(Cast(Box::new(Num(1)), AlfaType::Dyn)));
        let e = typecheck("(fun x -> x.snd)((1, false))");
        assert_eq!(eval(e), Ok(Cast(Box::new(Bool(false)), AlfaType::Dyn)));
        let e = typecheck("(fun x -> x.fst)(1)");
        assert_eq!(
            eval(e),
            Err("Cannot cast Num to Product(Dyn, Dyn)".to_string())
        );
        let e = typecheck("(fun x -> x.snd)(1)");
        assert_eq!(
            eval(e),
            Err("Cannot cast Num to Product(Dyn, Dyn)".to_string())
        );
    }

    fn test_num_ans(prog: &str, ans: i32) {
        assert_eq!(
            eval(typecheck(prog)).expect("Ill typed program being tested against"),
            AlfaValue::Num(ans),
        )
    }

    fn test_bool_ans(prog: &str, ans: bool) {
        assert_eq!(
            eval(typecheck(prog)).expect("Ill typed program being tested against"),
            AlfaValue::Bool(ans),
        )
    }

    #[test]
    fn test_operators() {
        use AlfaValue::*;
        test_num_ans("1 + 2 - 3", 0);
        test_num_ans("3 - 1", 2);
        test_num_ans("0", 0);
        test_num_ans("0 * 2", 0);
        test_num_ans("-1", -1);
        test_num_ans("-1 * 2", -2);
        test_num_ans("-1 * 2", -2);
        // Return type can be deduced from the arithmetic operation
        test_num_ans("(fun x -> x + 1)(2)", 3);
        test_bool_ans("2 > 3", false);
        test_bool_ans("3 > 1", true);
        test_bool_ans("2 > 2", false);
        test_bool_ans("3 < 1", false);
        test_bool_ans("1 < 3", true);
        test_bool_ans("3 =? 1", false);
        test_bool_ans("1 =? 3", false);
        test_bool_ans("1 =? 1", true);
        let e = typecheck("let (x: ?) be 2 + 3 in x");
        assert_eq!(eval(e), Ok(Cast(Box::new(Num(5)), AlfaType::Dyn)));
    }
}
