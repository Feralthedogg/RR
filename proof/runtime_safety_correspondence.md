# Runtime Safety Proof ↔ Rust Correspondence

This note ties the reduced runtime-safety proof slice in `proof/` to the
concrete range-analysis and diagnostic checks in `src/mir/`.

It is not a full proof of `validate_runtime_safety`. The point is narrower:

- make explicit which reduced theorem matches the current field-range hazard
  story
- identify the Rust helpers that consume those range facts
- keep the remaining proof gap visible

## Field Range Hazard Slice

Proof layers:
- [RuntimeSafetyFieldRangeSubset.lean](/Users/feral/Desktop/Programming/RR/proof/lean/RRProofs/RuntimeSafetyFieldRangeSubset.lean:1)
- [RuntimeSafetyFieldRangeSubset.v](/Users/feral/Desktop/Programming/RR/proof/coq/RuntimeSafetyFieldRangeSubset.v:1)

Core proof claim:
- reduced record-field interval propagation preserves exact singleton intervals
- negative singleton intervals survive
  - plain field reads
  - nested field reads
  - negative `FieldSet` overrides
- positive `FieldSet` overrides clear the reduced `< 1` hazard

Primary Rust correspondence:
- [range.rs](/Users/feral/Desktop/Programming/RR/src/mir/analyze/range.rs:349)
  `ensure_field_range()` joins the candidate field values collected from
  `RecordLit` / `FieldSet` structure
- [runtime_proofs.rs](/Users/feral/Desktop/Programming/RR/src/mir/semantics/runtime_proofs.rs:108)
  `interval_guarantees_below_one()` projects reduced range facts into the
  1-based indexing hazard
- [runtime_proofs.rs](/Users/feral/Desktop/Programming/RR/src/mir/semantics/runtime_proofs.rs:114)
  `interval_guarantees_negative()` projects reduced range facts into the
  negative-length hazard
- [semantics.rs](/Users/feral/Desktop/Programming/RR/src/mir/semantics.rs:366)
  `validate_function_runtime()` is the concrete consumer that combines those
  range predicates with E2007-style diagnostics

Concrete Rust regressions:
- [field_get_reads_exact_field_interval_from_record_literal](/Users/feral/Desktop/Programming/RR/src/mir/analyze/range.rs:600)
- [field_get_tracks_fieldset_override_range_precisely](/Users/feral/Desktop/Programming/RR/src/mir/analyze/range.rs:633)
- [nested_field_get_reads_exact_interval](/Users/feral/Desktop/Programming/RR/src/mir/analyze/range.rs:676)
- [field_get_reads_exact_interval_through_phi_merged_records](/Users/feral/Desktop/Programming/RR/src/mir/analyze/range.rs:726)
- [field_get_joins_interval_through_phi_merged_records](/Users/feral/Desktop/Programming/RR/src/mir/analyze/range.rs:779)
- [runtime_safety_flags_negative_index_through_phi_merged_record_field](/Users/feral/Desktop/Programming/RR/src/mir/semantics.rs:155)
- [runtime_safety_does_not_treat_unknown_index_as_proven_below_one](/Users/feral/Desktop/Programming/RR/src/mir/semantics.rs:269)
- [runtime_safety_does_not_treat_unknown_seq_len_arg_as_proven_negative](/Users/feral/Desktop/Programming/RR/src/mir/semantics.rs:329)
- [runtime_safety_flags_negative_seq_len_through_nested_record_field](/Users/feral/Desktop/Programming/RR/src/mir/semantics.rs:366)
- [runtime_safety_flags_negative_seq_len_through_fieldset_override](/Users/feral/Desktop/Programming/RR/src/mir/semantics.rs:435)
- [runtime_safety_does_not_flag_positive_seq_len_after_fieldset_override](/Users/feral/Desktop/Programming/RR/src/mir/semantics.rs:503)

Current gap:
- proof is still expression/range-level only
- it does not yet model the whole block/dataflow fixed-point used by
  `validate_function_runtime()`
- it also abstracts away unrelated runtime hazards such as NA propagation,
  aliasing, and non-field index arithmetic
