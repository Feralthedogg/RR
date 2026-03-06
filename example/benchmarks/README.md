# Benchmark Workloads

This directory holds RR programs intended for repeatable compiler and runtime performance comparison.

Workloads are chosen to cover:

- vector-heavy transforms
- resampling/data-science loops
- stencil-style physics updates
- iterative orbital integration
- reaction-diffusion style state evolution

All benchmark programs are expected to both compile and execute.
Benchmarks that need pseudo-random input should size their generated draw buffers
from the same loop bounds they consume, rather than hard-coding shorter buffers.

Suggested runner:

```bash
scripts/bench_examples.sh
```
