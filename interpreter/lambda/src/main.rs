use std::io::{self, BufRead, Write};
use lalrpop_util::lalrpop_mod;
use std::sync::atomic::{AtomicU64, Ordering};
use std::rc::Rc;
use ast::*;

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
pub enum Env<V> {
    Empty,
    Node(String, Rc<V>, Rc<Env<V>>),
}

type Value = Rc<ValueNode>;
#[derive(Debug, Clone, PartialEq)]
pub enum ValueNode {
    Var(String),
    Num(i32),
    Closure(String, Value, Rc<Env<Value>>),
}

/// A lambda term using De Bruijn indices: bound variables are represented by
/// their binding depth (0 = the nearest enclosing `Abs`), so `Abs` no longer
/// needs to carry a name. Free variables are kept by name, matching how
/// `Expr::Var`/`Value::Var` already treat them as ordinary values rather than
/// errors.
type ExprDb = Rc<ExprDbNode>;

#[derive(Debug, Clone, PartialEq)]
enum ExprDbNode {
    Var(usize, String),
    Free(String),
    Num(i32),
    Add(ExprDb, ExprDb),
    Abs(ExprDb),
    App(ExprDb, ExprDb),
}


impl<V:Clone> Env<V> {
    fn push(&self, var: &String, val: Rc<V>) -> Env<V> {
        Env::Node(var.clone(), val, Rc::new(self.clone()))
    }

    fn find(&self, var: &String) -> Result<Rc<V>,String> {
        match self {
            Env::Empty => Err(format!("Variable not found {}",var)),
            Env::Node(x,val,next) => 
                if x == var { Ok(val.clone()) } 
                else { next.find(var) }
        }
    }

    fn find_idx(&self, var: &String) -> Result<usize,String> {
        match self {
            Env::Empty => Err(format!("Variable not found {}",var)),
            Env::Node(x, _, next) => 
                if x == var { Ok(0) } 
                else { next.find_idx(var).map(|idx| idx+1) }
        }
    } 
}

trait Eval {
    fn eval(&self) -> Result<Rc<Self>, String>;
}

trait EvalEnv {
    fn eval_env(&self, env:Rc<Env<Value>>) -> Result<Value, String>;
}

trait Subst<T> {
    fn subst(&self, var: &T, val: &Self) -> Self;
}

trait ToDebruijn {
    fn to_debruijn(&self, env: &Env<()>) -> ExprDb;
}

impl Subst<String> for Expr {
    fn subst(&self, var: &String, val: &Expr) -> Expr {
        match self.as_ref() {
            ExprNode::Var(x) => if x == var { val.clone() } else { self.clone() },

            ExprNode::App(e1,e2) => 
                Rc::new(ExprNode::App(e1.subst(var,val),e2.subst(var,val))),

            ExprNode::Abs(x,e) => if x == var { self.clone() } else { 
                let new_x = fresh_name(x);
                let new_body = e.subst(x,&Rc::new(ExprNode::Var(new_x.clone())));
                Rc::new(ExprNode::Abs(new_x, new_body.subst(var,val)))
            }, 

            ExprNode::Num(_) => self.clone(),

            ExprNode::Add(e1,e2) => 
                Rc::new(ExprNode::Add(e1.subst(var,val),e2.subst(var,val))),
        }
    }

}

impl ToDebruijn for Expr {
    fn to_debruijn(&self, env: &Env<()>) -> ExprDb {
        match self.as_ref() {
            ExprNode::Var(x) => {
                let res_idx = env.find_idx(x);
                match res_idx {
                    Ok(idx) => Rc::new(ExprDbNode::Var(idx, x.clone())),
                    Err(_) => Rc::new(ExprDbNode::Free(x.clone()))
                }
            }
            ExprNode::Abs(x, exp) => {
                let new_env = env.push(x, Rc::new(()));
                Rc::new(ExprDbNode::Abs(exp.to_debruijn(&new_env)))
            },
            ExprNode::App(e1,e2) => 
                Rc::new(ExprDbNode::App(e1.to_debruijn(&env),e2.to_debruijn(&env))),

            ExprNode::Num(n) => Rc::new(ExprDbNode::Num(*n)),

            ExprNode::Add(e1,e2) => 
                Rc::new(ExprDbNode::Add(e1.to_debruijn(&env),e2.to_debruijn(&env)))
        }
    }
}

impl Subst<usize> for ExprDb {
    fn subst(&self, var: &usize, val: &ExprDb) -> ExprDb {
        match self.as_ref() {
            ExprDbNode::Var(x, _) => if x == var { val.clone() } else { self.clone() },

            ExprDbNode::App(e1, e2) => 
                Rc::new(ExprDbNode::App(e1.subst(var,val),e2.subst(var,val))),

            ExprDbNode::Abs(body) => 
                Rc::new(ExprDbNode::Abs(body.subst(&(var+1),val))),

            ExprDbNode::Free(..) => self.clone(),

            ExprDbNode::Num(_) => self.clone(),

            ExprDbNode::Add(e1, e2) => 
                Rc::new(ExprDbNode::Add(e1.subst(var,val),e2.subst(var,val)))
        }
    }

}

