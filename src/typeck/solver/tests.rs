#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::Facts;
    use crate::syntax::ast::Lit;

    fn init_entry(fn_ir: &mut FnIR) {
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;
    }

    #[test]
    fn analyze_program_propagates_scalar_index_return_demand() {
        let mut producer = FnIR::new("Sym_1".to_string(), vec!["x".to_string()]);
        init_entry(&mut producer);
        let prod_param = producer.add_value(
            ValueKind::Param { index: 0 },
            crate::utils::Span::dummy(),
            Facts::empty(),
            Some("x".to_string()),
        );
        producer.blocks[producer.entry].term = Terminator::Return(Some(prod_param));

        let mut consumer = FnIR::new(
            "Sym_2".to_string(),
            vec!["arr".to_string(), "seed".to_string()],
        );
        init_entry(&mut consumer);
        let arr = consumer.add_value(
            ValueKind::Param { index: 0 },
            crate::utils::Span::dummy(),
            Facts::empty(),
            Some("arr".to_string()),
        );
        let seed = consumer.add_value(
            ValueKind::Param { index: 1 },
            crate::utils::Span::dummy(),
            Facts::empty(),
            Some("seed".to_string()),
        );
        let call_idx = consumer.add_value(
            ValueKind::Call {
                callee: "Sym_1".to_string(),
                args: vec![seed],
                names: vec![None],
            },
            crate::utils::Span::dummy(),
            Facts::empty(),
            None,
        );
        let read = consumer.add_value(
            ValueKind::Index1D {
                base: arr,
                idx: call_idx,
                is_safe: false,
                is_na_safe: false,
            },
            crate::utils::Span::dummy(),
            Facts::empty(),
            None,
        );
        consumer.blocks[consumer.entry].term = Terminator::Return(Some(read));

        let mut all_fns: FxHashMap<String, FnIR> = FxHashMap::default();
        all_fns.insert("Sym_1".to_string(), producer);
        all_fns.insert("Sym_2".to_string(), consumer);

        analyze_program(
            &mut all_fns,
            TypeConfig {
                mode: TypeMode::Gradual,
                native_backend: NativeBackend::Off,
            },
        )
        .expect("type analysis should succeed");

        let consumer_after = all_fns.get("Sym_2").expect("missing Sym_2");
        let call_ty = consumer_after.values[call_idx].value_ty;
        assert_eq!(call_ty.shape, ShapeTy::Scalar);
        assert_eq!(call_ty.prim, PrimTy::Int);
    }

    #[test]
    fn analyze_program_propagates_vector_index_return_demand() {
        let mut producer = FnIR::new("Sym_10".to_string(), vec!["x".to_string()]);
        init_entry(&mut producer);
        let prod_param = producer.add_value(
            ValueKind::Param { index: 0 },
            crate::utils::Span::dummy(),
            Facts::empty(),
            Some("x".to_string()),
        );
        producer.blocks[producer.entry].term = Terminator::Return(Some(prod_param));

        let mut kernel = FnIR::new(
            "Sym_20".to_string(),
            vec!["arr".to_string(), "idx_vec".to_string()],
        );
        init_entry(&mut kernel);
        let arr = kernel.add_value(
            ValueKind::Param { index: 0 },
            crate::utils::Span::dummy(),
            Facts::empty(),
            Some("arr".to_string()),
        );
        let idx_vec = kernel.add_value(
            ValueKind::Param { index: 1 },
            crate::utils::Span::dummy(),
            Facts::empty(),
            Some("idx_vec".to_string()),
        );
        let one = kernel.add_value(
            ValueKind::Const(Lit::Int(1)),
            crate::utils::Span::dummy(),
            Facts::empty(),
            None,
        );
        let idx_read = kernel.add_value(
            ValueKind::Index1D {
                base: idx_vec,
                idx: one,
                is_safe: false,
                is_na_safe: false,
            },
            crate::utils::Span::dummy(),
            Facts::empty(),
            None,
        );
        let floored = kernel.add_value(
            ValueKind::Call {
                callee: "floor".to_string(),
                args: vec![idx_read],
                names: vec![None],
            },
            crate::utils::Span::dummy(),
            Facts::empty(),
            None,
        );
        let gather = kernel.add_value(
            ValueKind::Index1D {
                base: arr,
                idx: floored,
                is_safe: false,
                is_na_safe: false,
            },
            crate::utils::Span::dummy(),
            Facts::empty(),
            None,
        );
        kernel.blocks[kernel.entry].term = Terminator::Return(Some(gather));

        let mut wrapper = FnIR::new(
            "Sym_30".to_string(),
            vec!["arr".to_string(), "seed".to_string()],
        );
        init_entry(&mut wrapper);
        let wrapper_arr = wrapper.add_value(
            ValueKind::Param { index: 0 },
            crate::utils::Span::dummy(),
            Facts::empty(),
            Some("arr".to_string()),
        );
        let wrapper_seed = wrapper.add_value(
            ValueKind::Param { index: 1 },
            crate::utils::Span::dummy(),
            Facts::empty(),
            Some("seed".to_string()),
        );
        let call_idx_vec = wrapper.add_value(
            ValueKind::Call {
                callee: "Sym_10".to_string(),
                args: vec![wrapper_seed],
                names: vec![None],
            },
            crate::utils::Span::dummy(),
            Facts::empty(),
            None,
        );
        let call_kernel = wrapper.add_value(
            ValueKind::Call {
                callee: "Sym_20".to_string(),
                args: vec![wrapper_arr, call_idx_vec],
                names: vec![None, None],
            },
            crate::utils::Span::dummy(),
            Facts::empty(),
            None,
        );
        wrapper.blocks[wrapper.entry].term = Terminator::Return(Some(call_kernel));

        let mut all_fns: FxHashMap<String, FnIR> = FxHashMap::default();
        all_fns.insert("Sym_10".to_string(), producer);
        all_fns.insert("Sym_20".to_string(), kernel);
        all_fns.insert("Sym_30".to_string(), wrapper);

        let index_slots = collect_index_vector_param_slots_by_function(&all_fns);
        assert!(
            index_slots
                .get("Sym_20")
                .is_some_and(|slots| slots.contains(&1)),
            "expected Sym_20 arg #2 to be detected as index-vector parameter"
        );
        let vec_demands = collect_vector_index_return_demands(&all_fns, &index_slots);
        assert!(
            vec_demands.contains("Sym_10"),
            "expected Sym_10 return to be demanded as index-vector producer"
        );

        analyze_program(
            &mut all_fns,
            TypeConfig {
                mode: TypeMode::Gradual,
                native_backend: NativeBackend::Off,
            },
        )
        .expect("type analysis should succeed");

        let wrapper_after = all_fns.get("Sym_30").expect("missing Sym_30");
        let call_ty = wrapper_after.values[call_idx_vec].value_ty;
        assert_eq!(call_ty.shape, ShapeTy::Vector);
        assert_eq!(call_ty.prim, PrimTy::Int);
    }
}
