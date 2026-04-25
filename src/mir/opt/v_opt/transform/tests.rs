#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::opt::loop_analysis::LoopInfo;
    use crate::utils::Span;

    fn simple_non_vectorizable_fn() -> FnIR {
        let mut fn_ir = FnIR::new("tx_fail".to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;
        let ret = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].term = Terminator::Return(Some(ret));
        fn_ir
    }

    fn loop_info_without_apply_site() -> LoopInfo {
        LoopInfo {
            header: 0,
            latch: 0,
            exits: Vec::new(),
            body: FxHashSet::default(),
            is_seq_len: None,
            is_seq_along: None,
            iv: None,
            limit: None,
            limit_adjust: 0,
        }
    }

    #[test]
    fn transactional_apply_preserves_original_ir_on_failure() {
        let mut fn_ir = simple_non_vectorizable_fn();
        let original = format!("{:?}", fn_ir);
        let lp = loop_info_without_apply_site();
        let plan = VectorPlan::Map {
            dest: 0,
            src: 0,
            op: BinOp::Add,
            other: 0,
            shadow_vars: Vec::new(),
        };

        let applied = try_apply_vectorization_transactionally(&mut fn_ir, &lp, plan);
        assert!(!applied);
        assert_eq!(original, format!("{:?}", fn_ir));
    }
}