impl Eval for ExprNode {
    fn eval(&self) -> Result<Expr, String> {
        match self {
            ExprNode::Var(_) => Ok(self),

            ExprNode::Abs(..) => Ok(self),

            ExprNode::Num(_) => Ok(self.clone()),

            ExprNode::App(e1, e2) => {
                let e1 = e1.eval();
                match e1 {
                    Ok(rc) => match rc.as_ref() {
                        ExprNode::Abs(param, body) => {
                            let body = body.subst(&param, &e2);
                            body.eval()
                        },
                        _ => Err("Not an abstraction".into())
                    },
                    Err(s) => Err(s)
                }
            },

            ExprNode::Add(e1,e2) => {
                let v1 = e1.eval();
                let v2 = e2.eval();
                match (v1,v2) {
                    (Ok(n), Ok(m)) => match (n.as_ref(),m.as_ref()) {
                        (ExprNode::Num(n), ExprNode::Num(m)) => Ok(Rc::new(ExprNode::Num(*n + *m))),
                        _ => Err("Expecting integers".into())
                    },
                    (Err(s),_) => Err(s),
                    (_,Err(s)) => Err(s)
                }
            }
        }
    }
}

impl EvalEnv for Expr {

    fn eval_env(&self, env: Rc<Env<Value>>) -> Result<Value, String> {
        match self.as_ref() {
            ExprNode::Var(x) => 
                env
                .find(x)
                .map(|v| v.clone())
                .or(Ok(ValueNode::Var(x.clone()))),
            ExprNode::Abs(x, e) => Ok(Rc::new(ValueNode::Closure(x.clone(), e.clone(), env.clone()))),
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
            Expr::Num(x) => Ok(Rc::new(Value::Num(*x))),
            Expr::Add(e1,e2) => {
                let v1 = e1.eval_env(env.clone());
                let v2 = e2.eval_env(env);
                match (v1,v2) {
                    (Ok(n), Ok(m)) => match (n.as_ref(),m.as_ref()) {
                        (Value::Num(n), Value::Num(m)) => Ok(Rc::new(Value::Num(*n + *m))),
                        _ => Err("Expecting integers".into())
                    },
                    (Err(s),_) => Err(s),
                    (_,Err(s)) => Err(s)
                }
            }
        }
    }
}

impl ExprDb {
    fn shift(&self, barrier:usize) -> ExprDb {
        match self {
            ExprDbNode::Var(idx,name) => 
                if *idx >= barrier { ExprDbNode::Var(idx+1, name.clone()) } else { self.clone() },
            
            ExprDbNode::Free(_) => self.clone(),
            
            ExprDbNode::Abs(body) => 
                ExprDbNode::Abs(Box::new(body.shift(barrier+1))),

            ExprDbNode::App(e1, e2) => 
                ExprDbNode::App(Box::new(e1.shift(barrier)), Box::new(e2.shift(barrier))),
                
            ExprDbNode::Num(_) => self.clone(),

            ExprDbNode::Add(e1, e2) => 
                ExprDbNode::Add(Box::new(e1.shift(barrier)), Box::new(e2.shift(barrier))),
            }
    }
}

impl Eval for ExprDb {

    fn eval(&self) -> Result<Rc<ExprDb>, String> {
        match self {
            ExprDbNode::Var(..) => Err("Should not occur".to_string()),
            ExprDbNode::Free(..) => Ok(Rc::new(self.clone())),
            ExprDbNode::Num(_) => Ok(Rc::new(self.clone())),
            ExprDbNode::Add(e1,e2) => {
                let v1 = e1.eval();
                let v2 = e2.eval();
                match (v1,v2) {
                    (Ok(n), Ok(m)) => match (n.as_ref(),m.as_ref()) {
                        (ExprDbNode::Num(n), ExprDbNode::Num(m)) => Ok(Rc::new(ExprDbNode::Num(*n + *m))),
                        _ => Err("Expecting integers".into())
                    },
                    (Err(s),_) => Err(s),
                    (_,Err(s)) => Err(s)
                }
            }

            ExprDbNode::Abs(..) =>  Ok(Rc::new(self.clone())),
            ExprDbNode::App(e1, e2) => {
                let e1 = e1.eval();
                match e1 {
                    Ok(rc) => match rc.as_ref() {
                        ExprDbNode::Abs(body) => {
                            let shifted_arg = e2.shift(0);
                            let body = body.subst(&0, &shifted_arg);
                            body.eval()
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
            Ok(expr) => { 
                print!("{:?} = ", expr); 
                println!("{:?} = {:?} = {:?}", 
                    expr.eval().unwrap(), 
                    expr.eval_env(Rc::new(Env::Empty)).unwrap(),
                    expr.to_debruijn(&Env::Empty).eval().unwrap())
                },
            Err(err) => println!("parse error: {}", err),
        }
    }
}
