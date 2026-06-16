

use std::rc::Rc;

pub type Expr = Rc<ExprNode>;

#[derive(Debug, Clone, PartialEq)]
pub enum ExprNode {
    Var(String),
    Num(i32),
    Add(Expr, Expr),
    Abs(String, Expr),
    App(Expr, Expr),
}
