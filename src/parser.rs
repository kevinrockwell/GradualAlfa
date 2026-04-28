use pest::Parser;
use pest::iterators::{Pair, Pairs};
use pest::pratt_parser::{Assoc, Op, PrattParser};
use pest_derive::Parser;

use crate::ast::*;

pub type ParseResult = Result<Expr, String>;
type AlfaTypeResult = Result<AlfaType, String>;

#[derive(Parser, Debug)]
#[grammar = "alfa.pest"]
struct AlfaParser;

// TODO: Types
fn parse_alfatype(pair: Option<Pair<Rule>>, _pratt: &PrattParser<Rule>) -> Option<AlfaType> {
    // We can use the same PrattParser for composite types
    println!("DEBUG: Parsing type: {:?}", pair?);
    None
}

fn parse_var_dec(pair: Pair<Rule>, pratt: &PrattParser<Rule>) -> Id {
    if Rule::vardec == pair.as_rule() {
        let mut inner = pair.into_inner();
        Id {
            id: inner.next().unwrap().to_string(),
            typ: parse_alfatype(inner.next(), pratt),
        }
    } else if Rule::id == pair.as_rule() {
        Id {
            id: pair.to_string(),
            typ: None
        }
    } else {
        panic!("Trying to parse variable out of {:?}", pair)
    }
}

fn parse_arith(pairs: Pairs<Rule>, pratt: &PrattParser<Rule>) -> ParseResult {
    println!("DEBUG arith pairs: {:?}", pairs);
    pratt
        .map_primary(|primary| match primary.as_rule() {
            Rule::numlit => Ok(Expr::Num(primary.as_str().parse().unwrap())),
            Rule::boollit => {
                if primary.as_str() == "true" {
                    Ok(Expr::Bool(true))
                } else if primary.as_str() == "false" {
                    Ok(Expr::Bool(false))
                } else {
                    Err(format!("Unexpected boolean literal {:?}", primary))
                }
            }
            Rule::unit => Ok(Expr::Unit),
            Rule::id => Ok(Expr::Var(Id {
                id: primary.to_string(),
                typ: None,
            })),
            // Kinda janky --- we want to be able to do (1,2).fst because the
            // parentheses of the pair make this clear, but without this
            // rule would need to do ((1, 2)).fst
            Rule::pair => parse_expr(primary, pratt),
            // TODO: can bool/unit be removed?
            Rule::expr => parse_expr(primary, pratt),
            // TODO: better error message
            _ => Err(format!("Unexpected arithmetic atom, got {:?}", primary)),
        })
        .map_infix(|lhs, op, rhs| {
            let binop = match op.as_rule() {
                Rule::plus => Infix::Plus,
                Rule::minus => Infix::Minus,
                Rule::times => Infix::Times,
                Rule::greater => Infix::GreaterThan,
                Rule::less => Infix::LessThan,
                Rule::eq => Infix::EqualTo,
                _ => return Err(format!("Unexpected infix operator {:?}", op)),
            };
            Ok(Expr::BinOp(Box::new(lhs?), binop, Box::new(rhs?)))
        })
        .map_prefix(|op, expr| match op.as_rule() {
            Rule::neg => Ok(Expr::UnOp(Prefix::Neg, Box::new(expr?))),
            _ => return Err(format!("Unexpected prefix operator {:?}", op)),
        })
        .map_postfix(|expr, op| match op.as_rule() {
            Rule::fst => Ok(Expr::PrjL(Box::new(expr?))),
            Rule::snd => Ok(Expr::PrjR(Box::new(expr?))),
            _ => return Err(format!("Unexpected postfix operator {:?}", op)),
        })
        .parse(pairs)
}

