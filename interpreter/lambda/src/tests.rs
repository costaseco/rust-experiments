use std::rc::Rc;

use crate::ast::Expr;
use crate::ast::ExprNode;
use crate::lambda;
use crate::{Env, Eval, EvalEnv, ExprDb, ToDebruijn, Value, ValueNode, ExprDbNode};

fn parse(input: &str) -> Expr {
    lambda::ExprParser::new().parse(input).unwrap()
}

#[test]
fn parses_variable() {
    assert_eq!(*parse("x"), ExprNode::Var("x".to_string()));
}

#[test]
fn parses_integers() {
    assert_eq!(*parse("32"), ExprNode::Num(32));
}

#[test]
fn parses_abstraction() {
    assert_eq!(
        *parse(r"\x. x"),
        ExprNode::Abs("x".to_string(), Rc::new(ExprNode::Var("x".to_string())))
    );
}

#[test]
fn parses_application_left_associative() {
    assert_eq!(
        *parse("x y z"),
        ExprNode::App(
            Rc::new(ExprNode::App(
                Rc::new(ExprNode::Var("x".to_string())),
                Rc::new(ExprNode::Var("y".to_string()))
            )),
            Rc::new(ExprNode::Var("z".to_string()))
        )
    );
}

#[test]
fn abstraction_body_extends_as_far_as_possible() {
    assert_eq!(
        *parse(r"\x. x y"),
        ExprNode::Abs(
            "x".to_string(),
            Rc::new(ExprNode::App(
                Rc::new(ExprNode::Var("x".to_string())),
                Rc::new(ExprNode::Var("y".to_string()))
            ))
        )
    );
}

#[test]
fn parses_parenthesized_expression() {
    assert_eq!(
        *parse(r"(\x. x) y"),
        ExprNode::App(
            Rc::new(ExprNode::Abs(
                "x".to_string(),
                Rc::new(ExprNode::Var("x".to_string()))
            )),
            Rc::new(ExprNode::Var("y".to_string()))
        )
    );
}

#[test]
fn parses_nested_abstractions() {
    assert_eq!(
        *parse(r"\x. \y. x y"),
        ExprNode::Abs(
            "x".to_string(),
            Rc::new(ExprNode::Abs(
                "y".to_string(),
                Rc::new(ExprNode::App(
                    Rc::new(ExprNode::Var("x".to_string())),
                    Rc::new(ExprNode::Var("y".to_string()))
                ))
            ))
        )
    );
}

#[test]
fn rejects_invalid_syntax() {
    assert!(lambda::ExprParser::new().parse(r"\x.").is_err());
    assert!(lambda::ExprParser::new().parse("(x").is_err());
    assert!(lambda::ExprParser::new().parse("").is_err());
    assert!(lambda::ExprParser::new().parse("01").is_err());
}

/// Evaluates `input` via substitution (`eval`), environment (`eval_env`), and
/// De Bruijn index evaluation (convert then `eval`), all starting from an empty
/// environment.
fn eval_all(
    input: &str,
) -> (
    Result<Expr, String>,
    Result<Value, String>,
    Result<ExprDb, String>,
) {
    let expr = parse(input);
    let by_subst = expr.eval();
    let by_env = expr.eval_env(Rc::new(Env::Empty));
    let by_db = expr.to_debruijn(&Env::Empty).eval();
    (by_subst, by_env, by_db)
}

/// Asserts all three evaluation strategies agree that `input` reduces to the
/// free variable `name`.
fn assert_reduces_to_var(input: &str, name: &str) {
    let (by_subst, by_env, by_db) = eval_all(input);
    assert_eq!(*by_subst.unwrap(), ExprNode::Var(name.to_string()));
    assert_eq!(*by_env.unwrap(), ValueNode::Var(name.to_string()));
    assert_eq!(*by_db.unwrap(), ExprDbNode::Free(name.to_string()));
}

/// Asserts all three evaluation strategies agree that `input` reduces to the
/// number `value`.
fn assert_reduces_to_number(input: &str, value: i32) {
    let (by_subst, by_env, by_db) = eval_all(input);
    assert_eq!(*by_subst.unwrap(), ExprNode::Num(value));
    assert_eq!(*by_env.unwrap(), ValueNode::Num(value));
    assert_eq!(*by_db.unwrap(), ExprDbNode::Num(value));
}

/// Asserts all three evaluation strategies agree that `input` is ill-formed
/// (applies something that isn't a function).
fn assert_invalid_application(input: &str) {
    let (by_subst, by_env, by_db) = eval_all(input);
    assert!(by_subst.is_err());
    assert!(by_env.is_err());
    assert!(by_db.is_err());
}

#[test]
fn eval_variable_returns_itself() {
    assert_reduces_to_var("x", "x");
}

#[test]
fn eval_numbers() {
    assert_reduces_to_number("1", 1);
    assert_reduces_to_number("123", 123);
}

#[test]
fn eval_add() {
    assert_reduces_to_number("1+1", 2);
    assert_reduces_to_number("1+1+1+1", 4);
    assert_reduces_to_number("123+321", 444);
}

