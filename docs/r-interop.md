# R Interop

This page is the R package interop manual for RR.

It describes the supported interop surface as implemented today.

## Audience

Read this page when you need to decide whether a package call should be:

- kept on RR's direct typed surface
- preserved as opaque namespaced interop
- treated as dynamic fallback

## Interop Model

RR uses three interop tiers.

| Tier | Meaning | Optimization policy | Typical cases |
| --- | --- | --- | --- |
| direct interop | RR understands and preserves the call shape intentionally | normal compile path | `graphics::plot`, `ggplot2::aes`, selected `dplyr`/`stats` |
| opaque interop | RR preserves the namespaced call but does not reason deeply about it | conservative optimization | unsupported namespaced package calls |
| hybrid fallback | RR must defer dynamic behavior to runtime | aggressive optimization disabled | `eval`, `parse`, `get`, `assign`, `do.call` |

The lists later on this page are authoritative for the direct tier. If a call is
not listed there, do not assume RR models it deeply even if it still emits runnable R.

`base::` is a special case: unlisted namespaced `base::` exports still stay on RR's
direct surface through a conservative package-wide fallback, but they default to
opaque/object-like typing unless a more precise rule is documented below.

## Current Package Status

As of the current RR surface:

- the [base-priority package line](compatibility.md#base-priority-package-line)
  is broadly implemented on RR's direct surface
- that term includes `datasets`; today its strongest direct surface is
  namespaced data objects with typed models rather than an export-for-export
  function closure
- `base` is effectively closed through explicit direct support plus a
  conservative package-wide `base::` fallback for awkward operator/replacement
  exports
- `tcltk` is supported through a conservative proxy/direct surface, but not yet
  documented as an exact export-for-export closure in the same way as the core
  package line above
- `readr`, `tidyr`, `dplyr`, and `ggplot2` also have large direct surfaces and
  dedicated reference pages here
- the next major compatibility frontier is the recommended package line
  (`MASS`, `Matrix`, `survival`, `nlme`, `mgcv`, and similar)

## Practical Rule

Prefer the most explicit tier RR can still reason about:

1. direct namespaced interop
2. opaque namespaced interop
3. hybrid fallback only when dynamic behavior is truly required

## Import Forms

| RR form | Meaning | Emitted R shape |
| --- | --- | --- |
| `import r "graphics"` | package-name namespace sugar | `graphics.plot(...) -> graphics::plot(...)` |
| `import r { plot } from "graphics"` | named import | `plot(...) -> graphics::plot(...)` |
| `import r { plot as draw_plot } from "graphics"` | named import with alias | `draw_plot(...) -> graphics::plot(...)` |
| `import r * as grDevices from "grDevices"` | explicit namespace import | `grDevices.png(...) -> grDevices::png(...)` |
| `import r default from "ggplot2"` | package-name default namespace alias | `ggplot2.ggplot(...) -> ggplot2::ggplot(...)` |

`import r "pkg"` is namespace sugar. It does not emit `library("pkg")`.

Direct interop support does not mean RR auto-imports package symbols into the
current scope.

- packages such as `graphics`, `grDevices`, `stats`, and `datasets` still need
  `import r ...` if you want stable namespaced lowering instead of search-path
  dependent fallback
- common unqualified RR/base helpers such as `c`, `length`, and `print` are a
  separate builtin surface; they work without `import r`, and explicit
  `import r * as base from "base"` is only needed when you want namespaced
  forms such as `base.c(...)` or `base.print(...)`

Package data objects can also be accessed through the same namespace forms:

- `import r * as datasets from "datasets"` then `datasets.iris`
- `import r { iris as iris_df } from "datasets"` then `iris_df`

Those lower to namespaced variable references such as `datasets::iris`.
RR currently treats their precise type conservatively unless a direct typed model exists.

Currently typed package-data bindings include:

- `datasets::iris`
- `datasets::mtcars`
- `datasets::airquality`
- `datasets::ToothGrowth`
- `datasets::CO2`
- `datasets::USArrests`
- `datasets::cars`
- `datasets::pressure`
- `datasets::faithful`
- `datasets::women`
- `datasets::BOD`
- `datasets::attitude`
- `datasets::PlantGrowth`
- `datasets::InsectSprays`
- `datasets::sleep`
- `datasets::Orange`
- `datasets::rock`
- `datasets::trees`
- `datasets::esoph`
- `datasets::stackloss`
- `datasets::warpbreaks`
- `datasets::quakes`
- `datasets::LifeCycleSavings`
- `datasets::ChickWeight`
- `datasets::DNase`
- `datasets::Formaldehyde`
- `datasets::Indometh`
- `datasets::Loblolly`
- `datasets::Puromycin`
- `datasets::USJudgeRatings`
- `datasets::anscombe`
- `datasets::attenu`
- `datasets::chickwts`
- `datasets::infert`
- `datasets::longley`
- `datasets::morley`
- `datasets::npk`
- `datasets::swiss`
- `datasets::volcano`
- `datasets::state.x77`
- `datasets::USPersonalExpenditure`
- `datasets::WorldPhones`
- `datasets::EuStockMarkets`
- `datasets::VADeaths`
- `datasets::AirPassengers`
- `datasets::JohnsonJohnson`
- `datasets::Nile`
- `datasets::lynx`
- `datasets::nottem`
- `datasets::sunspot.year`
- `datasets::precip`
- `datasets::islands`
- `datasets::state.area`
- `datasets::state.abb`
- `datasets::state.name`
- `datasets::state.region`
- `datasets::state.division`
- `datasets::airmiles`
- `datasets::austres`
- `datasets::co2`
- `datasets::discoveries`
- `datasets::fdeaths`
- `datasets::ldeaths`
- `datasets::mdeaths`
- `datasets::nhtemp`
- `datasets::sunspots`
- `datasets::treering`
- `datasets::uspop`
- `datasets::rivers`
- `datasets::UKDriverDeaths`
- `datasets::UKgas`
- `datasets::USAccDeaths`
- `datasets::WWWusage`
- `datasets::eurodist`
- `datasets::UScitiesD`
- `datasets::euro`
- `datasets::stack.loss`
- `datasets::sunspot.m2014`
- `datasets::sunspot.month`
- `datasets::LakeHuron`
- `datasets::lh`
- `datasets::presidents`
- `datasets::Seatbelts`
- `datasets::OrchardSprays`
- `datasets::Theoph`
- `datasets::penguins`
- `datasets::penguins_raw`
- `datasets::gait`
- `datasets::crimtab`
- `datasets::occupationalStatus`
- `datasets::ability.cov`
- `datasets::Harman23.cor`
- `datasets::Harman74.cor`
- `datasets::state.center`
- `datasets::BJsales`
- `datasets::BJsales.lead`
- `datasets::beaver1`
- `datasets::beaver2`
- `datasets::euro.cross`
- `datasets::randu`
- `datasets::freeny`
- `datasets::stack.x`
- `datasets::freeny.x`
- `datasets::freeny.y`
- `datasets::iris3`
- `datasets::Titanic`
- `datasets::UCBAdmissions`
- `datasets::HairEyeColor`

## Direct Interop Surface

The direct interop surface is split into per-package reference pages.
Each page lists every symbol RR recognizes on that package's direct tier.

| Package | Page |
| --- | --- |
| base / data | [Base / Data](./r-interop/base.md) |
| stats | [Stats](./r-interop/stats.md) |
| stats4 | [Stats4](./r-interop/stats4.md) |
| methods | [Methods](./r-interop/methods.md) |
| compiler | [Compiler](./r-interop/compiler.md) |
| utils | [Utils](./r-interop/utils.md) |
| tools | [Tools](./r-interop/tools.md) |
| parallel | [Parallel](./r-interop/parallel.md) |
| splines | [Splines](./r-interop/splines.md) |
| tcltk | [Tcl/Tk](./r-interop/tcltk.md) |
| graphics / grDevices / ggplot2 / grid | [Graphics / Visualization](./r-interop/graphics.md) |
| readr / tidyr | [IO / Reshape](./r-interop/io-reshape.md) |
| dplyr / tidyr verbs | [dplyr / tidyr](./r-interop/dplyr.md) |

## Tidy-Eval Surface

Inside selected `ggplot2`, `dplyr`, and `tidyr` calls, RR preserves bare names
as raw R symbols instead of rejecting them as undefined RR variables.

Currently supported tidy-aware calls:

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
- `tidyr::separate`
- `tidyr::unite`

That list is exact, not illustrative.
Implicit bare symbols are only preserved inside those calls. Outside that exact
surface, use normal RR expressions, force a raw R symbol with `@name`, or force
an RR environment value with `^expr`.

Currently supported tidy helpers:

- `starts_with`
- `ends_with`
- `contains`
- `matches`
- `everything`
- `all_of`
- `any_of`
- `where`
- `desc`
- `between`
- `n`
- `row_number`

Special forms:

- `@name`
  - force a raw R symbol
- `^expr`
  - force an RR environment expression
- `~name`
  - formula shorthand currently lowered as `stats::as.formula("~name")`
  - intended for direct interop cases such as `ggplot2::facet_wrap(~name)`
- `lhs ~ rhs`
  - model/faceting formula shorthand is also supported
  - lowered as `stats::as.formula("lhs ~ rhs")`
- `~a + b`
  - simple infix formula shorthand is also supported
  - currently limited to `+`, `-`, `*`, `/` over names, columns, dotted field paths, and string literals

## Stability Rules

- Supported direct interop should not force whole-function hybrid fallback.
- Unsupported namespaced calls should prefer opaque interop over hybrid fallback.

## Conflict Rules

Imported R locals and namespace aliases share one top-level name table.

If the same local name would refer to two different package bindings, lowering
fails and requires an alias change.

## Examples

```rr
import r { plot as draw_plot, lines } from "graphics"
import r * as grDevices from "grDevices"

let main <- function() {
  grDevices.png(filename = "plot.png", width = 640, height = 360)
  draw_plot(c(1, 2, 3), c(1, 4, 9), type = "l")
  lines(c(1, 2, 3), c(1, 2, 3), col = "tomato")
  grDevices.dev.off()
  0L
}
```

## Related Manuals

- [Language Reference](language.md)
- [RR for R Users](r-for-r-users.md)
- [Compatibility and Limits](compatibility.md)
