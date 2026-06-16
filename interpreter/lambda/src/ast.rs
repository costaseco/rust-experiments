
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Var(String),
    Num(i32),
    Add(Box<Expr>, Box<Expr>),
    Abs(String, Box<Expr>),
    App(Box<Expr>, Box<Expr>),
}