fn parse_expr(current_rule: Pair<Rule>, pratt: &PrattParser<Rule>) -> ParseResult {
    println!("DEBUG: parse_expr parsing {:?}", current_rule);
    match current_rule.as_rule() {
        Rule::fun => {
            let mut inner = current_rule.into_inner();
            let id = parse_var_dec(inner.next().unwrap(), pratt);
            let body = parse_expr(inner.next().unwrap(), pratt)?;
            Ok(Expr::Fun(id, Box::new(body)))
        }
        Rule::ifexpr => {
            let mut inner = current_rule.into_inner();
            let cond = parse_expr(inner.next().unwrap(), pratt)?;
            let then = parse_expr(inner.next().unwrap(), pratt)?;
            let else_ = parse_expr(inner.next().unwrap(), pratt)?;
            Ok(Expr::If(Box::new(cond), Box::new(then), Box::new(else_)))
        }
        Rule::letexpr => {
            let mut inner = current_rule.into_inner();
            let id = parse_var_dec(inner.next().unwrap(), pratt);
            let def = parse_expr(inner.next().unwrap(), pratt)?;
            let body = parse_expr(inner.next().unwrap(), pratt)?;
            Ok(Expr::Let(id, Box::new(def), Box::new(body)))
        }
        Rule::case => {
            let mut inner = current_rule.into_inner();
            let cond = parse_expr(inner.next().unwrap(), pratt)?;
            let id_left = parse_var_dec(inner.next().unwrap(), pratt);
            let left = parse_expr(inner.next().unwrap(), pratt)?;
            let id_right = parse_var_dec(inner.next().unwrap(), pratt);
            let right = parse_expr(inner.next().unwrap(), pratt)?;
            Ok(Expr::Case(
                Box::new(cond),
                id_left,
                Box::new(left),
                id_right,
                Box::new(right),
            ))
        }
        Rule::InjL => {
            let mut inner = current_rule.into_inner();
            let body = parse_expr(inner.next().unwrap(), pratt)?;
            Ok(Expr::InjL(Box::new(body)))
        }
        Rule::InjR => {
            let mut inner = current_rule.into_inner();
            let body = parse_expr(inner.next().unwrap(), pratt)?;
            Ok(Expr::InjR(Box::new(body)))
        }
        Rule::ap => {
            let mut inner = current_rule.into_inner();
            let fun = parse_expr(inner.next().unwrap(), pratt)?;
            let arg = parse_expr(inner.next().unwrap(), pratt)?;
            Ok(Expr::Ap(Box::new(fun), Box::new(arg)))
        }
        Rule::arith => parse_arith(current_rule.into_inner(), pratt),
        Rule::pair => {
            let mut inner = current_rule.into_inner();
            let fst = parse_expr(inner.next().unwrap(), pratt)?;
            let snd = parse_expr(inner.next().unwrap(), pratt)?;
            Ok(Expr::Pair(Box::new(fst), Box::new(snd)))
        }
        Rule::numlit => {
            let num = current_rule.as_str().parse::<i32>().unwrap();
            Ok(Expr::Num(num))
        }
        Rule::boollit => {
            if current_rule.as_str() == "true" {
                Ok(Expr::Bool(true))
            } else if current_rule.as_str() == "false" {
                Ok(Expr::Bool(false))
            } else {
                Err(format!("Unexpected boolean literal {:?}", current_rule))
            }
        }
        Rule::unit => Ok(Expr::Unit),
        Rule::id => Ok(Expr::Var(Id {
            id: current_rule.to_string(),
            typ: None,
        })),
        Rule::expr => {
            // This handles the recursive case, e.g. ( expr ), or when
            // an expression is introduced and we need to get to the contents
            parse_expr(current_rule.into_inner().next().unwrap(), pratt)
        }
        _ => Ok(Expr::Unit),
    }
}

