use super::*;

#[test]
fn trivial_phi_is_folded_during_sealing_for_pred_local_load_aliases() {
    let symbols = FxHashMap::default();
    let known_functions = FxHashMap::default();
    let mut var_names = FxHashMap::default();
    let local_x = hir::LocalId(0);
    var_names.insert(local_x, "x".to_string());

    let mut lowerer = MirLowerer::new(
        "seal_phi_alias".to_string(),
        vec![],
        var_names,
        &symbols,
        &known_functions,
    );

    let left = lowerer.fn_ir.body_head;
    let right = lowerer.fn_ir.add_block();
    let merge = lowerer.fn_ir.add_block();
    lowerer.defs.entry(right).or_default();
    lowerer.defs.entry(merge).or_default();

    let one = lowerer.fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::default(),
        Facts::empty(),
        None,
    );
    let load_left = lowerer.fn_ir.add_value(
        ValueKind::Load {
            var: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let load_right = lowerer.fn_ir.add_value(
        ValueKind::Load {
            var: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let phi = lowerer.add_phi_placeholder(merge, Span::default());

    lowerer.fn_ir.blocks[left].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: one,
        span: Span::default(),
    });
    lowerer.fn_ir.blocks[left].term = Terminator::Goto(merge);
    lowerer.fn_ir.blocks[right].instrs.push(Instr::Assign {
        dst: "x".to_string(),
        src: one,
        span: Span::default(),
    });
    lowerer.fn_ir.blocks[right].term = Terminator::Goto(merge);
    lowerer.preds.insert(merge, vec![left, right]);
    lowerer
        .defs
        .entry(left)
        .or_default()
        .insert(local_x, load_left);
    lowerer
        .defs
        .entry(right)
        .or_default()
        .insert(local_x, load_right);

    lowerer.add_phi_operands(merge, local_x, phi).unwrap();

    assert!(
        !matches!(lowerer.fn_ir.values[phi].kind, ValueKind::Phi { .. }),
        "sealing should fold load-alias phi inputs that resolve to the same predecessor-local assignment"
    );
    assert_eq!(
        lowerer
            .defs
            .get(&merge)
            .and_then(|m| m.get(&local_x))
            .copied(),
        Some(one)
    );
}
