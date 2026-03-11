# Language Reference

This page is the surface-language reference for RR.

It documents behavior as implemented today, not as an aspirational future
language design.

Primary implementation sources:

- `src/syntax/{token,lex,parse,ast}.rs`
- `src/hir/lower.rs`
- `src/mir/lower_hir.rs`
- syntax/lowering tests

## Scope of This Reference

This page answers:

- which tokens and keywords exist
- which statement and expression forms are accepted
- how RR resolves ambiguous surface forms
- what the current parser/lowering limits are

It does not try to explain optimization strategy. For that, see
[Writing RR for Performance and Safety](writing-rr.md) and
[Tachyon Engine](optimization.md).

## Reading Notes

- If syntax and implementation disagree, implementation wins.
- If a form is parsed but lowered conservatively, that is part of the current language contract.
- When a feature is accepted only in a restricted form, that restriction is part of the language.

## Language Summary

RR currently provides:

- R-style assignment and function forms
- native-style `fn` and expression-bodied functions
- scalar, vector, matrix, and selected 3D indexing
- records, lists, closures, and pattern matching
- import/export and direct R package interop
- strict declaration by default

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

RR allows developers to write in a modern, Rust-like syntax while still supporting
many traditional R-style surface forms. You can mix and match these styles within
the subset RR currently implements.

With the default strict declaration rules, the first assignment to a name still
needs `let`, even when you use traditional R-style `<-` syntax.

