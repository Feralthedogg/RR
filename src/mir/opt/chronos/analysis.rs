use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::mir::opt) struct ChronosAnalysisSet(u64);

impl ChronosAnalysisSet {
    pub(in crate::mir::opt) const EMPTY: Self = Self(0);
    pub(in crate::mir::opt) const CONTROL_FLOW: Self = Self(1 << 0);
    pub(in crate::mir::opt) const VALUE_GRAPH: Self = Self(1 << 1);
    pub(in crate::mir::opt) const LOOP_INFO: Self = Self(1 << 2);
    pub(in crate::mir::opt) const ALIAS_INFO: Self = Self(1 << 3);
    pub(in crate::mir::opt) const SSA_FORM: Self = Self(1 << 4);
    pub(in crate::mir::opt) const RECORD_SHAPE: Self = Self(1 << 5);
    pub(in crate::mir::opt) const RANGE_BOUNDS: Self = Self(1 << 6);
    pub(in crate::mir::opt) const EFFECTS: Self = Self(1 << 7);
    pub(in crate::mir::opt) const CALL_GRAPH: Self = Self(1 << 8);
    pub(in crate::mir::opt) const ESCAPE_INFO: Self = Self(1 << 9);
    pub(in crate::mir::opt) const DOMINANCE: Self = Self(1 << 10);

    pub(in crate::mir::opt) const ALL: Self = Self(
        Self::CONTROL_FLOW.0
            | Self::VALUE_GRAPH.0
            | Self::LOOP_INFO.0
            | Self::ALIAS_INFO.0
            | Self::SSA_FORM.0
            | Self::RECORD_SHAPE.0
            | Self::RANGE_BOUNDS.0
            | Self::EFFECTS.0
            | Self::CALL_GRAPH.0
            | Self::ESCAPE_INFO.0
            | Self::DOMINANCE.0,
    );

    pub(in crate::mir::opt) const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub(in crate::mir::opt) const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub(in crate::mir::opt) const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub(in crate::mir::opt) fn invalidate(&mut self, invalidated: Self) {
        self.0 &= !invalidated.0;
    }

    pub(in crate::mir::opt) fn labels(self) -> Vec<String> {
        let families = [
            (Self::CONTROL_FLOW, "control-flow"),
            (Self::VALUE_GRAPH, "value-graph"),
            (Self::LOOP_INFO, "loop-info"),
            (Self::ALIAS_INFO, "alias-info"),
            (Self::SSA_FORM, "ssa-form"),
            (Self::RECORD_SHAPE, "record-shape"),
            (Self::RANGE_BOUNDS, "range-bounds"),
            (Self::EFFECTS, "effects"),
            (Self::CALL_GRAPH, "call-graph"),
            (Self::ESCAPE_INFO, "escape-info"),
            (Self::DOMINANCE, "dominance"),
        ];
        let mut out = Vec::new();
        for (family, label) in families {
            if self.contains(family) {
                out.push(label.to_string());
            }
        }
        out
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(in crate::mir::opt) struct ChronosFactSnapshot {
    pub(in crate::mir::opt) ir_size: usize,
    pub(in crate::mir::opt) loops: usize,
    pub(in crate::mir::opt) canonical_loops: usize,
    pub(in crate::mir::opt) branches: usize,
    pub(in crate::mir::opt) calls: usize,
    pub(in crate::mir::opt) side_effecting_calls: usize,
    pub(in crate::mir::opt) index_values: usize,
    pub(in crate::mir::opt) stores: usize,
    pub(in crate::mir::opt) unsafe_blocks: usize,
}

#[derive(Debug, Default)]
pub(in crate::mir::opt) struct ChronosAnalysisCache {
    pub(crate) loops: Option<Vec<loop_analysis::LoopInfo>>,
    pub(crate) phase_features: Option<types::FunctionPhaseFeatures>,
    pub(crate) fact_snapshot: Option<ChronosFactSnapshot>,
}

impl ChronosAnalysisCache {
    pub(in crate::mir::opt) fn invalidate(&mut self, invalidated: ChronosAnalysisSet) {
        if invalidated.is_empty() {
            return;
        }
        self.phase_features = None;
        self.fact_snapshot = None;
        if invalidated.contains(ChronosAnalysisSet::CONTROL_FLOW)
            || invalidated.contains(ChronosAnalysisSet::LOOP_INFO)
        {
            self.loops = None;
        }
    }

    pub(in crate::mir::opt) fn loops(&mut self, fn_ir: &FnIR) -> &[loop_analysis::LoopInfo] {
        self.loops
            .get_or_insert_with(|| loop_analysis::LoopAnalyzer::new(fn_ir).find_loops())
            .as_slice()
    }

    pub(in crate::mir::opt) fn phase_features(
        &mut self,
        fn_ir: &FnIR,
    ) -> types::FunctionPhaseFeatures {
        if let Some(features) = self.phase_features {
            return features;
        }

        let loops = self.loops(fn_ir);
        let features = TachyonEngine::extract_function_phase_features_with_loops(fn_ir, loops);
        self.phase_features = Some(features);
        features
    }

    pub(in crate::mir::opt) fn fact_snapshot(&mut self, fn_ir: &FnIR) -> ChronosFactSnapshot {
        if let Some(snapshot) = self.fact_snapshot {
            return snapshot;
        }

        let features = self.phase_features(fn_ir);
        let mut unsafe_blocks = 0usize;
        for block in &fn_ir.blocks {
            for instr in &block.instrs {
                if matches!(instr, Instr::UnsafeRBlock { .. }) {
                    unsafe_blocks += 1;
                }
            }
        }
        let snapshot = ChronosFactSnapshot {
            ir_size: features.ir_size,
            loops: features.loop_count,
            canonical_loops: features.canonical_loop_count,
            branches: features.branch_terms,
            calls: features.call_values + features.intrinsic_values,
            side_effecting_calls: features.side_effecting_calls,
            index_values: features.index_values,
            stores: features.store_instrs,
            unsafe_blocks,
        };
        self.fact_snapshot = Some(snapshot);
        snapshot
    }
}
