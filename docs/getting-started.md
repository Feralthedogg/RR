# Getting Started

## Prerequisites

- Rust toolchain (`cargo`)
- `Rscript` in `PATH` for runtime execution (`rr run`)

## Build RR

```bash
cargo build
```

Run the binary through Cargo:

```bash
cargo run -- --help
```

## First Program

Create `main.rr`:

```rr
main <- function() {
  x <- 1 + 2
  print(x)
  x
}

print(main())
```

Run it:

```bash
cargo run -- run . -O1
```

## Compile to R only

```bash
cargo run -- main.rr -o main.R --no-runtime -O2
```

This writes a self-contained `.R` script with RR runtime helpers injected at the top.

## Syntax Notes

- RR accepts both `=` and `<-` for assignment.
- `fn(...) { ... }` and `function(...) { ... }` are both valid lambda forms.
- `fn name(...) = expr` short function form is supported.
- Dotted names like `solve.cg` are valid identifiers.
- Parameter defaults are accepted: `function(a = 0.0, b = 0L) { ... }`.
- Type hints are accepted:
  - params/return: `fn add(a: float, b: int) -> float = a + b`
  - declarations: `x: int = 10L`
- `if/while/for` support single-statement bodies (no braces required).
- `if` / `while` conditions may be written with or without parentheses:
  - `if (x < 1) y <- 1 else y <- 2`
  - `if x < 1 { y <- 1 } else { y <- 2 }`
- `for` supports both styles:
  - `for (i in 1..n) s <- s + i`
  - `for i in 1..n { s += i }`
- Compound assignments are supported for native style:
  - `x += 1`, `x -= 1`, `x *= 2`, `x /= 2`, `x %= 2`
- Semicolons are optional across lines, but still required between statements on the same line.
- Recommended user-facing style is R-like: `name <- function(...) { ... }`.
- Undeclared assignment is allowed by default; set `RR_STRICT_LET=1` to make it an error.
- To surface implicit declaration warnings, set `RR_WARN_IMPLICIT_DECL=1`.

## Build a directory

```bash
cargo run -- build . --out-dir build -O2
```

Behavior:

- Recursively finds `.rr` files.
- Skips `build/`, `target/`, `.git/` while scanning.
- Writes mirrored output paths under `build/` with `.R` extension.
