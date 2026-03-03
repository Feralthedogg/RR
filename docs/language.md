# Language Reference

This page documents RR language behavior from implementation code, not aspirational design.
Primary sources: `src/syntax/{token,lex,parse,ast}.rs`, `src/hir/lower.rs`, `src/mir/lower_hir.rs`, and syntax-focused tests.

## Keywords

- `fn`, `function` (`function` lexes as `fn`)
- `let`
- `if`, `else`
- `while`, `for`, `in`
- `return`, `break`, `next`
- `match`
- `import`, `export`

Literal keywords:

- booleans: `true`, `false`, `TRUE`, `FALSE`
- null: `null`, `NULL`
- missing: `na`, `NA`

## Modern vs. Traditional Syntax Examples

RR allows developers to write in a modern, Rust-like syntax, while fully supporting traditional R syntax to ensure seamless portability of existing code. You can mix and match these styles.

### 1. Assignment and Declarations
**Traditional R:**
```R
x <- 10L
y <- "hello"
```
**Modern RR:**
```rust
let x = 10L
let y = "hello"

// Or with type hints:
x: int = 10L
```

### 2. Function Definitions
**Traditional R:**
```R
add <- function(a, b) {
  return(a + b)
}
```
**Modern RR:**
```rust
fn add(a, b) {
    return a + b
}

// Or as a typed expression-bodied function:
fn add(a: float, b: float) -> float = a + b
```

### 3. Control Flow (Loops and Ifs)
**Traditional R:**
```R
if (x > 0) {
  for (i in 1:n) {
    x <- x + i
  }
}
```
**Modern RR:**
```rust
if x > 0 {
    for i in 1..n {
        x += i  // Compound assignments are supported!
    }
}
```

### 4. Data Structures
**Traditional R:**
```R
vec <- c(1, 2, 3)
lst <- list(name = "rr", ver = 1.0)
```
**Modern RR:**
```rust
let vec = [1, 2, 3]
let lst = {name: "rr", ver: 1.0}
```

## Lexical Rules

### Numbers

- Integer literals: `1`, `42`, `1L`, `1l`
- Float literals: `1.0`, `.5`

Current lexer limits:

- `1.` is not lexed as float (`1` then `.`)
- scientific notation like `1e3` is not lexed as one numeric token

### Strings

- Double-quoted only: `"text"`
- Escapes supported: `\\n`, `\\r`, `\\t`, `\\"`, `\\\\`
- Unterminated strings produce parse diagnostics

### Comments

- Line comment: `// ...`
- Block comment: `/* ... */`

### Operators and Delimiters

- Assignment: `=` and `<-` (same token)
- Compound assignment: `+=`, `-=`, `*=`, `/=`, `%=`
- Arithmetic/comparison: `+ - * / % %*% == != < <= > >=`
- Logical: `!`, `&&`, `||`
- Single `&` and `|` are also tokenized as logical operators
- Others: `..`, `.`, `|>`, `?`, `@`, `^`, `=>`, `->`
- Delimiters: `()`, `{}`, `[]`, `,`, `:`, `;`

## Statements

### Declarations and Assignment

- `let` declaration:
  - `let x = expr`
  - `let x: int = 10L`
- Typed declaration sugar:
  - `x: int = 10L`
  - target must be a plain name (not index/field)
- Assignment:
  - `x = expr`, `x <- expr`
  - `x[i] = expr`
  - `rec.x = expr`
- Compound assignment sugar:
  - `x += y`
  - `arr[i] += y`
  - `rec.x -= y`
  - lowered as `lhs = lhs <op> rhs`

### Functions

- Declaration forms:
  - `fn add(a, b) { ... }`
  - `function add(a, b) { ... }`
- Expression-bodied form:
  - `fn add(a, b) = a + b`
- Type hints:
  - params: `fn add(a: float, b: int) { ... }`
  - return: `fn add(a: float, b: float) -> float { ... }`
  - parser accepts both `->` and `=>` as return-arrow tokens

### Control Flow

- `if` / `else`
- `while`
- `for`
  - `for (i in expr) ...`
  - `for i in expr ...`
- `return expr` or `return`
- `break`
- `next`

`if`/`while` conditions accept both:

- parenthesized form: `if (x < 1) ...`
- no-paren form: `if x < 1 { ... }`

### Modules

