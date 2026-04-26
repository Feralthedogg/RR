use crate::compiler::scheduler::{
    CompilerParallelConfig, CompilerParallelStage, CompilerScheduler,
};
use crate::mir::*;
use crate::syntax::ast::BinOp;
use crate::typeck::{LenSym, PrimTy, TypeState, TypeTerm};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::time::Instant;

#[path = "opt/always_tier.rs"]
mod always_tier;
#[path = "opt/callmap.rs"]
mod callmap;
#[path = "opt/cfg_cleanup.rs"]
mod cfg_cleanup;
#[path = "opt/config.rs"]
mod config;
#[path = "opt/copy_cleanup.rs"]
mod copy_cleanup;
#[path = "opt/engine.rs"]
mod engine;
#[path = "opt/helpers.rs"]
mod helpers;
#[path = "opt/index_canonicalization.rs"]
mod index_canonicalization;
#[path = "opt/phase_order.rs"]
mod phase_order;
#[path = "opt/plan.rs"]
mod plan;
#[path = "opt/program_driver.rs"]
mod program_driver;
#[path = "opt/safety.rs"]
mod safety;
#[path = "opt/sroa.rs"]
mod sroa;
#[path = "opt/stabilize.rs"]
mod stabilize;
#[path = "opt/types.rs"]
mod types;
#[path = "opt/value_utils.rs"]
mod value_utils;
#[path = "opt/verify_gate.rs"]
mod verify_gate;

pub mod bce;
pub mod de_ssa;
pub mod fresh_alias;
pub mod fresh_alloc;
#[path = "opt/function_driver.rs"]
mod function_driver;
pub mod gvn;
pub mod inline;
pub mod intrinsics;
pub mod licm;
pub mod loop_analysis;
pub mod loop_opt;
pub mod parallel_copy;
#[path = "opt/poly/mod.rs"]
pub mod poly;
pub mod sccp;
pub mod simplify;
pub mod tco;
pub mod type_specialize;
#[path = "opt/v_opt/mod.rs"]
pub mod v_opt;