#[test]
fn eval_abstraction_returns_itself() {
    let expr = parse(r"\x. x");
    assert_eq!(*expr.eval().unwrap(), *expr);

    match expr.eval_env(Rc::new(Env::Empty)).unwrap().as_ref() {
        ValueNode::Closure(param, body, _) => {
            assert_eq!(param, "x");
            assert_eq!(**body, ExprNode::Var("x".to_string()));
        }
        v => panic!("expected a closure, got {v:?}"),
    }

    // \x. x in De Bruijn is Abs(Var(0, "x")); eval returns it unchanged (it's a value).
    assert_eq!(
        *expr.to_debruijn(&Env::Empty).eval().unwrap(),
        ExprDbNode::Abs(Rc::new(ExprDbNode::Var(0, "x".to_string())))
    );
}

#[test]
fn eval_identity_application() {
    assert_reduces_to_var(r"(\x. x) y", "y");
}

#[test]
fn eval_functions_and_numbers() {
    assert_reduces_to_number(r"(\x. x) 1", 1);
    assert_reduces_to_number(r"(\x. (1+1)) 1", 2);
    assert_reduces_to_number(r"(\x. x+1) 1", 2);
}

#[test]
fn eval_constant_function_application() {
    // (\x. \y. x) a b => a, the K combinator discarding its second argument.
    assert_reduces_to_var(r"(\x. \y. x) a b", "a");
}

#[test]
fn eval_application_of_non_function_is_an_error() {
    // `x y` applies the free variable `x`, which is not an abstraction.
    assert_invalid_application("x y");
}

// The following tests document a known limitation of `subst`: it is not
// capture-avoiding (see the "allows capturing" comment on `Eval::subst`).
// They encode the *correct*, capture-avoiding result of evaluation.
// De Bruijn evaluation is capture-free by construction and agrees with the
// correct result in both cases.

#[test]
fn eval_capturing_substitution_loses_free_variable() {
    // (\x. \y. x) y is the constant function that always returns the free
    // variable `y`, regardless of its argument. Applying it to `z` should
    // therefore still yield `y`. Naive substitution would instead capture
    // the free `y` and turn the result into the identity function `\y. y`,
    // making this evaluate to `z`. Neither `eval_env` nor De Bruijn eval
    // have this problem.
    assert_reduces_to_var(r"(\x. \y. x) y z", "y");
}

#[test]
fn eval_capturing_substitution_changes_function_identity() {
    // (\x. \y. x) y must NOT be the identity function `\y. y`.
    // For substitution-based eval the result is a renamed abstraction;
    // for De Bruijn eval it is Abs(Free("y")) — a function that ignores its
    // argument and returns the free `y`, not the bound one.
    let result = parse(r"(\x. \y. x) y").eval().unwrap();
    assert_ne!(
        *result,
        ExprNode::Abs("y".to_string(), Rc::new(ExprNode::Var("y".to_string())))
    );

    let db_result = parse(r"(\x. \y. x) y").to_debruijn(&Env::Empty).eval().unwrap();
    assert_ne!(
        *db_result,
        ExprDbNode::Abs(Rc::new(ExprDbNode::Var(0, "y".to_string())))
    );
    assert_eq!(
        *db_result,
        ExprDbNode::Abs(Rc::new(ExprDbNode::Free("y".to_string())))
    );
}

#[test]
fn eval_self_application() {
    // (\x. x x) (\y. y) — self-application of the identity function.
    // Exercises substituting a lambda as both function and argument simultaneously.
    let expr = parse(r"(\x. x x) (\y. y)");

    assert_eq!(
        *expr.eval().unwrap(),
        ExprNode::Abs("y".to_string(), Rc::new(ExprNode::Var("y".to_string())))
    );

    match expr.eval_env(Rc::new(Env::Empty)).unwrap().as_ref() {
        ValueNode::Closure(param, body, _) => {
            assert_eq!(param, "y");
            assert_eq!(**body, ExprNode::Var("y".to_string()));
        }
        v => panic!("expected a closure, got {v:?}"),
    }

    assert_eq!(
        *expr.to_debruijn(&Env::Empty).eval().unwrap(),
        ExprDbNode::Abs(Rc::new(ExprDbNode::Var(0, "y".to_string())))
    );
}

#[test]
fn eval_ski_combinator() {
    // S K K w = w — the SKI algebraic identity.
    // Requires four nested beta reductions through three distinct variable
    // scopes; stresses that indices/environments stay consistent across reductions.
    // S = \x.\y.\z. x z (y z),  K = \a.\b. a
    assert_reduces_to_var(
        r"(\x.\y.\z. x z (y z)) (\a.\b.a) (\a.\b.a) w",
        "w",
    );
}

#[test]
fn eval_church_succ_zero() {
    // Church encoding: succ zero, when applied to the identity and a free
    // variable, returns that variable.  succ zero = one, so one id a = id a = a.
    // succ = \n.\f.\x. f (n f x),  zero = \f.\x. x,  id = \y. y
    assert_reduces_to_var(
        r"(\n.\f.\x. f (n f x)) (\f.\x.x) (\y.y) a",
        "a",
    );
}