### 1. Assignment and Declarations
**Traditional R:**
```R
let x <- 10L
let y <- "hello"
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
let add <- function(a, b) {
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

When a function has vector slice parameters RR can prove from explicit hints or
flow-typed straight-line bindings, and it lowers to a slice-stable vector return
expression, RR may emit it as:

- an internal implementation helper
- a public parallel wrapper that can dispatch through the runtime parallel path

This is automatic at codegen time; you do not need a separate parallel annotation.

### 3. Control Flow (Loops and Ifs)
**Traditional R:**
```R
let x <- 0L
let n <- 4L
if (x > 0) {
  for (i in seq_len(n)) {
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
let vec <- c(1, 2, 3)
let lst <- list(name = "rr", ver = 1.0)
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
- Delimiters: `()`, `{}`, `[]`, `,`, `:`
- `;` is rejected; statements are newline-delimited

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
  - generic hints: `vector<float>`, `matrix<float>`, `option<int>`, `list<float>`, `box<float>`
  - nested generics are accepted, e.g. `list<box<float>>`
  - supported primitive names: `int`, `float`, `bool`, `str`, `any`, `null`
  - parser accepts both `->` and `=>` as return-arrow tokens

Type mode behavior:

- `strict` (default): compiler reports hint conflicts, call mismatches, and unresolved strict positions (`E1010`/`E1011`/`E1012`)
- `gradual`: unresolved regions keep runtime-guarded dynamic behavior

### Builtin Resolution and Shadowing

RR does not treat all function names equally at lowering time.

- Reserved builtin/intrinsic names such as `abs`, `sqrt`, `exp`, `sum`, `mean`, `pmax`, `pmin`, `print`, and similar math/data helpers keep builtin lowering semantics.
- A small scalar-indexing group may be user-defined and shadow builtin names:
  - `length`
  - `floor`
  - `round`
  - `ceiling`
  - `trunc`

This split exists because the optimizer and runtime backend rely on intrinsic treatment for most math/vector helpers, while index/rounding helpers need RR-specific scalar semantics in some programs.

Practical rule:

- if you want custom math helpers, use a distinct name such as `demo_abs` or `my_sqrt`
- only rely on shadowing for the small scalar-indexing group above

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

- `import "path.rr"`
- `import r "graphics"` for package-name namespace interop; lowers `graphics.plot(...)` to `graphics::plot(...)`
- `import r { plot, lines } from "graphics"` for named R symbol imports; lowers calls to `graphics::plot(...)`, `graphics::lines(...)`
- `import r { plot as draw_plot } from "graphics"` supports local aliasing while still lowering to `graphics::plot(...)`
- `import r * as grDevices from "grDevices"` supports namespace-style access such as `grDevices.png(...)`, lowered to `grDevices::png(...)`
- `import r default from "ggplot2"` is sugar for namespace import using the package name as the local alias; `ggplot2.ggplot(...)` lowers to `ggplot2::ggplot(...)`
- `export fn name(...) { ... }`
- `export function name(...) { ... }`

Example:

```rr
import r { plot as draw_plot, lines } from "graphics"
import r default from "grDevices"

let main <- function() {
  grDevices.png(filename = "plot.png", width = 640, height = 360)
  draw_plot(c(1, 2, 3), c(1, 4, 9), type = "l")
  lines(c(1, 2, 3), c(1, 2, 3), col = "tomato")
  grDevices.dev.off()
  0L
}
```

Modern RR-style package interop uses the same import forms:

```rr
import r default from "ggplot2"
import r default from "dplyr"
import r * as base from "base"

fn main() {
    let raw = base.data.frame(x = c(0, 1, 2), signal = c(0.1, 0.5, 0.9))
    let series = raw |> dplyr.mutate(
        trend = x * 0.5 + 0.2,
        smooth = signal * 0.8 + 0.1
    )
    let p = ggplot2.ggplot(series, ggplot2.aes(x = x, y = trend)) +
        ggplot2.geom_line(color = "steelblue") +
        ggplot2.geom_point(ggplot2.aes(y = smooth), color = "tomato") +
        ggplot2.theme_minimal()
    ggplot2.ggsave(filename = "plot.png", plot = p, width = 6, height = 4, dpi = 120)
}

main()
```

Direct tidy-eval support is limited but intentional:

- inside `ggplot2::aes(...)` and selected `dplyr::*` verbs, bare unresolved names such as `x`, `signal`, `trend` are preserved as raw R symbols instead of becoming RR undefined-variable errors
- currently supported tidy data-mask calls:
  - `ggplot2::aes`
  - `dplyr::mutate`
  - `dplyr::filter`
  - `dplyr::select`
  - `dplyr::summarise`
  - `dplyr::arrange`
  - `dplyr::group_by`
  - `dplyr::rename`
  - `tidyr::pivot_longer`
  - `tidyr::pivot_wider`
- currently supported helper names inside those calls:
  - `starts_with`, `ends_with`, `contains`, `matches`, `everything`
  - `all_of`, `any_of`, `where`, `desc`, `between`, `n`, `row_number`

Supported package calls in this surface are handled as direct RR interop, so they do not force whole-function hybrid fallback.
Current direct-interop package surface includes:

- `base::data.frame`
- `stats::median`, `stats::sd`, `stats::lm`, `stats::predict`, `stats::quantile`, `stats::glm`, `stats::as.formula`
- `readr::read_csv`, `readr::write_csv`
- `tidyr::pivot_longer`, `tidyr::pivot_wider`
- `graphics::plot`, `graphics::lines`, `graphics::legend`
- `grDevices::png`, `grDevices::dev.off`
- `ggplot2::aes`, `ggplot2::ggplot`, `ggplot2::geom_line`, `ggplot2::geom_point`, `ggplot2::ggtitle`, `ggplot2::theme_minimal`, `ggplot2::ggsave`
- `dplyr::mutate`, `dplyr::filter`, `dplyr::select`, `dplyr::summarise`, `dplyr::arrange`, `dplyr::group_by`, `dplyr::rename`

Unsupported namespaced package calls are still emitted directly, but the function is marked as opaque interop and optimized conservatively.
Only truly dynamic runtime features such as `eval`, `parse`, `get`, `assign`, and `do.call` remain hybrid fallback.

Conflict rule:

- imported R locals and namespace aliases share one top-level name table
- if the same local name would refer to two different package symbols, lowering fails with an error and tells you which earlier binding won
- use `as` or a different namespace alias to resolve the conflict

Note: `export` is parsed as `export` + function declaration, not as general export of arbitrary assignment expressions.

## Expressions

- Name: `x`
- Unary: `-x`, `!x`
- Binary: `+ - * / % %*% == != < <= > >= && ||` (or `&`, `|`)
- Range: `a .. b`
- Call: `f(x, y)`
- Named call args: `f(x = 1, y = 2)`
- Index: `x[i]`, `m[i, j]`, `a[i, j, k]`
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

- Semicolons are not part of RR statement syntax
- End statements with a newline or `}`
- Same-line statement packing is rejected
- Same-line statement boundary failures report:
  - `statements must be separated by a newline or '}' before ...`

Important newline rule:

- postfix continuations `(`, `[`, `.` do not continue across a newline
- this keeps single-line control bodies stable and avoids accidental postfix chaining on the next line

## Assignment Policy (`let` strictness)

From `src/hir/lower.rs`:

- default: assignment to undeclared name is a compile error
- legacy relaxed mode: `RR_STRICT_LET=0` or `RR_STRICT_ASSIGN=0` allows implicit declaration
- warning mode: `RR_WARN_IMPLICIT_DECL=1` emits implicit-declaration warnings when relaxed mode is enabled

## Function and Closure Semantics

- Parameter defaults are supported in syntax
- Type hint aliases recognized in lowering include:
  - ints: `int`, `integer`, `i32`, `i64`, `isize`
  - floats: `float`, `double`, `numeric`, `f32`, `f64`
  - bools: `bool`, `boolean`, `logical`
  - strings: `str`, `string`, `char`, `character`
  - `any`, `null`
- Generic containers lowered from type hints:
  - `vector<T>`
  - `matrix<T>`
  - `option<T>`
  - `list<T>`
  - `box<T>`
- If a function/lambda body has no explicit `return` statements, the trailing expression statement is converted to an implicit return
- Lambdas are lambda-lifted; captures are packed through runtime closure helpers

## Pipe/Try/Column/Unquote Lowering Notes

- `x |> f(a)` lowers like `f(x, a)`
- `x |> f(a)?` lowers to `Try(Call(...))`
- `expr?` currently lowers through MIR mostly as pass-through of the inner expression
- `@name` lowers to a raw R symbol value and is mainly intended for tidy-eval package interop such as `dplyr::mutate(...)` and `ggplot2::aes(...)`
- inside tidy-aware package calls, unresolved bare names like `x` or `trend` are also preserved as raw R symbols
- `^expr` lowers to the inner RR expression and is mainly useful to force an environment value inside tidy-eval contexts

## Dynamic Builtins (Hybrid Fallback)

Calls to these builtins mark MIR functions as `unsupported_dynamic` and restrict aggressive optimization:

- `eval`, `parse`, `get`, `assign`, `exists`, `mget`, `rm`, `ls`
- `parent.frame`, `environment`, `sys.frame`, `sys.call`, `do.call`

RR still emits runnable R code for these paths, but keeps optimization conservative for correctness.
