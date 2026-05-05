use super::*;

#[test]
pub(crate) fn sroa_analysis_accepts_straight_line_record_projection() {
    let mut fn_ir = test_fn();
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
    let record = record_xy(&mut fn_ir, x, y);
    let get_x = fn_ir.add_value(
        ValueKind::FieldGet {
            base: record,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(get_x));

    let analysis = analyze_function(&fn_ir);
    let candidate = analysis.candidate(record).expect("record candidate");

    assert_eq!(
        candidate.shape.as_deref(),
        Some(&["x".to_string(), "y".to_string()][..])
    );
    assert_eq!(candidate.status, SroaCandidateStatus::ScalarOnly);
    assert!(
        candidate
            .uses
            .iter()
            .any(|value_use| value_use.kind == SroaUseKind::Projection)
    );
}

#[test]
pub(crate) fn sroa_analysis_marks_returned_record_for_rematerialization() {
    let mut fn_ir = test_fn();
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
    let record = record_xy(&mut fn_ir, x, y);
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(record));

    let analysis = analyze_function(&fn_ir);
    let candidate = analysis.candidate(record).expect("record candidate");

    assert_eq!(
        candidate.status,
        SroaCandidateStatus::NeedsRematerialization
    );
    assert!(
        candidate
            .uses
            .iter()
            .any(|value_use| value_use.kind == SroaUseKind::Materialize)
    );
}

#[test]
pub(crate) fn sroa_analysis_marks_eval_record_for_rematerialization() {
    let mut fn_ir = test_fn();
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
    let done = int_value(&mut fn_ir, 0);
    let record = record_xy(&mut fn_ir, x, y);
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Eval {
        val: record,
        span: Span::default(),
    });
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(done));

    let analysis = analyze_function(&fn_ir);
    let candidate = analysis.candidate(record).expect("record candidate");

    assert_eq!(
        candidate.status,
        SroaCandidateStatus::NeedsRematerialization
    );
    assert!(
        candidate
            .uses
            .iter()
            .any(|value_use| value_use.kind == SroaUseKind::Materialize)
    );
}