pub use self::engine::{MirOptimizer, TachyonEngine, TachyonRunProfile};
use self::types::{FunctionBudgetProfile, FunctionPhasePlan, PhaseScheduleId, ProgramOptPlan};
pub use self::types::{
    TachyonPassTiming, TachyonPassTimings, TachyonProgress, TachyonProgressTier, TachyonPulseStats,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::Span;
    use std::fs;

    fn dummy_fn(name: &str, approx_size: usize) -> FnIR {
        let mut fn_ir = FnIR::new(name.to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let mut ret = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        // fn_ir_size = values + instrs; keep instrs=0 and control value count directly.
        let target_values = approx_size.max(1);
        while fn_ir.values.len() < target_values {
            ret = fn_ir.add_value(
                ValueKind::Const(Lit::Int(fn_ir.values.len() as i64)),
                Span::default(),
                Facts::empty(),
                None,
            );
        }
        fn_ir.blocks[entry].term = Terminator::Return(Some(ret));
        fn_ir
    }

    fn fn_with_unreachable_block(name: &str) -> FnIR {
        let mut fn_ir = FnIR::new(name.to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;
        let dead = fn_ir.add_block();
        let ret = fn_ir.add_value(
            ValueKind::Const(Lit::Int(7)),
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].term = Terminator::Return(Some(ret));
        fn_ir.blocks[dead].term = Terminator::Return(Some(ret));
        fn_ir
    }

    fn program_snapshot(all_fns: &FxHashMap<String, FnIR>) -> Vec<(String, String)> {
        let mut names: Vec<String> = all_fns.keys().cloned().collect();
        names.sort();
        names
            .into_iter()
            .filter_map(|name| {
                all_fns
                    .get(&name)
                    .map(|fn_ir| (name, format!("{:?}", fn_ir)))
            })
            .collect()
    }

    #[test]
    fn opt_plan_selects_all_under_budget() {
        let mut all = FxHashMap::default();
        all.insert("a".to_string(), dummy_fn("a", 120));
        all.insert("b".to_string(), dummy_fn("b", 180));

        let plan = TachyonEngine::build_opt_plan(&all);
        assert!(!plan.selective_mode);
        assert_eq!(plan.selected_functions.len(), all.len());
    }

    #[test]
    fn opt_plan_selects_subset_over_budget() {
        let mut all = FxHashMap::default();
        for i in 0..5 {
            let name = format!("f{}", i);
            all.insert(name.clone(), dummy_fn(&name, 2_100));
        }

        let plan = TachyonEngine::build_opt_plan(&all);
        assert!(plan.selective_mode);
        assert!(!plan.selected_functions.is_empty());
        assert!(plan.selected_functions.len() < all.len());
    }

    #[test]
    fn opt_plan_prefers_profile_hot_function_under_budget() {
        let mut all = FxHashMap::default();
        all.insert("a".to_string(), dummy_fn("a", 620));
        all.insert("b".to_string(), dummy_fn("b", 620));
        all.insert("c".to_string(), dummy_fn("c", 620));
        all.insert("d".to_string(), dummy_fn("d", 620));
        all.insert("hot".to_string(), dummy_fn("hot", 620));

        let mut profile = FxHashMap::default();
        profile.insert("hot".to_string(), 1000usize);
        let plan = TachyonEngine::build_opt_plan_with_profile(&all, &profile);
        assert!(plan.selected_functions.contains("hot"));
    }

    #[test]
    fn always_tier_runs_light_cleanup() {
        let mut f = fn_with_unreachable_block("cleanup");
        let floor_helpers = FxHashSet::default();
        let stats = TachyonEngine::new().run_always_tier_with_stats(&mut f, None, &floor_helpers);
        assert_eq!(stats.always_tier_functions, 1);
        assert!(crate::mir::verify::verify_ir(&f).is_ok());
    }

    #[test]
    fn copy_cleanup_skips_conservative_functions() {
        let mut fn_ir = FnIR::new("opaque_alias".to_string(), vec!["a".to_string()]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let load_a = fn_ir.add_value(
            ValueKind::Load {
                var: "a".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("a".to_string()),
        );
        let load_tmp = fn_ir.add_value(
            ValueKind::Load {
                var: "tmp".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("tmp".to_string()),
        );

        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: "tmp".to_string(),
            src: load_a,
            span: Span::default(),
        });
        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: "out".to_string(),
            src: load_tmp,
            span: Span::default(),
        });
        fn_ir.blocks[entry].term = Terminator::Return(None);
        fn_ir.mark_opaque_interop("preserve raw aliasing".to_string());

        let mut all = FxHashMap::default();
        all.insert(fn_ir.name.clone(), fn_ir);
        let _ = TachyonEngine::new().run_program_with_stats(&mut all);

        let fn_ir = all
            .get("opaque_alias")
            .expect("function should remain present");
        let Instr::Assign { src, .. } = &fn_ir.blocks[entry].instrs[1] else {
            panic!("expected alias assignment to remain in place");
        };
        match &fn_ir.values[*src].kind {
            ValueKind::Load { var } => assert_eq!(var, "tmp"),
            other => panic!("expected load(tmp) to be preserved, got {:?}", other),
        }
    }

    #[test]
    fn run_program_applies_tco_before_recursive_functions_are_skipped() {
        let mut fn_ir = FnIR::new("recur".to_string(), vec!["n".to_string()]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let n = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("n".to_string()),
        );
        let recur = fn_ir.add_value(
            ValueKind::Call {
                callee: "recur".to_string(),
                args: vec![n],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].term = Terminator::Return(Some(recur));

        let mut all = FxHashMap::default();
        all.insert(fn_ir.name.clone(), fn_ir);
        let stats = TachyonEngine::new().run_program_with_stats(&mut all);

        assert!(
            stats.tco_hits > 0,
            "tail recursion should be rewritten before skip logic"
        );
        let fn_ir = all.get("recur").expect("function should remain present");
        assert_ne!(
            fn_ir.body_head, entry,
            "TCO should split a dedicated body_head when entry would otherwise become cyclic"
        );
        assert!(matches!(
            fn_ir.blocks[entry].term,
            Terminator::Goto(target) if target == fn_ir.body_head
        ));
        assert!(matches!(
            fn_ir.blocks[fn_ir.body_head].term,
            Terminator::Goto(target) if target == fn_ir.body_head
        ));
        assert!(crate::mir::verify::verify_ir(fn_ir).is_ok());
    }

    #[test]
    fn floor_helper_detection_and_rewrite_use_builtin_floor() {
        let mut helper = FnIR::new("floorish".to_string(), vec!["x".to_string()]);
        let helper_entry = helper.add_block();
        helper.entry = helper_entry;
        helper.body_head = helper_entry;
        let param = helper.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let one = helper.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let modulo = helper.add_value(
            ValueKind::Binary {
                op: BinOp::Mod,
                lhs: param,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let floorish = helper.add_value(
            ValueKind::Binary {
                op: BinOp::Sub,
                lhs: param,
                rhs: modulo,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        helper.blocks[helper_entry].term = Terminator::Return(Some(floorish));

        let mut caller = FnIR::new("caller".to_string(), vec!["y".to_string()]);
        let caller_entry = caller.add_block();
        caller.entry = caller_entry;
        caller.body_head = caller_entry;
        let caller_param = caller.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("y".to_string()),
        );
        let call = caller.add_value(
            ValueKind::Call {
                callee: "floorish".to_string(),
                args: vec![caller_param],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        caller.blocks[caller_entry].term = Terminator::Return(Some(call));

        let mut all = FxHashMap::default();
        all.insert(helper.name.clone(), helper);
        all.insert(caller.name.clone(), caller);

        let helpers = TachyonEngine::collect_floor_helpers(&all);
        assert!(helpers.contains("floorish"));
        let rewrites = TachyonEngine::rewrite_floor_helper_calls(&mut all, &helpers);
        assert_eq!(rewrites, 1);

        let caller = all.get("caller").expect("caller should exist");
        let ValueKind::Call { callee, .. } = &caller.values[call].kind else {
            panic!("caller value should remain a call");
        };
        assert_eq!(callee, "floor");
    }

    #[test]
    fn run_program_is_stable_across_insertion_order() {
        let mut all_a = FxHashMap::default();
        all_a.insert("alpha".to_string(), dummy_fn("alpha", 450));
        all_a.insert("beta".to_string(), dummy_fn("beta", 460));
        all_a.insert("gamma".to_string(), dummy_fn("gamma", 470));

        let mut all_b = FxHashMap::default();
        all_b.insert("gamma".to_string(), dummy_fn("gamma", 470));
        all_b.insert("beta".to_string(), dummy_fn("beta", 460));
        all_b.insert("alpha".to_string(), dummy_fn("alpha", 450));

        let engine = TachyonEngine::new();
        let _ = engine.run_program_with_stats(&mut all_a);
        let _ = engine.run_program_with_stats(&mut all_b);

        assert_eq!(program_snapshot(&all_a), program_snapshot(&all_b));
    }

    #[test]
    fn run_program_emits_progress_events_in_deterministic_order() {
        let mut all = FxHashMap::default();
        all.insert("gamma".to_string(), dummy_fn("gamma", 470));
        all.insert("beta".to_string(), dummy_fn("beta", 460));
        all.insert("alpha".to_string(), dummy_fn("alpha", 450));

        let engine = TachyonEngine::new();
        let mut events = Vec::new();
        {
            let mut cb = |event: TachyonProgress| events.push(event);
            let _ = engine.run_program_with_stats_progress(&mut all, &mut cb);
        }

        for tier in [
            TachyonProgressTier::Always,
            TachyonProgressTier::Heavy,
            TachyonProgressTier::DeSsa,
        ] {
            let tier_events: Vec<&TachyonProgress> =
                events.iter().filter(|e| e.tier == tier).collect();
            assert_eq!(tier_events.len(), 3);
            assert_eq!(
                tier_events
                    .iter()
                    .map(|e| e.function.as_str())
                    .collect::<Vec<_>>(),
                vec!["alpha", "beta", "gamma"]
            );
            assert!(
                tier_events
                    .windows(2)
                    .all(|w| w[0].completed < w[1].completed)
            );
            let last = tier_events.last().expect("non-empty tier events");
            assert_eq!(last.completed, last.total);
        }
    }

    #[test]
    fn verify_failure_dump_writes_stage_and_function_snapshot() {
        let fn_ir = dummy_fn("dump_target", 8);
        let out_dir = std::env::temp_dir().join(format!(
            "rr-verify-dump-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        TachyonEngine::dump_verify_failure_to(&out_dir, &fn_ir, "vectorize/post", "bad phi");
        let dump_path = out_dir.join("vectorize_post__dump_target.mir.txt");
        let dump = fs::read_to_string(&dump_path).expect("verify dump should be written");
        assert!(dump.contains("stage: vectorize/post"));
        assert!(dump.contains("function: dump_target"));
        assert!(dump.contains("reason: bad phi"));
        assert!(dump.contains("FnIR"));
        let _ = fs::remove_file(&dump_path);
        let _ = fs::remove_dir(&out_dir);
    }

    #[test]
    fn dce_removes_shadowed_dead_assign() {
        let mut fn_ir = FnIR::new("dead_assign".to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let one = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let two = fn_ir.add_value(
            ValueKind::Const(Lit::Int(2)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let load_x = fn_ir.add_value(
            ValueKind::Load {
                var: ".tachyon_x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some(".tachyon_x".to_string()),
        );
        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: ".tachyon_x".to_string(),
            src: one,
            span: Span::default(),
        });
        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: ".tachyon_x".to_string(),
            src: two,
            span: Span::default(),
        });
        fn_ir.blocks[entry].term = Terminator::Return(Some(load_x));

        let changed = TachyonEngine::new().dce(&mut fn_ir);
        assert!(changed);
        assert_eq!(fn_ir.blocks[entry].instrs.len(), 1);
        match &fn_ir.blocks[entry].instrs[0] {
            Instr::Assign { dst, src, .. } => {
                assert_eq!(dst, ".tachyon_x");
                assert_eq!(*src, two);
            }
            other => panic!("unexpected instruction after DCE: {:?}", other),
        }
    }

    #[test]
    fn dce_preserves_side_effectful_rhs_as_eval() {
        let mut fn_ir = FnIR::new("dead_assign_call".to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let side_effect_call = fn_ir.add_value(
            ValueKind::Call {
                callee: "unknown_effect".to_string(),
                args: Vec::new(),
                names: Vec::new(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let zero = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: ".tachyon_tmp".to_string(),
            src: side_effect_call,
            span: Span::default(),
        });
        fn_ir.blocks[entry].term = Terminator::Return(Some(zero));

        let changed = TachyonEngine::new().dce(&mut fn_ir);
        assert!(changed);
        assert_eq!(fn_ir.blocks[entry].instrs.len(), 1);
        match &fn_ir.blocks[entry].instrs[0] {
            Instr::Eval { val, .. } => assert_eq!(*val, side_effect_call),
            other => panic!(
                "side-effectful dead assign should become Eval, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn parse_phase_ordering_mode_defaults_to_off() {
        assert_eq!(
            TachyonEngine::parse_phase_ordering_mode(None),
            super::types::PhaseOrderingMode::Off
        );
        assert_eq!(
            TachyonEngine::parse_phase_ordering_mode(Some("")),
            super::types::PhaseOrderingMode::Off
        );
        assert_eq!(
            TachyonEngine::parse_phase_ordering_mode(Some("unknown")),
            super::types::PhaseOrderingMode::Off
        );
    }

    #[test]
    fn parse_phase_ordering_mode_accepts_supported_values() {
        assert_eq!(
            TachyonEngine::parse_phase_ordering_mode(Some("off")),
            super::types::PhaseOrderingMode::Off
        );
        assert_eq!(
            TachyonEngine::parse_phase_ordering_mode(Some("balanced")),
            super::types::PhaseOrderingMode::Balanced
        );
        assert_eq!(
            TachyonEngine::parse_phase_ordering_mode(Some("AUTO")),
            super::types::PhaseOrderingMode::Auto
        );
    }

    #[test]
    fn legacy_phase_plan_keeps_balanced_schedule_for_all_modes() {
        for mode in [
            super::types::PhaseOrderingMode::Off,
            super::types::PhaseOrderingMode::Balanced,
            super::types::PhaseOrderingMode::Auto,
        ] {
            let plan = super::types::FunctionPhasePlan::legacy("demo".to_string(), mode, false);
            assert_eq!(plan.function, "demo");
            assert_eq!(plan.mode, mode);
            assert_eq!(plan.profile, super::types::PhaseProfileKind::Balanced);
            assert_eq!(plan.schedule, super::types::PhaseScheduleId::Balanced);
            assert!(plan.features.is_none());
            assert!(!plan.trace_requested);
        }
    }

    #[test]
    fn phase_ordering_opt_level_default_maps_o1_and_o2() {
        assert_eq!(
            TachyonEngine::phase_ordering_opt_level_default(crate::compiler::OptLevel::O0),
            super::types::PhaseOrderingMode::Off
        );
        assert_eq!(
            TachyonEngine::phase_ordering_opt_level_default(crate::compiler::OptLevel::O1),
            super::types::PhaseOrderingMode::Balanced
        );
        assert_eq!(
            TachyonEngine::phase_ordering_opt_level_default(crate::compiler::OptLevel::O2),
            super::types::PhaseOrderingMode::Auto
        );
    }
}