- `import "path.rr"` (`;` optional)
- `export fn name(...) { ... }`
- `export function name(...) { ... }`

Note: `export` is parsed as `export` + function declaration, not as general export of arbitrary assignment expressions.

## Expressions

- Name: `x`
- Unary: `-x`, `!x`
- Binary: `+ - * / % %*% == != < <= > >= && ||` (or `&`, `|`)
- Range: `a .. b`
- Call: `f(x, y)`
- Named call args: `f(x = 1, y = 2)`
- Index: `x[i]`, `m[i, j]`
- Field: `rec.a`
- Vector literal: `[1, 2, 3]`
- Record literal: `{a: 1, b: 2}`
- Lambda: `fn(x) { ... }`, `function(x) { ... }`, `fn(x) = x + 1`
- Pipe: `x |> f(1)`
- Try postfix: `expr?`
- Match: `match (v) { ... }` (parentheses required)
- Column/unquote tokens: `@name`, `^expr`

### Operator Precedence (low -> high)

1. `|>`
2. `||`
3. `&&`
4. `==`, `!=`
5. `<`, `<=`, `>`, `>=`
6. `..`
7. `+`, `-`
8. `*`, `/`, `%`, `%*%`
9. prefix `-`, `!`
10. postfix call/index/field: `()`, `[]`, `.`
11. postfix `?`

## Dotted Identifiers and Disambiguation

RR supports dotted names such as `solve.cg`, `idx.cube`, and `is.na`.

Parser behavior:

- dotted references initially parse as field chains (`a.b.c`)

Lowering behavior (`src/hir/lower.rs`):

- if root name is bound in local scope, keep field-access semantics
- if root name is unbound locally, expression may be reinterpreted as dotted symbol name

This allows both:

- true field access (`rec.x`)
- R-style dotted function/variable names (`solve.cg(...)`)

## Match and Pattern Support

Match arm grammar:

- `pattern => expr`
- `pattern if guard_expr => expr`
- trailing comma after arm is allowed

Supported patterns:

- wildcard: `_`
- literals: int/float/string/bool/null/na
- binding: `name`
- list pattern: `[a, b, ..rest]`
- record pattern: `{a: x, b: y}`

Current limits:

- list spread `..` must be last
- record rest pattern (`{a: x, ..rest}`) is not supported

## Semicolon and Newline Policy

- Semicolons are optional in most places
- Same-line statement boundaries require `;`
- Missing same-line separator triggers:
  - `Missing ';' before ... on the same line`

Important newline rule:

- postfix continuations `(`, `[`, `.` do not continue across a newline
- this keeps single-line control bodies stable and avoids accidental postfix chaining on the next line

## Assignment Policy (`let` strictness)

From `src/hir/lower.rs`:

- default: assignment to undeclared name implicitly declares it
- strict mode: `RR_STRICT_LET=1` or `RR_STRICT_ASSIGN=1` makes it a compile error
- warning mode: `RR_WARN_IMPLICIT_DECL=1` emits implicit-declaration warnings

## Function and Closure Semantics

- Parameter defaults are supported in syntax
- Type hint aliases recognized in lowering include:
  - ints: `int`, `integer`, `i32`, `i64`, `isize`
  - floats: `float`, `double`, `numeric`, `f32`, `f64`
  - bools: `bool`, `boolean`, `logical`
  - strings: `str`, `string`, `char`, `character`
  - `any`, `null`
- If a function/lambda body has no explicit `return` statements, the trailing expression statement is converted to an implicit return
- Lambdas are lambda-lifted; captures are packed through runtime closure helpers

## Pipe/Try/Column/Unquote Lowering Notes

- `x |> f(a)` lowers like `f(x, a)`
- `x |> f(a)?` lowers to `Try(Call(...))`
- `expr?` currently lowers through MIR mostly as pass-through of the inner expression
- `@name` currently lowers to a string-like column reference value in MIR
- `^expr` lowers to the inner expression

## Dynamic Builtins (Hybrid Fallback)

Calls to these builtins mark MIR functions as `unsupported_dynamic` and restrict aggressive optimization:

- `eval`, `parse`, `get`, `assign`, `exists`, `mget`, `rm`, `ls`
- `parent.frame`, `environment`, `sys.frame`, `sys.call`, `do.call`

RR still emits runnable R code for these paths, but keeps optimization conservative for correctness.
