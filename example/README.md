# Example Catalog

This directory contains executable RR examples.

- `common/`: shared helper code for simulation examples.
- `data_science/`: deterministic data science and statistical simulation examples.
- `physics/`: deterministic numerical physics simulation examples.
- `visualization/`: RR examples that call R plotting libraries and emit figures.
- `benchmarks/`: repeatable compile/runtime benchmark workloads.
- `tesseract.rr`: large end-to-end showcase.

Notes:

- `tesseract.rr` intentionally demonstrates RR-style `floor`/`round` shadowing for cube-index helpers.
- Most math/aggregation builtin names in RR are reserved for intrinsic lowering. Example-only helpers should use distinct names such as `demo_abs` or `demo_sqrt`.

The data science and physics folders currently contain 17 entry examples total.

Useful commands:

```bash
cargo test -q --test example_simulations
cargo test -q --test example_visualization
scripts/bench_examples.sh
```

R interop note:

- use `import r "graphics"` when you want package-name namespace interop such as `graphics.plot(...)`
- use `import r { plot, lines } from "graphics"` when you want TS-like named symbol binding that lowers to `graphics::plot(...)`, `graphics::lines(...)`
- use `import r { plot as draw_plot } from "graphics"` when you want local aliasing over a package symbol
- use `import r * as grDevices from "grDevices"` when you want namespace-style calls like `grDevices.png(...)`
- use `import r default from "ggplot2"` when you want namespace-style access under the package name itself; this is sugar for `import r * as ggplot2 from "ggplot2"`
- use `import "path.rr"` for normal RR module imports
- inside `ggplot2::aes(...)` and selected `dplyr::*` verbs, bare unresolved names such as `x`, `signal`, or `trend` are preserved as raw tidy-eval symbols
- use `@col` to force a column symbol and `^expr` to force an RR environment expression inside those tidy-eval contexts
- supported visualization/data-frame package calls are treated as direct RR interop
- supported direct-interop packages also include selected `stats`, `readr`, and `tidyr` calls
- unsupported namespaced package calls are still emitted directly, but RR marks the function as opaque interop and keeps optimization conservative

Data-science interop example:

```bash
cargo run -- example/data_science/lm_predict_quantile_band.rr -O2 -o /tmp/lm_predict_quantile_band.R
Rscript --vanilla /tmp/lm_predict_quantile_band.R
```

That example exercises `stats::lm`, `stats::as.formula`, `stats::predict`, and `stats::quantile` in a deterministic, non-visual workflow.

Example:

```rr
import r { plot as draw_plot, lines } from "graphics";
import r default from "grDevices";

let main <- function() {
  grDevices.png(filename = "plot.png", width = 640, height = 360);
  draw_plot(c(1, 2, 3), c(1, 4, 9), type = "l");
  lines(c(1, 2, 3), c(1, 2, 3), col = "tomato");
  grDevices.dev.off();
  0L
}
```

`ggplot2` example:

```bash
cargo run -- example/visualization/ggplot2_line_plot.rr -O2 -o /tmp/ggplot2_line_plot.R
Rscript --vanilla /tmp/ggplot2_line_plot.R
```

Modern RR-style visualization variants:

- [example/visualization/graphics_sine_plot_modern.rr](/Users/feral/Desktop/Programming/RR/example/visualization/graphics_sine_plot_modern.rr)
- [example/visualization/ggplot2_line_plot_modern.rr](/Users/feral/Desktop/Programming/RR/example/visualization/ggplot2_line_plot_modern.rr)
- [example/visualization/dplyr_ggplot2_pipeline_modern.rr](/Users/feral/Desktop/Programming/RR/example/visualization/dplyr_ggplot2_pipeline_modern.rr)
- [example/visualization/readr_tidyr_ggplot2_pipeline_modern.rr](/Users/feral/Desktop/Programming/RR/example/visualization/readr_tidyr_ggplot2_pipeline_modern.rr)
- [example/visualization/stats_quantile_band_plot_modern.rr](/Users/feral/Desktop/Programming/RR/example/visualization/stats_quantile_band_plot_modern.rr)
- [example/visualization/tidyr_summary_pivot_wider_modern.rr](/Users/feral/Desktop/Programming/RR/example/visualization/tidyr_summary_pivot_wider_modern.rr)
- [example/visualization/stats_glm_predict_plot_modern.rr](/Users/feral/Desktop/Programming/RR/example/visualization/stats_glm_predict_plot_modern.rr)
- [example/visualization/readr_dplyr_tidyr_ggplot2_workflow_modern.rr](/Users/feral/Desktop/Programming/RR/example/visualization/readr_dplyr_tidyr_ggplot2_workflow_modern.rr)

