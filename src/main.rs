mod parser;
mod ast;

fn main() {
    println!("parsed: {:?}", parser::parse_alfa_program("2 + 3 * 6 + 4").unwrap());
}
