# Language Reference

This page describes RR syntax and semantics based on `src/syntax/*`, `src/hir/*`, and MIR lowering rules.

## Keywords

- `fn`, `function` (alias), `let`
- `if`, `else`
- `while`, `for`, `in`
- `return`, `break`, `next`
- `match`
- `import`, `export`
- literals: `true/false`, `null`, `na` (case-insensitive forms supported for booleans/null/na in lexer)

## Literals

- Integer: `1`, `42`, `1L`
- Float: `1.0`, `.5`
- String: `"text"` (with escaped forms)
- Boolean: `true`, `false`
- Null: `null`
- NA: `na`
- Vector literal: `[1, 2, 3]`
- Record literal: `{a: 1, b: 2}`

## Statements

- Preferred declaration/assignment: `x <- expr` (or `x = expr`)
- Explicit declaration (legacy/strict style): `let x = expr;`
- Compound assignment (native style): `x += expr`, `x -= expr`, `x *= expr`, `x /= expr`, `x %= expr`
- Typed declaration hints:
  - `let x: int = 10L`
  - `x: int = 10L` (declaration sugar)
- Function declaration: `f <- function(a, b) { ... }`
- Legacy alias function declaration: `fn f(a, b) { ... }`
- Control flow: `if`, `while`, `for`
- Return: `return expr;` or `return;`
- Loop control: `break;`, `next;`
- Module: `import "path.rr";`, `export fn ...`

## Expressions

- Unary: `-x`, `!x`
- Binary: `+ - * / % %*% == != < <= > >= && ||`
- Range: `a .. b`
- Call: `f(x, y)`, named args `f(x = 1, y = 2)`
- Index: `x[i]`, `m[i, j]`
- Field: `rec.a`
- Lambda: `function(x) { x + 1 }` (legacy alias: `fn(x) { return x + 1; }`)
- Pipe: `x |> f(1)`
- Try postfix: `expr?`
- Match: `match (v) { ... }`
- Column/unquote syntax tokens: `@name`, `^expr` (lowered through HIR tidy/unquote forms)

## Dotted Identifiers

RR supports dotted names (e.g. `solve.cg`, `idx.cube`) for function/variable identifiers.

Disambiguation rule:

- If the root name is bound in local scope, `a.b` is treated as field access.
- If the root name is unbound in local scope, `a.b` can be treated as a dotted symbol name.

This keeps `rec.x` field semantics intact while allowing R-style dotted API names.

## Operator Notes

- `%*%` is recognized as matrix multiplication token.
- `&&` and `||` both map to logical operators.
- `|>` is parsed and lowered as call rewriting.

## Pattern Matching

Supported pattern kinds:

- wildcard `_`
- literal patterns
- variable binding
- list pattern `[a, b, ..rest]`
- record pattern `{a: x, b: 1}`

Current limitation:

- record rest pattern (`{a: x, ..rest}`) is not supported.

## Semicolon Policy

Semicolons are optional across statement boundaries, except when two statements are on the same line.

If a new statement starts on the same line without `;`, parser raises:

- `Missing ';' before ... on the same line`

Additional parser rule:

- Postfix operators (`(` call, `[` index, `.` field) do not continue across a newline.
  - This keeps single-line control bodies stable:
    - `if (c) x <- 1`
    - next line starts a new statement, not a postfix chain of `x`.

## Assignment Policy (`let` vs `<-`)

RR accepts both `=` and `<-` assignment operators.

If assigning to an undeclared variable:

- default: implicit declaration is allowed (no warning)
- strict mode (`RR_STRICT_LET=1` or `RR_STRICT_ASSIGN=1`): treated as compile error

Notes:

- Parser treats `=` and `<-` as equivalent assignment operators.
- Optional warnings can be enabled with `RR_WARN_IMPLICIT_DECL=1`.
- Recommended style for user code: `name <- ...` and `name <- function(...) { ... }`.

## Functions and Closures

- Global functions are typically authored as `name <- function(...) { ... }`.
- `fn name(...) { ... }` remains supported as legacy alias form.
- `function(...) { ... }` is accepted as lambda form (legacy alias: `fn(...) { ... }`).
- Expression-bodied function syntax is supported:
  - `fn add(a, b) = a + b`
- Type hints are supported:
  - parameter: `fn add(a: float, b: int) { ... }`
  - return: `fn add(a: float, b: float) -> float { ... }`
  - native aliases also work: `f64/f32`, `i64/i32`
- Tail expression implicit return is supported in function/lambda bodies when no explicit `return` appears:
  - `function(a, b) { a + b }` returns `a + b`.
- Top-level function alias assignment (`name <- function(...) { ... }`) is tracked so calls resolve to lifted function symbols.
- Parameter defaults are supported in syntax:
  - `f <- function(a = 0.0, b = 0L) { a + b }`
  - defaults are lowered into HIR parameter metadata and used as type hints (`Double`, `Int`, etc.).
- Lambda expressions are lambda-lifted by HIR lowering.
- Captures are packed via runtime helpers:
  - `rr_closure_make`
  - `rr_call_closure`

## Single-Line Control Forms

`if/else`, `while`, and `for` accept either block bodies or single statements.
`if` and `while` conditions also accept optional parentheses in R++ style.
`for` supports both R-style and native-style headers.

Examples:

- `if (x < 1) y <- 1 else y <- 2`
- `if x < 1 { y <- 1 } else { y <- 2 }`
- `while (i < n) i <- i + 1`
- `while i < n { i <- i + 1 }`
- `for (i in 1..n) s <- s + i`
- `for i in 1..n { s += i }`

## Dynamic Builtins and Hybrid Handling

Calls to dynamic runtime features are marked as `unsupported_dynamic` in MIR and handled conservatively:

- `eval`, `parse`, `get`, `assign`, `exists`, `mget`, `rm`, `ls`
- `parent.frame`, `environment`, `sys.frame`, `sys.call`, `do.call`

These functions still emit runnable R code, but optimization is intentionally restricted for safety.
