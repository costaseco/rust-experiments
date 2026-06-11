use crate::ast::Expr;
use crate::lambda;
use crate::Eval;

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

#[test]
fn eval_variable_returns_itself() {
    assert_eq!(parse("x").eval().unwrap(), Expr::Var("x".to_string()));
}

#[test]
fn eval_abstraction_returns_itself() {
    let expr = parse(r"\x. x");
    assert_eq!(expr.eval().unwrap(), expr);
}

#[test]
fn eval_identity_application() {
    assert_eq!(parse(r"(\x. x) y").eval().unwrap(), Expr::Var("y".to_string()));
}

#[test]
fn eval_constant_function_application() {
    // (\x. \y. x) a b => a, the K combinator discarding its second argument.
    assert_eq!(parse(r"(\x. \y. x) a b").eval().unwrap(), Expr::Var("a".to_string()));
}

#[test]
fn eval_application_of_non_function_is_an_error() {
    // `x y` applies the free variable `x`, which is not an abstraction.
    assert!(parse("x y").eval().is_err());
}

// The following tests document a known limitation of `subst`: it is not
// capture-avoiding (see the "allows capturing" comment on `Eval::subst`).
// They encode the *correct*, capture-avoiding result of evaluation and are
// expected to fail against the current naive implementation.

#[test]
fn eval_capturing_substitution_loses_free_variable() {
    // (\x. \y. x) y is the constant function that always returns the free
    // variable `y`, regardless of its argument. Applying it to `z` should
    // therefore still yield `y`.
    //
    // The naive substitution instead substitutes `y` for `x` inside
    // `\y. x`, capturing the free `y` and turning the result into the
    // identity function `\y. y`. Applying that to `z` then yields `z`.
    assert_eq!(parse(r"(\x. \y. x) y z").eval().unwrap(), Expr::Var("y".to_string()));
}

#[test]
fn eval_capturing_substitution_changes_function_identity() {
    // (\x. \y. x) y is alpha-equivalent to a function that ignores its
    // argument and returns `y`, e.g. `\w. y` -- it must NOT be the identity
    // function `\y. y`.
    //
    // The naive substitution captures the free `y`, so the actual result is
    // `\y. y`, which is the identity function and is therefore wrong.
    let result = parse(r"(\x. \y. x) y").eval().unwrap();
    assert_ne!(
        result,
        Expr::Abs("y".to_string(), Box::new(Expr::Var("y".to_string())))
    );
}
