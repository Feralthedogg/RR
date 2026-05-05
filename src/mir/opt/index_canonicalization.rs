use super::*;

impl TachyonEngine {
    pub(crate) fn is_floor_like_single_positional_call(
        callee: &str,
        args: &[ValueId],
        names: &[Option<String>],
        floor_helpers: &FxHashSet<String>,
    ) -> bool {
        if !matches!(callee, "floor" | "ceiling" | "trunc") && !floor_helpers.contains(callee) {
            return false;
        }
        if args.len() != 1 || names.len() > 1 {
            return false;
        }
        names
            .first()
            .and_then(std::option::Option::as_ref)
            .is_none()
    }

    pub(crate) fn param_slot_for_value(fn_ir: &FnIR, vid: ValueId) -> Option<usize> {
        pub(crate) fn resolve_var_alias_slot(
            fn_ir: &FnIR,
            var: &str,
            seen_vals: &mut FxHashSet<ValueId>,
            seen_vars: &mut FxHashSet<String>,
        ) -> Option<usize> {
            if !seen_vars.insert(var.to_string()) {
                return None;
            }
            let mut found = false;
            let mut slot: Option<usize> = None;
            for bb in &fn_ir.blocks {
                for ins in &bb.instrs {
                    let Instr::Assign { dst, src, .. } = ins else {
                        continue;
                    };
                    if dst != var {
                        continue;
                    }
                    found = true;
                    let src_slot = resolve_value_slot(fn_ir, *src, seen_vals, seen_vars)?;
                    match slot {
                        None => slot = Some(src_slot),
                        Some(prev) if prev == src_slot => {}
                        Some(_) => return None,
                    }
                }
            }
            if !found {
                return None;
            }
            slot
        }

        pub(crate) fn resolve_value_slot(
            fn_ir: &FnIR,
            vid: ValueId,
            seen_vals: &mut FxHashSet<ValueId>,
            seen_vars: &mut FxHashSet<String>,
        ) -> Option<usize> {
            if !seen_vals.insert(vid) {
                return None;
            }
            match &fn_ir.values.get(vid)?.kind {
                ValueKind::Param { index } => Some(*index),
                ValueKind::Load { var } => fn_ir
                    .params
                    .iter()
                    .position(|p| p == var)
                    .or_else(|| resolve_var_alias_slot(fn_ir, var, seen_vals, seen_vars)),
                ValueKind::Phi { args } => {
                    let mut out: Option<usize> = None;
                    let mut saw = false;
                    for (a, _) in args {
                        if *a == vid {
                            continue;
                        }
                        let slot = resolve_value_slot(fn_ir, *a, seen_vals, seen_vars)?;
                        saw = true;
                        match out {
                            None => out = Some(slot),
                            Some(prev) if prev == slot => {}
                            Some(_) => return None,
                        }
                    }
                    if saw { out } else { None }
                }
                _ => None,
            }
        }

        resolve_value_slot(
            fn_ir,
            vid,
            &mut FxHashSet::default(),
            &mut FxHashSet::default(),
        )
    }

    pub(crate) fn int_vector_ty_for_param_slot(slot: usize) -> TypeState {
        TypeState::vector(PrimTy::Int, false).with_len(Some(LenSym((slot as u32) + 1)))
    }

