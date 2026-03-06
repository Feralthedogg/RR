# Example Catalog

This directory contains executable RR examples.

- `common/`: shared helper code for simulation examples.
- `data_science/`: deterministic data science and statistical simulation examples.
- `physics/`: deterministic numerical physics simulation examples.
- `benchmarks/`: repeatable compile/runtime benchmark workloads.
- `tesseract.rr`: large end-to-end showcase.

The data science and physics folders currently contain 16 entry examples total.

Useful commands:

```bash
cargo test -q --test example_simulations
scripts/bench_examples.sh
```