pub fn parse_alfa_program(prog: &str) -> ParseResult {
    // Set up the Pratt Parser
    let pratt = PrattParser::new()
        .op(Op::infix(Rule::greater, Assoc::Left)
            | Op::infix(Rule::less, Assoc::Left)
            | Op::infix(Rule::eq, Assoc::Left))
        .op(Op::infix(Rule::plus, Assoc::Left) | Op::infix(Rule::minus, Assoc::Left))
        .op(Op::infix(Rule::times, Assoc::Left))
        .op(Op::prefix(Rule::neg))
        .op(Op::postfix(Rule::fst) | Op::postfix(Rule::snd));
    // If the program parses correctly, there is exactly one expr at the top level
    let program = match AlfaParser::parse(Rule::alfa_prog, prog) {
        Ok(p) => p,
        // TODO handle this error reporting nicer
        Err(e) => return Err(format!("Parsing error: {:?}", e)),
    }
    .next()
    .unwrap();
    parse_expr(program, &pratt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use Expr::*;
    use Infix::*;

    #[test]
    fn test_literals() {
        assert_eq!(parse_alfa_program("1").unwrap(), Num(1));
        assert_eq!(parse_alfa_program("()").unwrap(), Unit);
        assert_eq!(parse_alfa_program("true").unwrap(), Bool(true));
        assert_eq!(parse_alfa_program("false").unwrap(), Bool(false));
        assert_eq!(
            parse_alfa_program("x").unwrap(),
            Var(Id {
                id: "x".to_string(),
                typ: None
            })
        );
    }

    #[test]
    fn test_arith() {
        // Precedence of *
        assert_eq!(
            parse_alfa_program("1 * 2 + 3 * 4").unwrap(),
            BinOp(
                Box::new(BinOp(Box::new(Num(1)), Times, Box::new(Num(2)))),
                Plus,
                Box::new(BinOp(Box::new(Num(3)), Times, Box::new(Num(4))))
            )
        );
        // Parentheses are respected
        assert_eq!(
            parse_alfa_program("1 * (2 + 3) - 4").unwrap(),
            BinOp(
                Box::new(BinOp(
                    Box::new(Num(1)),
                    Times,
                    Box::new(BinOp(Box::new(Num(2)), Plus, Box::new(Num(3))))
                )),
                Minus,
                Box::new(Num(4))
            )
        );
        // Associativity of + is correct
        assert_eq!(
            parse_alfa_program("1 + 2 + 3").unwrap(),
            BinOp(
                Box::new(BinOp(Box::new(Num(1)), Plus, Box::new(Num(2)))),
                Plus,
                Box::new(Num(3)),
            ),
        );
        // Associativity of - is correct
        assert_eq!(
            parse_alfa_program("1 - 2 - 3").unwrap(),
            BinOp(
                Box::new(BinOp(Box::new(Num(1)), Minus, Box::new(Num(2)))),
                Minus,
                Box::new(Num(3)),
            ),
        );
        // Associativity of * is correct
        assert_eq!(
            parse_alfa_program("1 * 2 * 3").unwrap(),
            BinOp(
                Box::new(BinOp(Box::new(Num(1)), Times, Box::new(Num(2)))),
                Times,
                Box::new(Num(3)),
            ),
        );
        // Precedence of comparison operators
        assert_eq!(
            parse_alfa_program("1 + 2 =? 3").unwrap(),
            BinOp(
                Box::new(BinOp(Box::new(Num(1)), Plus, Box::new(Num(2)))),
                EqualTo,
                Box::new(Num(3)),
            ),
        );
        assert_eq!(
            parse_alfa_program("1 * 2 < 3").unwrap(),
            BinOp(
                Box::new(BinOp(Box::new(Num(1)), Times, Box::new(Num(2)))),
                LessThan,
                Box::new(Num(3)),
            ),
        );
        assert_eq!(
            parse_alfa_program("1 - 2 > 3").unwrap(),
            BinOp(
                Box::new(BinOp(Box::new(Num(1)), Minus, Box::new(Num(2)))),
                GreaterThan,
                Box::new(Num(3)),
            ),
        );
        // Test unary minus
        assert_eq!(
            parse_alfa_program("-1").unwrap(),
            UnOp(Prefix::Neg, Box::new(Num(1)))
        );
        // Negation precedence
        assert_eq!(
            parse_alfa_program("-1 * 2").unwrap(),
            BinOp(
                Box::new(UnOp(Prefix::Neg, Box::new(Num(1)))),
                Times,
                Box::new(Num(2))
            )
        );
        // Negation precedence with postfix
        assert_eq!(
            parse_alfa_program("-s.fst * 3").unwrap(),
            BinOp(
                Box::new(UnOp(
                    Prefix::Neg,
                    Box::new(PrjL(Box::new(Var(Id {
                        id: "s".to_string(),
                        typ: None
                    }))))
                )),
                Times,
                Box::new(Num(3))
            )
        );
        assert_eq!(
            parse_alfa_program("-s.snd * 3").unwrap(),
            BinOp(
                Box::new(UnOp(
                    Prefix::Neg,
                    Box::new(PrjR(Box::new(Var(Id {
                        id: "s".to_string(),
                        typ: None
                    }))))
                )),
                Times,
                Box::new(Num(3))
            )
        );
    }

    #[test]
    fn test_fun() {
        assert_eq!(
            parse_alfa_program("fun x -> x - 2").unwrap(),
            Fun(
                Id {
                    id: "x".to_string(),
                    typ: None
                },
                Box::new(BinOp(
                    Box::new(Var(Id {
                        id: "x".to_string(),
                        typ: None
                    })),
                    Minus,
                    Box::new(Num(2))
                ))
            )
        );
        assert_eq!(
            parse_alfa_program("fun y -> x * 42").unwrap(),
            Fun(
                Id {
                    id: "y".to_string(),
                    typ: None
                },
                Box::new(BinOp(
                    Box::new(Var(Id {
                        id: "x".to_string(),
                        typ: None
                    })),
                    Times,
                    Box::new(Num(42))
                ))
            )
        );
    }

    #[test]
    fn test_if() {
        assert_eq!(
            parse_alfa_program("if 1 < 2 then 17 else 37").unwrap(),
            If(
                Box::new(BinOp(Box::new(Num(1)), LessThan, Box::new(Num(2)))),
                Box::new(Num(17)),
                Box::new(Num(37))
            )
        );
    }

    #[test]
    fn test_let() {
        assert_eq!(
            parse_alfa_program("let x be 42 in x * 12").unwrap(),
            Let(
                Id {
                    id: "x".to_string(),
                    typ: None
                },
                Box::new(Num(42)),
                Box::new(BinOp(
                    Box::new(Var(Id {
                        id: "x".to_string(),
                        typ: None
                    })),
                    Times,
                    Box::new(Num(12))
                ))
            )
        );
        assert_eq!(
            parse_alfa_program("let x be 12 - y in x").unwrap(),
            Let(
                Id {
                    id: "x".to_string(),
                    typ: None
                },
                Box::new(BinOp(
                    Box::new(Num(12)),
                    Minus,
                    Box::new(Var(Id {
                        id: "y".to_string(),
                        typ: None
                    }))
                )),
                Box::new(Var(Id {
                    id: "x".to_string(),
                    typ: None,
                }))
            )
        );
    }

    #[test]
    fn test_ap() {
        assert_eq!(
            parse_alfa_program("f(42)").unwrap(),
            Ap(
                Box::new(Var(Id {
                    id: "f".to_string(),
                    typ: None
                })),
                Box::new(Num(42))
            )
        );
        assert_eq!(
            parse_alfa_program("(fun x -> x + 1)(42)").unwrap(),
            Ap(
                Box::new(Fun(
                    Id {
                        id: "x".to_string(),
                        typ: None
                    },
                    Box::new(BinOp(
                        Box::new(Var(Id {
                            id: "x".to_string(),
                            typ: None
                        })),
                        Plus,
                        Box::new(Num(1))
                    ))
                )),
                Box::new(Num(42))
            )
        );
    }

    #[test]
    fn test_prod() {
        assert_eq!(
            parse_alfa_program("(1,2)").unwrap(),
            Pair(Box::new(Num(1)), Box::new(Num(2)))
        );
        assert_eq!(
            parse_alfa_program("(1,false)").unwrap(),
            Pair(Box::new(Num(1)), Box::new(Bool(false)))
        );
        assert_eq!(
            parse_alfa_program("(1, fun x -> 2)").unwrap(),
            Pair(
                Box::new(Num(1)),
                Box::new(Fun(
                    Id {
                        id: "x".to_string(),
                        typ: None
                    },
                    Box::new(Num(2))
                ))
            )
        );
        assert_eq!(
            parse_alfa_program("((1,2), 3).fst.snd").unwrap(),
            PrjR(Box::new(PrjL(Box::new(Pair(
                Box::new(Pair(Box::new(Num(1)), Box::new(Num(2)))),
                Box::new(Num(3))
            )))))
        );
    }

    #[test]
    fn test_sum() {
        // Basic InjL/InjR
        assert_eq!(parse_alfa_program("L(2)").unwrap(), InjL(Box::new(Num(2))));
        assert_eq!(parse_alfa_program("R(2)").unwrap(), InjR(Box::new(Num(2))));
        // The nicer syntax (spamming parentheses annoys me idk)
        assert_eq!(
            parse_alfa_program("L L R (2)").unwrap(),
            InjL(Box::new(InjL(Box::new(InjR(Box::new(Num(2)))))))
        );
        // Nicer syntax with unit
        assert_eq!(
            parse_alfa_program("L L R ()").unwrap(),
            InjL(Box::new(InjL(Box::new(InjR(Box::new(Unit))))))
        );
        // Fully parenthesized
        assert_eq!(
            parse_alfa_program("L(L(R((2))))").unwrap(),
            InjL(Box::new(InjL(Box::new(InjR(Box::new(Num(2)))))))
        );
        // Parentheses + unit
        assert_eq!(
            parse_alfa_program("L(L(R(())))").unwrap(),
            InjL(Box::new(InjL(Box::new(InjR(Box::new(Unit))))))
        );
    }

    #[test]
    fn test_case() {
        assert_eq!(
            parse_alfa_program("case L(1) of L(x) -> x else R(y) -> y + 1").unwrap(),
            Case(
                Box::new(InjL(Box::new(Num(1)))),
                Id {
                    id: "x".to_string(),
                    typ: None
                },
                Box::new(Var(Id {
                    id: "x".to_string(),
                    typ: None
                })),
                Id {
                    id: "y".to_string(),
                    typ: None
                },
                Box::new(BinOp(
                    Box::new(Var(Id {
                        id: "y".to_string(),
                        typ: None
                    })),
                    Plus,
                    Box::new(Num(1))
                ))
            )
        )
    }
}