These keep the same R interop calls but use `fn main() { ... }` and RR-style assignment instead of `let main <- function() { ... }`.

`dplyr + ggplot2` pipeline example:

```bash
cargo run -- example/visualization/dplyr_ggplot2_pipeline_modern.rr -O2 -o /tmp/dplyr_ggplot2_pipeline_modern.R
Rscript --vanilla /tmp/dplyr_ggplot2_pipeline_modern.R
```

That example now uses direct tidy-eval lowering, so the RR source can write:

```rr
let series = raw |> dplyr.mutate(
    trend = x * 0.18 + 0.35,
    smooth = signal * 0.8 + 0.1
)

let p = ggplot2.ggplot(series, ggplot2.aes(x = x)) +
    ggplot2.geom_line(ggplot2.aes(y = signal), color = "steelblue") +
    ggplot2.geom_line(ggplot2.aes(y = trend), color = "tomato", linetype = 2) +
    ggplot2.geom_point(ggplot2.aes(y = smooth), color = "darkgreen")
```

without precomputing `trend` and `smooth` as separate RR vectors first.

`readr + tidyr + ggplot2` reshape-and-plot example:

```bash
cargo run -- example/visualization/readr_tidyr_ggplot2_pipeline_modern.rr -O2 -o /tmp/readr_tidyr_ggplot2_pipeline_modern.R
Rscript --vanilla /tmp/readr_tidyr_ggplot2_pipeline_modern.R
```

That example writes a small CSV, reads it back with `readr::read_csv`, reshapes it with `tidyr::pivot_longer`, and plots the long-form data with `ggplot2`.

`stats + ggplot2` quantile-band example:

```bash
cargo run -- example/visualization/stats_quantile_band_plot_modern.rr -O2 -o /tmp/stats_quantile_band_plot_modern.R
Rscript --vanilla /tmp/stats_quantile_band_plot_modern.R
```

This example exercises direct `stats::quantile` and `stats::sd` interop without falling back to opaque or hybrid mode.

`stats::glm + predict` example:

```bash
cargo run -- example/visualization/stats_glm_predict_plot_modern.rr -O2 -o /tmp/stats_glm_predict_plot_modern.R
Rscript --vanilla /tmp/stats_glm_predict_plot_modern.R
```

This example builds a small model through `stats::glm(stats::as.formula(...))`, predicts over a grid with `stats::predict`, and plots the fitted line with `ggplot2`.

`tidyr + dplyr` reshape-summary example:

```bash
cargo run -- example/visualization/tidyr_summary_pivot_wider_modern.rr -O2 -o /tmp/tidyr_summary_pivot_wider_modern.R
Rscript --vanilla /tmp/tidyr_summary_pivot_wider_modern.R
```

This example uses `dplyr::group_by`, `dplyr::summarise`, `tidyr::pivot_wider`, and `tidyr::pivot_longer` together, then plots the reshaped summary with `ggplot2`.

Full CSV workflow example:

```bash
cargo run -- example/visualization/readr_dplyr_tidyr_ggplot2_workflow_modern.rr -O2 -o /tmp/readr_dplyr_tidyr_ggplot2_workflow_modern.R
Rscript --vanilla /tmp/readr_dplyr_tidyr_ggplot2_workflow_modern.R
```

This example writes CSV with `readr`, reads it back, reshapes with `tidyr`, enriches with `dplyr::mutate`, and plots the transformed series with `ggplot2`.
