#[cfg(test)]
pub(crate) mod tests {
    use super::super::super::TachyonEngine;
    use super::super::super::types::{
        FunctionPhaseFeatures, PhaseOrderingMode, PhaseProfileKind, PhaseScheduleId,
    };
    use super::super::HeavyPhaseIterationResult;

    use crate::mir::{Facts, FnIR, Instr, IntrinsicOp, Lit, Terminator, ValueKind};
    use crate::syntax::ast::{BinOp, UnaryOp};
    use crate::utils::Span;
    use rustc_hash::{FxHashMap, FxHashSet};

    fn sample_feature_fn() -> FnIR {
        let mut fn_ir = FnIR::new("phase_features".to_string(), vec!["x".to_string()]);
        let entry = fn_ir.add_block();
        let then_bb = fn_ir.add_block();
        let else_bb = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let param = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let zero = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let _phi = fn_ir.add_value(
            ValueKind::Phi {
                args: vec![(zero, entry), (one, then_bb)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let binary = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: zero,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let _unary = fn_ir.add_value(
            ValueKind::Unary {
                op: UnaryOp::Neg,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let _intrinsic = fn_ir.add_value(
            ValueKind::Intrinsic {
                op: IntrinsicOp::VecAbsF64,
                args: vec![param],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let pure_call = fn_ir.add_value(
            ValueKind::Call {
                callee: "abs".to_string(),
                args: vec![one],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let impure_call = fn_ir.add_value(
            ValueKind::Call {
                callee: "print".to_string(),
                args: vec![one],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let _index = fn_ir.add_value(
            ValueKind::Index1D {
                base: param,
                idx: one,
                is_safe: false,
                is_na_safe: false,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Lt,
                lhs: zero,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );

        fn_ir.blocks[entry].instrs.push(Instr::StoreIndex1D {
            base: param,
            idx: one,
            val: impure_call,
            is_safe: false,
            is_na_safe: false,
            is_vector: false,
            span: Span::default(),
        });
        fn_ir.blocks[entry].term = Terminator::If {
            cond,
            then_bb,
            else_bb,
        };
        fn_ir.blocks[then_bb].term = Terminator::Return(Some(binary));
        fn_ir.blocks[else_bb].term = Terminator::Return(Some(pure_call));
        fn_ir
    }

    #[test]
    fn extract_function_phase_features_counts_basic_shapes() {
        let fn_ir = sample_feature_fn();
        let features = TachyonEngine::extract_function_phase_features(&fn_ir);
        assert_eq!(features.ir_size, TachyonEngine::fn_ir_size(&fn_ir));
        assert_eq!(features.block_count, 3);
        assert_eq!(features.loop_count, 0);
        assert_eq!(features.canonical_loop_count, 0);
        assert_eq!(features.branch_terms, 1);
        assert_eq!(features.phi_count, 1);
        assert_eq!(features.arithmetic_values, 3);
        assert_eq!(features.intrinsic_values, 1);
        assert_eq!(features.call_values, 2);
        assert_eq!(features.side_effecting_calls, 1);
        assert_eq!(features.index_values, 1);
        assert_eq!(features.store_instrs, 1);
    }

    #[test]
    fn collect_function_phase_plans_only_keeps_selected_safe_candidates() {
        let mut all_fns = FxHashMap::default();
        all_fns.insert("selected".to_string(), sample_feature_fn());

        let mut self_recursive = FnIR::new("self_recursive".to_string(), vec![]);
        let entry = self_recursive.add_block();
        self_recursive.entry = entry;
        self_recursive.body_head = entry;
        let call = self_recursive.add_value(
            ValueKind::Call {
                callee: "self_recursive".to_string(),
                args: Vec::new(),
                names: Vec::new(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        self_recursive.blocks[entry].term = Terminator::Return(Some(call));
        all_fns.insert("self_recursive".to_string(), self_recursive);

        let mut conservative = FnIR::new("conservative".to_string(), vec![]);
        let entry = conservative.add_block();
        conservative.entry = entry;
        conservative.body_head = entry;
        conservative.blocks[entry].term = Terminator::Return(None);
        conservative.mark_unsupported_dynamic("test".to_string());
        all_fns.insert("conservative".to_string(), conservative);

        let ordered = vec![
            "conservative".to_string(),
            "selected".to_string(),
            "self_recursive".to_string(),
        ];
        let selected = FxHashSet::from_iter(["selected".to_string(), "self_recursive".to_string()]);

        let plans =
            TachyonEngine::new().collect_function_phase_plans(&all_fns, &ordered, Some(&selected));
        assert_eq!(plans.len(), 1);
        let plan = plans
            .get("selected")
            .expect("selected function should be planned");
        assert_eq!(plan.function, "selected");
        assert_eq!(plan.profile, PhaseProfileKind::Balanced);
        assert_eq!(plan.schedule, PhaseScheduleId::Balanced);
        assert!(plan.features.is_some());
    }

    #[test]
    fn classify_phase_profile_marks_compute_heavy_features() {
        let features = FunctionPhaseFeatures {
            ir_size: 180,
            block_count: 8,
            loop_count: 3,
            canonical_loop_count: 2,
            branch_terms: 1,
            phi_count: 2,
            arithmetic_values: 24,
            intrinsic_values: 6,
            call_values: 2,
            side_effecting_calls: 0,
            index_values: 8,
            store_instrs: 4,
        };
        assert_eq!(
            TachyonEngine::classify_phase_profile(&features),
            PhaseProfileKind::ComputeHeavy
        );
    }

    #[test]
    fn classify_phase_profile_marks_control_flow_heavy_features() {
        let features = FunctionPhaseFeatures {
            ir_size: 120,
            block_count: 6,
            loop_count: 0,
            canonical_loop_count: 0,
            branch_terms: 4,
            phi_count: 5,
            arithmetic_values: 2,
            intrinsic_values: 0,
            call_values: 3,
            side_effecting_calls: 2,
            index_values: 0,
            store_instrs: 0,
        };
        assert_eq!(
            TachyonEngine::classify_phase_profile(&features),
            PhaseProfileKind::ControlFlowHeavy
        );
    }

    #[test]
    fn build_phase_plan_in_auto_mode_exposes_classified_schedule() {
        let engine = TachyonEngine::new();
        let features = FunctionPhaseFeatures {
            ir_size: 220,
            block_count: 9,
            loop_count: 2,
            canonical_loop_count: 2,
            branch_terms: 1,
            phi_count: 1,
            arithmetic_values: 18,
            intrinsic_values: 4,
            call_values: 1,
            side_effecting_calls: 0,
            index_values: 4,
            store_instrs: 2,
        };
        let plan = engine.build_function_phase_plan_from_features(
            "auto_fn",
            PhaseOrderingMode::Auto,
            true,
            features,
        );
        assert_eq!(plan.profile, PhaseProfileKind::ComputeHeavy);
        assert_eq!(plan.schedule, PhaseScheduleId::ComputeHeavy);
        assert!(plan.trace_requested);
    }

    #[test]
    fn build_phase_plan_in_non_auto_modes_stays_balanced() {
        let engine = TachyonEngine::new();
        let features = FunctionPhaseFeatures {
            ir_size: 120,
            block_count: 4,
            loop_count: 2,
            canonical_loop_count: 2,
            branch_terms: 0,
            phi_count: 0,
            arithmetic_values: 8,
            intrinsic_values: 2,
            call_values: 0,
            side_effecting_calls: 0,
            index_values: 2,
            store_instrs: 1,
        };
        for mode in [PhaseOrderingMode::Off, PhaseOrderingMode::Balanced] {
            let plan =
                engine.build_function_phase_plan_from_features("non_auto", mode, false, features);
            assert_eq!(plan.profile, PhaseProfileKind::Balanced);
            assert_eq!(plan.schedule, PhaseScheduleId::Balanced);
        }
    }

    #[test]
    fn control_flow_structural_gate_requires_canonical_low_branch_features() {
        let mut features = FunctionPhaseFeatures {
            ir_size: 160,
            block_count: 8,
            loop_count: 2,
            canonical_loop_count: 1,
            branch_terms: 1,
            phi_count: 3,
            arithmetic_values: 6,
            intrinsic_values: 0,
            call_values: 1,
            side_effecting_calls: 0,
            index_values: 2,
            store_instrs: 1,
        };
        assert!(TachyonEngine::control_flow_structural_gate(&features));
        features.branch_terms = 4;
        assert!(!TachyonEngine::control_flow_structural_gate(&features));
    }

    #[test]
    fn control_flow_fallback_triggers_only_on_low_progress() {
        assert!(TachyonEngine::control_flow_should_fallback_to_balanced(
            HeavyPhaseIterationResult {
                changed: false,
                non_structural_changes: 1,
                structural_progress: false,
                ran_structural: false,
                skipped_structural: true,
            }
        ));
        assert!(!TachyonEngine::control_flow_should_fallback_to_balanced(
            HeavyPhaseIterationResult {
                changed: true,
                non_structural_changes: 3,
                structural_progress: false,
                ran_structural: false,
                skipped_structural: true,
            }
        ));
        assert!(!TachyonEngine::control_flow_should_fallback_to_balanced(
            HeavyPhaseIterationResult {
                changed: true,
                non_structural_changes: 0,
                structural_progress: true,
                ran_structural: true,
                skipped_structural: false,
            }
        ));
    }
}
