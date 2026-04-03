<script setup lang="ts">
const code = String.raw`flowchart TD
    CLI["CLI / API entrypoints<br/>RR file.rr | RR run | RR build | RR watch"] --> SA
    SRC["Entry source + imported modules"] --> SA

    subgraph FRONTEND["Source Analysis + Canonicalization"]
      SA["1. Source Analysis<br/>module loading + parse + HIR lowering<br/>pipeline/phases/source_emit.rs"]
      CA["2. Canonicalization<br/>HIR desugaring / normalization"]
      SA --> CA
    end

    subgraph MIR["MIR Synthesis + Static Validation"]
      SSA["3. MIR Synthesis<br/>HIR -> FnIR / CFG / SSA-like values"]
      TY["Type + term analysis<br/>typeck/solver.rs + typeck/sigs/*"]
      SEM["Static validation<br/>semantic + runtime-safety validation"]
      CA --> SSA
      SSA --> TY
      TY --> SEM
    end

    SEM --> OPTSEL{"optimize?"}

    subgraph TACHYON["Tachyon"]
      O0["4. Tachyon Stabilization<br/>-O0 safe/codegen-ready rewrites"]
      O12["4. Tachyon Optimization<br/>-O1/-O2 passes<br/>SCCP / GVN / LICM / BCE / DCE / vectorization / poly / de-SSA"]
    end

    OPTSEL -->|"-O0"| O0
    OPTSEL -->|"-O1 / -O2"| O12

    O0 --> VERIFY["Post-opt validation<br/>validate_program<br/>validate_runtime_safety<br/>verify_emittable_program"]
    O12 --> VERIFY

    subgraph EMIT["Emission + Artifact Finalization"]
      EMITR["5. R Code Emission<br/>structurize CFG<br/>emit function fragments + source maps<br/>codegen/mir_emit.rs + codegen/emit/*"]
      CLEAN["6. Artifact cleanup<br/>raw_rewrites + peephole canonicalization<br/>pipeline/* + compiler/peephole/*"]
      RT["7. Runtime Injection<br/>runtime subset + policy bootstrap<br/>helper-only or full runtime"]
      VERIFY --> EMITR
      EMITR --> CLEAN
      CLEAN --> RT
    end

    RT --> ART["Final .R artifact + source map"]

    style SRC fill:#4a9eff,color:#fff
    style ART fill:#22c55e,color:#fff
    style O12 fill:#f59e0b,color:#fff`
</script>

<template>
  <MermaidDiagram :code="code" />
</template>
