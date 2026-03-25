# Benchmark Workloads

This directory holds RR programs intended for repeatable compiler and runtime performance comparison.

Workloads are chosen to cover:

- vector-heavy transforms
- resampling/data-science loops
- stencil-style physics updates
- iterative orbital integration
- reaction-diffusion style state evolution
- cross-language comparison kernels that are easy to map to C, NumPy, Julia, and base R

All benchmark programs are expected to both compile and execute.
Benchmarks that need pseudo-random input should size their generated draw buffers
from the same loop bounds they consume, rather than hard-coding shorter buffers.

Highlighted current cross-language workload:

- `signal_pipeline_bench.rr`: 250k-sample preprocessing kernel with map, conditional
  map, call-map, and state update passes; used by `scripts/bench_signal_pipeline.py`
  for RR/base R/C/NumPy/Julia/Renjin comparison.

Highlighted backend-comparison workloads:

- `heat_diffusion_bench.rr` and `reaction_diffusion_bench.rr`: stencil-style
  state evolution kernels used by `scripts/bench_diffusion_backends.py` to
  compare emitted R, fused native, process-parallel R, and native + OpenMP
  backend paths.
- `vector_fusion_bench.rr`, `orbital_sweep_bench.rr`, and
  `bootstrap_resample_bench.rr`: backend-candidate workloads used by
  `scripts/bench_backend_candidates.py` to compare emitted R against fused
  native, process-parallel R, and native + OpenMP variants.

Suggested runner:

```bash
scripts/bench_examples.sh
```
