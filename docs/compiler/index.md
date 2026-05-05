# Compiler Docs

This section is the implementation-side view of RR. Read it when you are
changing the compiler, debugging emitted output, reviewing optimization
behavior, or validating a compiler patch.

If you just want to write RR and run it, go back to [Docs Home](../index.md)
and stay in the user docs flow.

## Page Map

- [Compiler Pipeline](pipeline.md): end-to-end compile flow from source loading
  to emitted `.R`
- [Adaptive Phase Ordering Design](adaptive-phase-ordering.md): implemented
  function-sensitive scheduling plus the remaining design boundary for Tachyon
  heavy-tier passes
- [Compile-Time Reduction Plan](compile-time-reduction.md): structured plan for
  replacing repeated peephole rescans with function facts and pass-local caches
- [Parallel Compilation Design](parallel-compilation.md): where compile-time
  parallelism lives and what it is allowed to do
- [IR Model](ir-model.md): HIR and MIR structure, invariants, and ownership
- [Tachyon Engine](optimization.md): optimizer stages, proof model, and
  pass-level behavior
- [MIR SROA Design](sroa.md): implemented aggregate scalar replacement subset
  for `RecordLit` / `FieldGet` / `FieldSet`, plus the remaining ABI boundary
- [Runtime and Error Model](runtime-and-errors.md): emitted helpers,
  diagnostics, and runtime policy
- [Unsafe Boundaries](unsafe-boundaries.md): Rust `unsafe` use in the compiler
  and the separate RR `unsafe r` escape hatch
- [Testing and Quality Gates](testing.md): CI tiers, optimizer suites, soak
  coverage, and regression workflows
- [`CONTRIBUTING.md`](https://github.com/Feralthedogg/RR/blob/main/CONTRIBUTING.md):
  repository-wide compiler code rules that the audit/checklist verifies
- [Contributing Audit Checklist](contributing-audit.md): pre-landing review
  rules for compiler changes

## Suggested Reading Paths

If you are orienting to the compiler:

1. Read [Compiler Pipeline](pipeline.md).
2. Read [IR Model](ir-model.md).
3. Read [Tachyon Engine](optimization.md).

If you are debugging a wrong-code or emitted-R issue:

1. Read [Compiler Pipeline](pipeline.md).
2. Read [Tachyon Engine](optimization.md).
3. Read [Runtime and Error Model](runtime-and-errors.md).
4. Read [Testing and Quality Gates](testing.md).

If you are changing scheduling or compile latency:

1. Read [Parallel Compilation Design](parallel-compilation.md).
2. Read [Adaptive Phase Ordering Design](adaptive-phase-ordering.md).
3. Read [Compile-Time Reduction Plan](compile-time-reduction.md).
4. Read [Compiler Pipeline](pipeline.md).
5. Read [Testing and Quality Gates](testing.md).

If you are preparing a patch for review:

1. Read [`CONTRIBUTING.md`](https://github.com/Feralthedogg/RR/blob/main/CONTRIBUTING.md).
2. Read [Testing and Quality Gates](testing.md).
3. Read [Contributing Audit Checklist](contributing-audit.md).
4. If the patch touches `unsafe` or raw R escape hatches, read
   [Unsafe Boundaries](unsafe-boundaries.md).
