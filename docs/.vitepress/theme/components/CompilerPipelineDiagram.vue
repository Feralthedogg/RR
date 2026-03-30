<script setup lang="ts">
const code = String.raw`flowchart TD
    CLI["CLI / API entrypoints<br/>RR file.rr | RR run | RR build | RR watch"] --> SA
    SRC["Entry source + imported modules"] --> SA

    subgraph FRONTEND["Source Analysis + Canonicalization"]
      SA["1. Source Analysis<br/>parse + scope resolution<br/>module loading<br/>AST -> HIR lowering"]
      CA["2. Canonicalization<br/>HIR desugaring / normalization"]
      SA --> CA
    end

    subgraph MIR["MIR Synthesis + Static Validation"]
      SSA["3. SSA Graph Synthesis<br/>HIR -> FnIR / CFG / SSA-like values"]
      TY["Type analysis<br/>analyze_program_with_compiler_parallel"]
      SEM["Static validation<br/>semantic + runtime-safety validation"]
      CA --> SSA
      SSA --> TY
      TY --> SEM
    end

    SEM --> OPTSEL{"optimize?"}

    subgraph TACHYON["Tachyon"]
      O0["4. Tachyon Stabilization<br/>-O0 safe/codegen-ready rewrites"]
      O12["4. Tachyon Optimization<br/>-O1/-O2 aggressive passes<br/>SCCP / GVN / LICM / BCE / DCE / TCO<br/>vectorization / reductions / de-SSA"]
    end

    OPTSEL -->|"-O0"| O0
    OPTSEL -->|"-O1 / -O2"| O12

    O0 --> VERIFY["Post-opt validation<br/>validate_program<br/>validate_runtime_safety<br/>verify_emittable_program"]
    O12 --> VERIFY

    subgraph EMIT["Emission + Artifact Finalization"]
      EMITR["5. R Code Emission<br/>structurize CFG<br/>emit functions + source maps<br/>peephole cleanup"]
      RT["6. Runtime Injection<br/>full runtime or helper-only (--no-runtime)<br/>append compile-time policy"]
      VERIFY --> EMITR
      EMITR --> RT
    end

    RT --> ART["Final .R artifact + source map"]

    style SRC fill:#4a9eff,color:#fff
    style ART fill:#22c55e,color:#fff
    style O12 fill:#f59e0b,color:#fff`
</script>

<template>
  <MermaidDiagram :code="code" />
</template>
