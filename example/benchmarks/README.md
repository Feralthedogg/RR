# Benchmark Workloads

This directory holds RR programs intended for repeatable compiler and runtime performance comparison.

Workloads are chosen to cover:

- vector-heavy transforms
- resampling/data-science loops
- stencil-style physics updates
- iterative orbital integration
- reaction-diffusion style state evolution

Suggested runner:

```bash
scripts/bench_examples.sh
```
