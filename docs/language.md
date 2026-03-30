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
[Writing RR for Performance and Safety](writing-rr.md).

## Reading Notes

- If syntax and implementation disagree, implementation wins.
- If a form is parsed but lowered conservatively, that is part of the current language contract.
- When a feature is accepted only in a restricted form, that restriction is part of the language.

## Stability Model

This reference follows a GCC/LLVM-style "implemented surface wins" rule.

- accepted and lowered forms are part of the current surface contract
- accepted but conservative forms are still supported, just not aggressively optimized
- rejected or opaque forms are not part of the typed/optimizing core language

## Language Summary

RR currently provides:

- R-style assignment and function forms
- native-style `fn` and expression-bodied functions
- scalar, vector, matrix, and selected 3D indexing
- records, lists, closures, and pattern matching
- import/export and direct R package interop
- strict declaration by default

## What This Page Does Not Cover

This page does not try to be:

- a performance guide
- a runtime contract page
- a contributor implementation walkthrough

Use:

- [Writing RR for Performance and Safety](writing-rr.md)
- [Configuration](configuration.md)
- [Compatibility and Limits](compatibility.md)

for those topics.

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

Type precision notes:

- RR now keeps the `int` / `float` boundary more precisely than older releases:
  - `/` widens numeric expressions to floating-point
  - `%%` stays integer when both operands are inferred integer
  - `sum(int-vector)` stays integer; `mean(...)`, `log10(...)`, `atan2(...)`, and similar math builtins widen to floating-point
- vector-valued math/logical builtins such as `abs`, `pmax`, `pmin`, `log10`, `is.na`, and `is.finite` preserve vector shape, and keep a symbolic length when RR can prove it from the arguments
- constructor builtins such as `numeric`, `double`, `integer`, `logical`, and `character` stay on RR's direct builtin surface instead of degrading to opaque interop
- collection/reduction helpers such as `rep`, `any`, `all`, `which`, `prod`, and `var` are also recognized directly by the type layer
- metadata and ordering helpers such as `names`, `rownames`, `colnames`, `sort`, `order`, `unique`, `duplicated`, and `anyDuplicated` now stay on the builtin surface too; `match` remains an RR keyword for match expressions, but namespaced or imported package forms such as `base.match(...)` and `import r { match as base_match } from "base"` are now accepted
- string and formatting helpers such as `paste`, `paste0`, `sprintf`, and `cat` also stay on the builtin surface instead of degrading to opaque interop
- direct package interop also preserves more structure now: `base::data.frame(...)` keeps a shared row-count symbol when RR can prove one from the input columns, and `stats::predict(..., newdata = df)` keeps the `newdata` length on its result when available
- `matrix<T>` hints now stay matrix-typed internally instead of collapsing immediately to `vector<T>`
- matrix-oriented builtins such as `matrix`, `rowSums`, `colSums`, `crossprod`, and `tcrossprod` now preserve matrix/vector intent in the type layer instead of collapsing to unknown
- shape helpers such as `dim`, `nrow`, `ncol`, and `dimnames` are also recognized directly instead of falling back to opaque package interop
- matrix-shape algebra helpers such as `t`, `diag`, `rbind`, `cbind`, and `%*%`
  also stay on the direct typed surface, so RR can preserve matrix shape information
- dataframe schemas are still selective at the optimizer layer, but the type-term layer now keeps dataframe column terms instead of treating every dataframe hint as plain `any`
- when RR lowers a typed dataframe schema through HIR/MIR, field access such as `df.col` can now refine to the matching column term instead of conservatively joining every column type
- nested generic hints such as `list<box<float>>` are preserved through strict call checking and index-element inference instead of collapsing immediately to `any`
- in strict mode, 2D indexing and 2D assignment now expect a matrix-typed base; using `a[i, j]` on a value hinted as `vector<T>` is diagnosed instead of silently degrading

### Builtin Resolution and Shadowing

RR does not treat all function names equally at lowering time.

- Reserved builtin/intrinsic names such as `abs`, `sqrt`, `exp`, `sum`, `mean`, `prod`, `any`, `all`, `which`, `names`, `sort`, `order`, `unique`, `duplicated`, `anyDuplicated`, `paste`, `paste0`, `sprintf`, `cat`, `pmax`, `pmin`, `print`, `rep`, and similar math/data helpers keep builtin lowering semantics.
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

