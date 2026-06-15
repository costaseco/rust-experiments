use std::rc::Rc;

use crate::ast::Expr;
use crate::lambda;
use crate::{Env, Eval, Value};

fn parse(input: &str) -> Expr {
    *lambda::ExprParser::new().parse(input).unwrap()
}

#[test]
fn parses_variable() {
    assert_eq!(parse("x"), Expr::Var("x".to_string()));
}

#[test]
fn parses_abstraction() {
    assert_eq!(
        parse(r"\x. x"),
        Expr::Abs("x".to_string(), Box::new(Expr::Var("x".to_string())))
    );
}

#[test]
fn parses_application_left_associative() {
    assert_eq!(
        parse("x y z"),
        Expr::App(
            Box::new(Expr::App(
                Box::new(Expr::Var("x".to_string())),
                Box::new(Expr::Var("y".to_string()))
            )),
            Box::new(Expr::Var("z".to_string()))
        )
    );
}

#[test]
fn abstraction_body_extends_as_far_as_possible() {
    assert_eq!(
        parse(r"\x. x y"),
        Expr::Abs(
            "x".to_string(),
            Box::new(Expr::App(
                Box::new(Expr::Var("x".to_string())),
                Box::new(Expr::Var("y".to_string()))
            ))
        )
    );
}

#[test]
fn parses_parenthesized_expression() {
    assert_eq!(
        parse(r"(\x. x) y"),
        Expr::App(
            Box::new(Expr::Abs(
                "x".to_string(),
                Box::new(Expr::Var("x".to_string()))
            )),
            Box::new(Expr::Var("y".to_string()))
        )
    );
}

#[test]
fn parses_nested_abstractions() {
    assert_eq!(
        parse(r"\x. \y. x y"),
        Expr::Abs(
            "x".to_string(),
            Box::new(Expr::Abs(
                "y".to_string(),
                Box::new(Expr::App(
                    Box::new(Expr::Var("x".to_string())),
                    Box::new(Expr::Var("y".to_string()))
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
}

/// Evaluates `input` both by substitution (`eval`) and by environment
/// (`eval_env`, starting from an empty environment).
fn eval_both(input: &str) -> (Result<Expr, String>, Result<Rc<Value>, String>) {
    let expr = parse(input);
    let by_subst = expr.eval();
    let by_env = expr.eval_env(Rc::new(Env::Empty));
    (by_subst, by_env)
}

/// Asserts both evaluation strategies agree that `input` reduces to the free
/// variable `name`.
fn assert_reduces_to_var(input: &str, name: &str) {
    let (by_subst, by_env) = eval_both(input);
    assert_eq!(by_subst.unwrap(), Expr::Var(name.to_string()));
    assert_eq!(*by_env.unwrap(), Value::Var(name.to_string()));
}

/// Asserts both evaluation strategies agree that `input` is ill-formed
/// (applies something that isn't a function).
fn assert_invalid_application(input: &str) {
    let (by_subst, by_env) = eval_both(input);
    assert!(by_subst.is_err());
    assert!(by_env.is_err());
}

#[test]
fn eval_variable_returns_itself() {
    assert_reduces_to_var("x", "x");
}

#[test]
fn eval_abstraction_returns_itself() {
    let expr = parse(r"\x. x");
    assert_eq!(expr.eval().unwrap(), expr);

    match expr.eval_env(Rc::new(Env::Empty)).unwrap().as_ref() {
        Value::Closure(param, body, _) => {
            assert_eq!(param, "x");
            assert_eq!(**body, Expr::Var("x".to_string()));
        }
        v => panic!("expected a closure, got {v:?}"),
    }
}

#[test]
fn eval_identity_application() {
    assert_reduces_to_var(r"(\x. x) y", "y");
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

#[test]
fn eval_capturing_substitution_loses_free_variable() {
    // (\x. \y. x) y is the constant function that always returns the free
    // variable `y`, regardless of its argument. Applying it to `z` should
    // therefore still yield `y`. Naive substitution would instead capture
    // the free `y` and turn the result into the identity function `\y. y`,
    // making this evaluate to `z`. `eval_env` never has this problem, since
    // it never substitutes terms into each other -- free variables are
    // resolved by environment lookup instead.
    assert_reduces_to_var(r"(\x. \y. x) y z", "y");
}

#[test]
fn eval_capturing_substitution_changes_function_identity() {
    // (\x. \y. x) y is alpha-equivalent to a function that ignores its
    // argument and returns `y`, e.g. `\w. y` -- it must NOT be the identity
    // function `\y. y`. This is a property of `eval`'s *syntactic* output;
    // `eval_env` represents the result as a closure capturing `y` in its
    // environment, so there's no equivalent "looks like the identity
    // function" representation to check here.
    let result = parse(r"(\x. \y. x) y").eval().unwrap();
    assert_ne!(
        result,
        Expr::Abs("y".to_string(), Box::new(Expr::Var("y".to_string())))
    );
}
