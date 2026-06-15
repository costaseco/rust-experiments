use std::io::{self, BufRead, Write};
use lalrpop_util::lalrpop_mod;
use std::sync::atomic::{AtomicU64, Ordering};
use std::rc::Rc;
use ast::*;

use crate::Value::Closure;

mod ast;

lalrpop_mod!(pub lambda);

#[cfg(test)]
mod tests;


static COUNTER: AtomicU64 = AtomicU64::new(0);

fn fresh_name(base: &str) -> String {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{base}#{n}")
}

#[derive(Debug, Clone, PartialEq)]
enum Env<V> {
    Empty,
    Node(String, Rc<V>, Rc<Env<V>>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Var(String),
    Closure(String, Box<Expr>, Rc<Env<Value>>),
}

impl<V:Clone> Env<V> {
    fn push(&self, var:&String, val: Rc<V>) -> Env<V> {
        Env::Node(var.clone(), val, Rc::new(self.clone()))
    }

    fn find(&self, var:&String) -> Result<Rc<V>,String> {
        match self {
            Env::Empty => Err(format!("Variable not found {}",var)),
            Env::Node(x,val,next) => 
                if x == var { Ok(val.clone()) } 
                else { next.find(var) }
        }
    }
}

trait Eval {
    fn eval(&self) -> Result<Expr, String>;
    fn eval_env(&self, env:Rc<Env<Value>>) -> Result<Rc<Value>, String>;
    fn subst(&self, var: &str, val: &Expr) -> Expr;
}

impl Eval for Expr {
    fn subst(&self, var: &str, val: &Expr) -> Expr {
        match self {
            Expr::Var(x) => if x == var { val.clone() } else { self.clone() },
            Expr::App(e1,e2) => Expr::App(Box::new(e1.subst(var,val)),Box::new(e2.subst(var,val))),
            Expr::Abs(x,e) => if x == var { self.clone() } else { 
                let new_x = fresh_name(x);
                let new_body = e.subst(x,&Expr::Var(new_x.clone()));
                Expr::Abs(new_x, Box::new(new_body.subst(var,val)))
            } 
        }
    }

    fn eval(&self) -> Result<Expr, String> {
        match self {
            Expr::Var(_) => Ok(self.clone()),
            Expr::Abs(_, _) => Ok(self.clone()),
            Expr::App(e1, e2) => {
                let e1 = e1.eval();
                match e1 {
                    Ok(Expr::Abs(param, body)) => {
                        let body = body.subst(&param, &e2);
                        body.eval()
                    }
                    Ok(_) => Err("Invalid application".into()),
                    Err(s) => Err(s)
                }
            }
        }
    }

    fn eval_env(&self, env: Rc<Env<Value>>) -> Result<Rc<Value>, String> {
        match self {
            Expr::Var(x) => env.find(x).map(|v: Rc<Value>| v.clone()).or(Ok(Rc::new(Value::Var(x.clone())))),
            Expr::Abs(x, e) => Ok(Rc::new(Closure(x.clone(), e.clone(), env.clone()))),
            Expr::App(e1, e2) => {
                let v1 = e1.eval_env(env.clone());
                let v2 = e2.eval_env(env);
                match v1 {
                    Ok(rc) => match rc.as_ref() {
                            Value::Closure(param, body, env_closure) => {
                            match v2 {
                                Ok(v2  ) =>
                                    body.eval_env(Rc::new(env_closure.clone().push(param, v2.clone()))),
                                Err(_) => v2
                            }
                        },
                        _ => Err("Invalid application".into())
                    },
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
            Ok(expr) => { println!("{:?} = {:?} = {:?}", expr, expr.eval().unwrap(), expr.eval_env(Rc::new(Env::Empty)).unwrap())},
            Err(err) => println!("parse error: {}", err),
        }
    }
}