    pub(crate) fn collect_floor_index_param_slots(
        fn_ir: &FnIR,
        floor_helpers: &FxHashSet<String>,
    ) -> FxHashSet<usize> {
        let mut slots = FxHashSet::default();
        for v in &fn_ir.values {
            let ValueKind::Call {
                callee,
                args,
                names,
            } = &v.kind
            else {
                continue;
            };
            if callee == "rr_index1_read_idx" && !args.is_empty() {
                if let Some(slot) = Self::param_slot_for_value(fn_ir, args[0]) {
                    slots.insert(slot);
                }
                continue;
            }
            if !Self::is_floor_like_single_positional_call(callee, args, names, floor_helpers) {
                continue;
            }
            let Some(inner) = args.first().copied() else {
                continue;
            };
            match &fn_ir.values[inner].kind {
                ValueKind::Index1D { base, .. } => {
                    if let Some(slot) = Self::param_slot_for_value(fn_ir, *base) {
                        slots.insert(slot);
                    }
                }
                ValueKind::Call {
                    callee: inner_callee,
                    args: inner_args,
                    names: inner_names,
                } if matches!(
                    inner_callee.as_str(),
                    "rr_index1_read" | "rr_index1_read_strict" | "rr_index1_read_floor"
                ) && (inner_args.len() == 2 || inner_args.len() == 3)
                    && inner_names.iter().take(2).all(std::option::Option::is_none) =>
                {
                    if let Some(slot) = Self::param_slot_for_value(fn_ir, inner_args[0]) {
                        slots.insert(slot);
                    }
                }
                _ => {}
            }
        }
        slots
    }

    pub(crate) fn value_base_var_name(fn_ir: &FnIR, vid: ValueId) -> Option<String> {
        pub(crate) fn rec(
            fn_ir: &FnIR,
            vid: ValueId,
            seen: &mut FxHashSet<ValueId>,
        ) -> Option<String> {
            if !seen.insert(vid) {
                return None;
            }
            match &fn_ir.values.get(vid)?.kind {
                ValueKind::Load { var } => Some(var.clone()),
                ValueKind::Param { index } => fn_ir.params.get(*index).cloned(),
                ValueKind::Phi { args } => {
                    let mut out: Option<String> = None;
                    let mut saw = false;
                    for (a, _) in args {
                        if *a == vid {
                            continue;
                        }
                        let name = rec(fn_ir, *a, seen)?;
                        saw = true;
                        match &out {
                            None => out = Some(name),
                            Some(prev) if prev == &name => {}
                            Some(_) => return None,
                        }
                    }
                    if saw { out } else { None }
                }
                _ => None,
            }
        }
        rec(fn_ir, vid, &mut FxHashSet::default())
    }

    pub(crate) fn collect_floor_index_base_vars(
        fn_ir: &FnIR,
        floor_helpers: &FxHashSet<String>,
    ) -> FxHashSet<String> {
        let mut vars = FxHashSet::default();
        for v in &fn_ir.values {
            let ValueKind::Call {
                callee,
                args,
                names,
            } = &v.kind
            else {
                continue;
            };
            if callee == "rr_index1_read_idx" && !args.is_empty() {
                if let Some(var) = Self::value_base_var_name(fn_ir, args[0]) {
                    vars.insert(var);
                }
                continue;
            }
            if !Self::is_floor_like_single_positional_call(callee, args, names, floor_helpers) {
                continue;
            }
            let Some(inner) = args.first().copied() else {
                continue;
            };
            match &fn_ir.values[inner].kind {
                ValueKind::Index1D { base, .. } => {
                    if let Some(var) = Self::value_base_var_name(fn_ir, *base) {
                        vars.insert(var);
                    }
                }
                ValueKind::Call {
                    callee: inner_callee,
                    args: inner_args,
                    names: inner_names,
                } if matches!(
                    inner_callee.as_str(),
                    "rr_index1_read" | "rr_index1_read_strict" | "rr_index1_read_floor"
                ) && (inner_args.len() == 2 || inner_args.len() == 3)
                    && inner_names.iter().take(2).all(std::option::Option::is_none) =>
                {
                    if let Some(var) = Self::value_base_var_name(fn_ir, inner_args[0]) {
                        vars.insert(var);
                    }
                }
                _ => {}
            }
        }
        vars
    }

    pub(crate) fn value_is_proven_int_index_vector(
        fn_ir: &FnIR,
        vid: ValueId,
        proven_param_slots: &FxHashSet<usize>,
    ) -> bool {
        let val = &fn_ir.values[vid];
        if val.value_ty.is_numeric_vector() && val.value_ty.prim == PrimTy::Int {
            return true;
        }
        if matches!(&val.value_term, TypeTerm::Vector(inner) if **inner == TypeTerm::Int) {
            return true;
        }
        if let Some(slot) = Self::param_slot_for_value(fn_ir, vid) {
            return proven_param_slots.contains(&slot);
        }
        false
    }

