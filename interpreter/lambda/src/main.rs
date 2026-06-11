use std::io::{self, BufRead, Write};
use lalrpop_util::lalrpop_mod;
use std::sync::atomic::{AtomicU64, Ordering};

mod ast;

lalrpop_mod!(pub lambda);

#[cfg(test)]
mod tests;


static FRESH_COUNTER: AtomicU64 = AtomicU64::new(0);

fn fresh_name(base: &str) -> String {
    let n = FRESH_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{base}#{n}")
}

trait Eval {
    fn eval(&self) -> Result<ast::Expr, String>;
    fn subst(&self, var: &str, val: &ast::Expr) -> ast::Expr;
}

impl Eval for ast::Expr {
    fn subst(&self, var: &str, val: &ast::Expr) -> ast::Expr {
        match self {
            ast::Expr::Var(x) => if x == var { val.clone() } else { self.clone() },
            ast::Expr::App(e1,e2) => ast::Expr::App(Box::new(e1.subst(var,val)),Box::new(e2.subst(var,val))),
            ast::Expr::Abs(x,e) => if x == var { self.clone() } else { 
                let new_x = fresh_name(x);
                let new_body = e.subst(x,&ast::Expr::Var(new_x.clone()));
                ast::Expr::Abs(new_x, Box::new(new_body.subst(var,val)))
            } 
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
