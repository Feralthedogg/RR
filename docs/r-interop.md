# R Interop

This page defines RR's supported R package interop surface as implemented today.

## Model

RR uses three interop tiers.

| Tier | Meaning | Optimization policy | Typical examples |
| --- | --- | --- | --- |
| `direct interop` | RR understands the call shape and preserves it intentionally | normal compile path, no whole-function fallback marker | `graphics::plot`, `ggplot2::aes`, `dplyr::mutate`, `stats::predict` |
| `opaque interop` | RR preserves the namespaced R call but does not reason about it deeply | conservative optimization, emits `# rr-opaque-interop:` | unsupported namespaced calls such as `utils::head` |
| `hybrid fallback` | RR must defer truly dynamic runtime behavior | skips aggressive optimization, emits `# rr-hybrid-fallback:` | `eval`, `parse`, `get`, `assign`, `do.call` |

## Import Forms

| RR source form | Meaning | Emitted R shape |
| --- | --- | --- |
| `import r "graphics"` | package-name namespace sugar | `graphics.plot(...)` -> `graphics::plot(...)` |
| `import r { plot, lines } from "graphics"` | named import | `plot(...)` -> `graphics::plot(...)` |
| `import r { plot as draw_plot } from "graphics"` | named import with alias | `draw_plot(...)` -> `graphics::plot(...)` |
| `import r * as grDevices from "grDevices"` | explicit namespace import | `grDevices.png(...)` -> `grDevices::png(...)` |
| `import r default from "ggplot2"` | sugar for namespace import using package name | `ggplot2.ggplot(...)` -> `ggplot2::ggplot(...)` |

`import r "pkg"` does not emit `library("pkg")`. It is namespace-only sugar.

## Current Direct Interop Surface

### Base/Data

| Package | Supported calls |
| --- | --- |
| `base` | `data.frame` |

### Stats

| Package | Supported calls |
| --- | --- |
| `stats` | `median`, `sd`, `lm`, `glm`, `predict`, `quantile`, `as.formula` |

### IO / Reshape

| Package | Supported calls |
| --- | --- |
| `readr` | `read_csv`, `write_csv` |
| `tidyr` | `pivot_longer`, `pivot_wider` |

### Graphics / Visualization

| Package | Supported calls |
| --- | --- |
| `graphics` | `plot`, `lines`, `legend` |
| `grDevices` | `png`, `dev.off` |
| `ggplot2` | `aes`, `ggplot`, `geom_line`, `geom_point`, `ggtitle`, `theme_minimal`, `ggsave` |

### dplyr Verbs

| Package | Supported calls |
| --- | --- |
| `dplyr` | `mutate`, `filter`, `select`, `summarise`, `arrange`, `group_by`, `rename` |

## Tidy-Eval Support

Inside `ggplot2::aes(...)` and selected `dplyr::*` / `tidyr::*` verbs, RR preserves bare unresolved names as raw R symbols instead of rejecting them as undefined RR variables.

Currently supported tidy data-mask calls:

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

Currently supported helper names in those contexts:

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

Force rules:

- `@name` forces a raw R column symbol
- `^expr` forces an RR environment expression

## Stability Rules

- Supported direct interop calls should not emit `# rr-hybrid-fallback:`
- Unsupported namespaced calls should prefer opaque interop over hybrid fallback
- Truly dynamic runtime features remain hybrid fallback by design

## Examples

- [Language Reference](language.md)
- Visualization examples live under `/example/visualization`
- Data-science interop examples live under `/example/data_science`