#[test]
pub(crate) fn sroa_escape_analysis_classifies_materialization_boundaries() {
    let mut fn_ir = test_fn();
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
    let record = record_xy(&mut fn_ir, x, y);
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
        dst: "point".to_string(),
        src: record,
        span: Span::default(),
    });
    let load_for_eval = fn_ir.add_value(
        ValueKind::Load {
            var: "point".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("point".to_string()),
    );
    fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Eval {
        val: load_for_eval,
        span: Span::default(),
    });
    let load_for_call = fn_ir.add_value(
        ValueKind::Load {
            var: "point".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("point".to_string()),
    );
    let call = fn_ir.add_value(
        ValueKind::Call {
            callee: "opaque_helper".to_string(),
            args: vec![load_for_call],
            names: vec![None],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let load_for_intrinsic = fn_ir.add_value(
        ValueKind::Load {
            var: "point".to_string(),
        },
        Span::default(),
        Facts::empty(),
        Some("point".to_string()),
    );
    let intrinsic = fn_ir.add_value(
        ValueKind::Intrinsic {
            op: IntrinsicOp::VecMeanF64,
            args: vec![load_for_intrinsic],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let sum = binary_value(&mut fn_ir, BinOp::Add, call, intrinsic);
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(sum));

    let boundaries = collect_materialization_boundaries(&fn_ir);

    assert!(boundaries.contains(&SroaMaterializationBoundary {
        value: load_for_eval,
        kind: SroaMaterializationBoundaryKind::Eval,
    }));
    assert!(boundaries.contains(&SroaMaterializationBoundary {
        value: load_for_call,
        kind: SroaMaterializationBoundaryKind::CallArg,
    }));
    assert!(boundaries.contains(&SroaMaterializationBoundary {
        value: load_for_intrinsic,
        kind: SroaMaterializationBoundaryKind::IntrinsicArg,
    }));
    assert!(boundaries.contains(&SroaMaterializationBoundary {
        value: sum,
        kind: SroaMaterializationBoundaryKind::Return,
    }));
}

#[test]
pub(crate) fn sroa_analysis_tracks_field_set_shape() {
    let mut fn_ir = test_fn();
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
    let replacement = int_value(&mut fn_ir, 3);
    let record = record_xy(&mut fn_ir, x, y);
    let updated = fn_ir.add_value(
        ValueKind::FieldSet {
            base: record,
            field: "x".to_string(),
            value: replacement,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    let get_x = fn_ir.add_value(
        ValueKind::FieldGet {
            base: updated,
            field: "x".to_string(),
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(get_x));

    let analysis = analyze_function(&fn_ir);
    let candidate = analysis.candidate(updated).expect("fieldset candidate");

    assert_eq!(candidate.source, SroaCandidateSource::FieldSet);
    assert_eq!(
        candidate.shape.as_deref(),
        Some(&["x".to_string(), "y".to_string()][..])
    );
    assert_eq!(candidate.status, SroaCandidateStatus::ScalarOnly);
}

#[test]
pub(crate) fn sroa_analysis_tracks_same_shape_phi() {
    let mut fn_ir = test_fn();
    let x1 = int_value(&mut fn_ir, 1);
    let y1 = int_value(&mut fn_ir, 2);
    let x2 = int_value(&mut fn_ir, 3);
    let y2 = int_value(&mut fn_ir, 4);
    let left = record_xy(&mut fn_ir, x1, y1);
    let right = record_xy(&mut fn_ir, x2, y2);
    let phi = fn_ir.add_value(
        ValueKind::Phi {
            args: vec![(left, fn_ir.entry), (right, fn_ir.entry)],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.values[phi].phi_block = Some(fn_ir.entry);
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(phi));

    let analysis = analyze_function(&fn_ir);
    let candidate = analysis.candidate(phi).expect("phi candidate");

    assert_eq!(candidate.source, SroaCandidateSource::Phi);
    assert_eq!(
        candidate.shape.as_deref(),
        Some(&["x".to_string(), "y".to_string()][..])
    );
    assert_eq!(
        candidate.status,
        SroaCandidateStatus::NeedsRematerialization
    );
}

#[test]
pub(crate) fn sroa_analysis_rejects_unsupported_index_use() {
    let mut fn_ir = test_fn();
    let x = int_value(&mut fn_ir, 1);
    let y = int_value(&mut fn_ir, 2);
    let idx = int_value(&mut fn_ir, 0);
    let record = record_xy(&mut fn_ir, x, y);
    let indexed = fn_ir.add_value(
        ValueKind::Index1D {
            base: record,
            idx,
            is_safe: false,
            is_na_safe: false,
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(indexed));

    let analysis = analyze_function(&fn_ir);
    let candidate = analysis.candidate(record).expect("record candidate");

    assert_eq!(
        candidate.status,
        SroaCandidateStatus::NeedsRematerialization
    );
    assert!(
        candidate
            .uses
            .iter()
            .any(|value_use| value_use.kind == SroaUseKind::Materialize)
    );
}

#[test]
pub(crate) fn sroa_analysis_rejects_duplicate_fields() {
    let mut fn_ir = test_fn();
    let x = int_value(&mut fn_ir, 1);
    let duplicate = fn_ir.add_value(
        ValueKind::RecordLit {
            fields: vec![("x".to_string(), x), ("x".to_string(), x)],
        },
        Span::default(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(duplicate));

    let analysis = analyze_function(&fn_ir);
    let candidate = analysis.candidate(duplicate).expect("record candidate");

    assert_eq!(candidate.status, SroaCandidateStatus::Rejected);
    assert!(
        candidate.reject_reasons.iter().any(
            |reason| matches!(reason, SroaRejectReason::DuplicateField(field) if field == "x")
        )
    );
}
