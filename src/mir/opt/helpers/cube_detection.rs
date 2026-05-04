use super::*;
impl TachyonEngine {
    pub(crate) fn is_cube_index_helper_fn(fn_ir: &FnIR, round_helpers: &FxHashSet<String>) -> bool {
        macro_rules! fail {
            ($msg:expr) => {{
                if Self::wrap_trace_enabled() {
                    eprintln!("   [cube-detect] {}: {}", fn_ir.name, $msg);
                }
                return false;
            }};
        }

        if fn_ir.requires_conservative_optimization() || fn_ir.params.len() != 4 {
            return false;
        }
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                match ins {
                    Instr::Assign { .. } => {}
                    Instr::Eval { .. }
                    | Instr::StoreIndex1D { .. }
                    | Instr::StoreIndex2D { .. }
                    | Instr::StoreIndex3D { .. }
                    | Instr::UnsafeRBlock { .. } => fail!("contains eval/store"),
                }
            }
        }

        let Some(vars) = Self::cube_index_return_vars(fn_ir) else {
            fail!("return expression mismatch");
        };
        if !Self::assignments_match_cube_seed_source(fn_ir, round_helpers, &vars.face_var, 0)
            || !Self::assignments_match_cube_seed_source(fn_ir, round_helpers, &vars.x_var, 1)
            || !Self::assignments_match_cube_seed_source(fn_ir, round_helpers, &vars.y_var, 2)
            || !Self::assignments_match_cube_seed_source(fn_ir, round_helpers, &vars.size_var, 3)
        {
            fail!("seed assignment mismatch");
        }

        let mut rules: Vec<(String, bool, ClampBound)> = Vec::new();
        for bb in &fn_ir.blocks {
            let Terminator::If {
                cond,
                then_bb,
                else_bb,
            } = bb.term
            else {
                continue;
            };
            let Some(rule) = Self::parse_cube_if_rule(fn_ir, cond, then_bb, else_bb) else {
                fail!("if rule parse failed");
            };
            rules.push(rule);
        }
        if rules.len() != 6 {
            fail!("if rule count != 6");
        }

        let expected = [
            (
                vars.face_var.clone(),
                vec![(true, ClampBound::ConstOne), (false, ClampBound::ConstSix)],
            ),
            (
                vars.x_var.clone(),
                vec![
                    (true, ClampBound::ConstOne),
                    (false, ClampBound::Var(vars.size_var.clone())),
                ],
            ),
            (
                vars.y_var.clone(),
                vec![
                    (true, ClampBound::ConstOne),
                    (false, ClampBound::Var(vars.size_var.clone())),
                ],
            ),
        ];
        for (var, wanted) in expected {
            let seen: Vec<(bool, ClampBound)> = rules
                .iter()
                .filter(|(rule_var, _, _)| rule_var == &var)
                .map(|(_, is_lt, bound)| (*is_lt, bound.clone()))
                .collect();
            if seen.len() != wanted.len() {
                fail!("rule multiplicity mismatch");
            }
            for need in wanted {
                if !seen.contains(&need) {
                    fail!("missing clamp rule");
                }
            }
        }

        if Self::wrap_trace_enabled() {
            eprintln!("   [cube-detect] {}: matched", fn_ir.name);
        }
        true
    }

    pub(crate) fn cube_index_return_vars(fn_ir: &FnIR) -> Option<CubeIndexReturnVars> {
        let reachable = Self::reachable_blocks(fn_ir);
        let mut returns = Vec::new();
        for bb in &fn_ir.blocks {
            if !reachable.contains(&bb.id) {
                continue;
            }
            if let Terminator::Return(Some(v)) = bb.term {
                let v = Self::resolve_load_alias_value(fn_ir, v);
                if matches!(fn_ir.values[v].kind, ValueKind::Const(Lit::Null)) {
                    continue;
                }
                returns.push(v);
            }
        }
        if returns.len() != 1 {
            return None;
        }
        let ret = returns[0];
        let mut terms = Vec::new();
        Self::flatten_assoc_binop(fn_ir, ret, BinOp::Add, &mut terms);
        if terms.len() != 3 {
            return None;
        }

        let mut face_var: Option<String> = None;
        let mut x_var: Option<String> = None;
        let mut y_var: Option<String> = None;
        let mut size_var: Option<String> = None;

        for term in terms {
            if let Some(var) = Self::value_var_name(fn_ir, term) {
                if y_var.is_some() {
                    return None;
                }
                y_var = Some(var);
                continue;
            }

            let mut factors = Vec::new();
            Self::flatten_assoc_binop(fn_ir, term, BinOp::Mul, &mut factors);
            let sub_vars: Vec<String> = factors
                .iter()
                .filter_map(|f| Self::parse_var_minus_one(fn_ir, *f))
                .collect();
            let plain_vars: Vec<String> = factors
                .iter()
                .filter_map(|f| Self::value_var_name(fn_ir, *f))
                .collect();

            match (sub_vars.as_slice(), plain_vars.as_slice()) {
                ([sub], [size]) => {
                    if x_var.is_some() {
                        return None;
                    }
                    x_var = Some(sub.clone());
                    match &size_var {
                        None => size_var = Some(size.clone()),
                        Some(prev) if prev == size => {}
                        Some(_) => return None,
                    }
                }
                ([sub], [size_a, size_b]) if size_a == size_b => {
                    if face_var.is_some() {
                        return None;
                    }
                    face_var = Some(sub.clone());
                    match &size_var {
                        None => size_var = Some(size_a.clone()),
                        Some(prev) if prev == size_a => {}
                        Some(_) => return None,
                    }
                }
                _ => return None,
            }
        }

        Some(CubeIndexReturnVars {
            face_var: face_var?,
            x_var: x_var?,
            y_var: y_var?,
            size_var: size_var?,
        })
    }

    pub(crate) fn parse_cube_bound(fn_ir: &FnIR, vid: ValueId) -> Option<ClampBound> {
        if Self::value_is_const_one(fn_ir, vid) {
            return Some(ClampBound::ConstOne);
        }
        if Self::value_is_const_six(fn_ir, vid) {
            return Some(ClampBound::ConstSix);
        }
        Self::value_var_name(fn_ir, vid).map(ClampBound::Var)
    }

    pub(crate) fn parse_cube_cond(
        fn_ir: &FnIR,
        cond: ValueId,
    ) -> Option<(String, bool, ClampBound)> {
        let cond = Self::resolve_load_alias_value(fn_ir, cond);
        let ValueKind::Binary { op, lhs, rhs } = &fn_ir.values[cond].kind else {
            return None;
        };
        match op {
            BinOp::Lt | BinOp::Gt => {
                if let Some(var) = Self::value_non_param_var_name(fn_ir, *lhs) {
                    Some((
                        var,
                        matches!(op, BinOp::Lt),
                        Self::parse_cube_bound(fn_ir, *rhs)?,
                    ))
                } else if let Some(var) = Self::value_non_param_var_name(fn_ir, *rhs) {
                    Some((
                        var,
                        matches!(op, BinOp::Gt),
                        Self::parse_cube_bound(fn_ir, *lhs)?,
                    ))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub(crate) fn cube_bound_matches_value(fn_ir: &FnIR, bound: &ClampBound, vid: ValueId) -> bool {
        match bound {
            ClampBound::ConstOne => Self::value_is_const_one(fn_ir, vid),
            ClampBound::ConstSix => Self::value_is_const_six(fn_ir, vid),
            ClampBound::Var(var) => {
                Self::value_var_name(fn_ir, vid).as_deref() == Some(var.as_str())
            }
        }
    }

    pub(crate) fn is_benign_cube_aux_assignment(
        fn_ir: &FnIR,
        dst: &str,
        src: ValueId,
        cond_var: &str,
        bound: &ClampBound,
    ) -> bool {
        match bound {
            ClampBound::Var(bound_var) if dst == bound_var => {
                let src_var = Self::value_var_name(fn_ir, src);
                src_var.as_deref() == Some(cond_var)
                    || src_var.as_deref() == Some(bound_var.as_str())
            }
            _ => false,
        }
    }

    pub(crate) fn parse_cube_if_rule(
        fn_ir: &FnIR,
        cond: ValueId,
        then_bb: BlockId,
        else_bb: BlockId,
    ) -> Option<(String, bool, ClampBound)> {
        let then_assigns = Self::block_assignments(fn_ir, then_bb)?;
        if then_assigns.is_empty() {
            return None;
        }
        let else_assigns = Self::block_assignments(fn_ir, else_bb)?;
        if !else_assigns.is_empty() {
            return None;
        }
        let (cond_var, is_lt, bound) = Self::parse_cube_cond(fn_ir, cond)?;
        let mut saw_primary = false;
        for (dst, src) in then_assigns {
            if dst == cond_var && Self::cube_bound_matches_value(fn_ir, &bound, src) {
                saw_primary = true;
                continue;
            }
            if !Self::is_benign_cube_aux_assignment(fn_ir, &dst, src, &cond_var, &bound) {
                return None;
            }
        }
        if !saw_primary {
            return None;
        }
        Some((cond_var, is_lt, bound))
    }

    pub(crate) fn assignments_match_cube_seed_source(
        fn_ir: &FnIR,
        round_helpers: &FxHashSet<String>,
        var: &str,
        param_idx: usize,
    ) -> bool {
        let mut saw_seed = false;
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                if Self::value_param_index(fn_ir, *src) == Some(param_idx)
                    || Self::is_round_call_of_param(fn_ir, round_helpers, *src, param_idx)
                {
                    saw_seed = true;
                }
            }
        }
        saw_seed
    }

    pub(crate) fn parse_trivial_clamp_cond(
        fn_ir: &FnIR,
        cond: ValueId,
    ) -> Option<(String, bool, usize)> {
        let cond = Self::resolve_load_alias_value(fn_ir, cond);
        let ValueKind::Binary { op, lhs, rhs } = &fn_ir.values[cond].kind else {
            return None;
        };
        match op {
            BinOp::Lt | BinOp::Gt => {
                if let Some(var) = Self::value_non_param_var_name(fn_ir, *lhs) {
                    let bound = Self::value_param_index(fn_ir, *rhs)?;
                    Some((var, matches!(op, BinOp::Lt), bound))
                } else if let Some(var) = Self::value_non_param_var_name(fn_ir, *rhs) {
                    let bound = Self::value_param_index(fn_ir, *lhs)?;
                    Some((var, matches!(op, BinOp::Gt), bound))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub(crate) fn parse_unit_index_if_rule(
        fn_ir: &FnIR,
        cond: ValueId,
        then_bb: BlockId,
        else_bb: BlockId,
    ) -> Option<(String, bool, bool)> {
        let (dst, src) = Self::single_assign_block(fn_ir, then_bb)?;
        let else_assigns = Self::block_assignments(fn_ir, else_bb)?;
        if !else_assigns.is_empty() {
            return None;
        }
        let cond = Self::resolve_load_alias_value(fn_ir, cond);
        let ValueKind::Binary { op, lhs, rhs } = &fn_ir.values[cond].kind else {
            return None;
        };

        match op {
            BinOp::Lt => {
                if let Some(var) = Self::value_non_param_var_name(fn_ir, *lhs)
                    && Self::value_is_const_one(fn_ir, *rhs)
                    && dst == var
                    && Self::value_is_const_one(fn_ir, src)
                {
                    return Some((var, true, false));
                }
                if let Some(var) = Self::value_non_param_var_name(fn_ir, *rhs)
                    && Self::value_is_const_one(fn_ir, *lhs)
                    && dst == var
                    && Self::value_is_const_one(fn_ir, src)
                {
                    return Some((var, true, false));
                }
            }
            BinOp::Gt => {
                if let Some(var) = Self::value_non_param_var_name(fn_ir, *lhs)
                    && Self::value_param_index(fn_ir, *rhs) == Some(1)
                    && dst == var
                    && Self::value_param_index(fn_ir, src) == Some(1)
                {
                    return Some((var, false, true));
                }
                if let Some(var) = Self::value_non_param_var_name(fn_ir, *rhs)
                    && Self::value_param_index(fn_ir, *lhs) == Some(1)
                    && dst == var
                    && Self::value_param_index(fn_ir, src) == Some(1)
                {
                    return Some((var, false, true));
                }
            }
            _ => {}
        }
        None
    }

    pub(crate) fn is_trivial_clamp_helper_fn(fn_ir: &FnIR) -> bool {
        macro_rules! fail {
            ($msg:expr) => {{
                if Self::wrap_trace_enabled() {
                    eprintln!("   [clamp-detect] {}: {}", fn_ir.name, $msg);
                }
                return false;
            }};
        }
        if fn_ir.requires_conservative_optimization() || fn_ir.params.len() != 3 {
            fail!("conservative interop or arity != 3");
        }
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                match ins {
                    Instr::Assign { .. } => {}
                    Instr::Eval { .. }
                    | Instr::StoreIndex1D { .. }
                    | Instr::StoreIndex2D { .. }
                    | Instr::StoreIndex3D { .. }
                    | Instr::UnsafeRBlock { .. } => fail!("contains eval/store"),
                }
            }
        }

        let reachable = Self::reachable_blocks(fn_ir);
        let mut return_vals = Vec::new();
        let mut rules = Vec::new();
        for bb in &fn_ir.blocks {
            if !reachable.contains(&bb.id) {
                continue;
            }
            match bb.term {
                Terminator::If {
                    cond,
                    then_bb,
                    else_bb,
                } => {
                    let then_assign = Self::single_assign_block(fn_ir, then_bb)
                        .ok_or_else(|| "then block is not single-assign".to_string());
                    let else_assigns = Self::block_assignments(fn_ir, else_bb)
                        .ok_or_else(|| "else block is not assign-only".to_string());
                    let Ok((dst, src)) = then_assign else {
                        fail!("then block is not single-assign");
                    };
                    let Ok(else_assigns) = else_assigns else {
                        fail!("else block is not assign-only");
                    };
                    if !else_assigns.is_empty() {
                        fail!("else block is not empty");
                    }
                    let Some((cond_var, is_lower, bound_param)) =
                        Self::parse_trivial_clamp_cond(fn_ir, cond)
                    else {
                        fail!("cond parse failed");
                    };
                    if dst != cond_var || Self::value_param_index(fn_ir, src) != Some(bound_param) {
                        fail!("then assignment does not clamp the compared variable");
                    }
                    rules.push((cond_var, is_lower, bound_param));
                }
                Terminator::Return(Some(v)) => {
                    let v = Self::resolve_load_alias_value(fn_ir, v);
                    if !matches!(fn_ir.values[v].kind, ValueKind::Const(Lit::Null)) {
                        return_vals.push(v);
                    }
                }
                _ => {}
            }
        }

        if rules.len() != 2 {
            fail!("if rule count != 2");
        }
        if return_vals.len() != 1 {
            fail!("missing single non-null return");
        }

        let Some(ret_var) = Self::value_var_name(fn_ir, return_vals[0]) else {
            fail!("return is not a scalar state var");
        };

        let mut saw_seed = false;
        let mut saw_lo = false;
        let mut saw_hi = false;
        for bb in &fn_ir.blocks {
            if !reachable.contains(&bb.id) {
                continue;
            }
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst != &ret_var {
                    continue;
                }
                match Self::value_param_index(fn_ir, *src) {
                    Some(0) => saw_seed = true,
                    Some(1) => saw_lo = true,
                    Some(2) => saw_hi = true,
                    _ => fail!("state var assignment is not sourced from x/lo/hi"),
                }
            }
        }
        if !(saw_seed && saw_lo && saw_hi) {
            fail!("missing x/lo/hi state assignments");
        }

        let mut saw_lower = false;
        let mut saw_upper = false;
        for (var, is_lower, bound_param) in rules {
            if var != ret_var {
                fail!("rules do not update the returned state var");
            }
            match (is_lower, bound_param) {
                (true, 1) => saw_lower = true,
                (false, 2) => saw_upper = true,
                _ => fail!("rules do not match clamp(x, lo, hi)"),
            }
        }
        if !(saw_lower && saw_upper) {
            fail!("missing lower/upper clamp rules");
        }

        if Self::wrap_trace_enabled() {
            eprintln!("   [clamp-detect] {}: matched", fn_ir.name);
        }
        true
    }

    pub(crate) fn is_trivial_abs_helper_fn(fn_ir: &FnIR) -> bool {
        macro_rules! fail {
            ($msg:expr) => {{
                if Self::wrap_trace_enabled() {
                    eprintln!("   [abs-detect] {}: {}", fn_ir.name, $msg);
                }
                return false;
            }};
        }
        if fn_ir.requires_conservative_optimization() || fn_ir.params.len() != 1 {
            fail!("conservative interop or arity != 1");
        }
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                match ins {
                    Instr::Assign { .. } => {}
                    Instr::Eval { .. }
                    | Instr::StoreIndex1D { .. }
                    | Instr::StoreIndex2D { .. }
                    | Instr::StoreIndex3D { .. }
                    | Instr::UnsafeRBlock { .. } => fail!("contains eval/store"),
                }
            }
        }

        let reachable = Self::reachable_blocks(fn_ir);
        let branches = fn_ir
            .blocks
            .iter()
            .filter(|bb| reachable.contains(&bb.id))
            .filter_map(|bb| match bb.term {
                Terminator::If {
                    cond,
                    then_bb,
                    else_bb,
                } => Some((cond, then_bb, else_bb)),
                _ => None,
            })
            .collect::<Vec<_>>();
        if branches.len() != 1 {
            fail!("if rule count != 1");
        }
        let (cond, then_bb, else_bb) = branches[0];
        let Some(then_ret) = Self::simple_return_value_through_gotos(fn_ir, then_bb) else {
            fail!("then branch does not lead to simple return");
        };
        let Some(else_ret) = Self::simple_return_value_through_gotos(fn_ir, else_bb) else {
            fail!("else branch does not lead to simple return");
        };

        let cond = Self::resolve_load_alias_value(fn_ir, cond);
        let ValueKind::Binary { op, lhs, rhs } = &fn_ir.values[cond].kind else {
            fail!("cond is not binary");
        };
        let lhs_param = Self::value_param_index(fn_ir, *lhs) == Some(0);
        let rhs_param = Self::value_param_index(fn_ir, *rhs) == Some(0);
        let lhs_zero = Self::value_is_const_zero(fn_ir, *lhs);
        let rhs_zero = Self::value_is_const_zero(fn_ir, *rhs);
        let cond_matches = match op {
            BinOp::Lt | BinOp::Le => (lhs_param && rhs_zero) || (rhs_param && lhs_zero),
            BinOp::Gt | BinOp::Ge => (lhs_zero && rhs_param) || (rhs_zero && lhs_param),
            _ => false,
        };
        if !cond_matches {
            fail!("cond is not x < 0 form");
        }
        if !Self::returns_zero_minus_param_expr(fn_ir, then_ret, 0) {
            fail!("then return is not 0 - x");
        }
        if Self::value_param_index(fn_ir, else_ret) != Some(0) {
            fail!("else return is not x");
        }

        if Self::wrap_trace_enabled() {
            eprintln!("   [abs-detect] {}: matched", fn_ir.name);
        }
        true
    }

    pub(crate) fn is_unit_index_helper_fn(fn_ir: &FnIR) -> bool {
        macro_rules! fail {
            ($msg:expr) => {{
                if Self::wrap_trace_enabled() {
                    eprintln!("   [unit-index-detect] {}: {}", fn_ir.name, $msg);
                }
                return false;
            }};
        }
        if fn_ir.requires_conservative_optimization() || fn_ir.params.len() != 2 {
            fail!("conservative interop or arity != 2");
        }
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                match ins {
                    Instr::Assign { .. } => {}
                    Instr::Eval { .. }
                    | Instr::StoreIndex1D { .. }
                    | Instr::StoreIndex2D { .. }
                    | Instr::StoreIndex3D { .. }
                    | Instr::UnsafeRBlock { .. } => fail!("contains eval/store"),
                }
            }
        }

        let reachable = Self::reachable_blocks(fn_ir);
        let mut rules = Vec::new();
        let mut return_vals = Vec::new();
        for bb in &fn_ir.blocks {
            if !reachable.contains(&bb.id) {
                continue;
            }
            match bb.term {
                Terminator::If {
                    cond,
                    then_bb,
                    else_bb,
                } => {
                    let Some((var, lower_rule, uses_param_n)) =
                        Self::parse_unit_index_if_rule(fn_ir, cond, then_bb, else_bb)
                    else {
                        fail!("if rule parse failed");
                    };
                    rules.push((var, lower_rule, uses_param_n));
                }
                Terminator::Return(Some(v)) => {
                    let v = Self::resolve_load_alias_value(fn_ir, v);
                    if !matches!(fn_ir.values[v].kind, ValueKind::Const(Lit::Null)) {
                        return_vals.push(v);
                    }
                }
                _ => {}
            }
        }

        if rules.len() != 2 {
            fail!("if rule count != 2");
        }
        if return_vals.len() != 1 {
            fail!("missing single non-null return");
        }
        let Some(idx_var) = Self::value_var_name(fn_ir, return_vals[0]) else {
            fail!("return is not idx state var");
        };

        let mut saw_lower = false;
        let mut saw_upper = false;
        for (var, lower_rule, uses_param_n) in rules {
            if var != idx_var {
                fail!("if rules mutate different vars");
            }
            if lower_rule && !uses_param_n {
                saw_lower = true;
            } else if !lower_rule && uses_param_n {
                saw_upper = true;
            } else {
                fail!("unexpected clamp rule shape");
            }
        }
        if !(saw_lower && saw_upper) {
            fail!("missing lower/upper clamp rules");
        }

        let mut saw_seed = false;
        for bb in &fn_ir.blocks {
            if !reachable.contains(&bb.id) {
                continue;
            }
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst == &idx_var && Self::is_unit_index_seed_expr(fn_ir, *src) {
                    saw_seed = true;
                }
            }
        }
        if !saw_seed {
            fail!("missing unit-index seed assignment");
        }

        if Self::wrap_trace_enabled() {
            eprintln!("   [unit-index-detect] {}: matched", fn_ir.name);
        }
        true
    }

    pub(crate) fn trivial_minmax_helper_kind(fn_ir: &FnIR) -> Option<TrivialMinMaxHelperKind> {
        macro_rules! fail {
            ($msg:expr) => {{
                if Self::wrap_trace_enabled() {
                    eprintln!("   [minmax-detect] {}: {}", fn_ir.name, $msg);
                }
                return None;
            }};
        }
        if fn_ir.requires_conservative_optimization() || fn_ir.params.len() != 2 {
            fail!("conservative interop or arity != 2");
        }
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                match ins {
                    Instr::Assign { .. } => {}
                    Instr::Eval { .. }
                    | Instr::StoreIndex1D { .. }
                    | Instr::StoreIndex2D { .. }
                    | Instr::StoreIndex3D { .. }
                    | Instr::UnsafeRBlock { .. } => fail!("contains eval/store"),
                }
            }
        }

        let reachable = Self::reachable_blocks(fn_ir);
        let branches = fn_ir
            .blocks
            .iter()
            .filter(|bb| reachable.contains(&bb.id))
            .filter_map(|bb| match bb.term {
                Terminator::If {
                    cond,
                    then_bb,
                    else_bb,
                } => Some((cond, then_bb, else_bb)),
                _ => None,
            })
            .collect::<Vec<_>>();
        if branches.len() != 1 {
            fail!("if rule count != 1");
        }
        let (cond, then_bb, else_bb) = branches[0];
        let Some(then_ret) = Self::simple_return_value_through_gotos(fn_ir, then_bb) else {
            fail!("then branch does not lead to simple return");
        };
        let Some(else_ret) = Self::simple_return_value_through_gotos(fn_ir, else_bb) else {
            fail!("else branch does not lead to simple return");
        };
        if Self::value_param_index(fn_ir, then_ret) != Some(0)
            || Self::value_param_index(fn_ir, else_ret) != Some(1)
        {
            fail!("returns are not (a, b)");
        }

        let cond = Self::resolve_load_alias_value(fn_ir, cond);
        let ValueKind::Binary { op, lhs, rhs } = &fn_ir.values[cond].kind else {
            fail!("cond is not binary");
        };
        let lhs_a = Self::value_param_index(fn_ir, *lhs) == Some(0);
        let lhs_b = Self::value_param_index(fn_ir, *lhs) == Some(1);
        let rhs_a = Self::value_param_index(fn_ir, *rhs) == Some(0);
        let rhs_b = Self::value_param_index(fn_ir, *rhs) == Some(1);
        let kind = match op {
            BinOp::Lt if lhs_a && rhs_b => TrivialMinMaxHelperKind::Min,
            BinOp::Gt if lhs_a && rhs_b => TrivialMinMaxHelperKind::Max,
            BinOp::Lt if lhs_b && rhs_a => TrivialMinMaxHelperKind::Max,
            BinOp::Gt if lhs_b && rhs_a => TrivialMinMaxHelperKind::Min,
            _ => fail!("cond is not min/max compare"),
        };
        if Self::wrap_trace_enabled() {
            eprintln!("   [minmax-detect] {}: matched {:?}", fn_ir.name, kind);
        }
        Some(kind)
    }

    pub(crate) fn is_round_call_of_param(
        fn_ir: &FnIR,
        round_helpers: &FxHashSet<String>,
        vid: ValueId,
        param_idx: usize,
    ) -> bool {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        let ValueKind::Call { callee, args, .. } = &fn_ir.values[v].kind else {
            return false;
        };
        args.len() == 1
            && Self::value_param_index(fn_ir, args[0]) == Some(param_idx)
            && (callee == "round" || round_helpers.contains(callee))
    }

    pub(crate) fn is_rr_floor_helper_fn(fn_ir: &FnIR) -> bool {
        macro_rules! fail {
            ($msg:expr) => {{
                if Self::wrap_trace_enabled() {
                    eprintln!("   [floor-detect] {}: {}", fn_ir.name, $msg);
                }
                return false;
            }};
        }
        if fn_ir.requires_conservative_optimization() || fn_ir.params.len() != 1 {
            fail!("conservative interop or arity != 1");
        }
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                match ins {
                    Instr::Assign { .. } => {}
                    Instr::Eval { .. }
                    | Instr::StoreIndex1D { .. }
                    | Instr::StoreIndex2D { .. }
                    | Instr::StoreIndex3D { .. }
                    | Instr::UnsafeRBlock { .. } => fail!("contains eval/store"),
                }
            }
        }

        let reachable = Self::reachable_blocks(fn_ir);
        let mut returns = Vec::new();
        for bb in &fn_ir.blocks {
            if !reachable.contains(&bb.id) {
                continue;
            }
            if let Terminator::Return(Some(v)) = bb.term {
                returns.push(v);
            }
        }
        if returns.is_empty() {
            fail!("missing return");
        }
        let mut saw_real_return = false;
        for ret in returns {
            let ret = Self::resolve_load_alias_value(fn_ir, ret);
            if matches!(fn_ir.values[ret].kind, ValueKind::Const(Lit::Null)) {
                continue;
            }
            saw_real_return = true;
            let ValueKind::Binary {
                op: BinOp::Sub,
                lhs,
                rhs,
            } = &fn_ir.values[ret].kind
            else {
                if Self::wrap_trace_enabled() {
                    eprintln!(
                        "   [floor-detect] {}: return kind {:?}",
                        fn_ir.name, fn_ir.values[ret].kind
                    );
                }
                fail!("return is not subtraction");
            };
            if Self::value_param_index(fn_ir, *lhs) != Some(0) {
                fail!("sub lhs is not param");
            }
            let rhs = Self::resolve_load_alias_value(fn_ir, *rhs);
            let ValueKind::Binary {
                op: BinOp::Mod,
                lhs: mod_lhs,
                rhs: mod_rhs,
            } = &fn_ir.values[rhs].kind
            else {
                fail!("sub rhs is not modulo");
            };
            if Self::value_param_index(fn_ir, *mod_lhs) != Some(0)
                || !Self::value_is_const_one(fn_ir, *mod_rhs)
            {
                fail!("modulo operands do not match floor pattern");
            }
        }
        if !saw_real_return {
            fail!("missing non-null return");
        }
        true
    }

    pub(crate) fn is_rr_round_helper_fn(fn_ir: &FnIR) -> bool {
        macro_rules! fail {
            ($msg:expr) => {{
                if Self::wrap_trace_enabled() {
                    eprintln!("   [round-detect] {}: {}", fn_ir.name, $msg);
                }
                return false;
            }};
        }
        if fn_ir.requires_conservative_optimization() || fn_ir.params.len() != 1 {
            fail!("conservative interop or arity != 1");
        }
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                match ins {
                    Instr::Assign { .. } => {}
                    Instr::Eval { .. }
                    | Instr::StoreIndex1D { .. }
                    | Instr::StoreIndex2D { .. }
                    | Instr::StoreIndex3D { .. }
                    | Instr::UnsafeRBlock { .. } => fail!("contains eval/store"),
                }
            }
        }

        let mut saw_mod_seed = false;
        let mut rem_var: Option<String> = None;
        let mut branch_term: Option<(BlockId, BlockId, ValueId)> = None;
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                let v = Self::resolve_load_alias_value(fn_ir, *src);
                let ValueKind::Binary {
                    op: BinOp::Mod,
                    lhs,
                    rhs,
                } = &fn_ir.values[v].kind
                else {
                    continue;
                };
                if Self::value_param_index(fn_ir, *lhs) == Some(0)
                    && Self::value_is_const_one(fn_ir, *rhs)
                {
                    saw_mod_seed = true;
                    rem_var = Some(dst.clone());
                }
            }
            if let Terminator::If {
                cond,
                then_bb,
                else_bb,
            } = bb.term
            {
                branch_term = Some((then_bb, else_bb, cond));
            }
        }
        let Some(rem_var) = rem_var else {
            fail!("missing rem seed");
        };
        if !saw_mod_seed {
            fail!("mod seed not seen");
        }
        let Some((_, _, cond)) = branch_term else {
            fail!("missing branch");
        };
        let cond = Self::resolve_load_alias_value(fn_ir, cond);
        let ValueKind::Binary {
            op: BinOp::Ge,
            lhs,
            rhs,
        } = &fn_ir.values[cond].kind
        else {
            fail!("cond is not >= binary");
        };
        if Self::value_var_name(fn_ir, *lhs).as_deref() != Some(rem_var.as_str())
            || !Self::value_is_const_half(fn_ir, *rhs)
        {
            fail!("cond does not compare rem >= 0.5");
        }
        let mut saw_minus_rem = false;
        let mut saw_minus_rem_plus_one = false;
        for bb in &fn_ir.blocks {
            let Terminator::Return(Some(ret)) = bb.term else {
                continue;
            };
            let ret = Self::resolve_load_alias_value(fn_ir, ret);
            if matches!(fn_ir.values[ret].kind, ValueKind::Const(Lit::Null)) {
                continue;
            }
            if Self::returns_param_minus_rem_expr(fn_ir, ret, 0, &rem_var) {
                saw_minus_rem = true;
                continue;
            }
            if Self::returns_param_minus_rem_plus_one_expr(fn_ir, ret, 0, &rem_var) {
                saw_minus_rem_plus_one = true;
                continue;
            }
            fail!("return is neither x - r nor (x - r) + 1");
        }
        if !saw_minus_rem {
            fail!("missing x - r return");
        }
        if !saw_minus_rem_plus_one {
            fail!("missing (x - r) + 1 return");
        }
        true
    }
}
