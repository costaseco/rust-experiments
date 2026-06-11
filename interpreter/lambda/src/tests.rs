use crate::ast::Expr;
use crate::lambda;

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
