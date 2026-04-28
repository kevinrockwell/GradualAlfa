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
    if let Rule::vardec = pair.as_rule() {
        let mut inner = pair.into_inner();
        Id {
            id: inner.next().unwrap().to_string(),
            typ: parse_alfatype(inner.next(), pratt),
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
            // TODO: can bool/unit be removed?
            Rule::expr => parse_expr(primary, pratt),
            _ => Err(format!("Unexpected arithmetic atom, got {:?}", primary)),
        })
        .map_infix(|lhs, op, rhs| {
            let binop = match op.as_rule() {
                Rule::plus => BinOp::Plus,
                Rule::minus => BinOp::Minus,
                Rule::times => BinOp::Times,
                Rule::greater => BinOp::GreaterThan,
                Rule::less => BinOp::LessThan,
                Rule::eq => BinOp::EqualTo,
                _ => return Err(format!("Unexpected infix operator {:?}", op)),
            };
            Ok(Expr::BinaryExpr(Box::new(lhs?), binop, Box::new(rhs?)))
        })
        .map_prefix(|op, expr| match op.as_rule() {
            Rule::neg => Ok(Expr::UnaryExpr(UnOp::Neg, Box::new(expr?))),
            _ => return Err(format!("Unexpected prefix operator {:?}", op)),
        })
        .parse(pairs)
}

fn parse_expr(pair: Pair<Rule>, pratt: &PrattParser<Rule>) -> ParseResult {
    assert!(pair.as_rule() == Rule::expr, "parse_expr requires an expr");
    let current_rule = pair.into_inner().next().unwrap();
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
            let body = parse_expr(inner.next().unwrap(), pratt)?;
            Ok(Expr::Let(id, Box::new(body)))
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
        Rule::expr => parse_expr(current_rule, pratt), // ( expr )
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
        .op(Op::prefix(Rule::neg));
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
