mod ast;
mod eval;
mod parser;
mod typechecking;

fn main() {
    println!(
        "parsed: {:?}",
        parser::parse_alfa_program("fun (x: Num + Num -> Num) -> x").unwrap()
    );
}
