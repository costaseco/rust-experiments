use std::io::{self, BufRead, Write};

use lalrpop_util::lalrpop_mod;

mod ast;

lalrpop_mod!(pub lambda);

#[cfg(test)]
mod tests;

fn main() {
    let parser = lambda::ExprParser::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("> ");
        stdout.flush().unwrap();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap() == 0 {
            break;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "exit" || line == "quit" {
            break;
        }

        match parser.parse(line) {
            Ok(expr) => println!("{:?}", expr),
            Err(err) => println!("parse error: {}", err),
        }
    }
}
