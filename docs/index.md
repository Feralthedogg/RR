---
layout: home

hero:
  name: RR
  text: RR Compiler Manual
  tagline: RR Tachyon v7.0.0. R-oriented source, MIR-based optimization, and self-contained R artifacts documented against the implementation.
  actions:
    - theme: brand
      text: Getting Started
      link: /getting-started
    - theme: alt
      text: Language Reference
      link: /language
    - theme: alt
      text: Compiler Pipeline
      link: /compiler-pipeline

features:
  - title: Guide Manuals
    details: Start with build, compile, run, watch, and performance-oriented writing guidance before moving into reference material.
  - title: Reference Manuals
    details: Language, CLI, configuration, compatibility, and R interop are documented as stable user-facing interfaces.
  - title: Internal Manuals
    details: Compiler pipeline, IR model, optimizer, and runtime contracts are described from the current source tree and tests.
  - title: Development Manuals
    details: Testing, audit, and verification pages define the commands and invariants used to keep RR trustworthy.
---

## Documentation Sets

RR documentation is split into four manual sets, following the style used by
systems projects such as the Linux kernel, GCC, and LLVM.

### Guide

Use these pages when you want to get something done with RR.

- [Getting Started](/getting-started)
- [RR for R Users](/r-for-r-users)
- [Writing RR for Performance and Safety](/writing-rr)
- [CLI Reference](/cli)
- [Configuration](/configuration)

### Reference

Use these pages when you need a precise statement of the supported surface.

- [Language Reference](/language)
- [R Interop](/r-interop)
- [Compatibility and Limits](/compatibility)

### Internals

Use these pages when you need to understand why RR emits what it emits.

- [Compiler Pipeline](/compiler-pipeline)
- [IR Model](/ir-model)
- [Tachyon Engine](/optimization)
- [Runtime and Error Model](/runtime-and-errors)

### Development

Use these pages when you are changing the compiler, reviewing a patch, or
triaging a regression.

- [Testing and Quality Gates](/testing)
- [Contributing Audit Checklist](/contributing-audit)

## Reading Paths

If you are new to RR:

1. Read [Getting Started](/getting-started).
2. Read [RR for R Users](/r-for-r-users).
3. Read [CLI Reference](/cli).
4. Read [Language Reference](/language).

If you are validating generated output:

1. Read [Writing RR for Performance and Safety](/writing-rr).
2. Read [Compatibility and Limits](/compatibility).
3. Read [Runtime and Error Model](/runtime-and-errors).

If you are debugging the compiler:

1. Read [Compiler Pipeline](/compiler-pipeline).
2. Read [Tachyon Engine](/optimization).
3. Read [Testing and Quality Gates](/testing).

## Documentation Principles

- Pages describe RR as implemented in this repository, not as an aspirational design.
- User manuals prefer behavior, guarantees, and limits over implementation trivia.
- Internal manuals prefer contracts, phase boundaries, and failure modes over prose-only overviews.
- Testing pages are treated as part of the product contract, not as contributor-only notes.
