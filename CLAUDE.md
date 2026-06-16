# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository overview

This is a collection of standalone Rust experiments/learning projects. Each subproject under a top-level category directory (e.g. `interpreter/`) is its own independent Cargo crate with its own `Cargo.toml`. There is no workspace-level `Cargo.toml` tying them together — always `cd` into the specific crate before running `cargo` commands.

## Projects

### interpreter/lambda

A lambda calculus interpreter built with [LALRPOP](https://lalrpop.github.io/lalrpop/) as the parser generator.

- `src/ast.rs` — the `Expr` AST: `Var(String)`, `Abs(String, Box<Expr>)` (lambda abstraction), `App(Box<Expr>, Box<Expr>)` (application).
- `src/lambda.lalrpop` — the LALRPOP grammar. Defines syntax `\x. body` for abstraction and juxtaposition for application (left-associative). `build.rs` runs `lalrpop::process_root()` at build time to generate the parser module from this file (generated code lands in `target/`, not committed).
- `src/main.rs` — a REPL: reads a line, parses it into an `Expr`, then evaluates it via the `Eval` trait (`subst` for substitution, `eval` for call-by-name beta reduction to weak head normal form). Type `exit` or `quit` to leave the REPL.
- `src/tests.rs` — parser tests (included via `#[cfg(test)] mod tests;` in `main.rs`).

Note: `subst` does not avoid variable capture (see comment "allows capturing" in `main.rs`), so this interpreter is intentionally a minimal/naive implementation rather than a fully correct one.

#### Commands (run from `interpreter/lambda/`)

```sh
cargo build           # build the crate (also regenerates the LALRPOP parser)
cargo run             # start the REPL
cargo test            # run all tests in src/tests.rs
cargo test <name>     # run a single test, e.g. `cargo test parses_abstraction`
```
