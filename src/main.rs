mod ast;
mod eval;
mod parser;
mod typechecking;

fn run_alfa_prog(prog: &str) -> eval::EvalResult {
    let parsed = parser::parse_alfa_program(prog)?;
    let typed = typechecking::typecheck(parsed)?;
    eval::eval(typed)
}

fn print_result(r: eval::EvalResult) {
    match r {
        Ok(val) => println!("Output: {}", val),
        Err(e) => println!("{}", e)
    }
}

fn example_output(name: &str, prog: &str) {
    println!("Running {}:", name);
    println!("Program: {}", prog);
    print_result(run_alfa_prog(prog));
}

fn with_fix(name: &str, prog: &str) {
    let fix = "fun f -> (fun x -> f(fun v -> (x(x))(v))) (fun x -> f(fun v -> (x(x))(v)))";
    let full_prog = format!("let fix be {} in ({})", fix, prog);
    example_output(name, &full_prog);
}

fn main() {
    example_output("times_1", "let times be fun x -> fun y -> x * y in (times(1))(2)");
    example_output("times_2", "let times be fun (x: Num) -> fun y -> x * y in (times(1))(2)");
    example_output("times_2", "let times be fun x -> fun (y: Num) -> x * y in (times(1))(2)");
    example_output("times_3", "let times be fun (x: Num) -> fun (y: Num) -> x * y in (times(1))(2)");
    example_output("should_reject", "let succ be fun (n: Num) -> n + 1 in (fun (m: Num) -> succ(m))(false)");
    example_output("should_accept", "let succ be fun (n: Num) -> n + 1 in (fun m -> succ(m))(false)");
    // The Z Combinator!
    let fix = "fun f -> (fun x -> f(fun v -> (x(x))(v))) (fun x -> f(fun v -> (x(x))(v)))";
    example_output("The Z-Combinator", fix);
    // If you want to crash things, uncomment the below line :)
    // with_fix("run_forever", "let f be fun a -> a in fix(f)");
    // Factorial
    with_fix("fact_1", "let fact be fun f -> fun n -> if n =? 0 then 1 else n * f(n - 1) in (fix(fact))(4)");
    // We can add typing annotations
    with_fix("fact_2", "let (fact: ? -> Num -> Num) be fun f -> fun (n: Num) -> if n =? 0 then 1 else n * f(n - 1) in (fix(fact))(4)");
    with_fix("fact_2", "let (fact: ? -> ? -> Num) be fun f -> fun (n: Num) -> if n =? 0 then 1 else n * f(n - 1) in (fix(fact))(4)");
}