Package direct interop is not an automatic import mechanism.

- use `import r ...` for packages such as `graphics`, `grDevices`, `stats`, and
  other namespaced R interop you want RR to lower explicitly as `pkg::symbol`
- common helpers such as `c`, `length`, and `print` are separate RR/base
  builtins and therefore work unqualified without `import r`
- if you want explicit namespaced base forms, import `base` and call
  `base.c(...)`, `base.length(...)`, or `base.print(...)`

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
  - `tidyr::separate`
  - `tidyr::pivot_longer`
  - `tidyr::pivot_wider`
  - `tidyr::unite`
- currently supported helper names inside those calls:
  - `starts_with`, `ends_with`, `contains`, `matches`, `everything`
  - `all_of`, `any_of`, `where`, `desc`, `between`, `n`, `row_number`
- outside that exact tidy-aware call list, bare unresolved names go back to
  normal RR name resolution; use `@name` for a raw column symbol or `^expr` for
  an RR environment value when you need to force the boundary explicitly

Supported package calls in this surface are handled as direct RR interop, so they do not force whole-function hybrid fallback.
Current direct-interop package surface includes:

- `base::data.frame`, `base::globalenv`, `base::length`, `base::c`, `base::list`, `base::sum`, `base::mean`, `base::vector`, `base::seq`, `base::ifelse`, `base::abs`, `base::min`, `base::max`, `base::pmax`, `base::pmin`, `base::sqrt`, `base::log`, `base::log10`, `base::log2`, `base::exp`, `base::atan2`, `base::sin`, `base::cos`, `base::tan`, `base::asin`, `base::acos`, `base::atan`, `base::sinh`, `base::cosh`, `base::tanh`, `base::sign`, `base::gamma`, `base::lgamma`, `base::floor`, `base::ceiling`, `base::trunc`, `base::round`, `base::is.na`, `base::is.finite`, `base::print`, `base::numeric`, `base::matrix`, `base::dim`, `base::dimnames`, `base::nrow`, `base::ncol`, `base::seq_len`, `base::seq_along`, `base::diag`, `base::t`, `base::rbind`, `base::cbind`, `base::rowSums`, `base::colSums`, `base::crossprod`, `base::tcrossprod`, `base::character`, `base::logical`, `base::integer`, `base::double`, `base::rep`, `base::rep.int`, `base::any`, `base::all`, `base::which`, `base::prod`, `base::paste`, `base::paste0`, `base::sprintf`, `base::cat`, `base::tolower`, `base::toupper`, `base::substr`, `base::sub`, `base::gsub`, `base::nchar`, `base::nzchar`, `base::grepl`, `base::grep`, `base::startsWith`, `base::endsWith`, `base::which.min`, `base::which.max`, `base::isTRUE`, `base::isFALSE`, `base::lengths`, `base::union`, `base::intersect`, `base::setdiff`, `base::sample`, `base::sample.int`, `base::rank`, `base::factor`, `base::cut`, `base::table`, `base::trimws`, `base::chartr`, `base::strsplit`, `base::regexpr`, `base::gregexpr`, `base::regexec`, `base::agrep`, `base::agrepl`
- `stats::median`, `stats::median.default`, `stats::sd`, `stats::lm`, `stats::predict`, `stats::simulate`, `stats::summary.lm`, `stats::summary.glm`, `stats::summary.aov`, `stats::summary.stepfun`, `stats::quantile`, `stats::glm`, `stats::as.formula`, `stats::coef`, `stats::fitted`, `stats::resid`, `stats::residuals`, `stats::vcov`, `stats::confint`, `stats::model.matrix`, `stats::model.matrix.default`, `stats::model.matrix.lm`, `stats::AIC`, `stats::BIC`, `stats::logLik`, `stats::deviance`, `stats::sigma`, `stats::nobs`, `stats::df.residual`, `stats::anova`, `stats::update`, `stats::update.default`, `stats::update.formula`, `stats::terms`, `stats::drop.terms`, `stats::getCall`, `stats::model.frame`, `stats::model.frame.default`, `stats::glm.fit`, `stats::lm.fit`, `stats::lm.wfit`, `stats::lsfit`, `stats::ls.diag`, `stats::loadings`, `stats::makepredictcall`, `stats::na.contiguous`, `stats::na.action`, `stats::napredict`, `stats::naresid`, `stats::naprint`, `stats::weights`, `stats::model.weights`, `stats::offset`, `stats::model.offset`, `stats::na.omit`, `stats::na.exclude`, `stats::na.pass`, `stats::na.fail`, `stats::glm.control`, `stats::is.empty.model`, `stats::binomial`, `stats::gaussian`, `stats::poisson`, `stats::family`, `stats::make.link`, `stats::quasi`, `stats::quasibinomial`, `stats::quasipoisson`, `stats::inverse.gaussian`, `stats::SSasymp`, `stats::SSasympOff`, `stats::SSasympOrig`, `stats::SSbiexp`, `stats::SSfol`, `stats::SSfpl`, `stats::SSgompertz`, `stats::SSlogis`, `stats::SSmicmen`, `stats::SSweibull`, `stats::selfStart`, `stats::numericDeriv`, `stats::deriv`, `stats::deriv3`, `stats::dnorm`, `stats::pnorm`, `stats::qnorm`, `stats::rnorm`, `stats::dbinom`, `stats::pbinom`, `stats::qbinom`, `stats::rbinom`, `stats::dpois`, `stats::ppois`, `stats::qpois`, `stats::rpois`, `stats::dunif`, `stats::punif`, `stats::qunif`, `stats::runif`, `stats::dgamma`, `stats::pgamma`, `stats::qgamma`, `stats::rgamma`, `stats::dbeta`, `stats::pbeta`, `stats::qbeta`, `stats::rbeta`, `stats::dt`, `stats::pt`, `stats::qt`, `stats::rt`, `stats::df`, `stats::pf`, `stats::qf`, `stats::rf`, `stats::dchisq`, `stats::pchisq`, `stats::qchisq`, `stats::rchisq`, `stats::dexp`, `stats::pexp`, `stats::qexp`, `stats::rexp`, `stats::dlnorm`, `stats::plnorm`, `stats::qlnorm`, `stats::rlnorm`, `stats::dweibull`, `stats::pweibull`, `stats::qweibull`, `stats::rweibull`, `stats::dcauchy`, `stats::pcauchy`, `stats::qcauchy`, `stats::rcauchy`, `stats::dgeom`, `stats::pgeom`, `stats::qgeom`, `stats::rgeom`, `stats::dhyper`, `stats::phyper`, `stats::qhyper`, `stats::rhyper`, `stats::dnbinom`, `stats::pnbinom`, `stats::qnbinom`, `stats::rnbinom`, `stats::dlogis`, `stats::plogis`, `stats::qlogis`, `stats::rlogis`, `stats::pbirthday`, `stats::qbirthday`, `stats::ptukey`, `stats::qtukey`, `stats::psmirnov`, `stats::qsmirnov`, `stats::rsmirnov`, `stats::acf2AR`, `stats::dsignrank`, `stats::psignrank`, `stats::qsignrank`, `stats::rsignrank`, `stats::dwilcox`, `stats::pwilcox`, `stats::qwilcox`, `stats::rwilcox`, `stats::p.adjust`, `stats::ppoints`, `stats::qqnorm`, `stats::qqplot`, `stats::qqline`, `stats::dist`, `stats::toeplitz`, `stats::toeplitz2`, `stats::diffinv`, `stats::polym`, `stats::asOneSidedFormula`, `stats::variable.names`, `stats::addmargins`, `stats::ftable`, `stats::xtabs`, `stats::isoreg`, `stats::medpolish`, `stats::symnum`, `stats::smooth`, `stats::smoothEnds`, `stats::line`, `stats::varimax`, `stats::promax`, `stats::density`, `stats::density.default`, `stats::ecdf`, `stats::poly`, `stats::prcomp`, `stats::cmdscale`, `stats::princomp`, `stats::cancor`, `stats::power.anova.test`, `stats::power.prop.test`, `stats::power.t.test`, `stats::cov`, `stats::cor`, `stats::var`, `stats::cov.wt`, `stats::cov2cor`, `stats::mahalanobis`, `stats::rWishart`, `stats::r2dtable`, `stats::dmultinom`, `stats::rmultinom`, `stats::IQR`, `stats::mad`, `stats::bw.nrd`, `stats::bw.nrd0`, `stats::bw.ucv`, `stats::bw.bcv`, `stats::bw.SJ`, `stats::t.test`, `stats::wilcox.test`, `stats::binom.test`, `stats::prop.test`, `stats::poisson.test`, `stats::chisq.test`, `stats::fisher.test`, `stats::cor.test`, `stats::ks.test`, `stats::shapiro.test`, `stats::ansari.test`, `stats::bartlett.test`, `stats::Box.test`, `stats::fligner.test`, `stats::friedman.test`, `stats::kruskal.test`, `stats::mantelhaen.test`, `stats::mcnemar.test`, `stats::mood.test`, `stats::oneway.test`, `stats::prop.trend.test`, `stats::quade.test`, `stats::var.test`, `stats::termplot`, `stats::pairwise.t.test`, `stats::pairwise.wilcox.test`, `stats::pairwise.prop.test`, `stats::approx`, `stats::approxfun`, `stats::ksmooth`, `stats::lowess`, `stats::loess`, `stats::loess.control`, `stats::loess.smooth`, `stats::spline`, `stats::splinefun`, `stats::smooth.spline`, `stats::supsmu`, `stats::interaction.plot`, `stats::lag.plot`, `stats::monthplot`, `stats::scatter.smooth`, `stats::biplot`, `stats::aggregate`, `stats::aggregate.data.frame`, `stats::aggregate.ts`, `stats::reshape`, `stats::ave`, `stats::reorder`, `stats::relevel`, `stats::aov`, `stats::TukeyHSD`, `stats::alias`, `stats::model.tables`, `stats::factanal`, `stats::heatmap`, `stats::add1`, `stats::drop1`, `stats::extractAIC`, `stats::add.scope`, `stats::drop.scope`, `stats::factor.scope`, `stats::dummy.coef`, `stats::dummy.coef.lm`, `stats::effects`, `stats::setNames`, `stats::step`, `stats::optim`, `stats::optimHess`, `stats::optimize`, `stats::optimise`, `stats::nlm`, `stats::nlminb`, `stats::constrOptim`, `stats::uniroot`, `stats::integrate`, `stats::HoltWinters`, `stats::StructTS`, `stats::KalmanForecast`, `stats::KalmanRun`, `stats::KalmanSmooth`, `stats::arima`, `stats::arima0`, `stats::tsdiag`, `stats::nls`, `stats::nls.control`, `stats::getInitial`, `stats::ar`, `stats::ar.yw`, `stats::ar.mle`, `stats::ar.burg`, `stats::ar.ols`, `stats::arima.sim`, `stats::ARMAacf`, `stats::ARMAtoMA`, `stats::spec.ar`, `stats::spec.pgram`, `stats::spec.taper`, `stats::plot.spec.coherency`, `stats::plot.spec.phase`, `stats::kernel`, `stats::is.tskernel`, `stats::df.kernel`, `stats::bandwidth.kernel`, `stats::kernapply`, `stats::convolve`, `stats::fft`, `stats::mvfft`, `stats::nextn`, `stats::ts`, `stats::as.ts`, `stats::ts.intersect`, `stats::ts.union`, `stats::frequency`, `stats::time`, `stats::cycle`, `stats::is.ts`, `stats::is.mts`, `stats::hasTsp`, `stats::tsp`, `stats::start`, `stats::end`, `stats::deltat`, `stats::window`, `stats::lag`, `stats::embed`, `stats::weighted.mean`, `stats::runmed`, `stats::filter`, `stats::decompose`, `stats::spectrum`, `stats::stl`, `stats::stepfun`, `stats::as.stepfun`, `stats::is.stepfun`, `stats::plot.stepfun`, `stats::plot.ecdf`, `stats::plot.ts`, `stats::screeplot`, `stats::dendrapply`, `stats::is.leaf`, `stats::order.dendrogram`, `stats::as.dist`, `stats::as.hclust`, `stats::as.dendrogram`, `stats::cophenetic`, `stats::rect.hclust`, `stats::kmeans`, `stats::hclust`, `stats::cutree`, `stats::acf`, `stats::pacf`, `stats::ccf`, `stats::hatvalues`, `stats::hat`, `stats::cooks.distance`, `stats::covratio`, `stats::dfbeta`, `stats::dfbetas`, `stats::dffits`, `stats::rstandard`, `stats::rstudent`, `stats::weighted.residuals`, `stats::influence`, `stats::influence.measures`, `stats::qr.influence`, `stats::lm.influence`
- `stats4::mle`, `stats4::coef`, `stats4::vcov`, `stats4::confint`, `stats4::logLik`, `stats4::AIC`, `stats4::BIC`, `stats4::nobs`, `stats4::update`, `stats4::summary`, `stats4::profile`, `stats4::plot`, `stats4::show`
- `methods::isClass`, `methods::isGeneric`, `methods::hasMethod`, `methods::existsMethod`, `methods::getClass`, `methods::getClassDef`, `methods::getClasses`, `methods::getFunction`, `methods::getLoadActions`, `methods::getPackageName`, `methods::getSlots`, `methods::getGeneric`, `methods::getGenerics`, `methods::getGroup`, `methods::getGroupMembers`, `methods::formalArgs`, `methods::getAllSuperClasses`, `methods::existsFunction`, `methods::hasLoadAction`, `methods::hasArg`, `methods::findFunction`, `methods::hasMethods`, `methods::findMethodSignatures`, `methods::isGroup`, `methods::isGrammarSymbol`, `methods::isRematched`, `methods::isXS3Class`, `methods::isSealedClass`, `methods::isSealedMethod`, `methods::isClassDef`, `methods::classesToAM`, `methods::cacheMetaData`, `methods::cacheMethod`, `methods::findClass`, `methods::findUnique`, `methods::getDataPart`, `methods::getRefClass`, `methods::testInheritedMethods`, `methods::testVirtual`, `methods::getValidity`, `methods::is`, `methods::slot`, `methods::validObject`, `methods::isVirtualClass`, `methods::isClassUnion`, `methods::canCoerce`, `methods::selectMethod`, `methods::new`, `methods::getMethod`, `methods::findMethod`, `methods::getMethodsForDispatch`, `methods::standardGeneric`, `methods::show`, `methods::setClass`, `methods::setGeneric`, `methods::setMethod`, `methods::extends`, `methods::slotNames`, `methods::findMethods`
- `compiler::enableJIT`, `compiler::getCompilerOption`, `compiler::setCompilerOptions`, `compiler::compile`, `compiler::compilePKGS`, `compiler::cmpfun`, `compiler::disassemble`, `compiler::cmpfile`, `compiler::loadcmp`
- `tools::toTitleCase`, `tools::file_path_as_absolute`, `tools::R_user_dir`, `tools::md5sum`, `tools::sha256sum`, `tools::file_ext`, `tools::file_path_sans_ext`, `tools::list_files_with_exts`, `tools::list_files_with_type`, `tools::dependsOnPkgs`, `tools::getVignetteInfo`, `tools::pkgVignettes`, `tools::delimMatch`, `tools::parse_URI_reference`, `tools::parse_Rd`, `tools::Rd2txt`, `tools::Rd2HTML`, `tools::Rd2latex`, `tools::Rd2ex`, `tools::Rdindex`, `tools::read.00Index`, `tools::checkRd`, `tools::RdTextFilter`, `tools::Rd2txt_options`, `tools::encoded_text_to_latex`, `tools::parseLatex`, `tools::getBibstyle`, `tools::deparseLatex`, `tools::latexToUtf8`, `tools::showNonASCII`, `tools::showNonASCIIfile`, `tools::standard_package_names`, `tools::base_aliases_db`, `tools::base_rdxrefs_db`, `tools::CRAN_aliases_db`, `tools::CRAN_archive_db`, `tools::CRAN_package_db`, `tools::CRAN_authors_db`, `tools::CRAN_current_db`, `tools::CRAN_check_results`, `tools::CRAN_check_details`, `tools::CRAN_check_issues`, `tools::CRAN_rdxrefs_db`, `tools::summarize_CRAN_check_status`, `tools::package_dependencies`, `tools::Rd_db`
- `utils::head`, `utils::tail`, `utils::packageVersion`, `utils::maintainer`, `utils::packageDate`, `utils::object.size`, `utils::memory.size`, `utils::memory.limit`, `utils::compareVersion`, `utils::capture.output`, `utils::packageDescription`, `utils::sessionInfo`, `utils::citation`, `utils::person`, `utils::as.person`, `utils::as.personList`, `utils::as.roman`, `utils::hasName`, `utils::strcapture`, `utils::apropos`, `utils::find`, `utils::findMatches`, `utils::methods`, `utils::help.search`, `utils::data`, `utils::getAnywhere`, `utils::argsAnywhere`, `utils::contrib.url`, `utils::localeToCharset`, `utils::charClass`, `utils::fileSnapshot`, `utils::URLencode`, `utils::URLdecode`, `utils::glob2rx`, `utils::file_test`, `utils::installed.packages`, `utils::read.csv`, `utils::read.csv2`, `utils::read.table`, `utils::read.delim`, `utils::read.fwf`, `utils::write.csv`, `utils::write.csv2`, `utils::write.table`, `utils::str`, `utils::combn`, `utils::adist`, `utils::count.fields`, `utils::type.convert`
- `parallel::detectCores`, `parallel::makeCluster`, `parallel::stopCluster`, `parallel::parLapply`, `parallel::clusterExport`, `parallel::clusterEvalQ`, `parallel::clusterMap`, `parallel::clusterApply`, `parallel::clusterCall`, `parallel::mclapply`, `parallel::clusterSplit`, `parallel::splitIndices`, `parallel::clusterApplyLB`, `parallel::parSapply`, `parallel::parSapplyLB`, `parallel::parApply`, `parallel::mcparallel`, `parallel::mccollect`
- `splines::bs`, `splines::ns`, `splines::splineDesign`, `splines::interpSpline`, `splines::periodicSpline`, `splines::backSpline`, `splines::spline.des`, `splines::as.polySpline`, `splines::polySpline`, `splines::asVector`, `splines::splineKnots`, `splines::splineOrder`, `splines::xyVector`
- `tcltk::tclObj`, `tcltk::as.tclObj`, `tcltk::tclVar`, `tcltk::tclvalue`, `tcltk::is.tclObj`, `tcltk::is.tkwin`, `tcltk::tclfile.dir`, `tcltk::tclfile.tail`, `tcltk::addTclPath`, `tcltk::tclRequire`, `tcltk::tclVersion`, `tcltk::tkProgressBar`, `tcltk::getTkProgressBar`, `tcltk::setTkProgressBar`
- `grid::grid.newpage`, `grid::grid.draw`, `grid::grid.rect`, `grid::grid.text`, `grid::grid.circle`, `grid::grid.points`, `grid::grid.lines`, `grid::grid.segments`, `grid::grid.polygon`, `grid::grid.polyline`, `grid::grid.raster`, `grid::grid.curve`, `grid::grid.bezier`, `grid::grid.path`, `grid::nullGrob`, `grid::rectGrob`, `grid::circleGrob`, `grid::segmentsGrob`, `grid::pointsGrob`, `grid::rasterGrob`, `grid::bezierGrob`, `grid::pathGrob`, `grid::polygonGrob`, `grid::polylineGrob`, `grid::xsplineGrob`, `grid::frameGrob`, `grid::packGrob`, `grid::placeGrob`, `grid::roundrectGrob`, `grid::linesGrob`, `grid::curveGrob`, `grid::textGrob`, `grid::grobTree`, `grid::gList`, `grid::unit`, `grid::grobWidth`, `grid::grobHeight`, `grid::gpar`, `grid::viewport`, `grid::grid.layout`, `grid::grid.frame`, `grid::grid.pack`, `grid::grid.place`, `grid::vpStack`, `grid::vpList`, `grid::dataViewport`, `grid::pushViewport`, `grid::current.viewport`, `grid::seekViewport`, `grid::upViewport`, `grid::popViewport`
- `readr::read_csv`, `readr::read_delim`, `readr::read_rds`, `readr::read_tsv`, `readr::write_csv`, `readr::write_delim`, `readr::write_rds`, `readr::write_tsv`
- `tidyr::pivot_longer`, `tidyr::pivot_wider`
- `graphics::plot`, `graphics::lines`, `graphics::points`, `graphics::abline`, `graphics::title`, `graphics::box`, `graphics::text`, `graphics::axis`, `graphics::axTicks`, `graphics::strwidth`, `graphics::strheight`, `graphics::grconvertX`, `graphics::grconvertY`, `graphics::clip`, `graphics::xspline`, `graphics::pie`, `graphics::symbols`, `graphics::smoothScatter`, `graphics::stem`, `graphics::segments`, `graphics::arrows`, `graphics::mtext`, `graphics::rug`, `graphics::polygon`, `graphics::hist`, `graphics::boxplot`, `graphics::par`, `graphics::layout`, `graphics::layout.show`, `graphics::matplot`, `graphics::matlines`, `graphics::matpoints`, `graphics::pairs`, `graphics::stripchart`, `graphics::dotchart`, `graphics::contour`, `graphics::image`, `graphics::persp`, `graphics::assocplot`, `graphics::mosaicplot`, `graphics::fourfoldplot`, `graphics::legend`
- `grDevices::png`, `grDevices::jpeg`, `grDevices::bmp`, `grDevices::tiff`, `grDevices::pdf`, `grDevices::rgb`, `grDevices::hsv`, `grDevices::gray`, `grDevices::gray.colors`, `grDevices::palette.colors`, `grDevices::palette.pals`, `grDevices::palette`, `grDevices::hcl.colors`, `grDevices::colors`, `grDevices::heat.colors`, `grDevices::terrain.colors`, `grDevices::topo.colors`, `grDevices::cm.colors`, `grDevices::rainbow`, `grDevices::adjustcolor`, `grDevices::densCols`, `grDevices::col2rgb`, `grDevices::rgb2hsv`, `grDevices::convertColor`, `grDevices::n2mfrow`, `grDevices::dev.off`, `grDevices::dev.cur`, `grDevices::dev.next`, `grDevices::dev.prev`, `grDevices::dev.size`
- `ggplot2::aes`, `ggplot2::ggplot`, `ggplot2::geom_col`, `ggplot2::geom_bar`, `ggplot2::facet_grid`, `ggplot2::geom_line`, `ggplot2::geom_point`, `ggplot2::facet_wrap`, `ggplot2::ggtitle`, `ggplot2::labs`, `ggplot2::theme_bw`, `ggplot2::theme_minimal`, `ggplot2::ggsave`
- `dplyr::mutate`, `dplyr::filter`, `dplyr::full_join`, `dplyr::inner_join`, `dplyr::right_join`, `dplyr::select`, `dplyr::summarise`, `dplyr::arrange`, `dplyr::anti_join`, `dplyr::bind_rows`, `dplyr::group_by`, `dplyr::left_join`, `dplyr::rename`, `dplyr::semi_join`

- `tidyr::pivot_longer`, `tidyr::pivot_wider`, `tidyr::separate`, `tidyr::unite`

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
- Formula shorthand: `~label`, `y ~ x`, `~grp + kind`
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
- legacy relaxed mode: `--strict-let off` allows implicit declaration
- warning mode: `--warn-implicit-decl on` emits implicit-declaration warnings when relaxed mode is enabled

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
- inside the exact tidy-aware package-call surface above, unresolved bare names like `x` or `trend` are also preserved as raw R symbols
- `^expr` lowers to the inner RR expression and is mainly useful to force an environment value inside tidy-eval contexts

## Dynamic Builtins (Hybrid Fallback)

Calls to these builtins mark MIR functions as `unsupported_dynamic` and restrict aggressive optimization:

- `eval`, `parse`, `get`, `assign`, `exists`, `mget`, `rm`, `ls`
- `parent.frame`, `environment`, `sys.frame`, `sys.call`, `do.call`

RR still emits runnable R code for these paths, but keeps optimization conservative for correctness.