    pub(crate) fn collect_proven_floor_index_param_slots(
        all_fns: &FxHashMap<String, FnIR>,
        floor_helpers: &FxHashSet<String>,
    ) -> FxHashMap<String, FxHashSet<usize>> {
        let mut floor_slots_by_fn: FxHashMap<String, FxHashSet<usize>> = FxHashMap::default();
        let ordered_names = Self::sorted_fn_names(all_fns);
        for name in &ordered_names {
            let Some(fn_ir) = all_fns.get(name) else {
                continue;
            };
            let slots = Self::collect_floor_index_param_slots(fn_ir, floor_helpers);
            if !slots.is_empty() {
                floor_slots_by_fn.insert(name.clone(), slots);
            }
        }
        if floor_slots_by_fn.is_empty() {
            return FxHashMap::default();
        }

        let mut proven_by_fn: FxHashMap<String, FxHashSet<usize>> = FxHashMap::default();
        let mut changed = true;
        let mut guard = 0usize;
        while changed && guard < 16 {
            guard += 1;
            changed = false;
            for callee_name in &ordered_names {
                let Some(floor_slots) = floor_slots_by_fn.get(callee_name) else {
                    continue;
                };
                let mut sorted_slots: Vec<usize> = floor_slots.iter().copied().collect();
                sorted_slots.sort_unstable();
                for slot in sorted_slots {
                    if proven_by_fn
                        .get(callee_name)
                        .is_some_and(|slots| slots.contains(&slot))
                    {
                        continue;
                    }
                    let mut saw_call = false;
                    let mut all_calls_proven = true;
                    for caller_name in &ordered_names {
                        let Some(caller_ir) = all_fns.get(caller_name) else {
                            continue;
                        };
                        let empty_slots = FxHashSet::default();
                        let caller_slots = proven_by_fn.get(caller_name).unwrap_or(&empty_slots);
                        for val in &caller_ir.values {
                            let ValueKind::Call { callee, args, .. } = &val.kind else {
                                continue;
                            };
                            if callee != callee_name {
                                continue;
                            }
                            saw_call = true;
                            let Some(arg) = args.get(slot).copied() else {
                                all_calls_proven = false;
                                break;
                            };
                            if !Self::value_is_proven_int_index_vector(caller_ir, arg, caller_slots)
                            {
                                all_calls_proven = false;
                                break;
                            }
                        }
                        if !all_calls_proven {
                            break;
                        }
                    }
                    if saw_call && all_calls_proven {
                        let inserted = proven_by_fn
                            .entry(callee_name.clone())
                            .or_default()
                            .insert(slot);
                        if inserted {
                            changed = true;
                        }
                    }
                }
            }
        }

        proven_by_fn
    }

