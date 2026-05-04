use super::*;
use crate::mir::analyze::effects;
use crate::utils::Span;

pub(crate) type SroaFieldMap = FxHashMap<String, ValueId>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SroaAnalysis {
    pub(crate) candidates: Vec<SroaCandidate>,
}

impl SroaAnalysis {
    pub(crate) fn counts(&self) -> SroaAnalysisCounts {
        let mut counts = SroaAnalysisCounts {
            candidates: self.candidates.len(),
            ..SroaAnalysisCounts::default()
        };
        for candidate in &self.candidates {
            match candidate.source {
                SroaCandidateSource::RecordLit => counts.record_lits += 1,
                SroaCandidateSource::FieldSet => counts.field_sets += 1,
                SroaCandidateSource::Phi => counts.phis += 1,
                SroaCandidateSource::LoadAlias => counts.load_aliases += 1,
            }
            match candidate.status {
                SroaCandidateStatus::ScalarOnly => counts.scalar_only += 1,
                SroaCandidateStatus::NeedsRematerialization => counts.needs_rematerialization += 1,
                SroaCandidateStatus::Rejected => counts.rejected += 1,
            }
        }
        counts
    }

    pub(crate) fn candidate(&self, value: ValueId) -> Option<&SroaCandidate> {
        self.candidates
            .iter()
            .find(|candidate| candidate.value == value)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SroaAnalysisCounts {
    pub(crate) candidates: usize,
    pub(crate) record_lits: usize,
    pub(crate) field_sets: usize,
    pub(crate) phis: usize,
    pub(crate) load_aliases: usize,
    pub(crate) scalar_only: usize,
    pub(crate) needs_rematerialization: usize,
    pub(crate) rejected: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SroaCandidate {
    pub(crate) value: ValueId,
    pub(crate) source: SroaCandidateSource,
    pub(crate) shape: Option<Vec<String>>,
    pub(crate) uses: Vec<SroaUse>,
    pub(crate) status: SroaCandidateStatus,
    pub(crate) reject_reasons: Vec<SroaRejectReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SroaCandidateSource {
    RecordLit,
    FieldSet,
    Phi,
    LoadAlias,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SroaCandidateStatus {
    ScalarOnly,
    NeedsRematerialization,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SroaUse {
    pub(crate) user: SroaUser,
    pub(crate) kind: SroaUseKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SroaUseKind {
    Projection,
    Update,
    Alias,
    Phi,
    Materialize,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SroaUser {
    Value(ValueId),
    Instr { block: BlockId, instr: usize },
    Terminator { block: BlockId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SroaMaterializationBoundaryKind {
    Eval,
    Return,
    CallArg,
    IntrinsicArg,
    RecordField,
    FieldSetBase,
    FieldSetValue,
    ConcreteBase,
    StoreIndexOperand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SroaMaterializationBoundary {
    pub(crate) value: ValueId,
    pub(crate) kind: SroaMaterializationBoundaryKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StoreIndexOperand {
    Base,
    Index,
    Row,
    Column,
    Plane,
    Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SroaRejectReason {
    EmptyRecord,
    DuplicateField(String),
    MissingBaseShape,
    ShapeChangingFieldSet(String),
    EmptyPhi,
    InconsistentPhiShape,
    UnsupportedUse(SroaUseKind),
}

pub(crate) fn analyze_function(fn_ir: &FnIR) -> SroaAnalysis {
    let uses = build_use_graph(fn_ir);
    let (shapes, base_rejects) = infer_candidate_shapes(fn_ir);
    let mut candidates = Vec::new();

    for value in &fn_ir.values {
        let Some(source) = candidate_source(fn_ir, value.id, &shapes) else {
            continue;
        };
        let mut reject_reasons = base_rejects.get(&value.id).cloned().unwrap_or_default();
        let shape = shapes.get(&value.id).cloned();
        let value_uses = uses.get(&value.id).cloned().unwrap_or_default();

        for value_use in &value_uses {
            if value_use.kind == SroaUseKind::Reject {
                reject_reasons.push(SroaRejectReason::UnsupportedUse(value_use.kind));
            }
        }

        let status = if !reject_reasons.is_empty() || shape.is_none() {
            SroaCandidateStatus::Rejected
        } else if value_uses
            .iter()
            .any(|value_use| value_use.kind == SroaUseKind::Materialize)
        {
            SroaCandidateStatus::NeedsRematerialization
        } else {
            SroaCandidateStatus::ScalarOnly
        };

        candidates.push(SroaCandidate {
            value: value.id,
            source,
            shape,
            uses: value_uses,
            status,
            reject_reasons,
        });
    }

    SroaAnalysis { candidates }
}

pub(crate) fn optimize(fn_ir: &mut FnIR) -> bool {
    if fn_ir.requires_conservative_optimization() {
        return false;
    }

    let mut changed = false;
    let max_rounds = fn_ir.values.len().saturating_add(1).max(1);
    for _ in 0..max_rounds {
        let round_changed = optimize_once(fn_ir);
        changed |= round_changed;
        if !round_changed {
            break;
        }
    }

    changed
}

#[path = "sroa/core_rewrite.rs"]
mod core_rewrite;
pub(crate) use self::core_rewrite::*;
#[path = "sroa/call_specialization.rs"]
mod call_specialization;
pub(crate) use self::call_specialization::*;
#[path = "sroa/debug.rs"]
mod debug;
#[cfg(test)]
#[path = "sroa/tests.rs"]
pub(crate) mod tests;
