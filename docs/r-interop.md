# R Interop

This page is the R package interop manual for RR.

It describes the supported interop surface as implemented today.

## Interop Model

RR uses three interop tiers.

| Tier | Meaning | Optimization policy | Typical cases |
| --- | --- | --- | --- |
| direct interop | RR understands and preserves the call shape intentionally | normal compile path | `graphics::plot`, `ggplot2::aes`, selected `dplyr`/`stats` |
| opaque interop | RR preserves the namespaced call but does not reason deeply about it | conservative optimization | unsupported namespaced package calls |
| hybrid fallback | RR must defer dynamic behavior to runtime | aggressive optimization disabled | `eval`, `parse`, `get`, `assign`, `do.call` |

## Import Forms

| RR form | Meaning | Emitted R shape |
| --- | --- | --- |
| `import r "graphics"` | package-name namespace sugar | `graphics.plot(...) -> graphics::plot(...)` |
| `import r { plot } from "graphics"` | named import | `plot(...) -> graphics::plot(...)` |
| `import r { plot as draw_plot } from "graphics"` | named import with alias | `draw_plot(...) -> graphics::plot(...)` |
| `import r * as grDevices from "grDevices"` | explicit namespace import | `grDevices.png(...) -> grDevices::png(...)` |
| `import r default from "ggplot2"` | package-name default namespace alias | `ggplot2.ggplot(...) -> ggplot2::ggplot(...)` |

`import r "pkg"` is namespace sugar. It does not emit `library("pkg")`.

## Direct Interop Surface

### Base/Data

- `base::data.frame`

### Stats

- `stats::median`
- `stats::sd`
- `stats::lm`
- `stats::glm`
- `stats::predict`
- `stats::quantile`
- `stats::as.formula`

### IO / Reshape

- `readr::read_csv`
- `readr::write_csv`
- `tidyr::pivot_longer`
- `tidyr::pivot_wider`

### Graphics / Visualization

- `graphics::plot`
- `graphics::lines`
- `graphics::legend`
- `grDevices::png`
- `grDevices::dev.off`
- `ggplot2::aes`
- `ggplot2::ggplot`
- `ggplot2::geom_line`
- `ggplot2::geom_point`
- `ggplot2::ggtitle`
- `ggplot2::theme_minimal`
- `ggplot2::ggsave`

### dplyr Verbs

- `dplyr::mutate`
- `dplyr::filter`
- `dplyr::select`
- `dplyr::summarise`
- `dplyr::arrange`
- `dplyr::group_by`
- `dplyr::rename`

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

## Stability Rules

- Supported direct interop should not force whole-function hybrid fallback.
- Unsupported namespaced calls should prefer opaque interop over hybrid fallback.
- Dynamic metaprogramming remains hybrid fallback by design.

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
- [Compatibility and Limits](compatibility.md)
- [Runtime and Error Model](runtime-and-errors.md)