    pub(crate) fn has_var_index_vector_canonicalization(fn_ir: &FnIR, var_name: &str) -> bool {
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst != var_name {
                    continue;
                }
                let ValueKind::Call { callee, args, .. } = &fn_ir.values[*src].kind else {
                    continue;
                };
                if callee != "rr_index_vec_floor" || args.is_empty() {
                    continue;
                }
                if let Some(base_name) = Self::value_base_var_name(fn_ir, args[0])
                    && base_name == var_name
                {
                    return true;
                }
            }
        }
        false
    }

    pub(crate) fn mark_floor_index_param_metadata(
        fn_ir: &mut FnIR,
        slots: &FxHashSet<usize>,
        floor_helpers: &FxHashSet<String>,
    ) -> bool {
        let mut changed = false;
        for vid in 0..fn_ir.values.len() {
            let kind = fn_ir.values[vid].kind.clone();
            match kind {
                ValueKind::Param { index } if slots.contains(&index) => {
                    let int_vec = Self::int_vector_ty_for_param_slot(index);
                    if fn_ir.values[vid].value_ty != int_vec {
                        fn_ir.values[vid].value_ty = int_vec;
                        changed = true;
                    }
                    if !fn_ir.values[vid]
                        .facts
                        .has(Facts::IS_VECTOR | Facts::INT_SCALAR)
                    {
                        fn_ir.values[vid]
                            .facts
                            .add(Facts::IS_VECTOR | Facts::INT_SCALAR);
                        changed = true;
                    }
                }
                ValueKind::Load { var } => {
                    let Some(slot) = fn_ir.params.iter().position(|p| p == &var) else {
                        continue;
                    };
                    if !slots.contains(&slot) {
                        continue;
                    }
                    let int_vec = Self::int_vector_ty_for_param_slot(slot);
                    if fn_ir.values[vid].value_ty != int_vec {
                        fn_ir.values[vid].value_ty = int_vec;
                        changed = true;
                    }
                    if !fn_ir.values[vid]
                        .facts
                        .has(Facts::IS_VECTOR | Facts::INT_SCALAR)
                    {
                        fn_ir.values[vid]
                            .facts
                            .add(Facts::IS_VECTOR | Facts::INT_SCALAR);
                        changed = true;
                    }
                }
                ValueKind::Index1D { base, .. } => {
                    let Some(slot) = Self::param_slot_for_value(fn_ir, base) else {
                        continue;
                    };
                    if !slots.contains(&slot) {
                        continue;
                    }
                    let int_scalar = TypeState::scalar(PrimTy::Int, false);
                    if fn_ir.values[vid].value_ty != int_scalar {
                        fn_ir.values[vid].value_ty = int_scalar;
                        changed = true;
                    }
                    if !fn_ir.values[vid].facts.has(Facts::INT_SCALAR) {
                        fn_ir.values[vid].facts.add(Facts::INT_SCALAR);
                        changed = true;
                    }
                }
                ValueKind::Call {
                    callee,
                    args,
                    names,
                } if Self::is_floor_like_single_positional_call(
                    &callee,
                    &args,
                    &names,
                    floor_helpers,
                ) =>
                {
                    let Some(inner) = args.first().copied() else {
                        continue;
                    };
                    let slot = match &fn_ir.values[inner].kind {
                        ValueKind::Index1D { base, .. } => Self::param_slot_for_value(fn_ir, *base),
                        ValueKind::Call {
                            callee: inner_callee,
                            args: inner_args,
                            names: inner_names,
                        } if matches!(
                            inner_callee.as_str(),
                            "rr_index1_read" | "rr_index1_read_strict" | "rr_index1_read_floor"
                        ) && (inner_args.len() == 2 || inner_args.len() == 3)
                            && inner_names.iter().take(2).all(std::option::Option::is_none) =>
                        {
                            Self::param_slot_for_value(fn_ir, inner_args[0])
                        }
                        _ => None,
                    };
                    let Some(slot) = slot else {
                        continue;
                    };
                    if !slots.contains(&slot) {
                        continue;
                    }
                    let int_scalar = TypeState::scalar(PrimTy::Int, false);
                    if fn_ir.values[vid].value_ty != int_scalar {
                        fn_ir.values[vid].value_ty = int_scalar;
                        changed = true;
                    }
                    if !fn_ir.values[vid].facts.has(Facts::INT_SCALAR) {
                        fn_ir.values[vid].facts.add(Facts::INT_SCALAR);
                        changed = true;
                    }
                }
                ValueKind::Call { callee, args, .. } if callee == "rr_index1_read_idx" => {
                    if args.is_empty() {
                        continue;
                    }
                    let Some(slot) = Self::param_slot_for_value(fn_ir, args[0]) else {
                        continue;
                    };
                    if !slots.contains(&slot) {
                        continue;
                    }
                    let int_scalar = TypeState::scalar(PrimTy::Int, false);
                    if fn_ir.values[vid].value_ty != int_scalar {
                        fn_ir.values[vid].value_ty = int_scalar;
                        changed = true;
                    }
                    if !fn_ir.values[vid].facts.has(Facts::INT_SCALAR) {
                        fn_ir.values[vid].facts.add(Facts::INT_SCALAR);
                        changed = true;
                    }
                }
                ValueKind::Call {
                    callee,
                    args,
                    names,
                } if matches!(
                    callee.as_str(),
                    "rr_index1_read_vec" | "rr_index1_read_vec_floor" | "rr_gather"
                ) && (args.len() == 2 || args.len() == 3)
                    && names.iter().take(2).all(std::option::Option::is_none) =>
                {
                    let Some(slot) = Self::param_slot_for_value(fn_ir, args[0]) else {
                        continue;
                    };
                    if !slots.contains(&slot) {
                        continue;
                    }
                    let int_vec = TypeState::vector(PrimTy::Int, false);
                    if fn_ir.values[vid].value_ty != int_vec {
                        fn_ir.values[vid].value_ty = int_vec;
                        changed = true;
                    }
                    if !fn_ir.values[vid]
                        .facts
                        .has(Facts::IS_VECTOR | Facts::INT_SCALAR)
                    {
                        fn_ir.values[vid]
                            .facts
                            .add(Facts::IS_VECTOR | Facts::INT_SCALAR);
                        changed = true;
                    }
                }
                _ => {}
            }
        }
        changed
    }

    pub(crate) fn canonicalize_floor_index_params(
        fn_ir: &mut FnIR,
        proven_param_slots: Option<&FxHashSet<usize>>,
        floor_helpers: &FxHashSet<String>,
    ) -> bool {
        let slots = Self::collect_floor_index_param_slots(fn_ir, floor_helpers);
        let base_vars = Self::collect_floor_index_base_vars(fn_ir, floor_helpers);
        if slots.is_empty() && base_vars.is_empty() {
            return false;
        }

        let target_bb = if fn_ir.entry < fn_ir.blocks.len() {
            fn_ir.entry
        } else if fn_ir.body_head < fn_ir.blocks.len() {
            fn_ir.body_head
        } else {
            return false;
        };

        let mut sorted_vars: Vec<String> = base_vars.into_iter().collect();
        sorted_vars.sort();

        let mut prefix: Vec<Instr> = Vec::new();
        let mut changed = false;
        for var_name in sorted_vars {
            let skip_canonicalize = fn_ir
                .params
                .iter()
                .position(|p| p == &var_name)
                .is_some_and(|slot| proven_param_slots.is_some_and(|slots| slots.contains(&slot)));
            if skip_canonicalize {
                continue;
            }
            if Self::has_var_index_vector_canonicalization(fn_ir, &var_name) {
                continue;
            }
            let load = fn_ir.add_value(
                ValueKind::Load {
                    var: var_name.clone(),
                },
                crate::utils::Span::dummy(),
                Facts::empty(),
                Some(var_name.clone()),
            );
            let mut facts = Facts::empty();
            facts.add(Facts::IS_VECTOR | Facts::INT_SCALAR | Facts::ONE_BASED);
            let floor_vec = fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_index_vec_floor".to_string(),
                    args: vec![load],
                    names: vec![None],
                },
                crate::utils::Span::dummy(),
                facts,
                None,
            );
            fn_ir.values[load].value_ty = TypeState::vector(PrimTy::Int, false);
            fn_ir.values[floor_vec].value_ty = TypeState::vector(PrimTy::Int, false);
            prefix.push(Instr::Assign {
                dst: var_name,
                src: floor_vec,
                span: crate::utils::Span::dummy(),
            });
            changed = true;
        }

        if !prefix.is_empty() {
            let bb = &mut fn_ir.blocks[target_bb];
            let mut merged = prefix;
            merged.extend(std::mem::take(&mut bb.instrs));
            bb.instrs = merged;
        }

        changed | Self::mark_floor_index_param_metadata(fn_ir, &slots, floor_helpers)
    }
}
