---
layout: home

hero:
  name: RR
  text: An Optimizing Compiler for R
  tagline: R-first syntax with SSA-based MIR and Tachyon optimizer. Compiles to self-contained .R scripts.
  actions:
    - theme: brand
      text: Getting Started
      link: /getting-started
    - theme: alt
      text: Language Reference
      link: /language

features:
  - title: R-First Syntax
    details: Write with familiar R conventions like <code><-</code>, <code>function()</code>, and dotted identifiers, plus pattern matching and pipes.
  - title: Tachyon Optimizer
    details: SSA-based MIR with SCCP, GVN, LICM, vectorization, inlining, TCO, and bounds-check elimination across -O0/-O1/-O2 levels.
  - title: Self-Contained Output
    details: Compiles to plain .R scripts with an embedded runtime. Runs anywhere Rscript is available, no extra dependencies needed.
---