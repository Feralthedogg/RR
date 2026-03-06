---
layout: home

hero:
  name: RR
  text: An Optimizing Compiler for R-Oriented Code
  tagline: R-first syntax, MIR-based optimization, and self-contained R output documented from the implementation.
  actions:
    - theme: brand
      text: Getting Started
      link: /getting-started
    - theme: alt
      text: CLI Reference
      link: /cli
    - theme: alt
      text: Compiler Pipeline
      link: /compiler-pipeline

features:
  - title: R-First Surface
    details: RR accepts familiar R conventions such as <code><-</code>, <code>function(...)</code>, dotted identifiers, and range-style loops.
  - title: MIR-Based Optimization
    details: Programs lower through HIR and SSA-like MIR before Tachyon applies SCCP, BCE, vectorization, inlining, and related passes.
  - title: Self-Contained Output
    details: RR emits standalone <code>.R</code> scripts with runtime helpers injected when needed, so output can run through ordinary <code>Rscript</code>.
  - title: Implementation-Tracked Docs
    details: Reference and internals docs are written against the code in <code>src/compiler</code>, <code>src/mir</code>, <code>src/runtime</code>, and tests.
---
