use std::io::{self, BufRead, Write};

use lalrpop_util::lalrpop_mod;

mod ast;

lalrpop_mod!(pub lambda);

#[cfg(test)]
mod tests;

trait Eval {
    fn eval(&self) -> Result<ast::Expr, String>;
    fn subst(&self, var: &str, val: &ast::Expr) -> ast::Expr;
}

impl Eval for ast::Expr {
    fn subst(&self, var: &str, val: &ast::Expr) -> ast::Expr {
        match self {
            ast::Expr::Var(x) => if x == var { val.clone() } else { self.clone() },
            ast::Expr::App(e1,e2) => ast::Expr::App(Box::new(e1.subst(var,val)),Box::new(e2.subst(var,val))),
            ast::Expr::Abs(x,e) => if x == var { self.clone() } else { ast::Expr::Abs(x.clone(), Box::new(e.subst(var,val))) } // allows capturing
        }
    }

    fn eval(&self) -> Result<ast::Expr, String> {
        match self {
            ast::Expr::Var(_) => Ok(self.clone()),
            ast::Expr::Abs(_, _) => Ok(self.clone()),
            ast::Expr::App(e1, e2) => {
                let e1 = e1.eval();
                match e1 {
                    Ok(ast::Expr::Abs(param, body)) => {
                        let body = body.subst(&param, &e2);
                        body.eval()
                    }
                    Ok(_) => Err("Invalid application".into()),
                    Err(s) => Err(s)
                }
            }
        }
    }
}

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
            Ok(expr) => { println!("{:?} = {:?}", expr, expr.eval().unwrap())},
            Err(err) => println!("parse error: {}", err),
        }
    }
}
