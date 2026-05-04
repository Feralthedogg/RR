use super::*;
impl TachyonEngine {
    pub(crate) fn collect_wrap_index_helpers(
        all_fns: &FxHashMap<String, FnIR>,
    ) -> FxHashSet<String> {
        let mut helpers = FxHashSet::default();
        let ordered = Self::sorted_fn_names(all_fns);
        for name in ordered {
            let Some(fn_ir) = all_fns.get(&name) else {
                continue;
            };
            if Self::is_wrap_index_helper_fn(fn_ir) {
                helpers.insert(name);
            }
        }
        helpers
    }

    pub(crate) fn rewrite_wrap_index_helper_calls(
        all_fns: &mut FxHashMap<String, FnIR>,
        helpers: &FxHashSet<String>,
    ) -> usize {
        let mut rewrites = 0usize;
        for fn_ir in all_fns.values_mut() {
            for v in &mut fn_ir.values {
                let ValueKind::Call {
                    callee,
                    args,
                    names,
                } = &mut v.kind
                else {
                    continue;
                };
                if !helpers.contains(callee.as_str()) || args.len() != 4 {
                    continue;
                }
                *callee = "rr_wrap_index_vec_i".to_string();
                *names = vec![None, None, None, None];
                rewrites += 1;
            }
        }
        rewrites
    }

    pub(crate) fn collect_periodic_index_helpers(
        all_fns: &FxHashMap<String, FnIR>,
    ) -> FxHashMap<String, PeriodicIndexHelperKind> {
        let mut helpers = FxHashMap::default();
        let ordered = Self::sorted_fn_names(all_fns);
        for name in ordered {
            let Some(fn_ir) = all_fns.get(&name) else {
                continue;
            };
            let Some(kind) = Self::periodic_index_helper_kind(fn_ir) else {
                continue;
            };
            helpers.insert(name, kind);
        }
        helpers
    }

    pub(crate) fn rewrite_periodic_index_helper_calls(
        all_fns: &mut FxHashMap<String, FnIR>,
        helpers: &FxHashMap<String, PeriodicIndexHelperKind>,
    ) -> usize {
        let mut rewrites = 0usize;
        for fn_ir in all_fns.values_mut() {
            let original_len = fn_ir.values.len();
            for vid in 0..original_len {
                let (callee, args) = match &fn_ir.values[vid].kind {
                    ValueKind::Call { callee, args, .. } => (callee.clone(), args.clone()),
                    _ => continue,
                };
                let Some(kind) = helpers.get(callee.as_str()).copied() else {
                    continue;
                };
                if args.len() != 2 {
                    continue;
                }
                let bound = args[1];
                let one = fn_ir.add_value(
                    ValueKind::Const(Lit::Float(1.0)),
                    crate::utils::Span::dummy(),
                    Facts::empty(),
                    None,
                );
                let shifted = fn_ir.add_value(
                    ValueKind::Binary {
                        op: match kind {
                            PeriodicIndexHelperKind::Left => BinOp::Sub,
                            PeriodicIndexHelperKind::Right => BinOp::Add,
                        },
                        lhs: args[0],
                        rhs: one,
                    },
                    crate::utils::Span::dummy(),
                    Facts::empty(),
                    None,
                );
                let ValueKind::Call {
                    callee,
                    args,
                    names,
                    ..
                } = &mut fn_ir.values[vid].kind
                else {
                    continue;
                };
                *callee = "rr_wrap_index_vec_i".to_string();
                *args = vec![shifted, one, bound, one];
                *names = vec![None, None, None, None];
                rewrites += 1;
            }
        }
        rewrites
    }

    pub(crate) fn is_wrap_index_helper_fn(fn_ir: &FnIR) -> bool {
        macro_rules! fail {
            ($msg:expr) => {{
                if Self::wrap_trace_enabled() {
                    eprintln!("   [wrap-detect] {}: {}", fn_ir.name, $msg);
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

        let mut rules: Vec<(String, bool, usize)> = Vec::new();
        for bb in &fn_ir.blocks {
            let Terminator::If {
                cond,
                then_bb,
                else_bb,
            } = bb.term
            else {
                continue;
            };
            let Some((var, is_lt, bound_param)) =
                Self::parse_wrap_if_rule(fn_ir, cond, then_bb, else_bb)
            else {
                fail!("if rule parse failed");
            };
            rules.push((var, is_lt, bound_param));
        }

        if rules.len() != 4 {
            fail!("if rule count != 4");
        }

        let mut by_var: FxHashMap<String, Vec<(bool, usize)>> = FxHashMap::default();
        for (var, is_lt, bound) in rules {
            by_var.entry(var).or_default().push((is_lt, bound));
        }
        if by_var.len() != 2 {
            fail!("rule vars != 2");
        }

        let mut x_var: Option<String> = None;
        let mut y_var: Option<String> = None;
        for (var, rs) in &by_var {
            if rs.len() != 2 {
                fail!("rules per var != 2");
            }
            let mut saw_lt = None;
            let mut saw_gt = None;
            for (is_lt, bound) in rs {
                if *is_lt {
                    saw_lt = Some(*bound);
                } else {
                    saw_gt = Some(*bound);
                }
            }
            let Some(lt_bound) = saw_lt else {
                fail!("missing lt bound");
            };
            let Some(gt_bound) = saw_gt else {
                fail!("missing gt bound");
            };
            if lt_bound != gt_bound {
                fail!("lt/gt bound mismatch");
            }
            match lt_bound {
                2 => x_var = Some(var.clone()),
                3 => y_var = Some(var.clone()),
                _ => fail!("bound param not 2/3"),
            }
        }

        let Some(x_var) = x_var else {
            fail!("missing x var");
        };
        let Some(y_var) = y_var else {
            fail!("missing y var");
        };

        if !Self::assignments_match_wrap_sources(fn_ir, &x_var, 0, 2)
            || !Self::assignments_match_wrap_sources(fn_ir, &y_var, 1, 3)
        {
            fail!("assignment source mismatch");
        }

        if !Self::return_matches_wrap_expr(fn_ir, &x_var, &y_var) {
            fail!("return expression mismatch");
        }
        if Self::wrap_trace_enabled() {
            eprintln!("   [wrap-detect] {}: matched", fn_ir.name);
        }
        true
    }

    pub(crate) fn periodic_index_helper_kind(fn_ir: &FnIR) -> Option<PeriodicIndexHelperKind> {
        macro_rules! fail {
            ($msg:expr) => {{
                if Self::wrap_trace_enabled() {
                    eprintln!("   [wrap1d-detect] {}: {}", fn_ir.name, $msg);
                }
                return None;
            }};
        }

        if fn_ir.requires_conservative_optimization() || fn_ir.params.len() != 2 {
            return None;
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
        let mut branch: Option<(ValueId, BlockId, BlockId)> = None;
        for bb in &fn_ir.blocks {
            if !reachable.contains(&bb.id) {
                continue;
            }
            let Terminator::If {
                cond,
                then_bb,
                else_bb,
            } = bb.term
            else {
                continue;
            };
            if branch.is_some() {
                fail!("multiple branches");
            }
            branch = Some((cond, then_bb, else_bb));
        }
        let Some((cond, then_bb, else_bb)) = branch else {
            fail!("missing branch");
        };
        let Some(then_ret) = Self::simple_return_value_through_gotos(fn_ir, then_bb) else {
            fail!("then branch does not lead to simple return");
        };
        let Some(else_ret) = Self::simple_return_value_through_gotos(fn_ir, else_bb) else {
            fail!("else branch does not lead to simple return");
        };

        let kind = match Self::parse_periodic_index_cond(fn_ir, cond) {
            Some(PeriodicIndexHelperKind::Left)
                if Self::value_param_index(fn_ir, then_ret) == Some(1)
                    && Self::returns_param_plus_minus_one_expr(fn_ir, else_ret, 0, false) =>
            {
                PeriodicIndexHelperKind::Left
            }
            Some(PeriodicIndexHelperKind::Right)
                if Self::value_is_const_one(fn_ir, then_ret)
                    && Self::returns_param_plus_minus_one_expr(fn_ir, else_ret, 0, true) =>
            {
                PeriodicIndexHelperKind::Right
            }
            _ => fail!("cond/return shape mismatch"),
        };

        if Self::wrap_trace_enabled() {
            eprintln!("   [wrap1d-detect] {}: matched {:?}", fn_ir.name, kind);
        }
        Some(kind)
    }

    pub(crate) fn simple_return_value_through_gotos(
        fn_ir: &FnIR,
        start: BlockId,
    ) -> Option<ValueId> {
        let mut current = start;
        let mut seen = FxHashSet::default();
        loop {
            if !seen.insert(current) {
                return None;
            }
            let bb = &fn_ir.blocks[current];
            match bb.term {
                Terminator::Return(Some(ret)) => {
                    return Some(Self::resolve_load_alias_value(fn_ir, ret));
                }
                Terminator::Goto(next) if bb.instrs.is_empty() => current = next,
                _ => return None,
            }
        }
    }

    pub(crate) fn parse_periodic_index_cond(
        fn_ir: &FnIR,
        cond: ValueId,
    ) -> Option<PeriodicIndexHelperKind> {
        let cond = Self::resolve_load_alias_value(fn_ir, cond);
        let ValueKind::Binary { op, lhs, rhs } = &fn_ir.values[cond].kind else {
            return None;
        };
        let lhs_param = Self::value_param_index(fn_ir, *lhs);
        let rhs_param = Self::value_param_index(fn_ir, *rhs);
        let lhs_is_one = Self::value_is_const_one(fn_ir, *lhs);
        let rhs_is_one = Self::value_is_const_one(fn_ir, *rhs);

        match op {
            BinOp::Le | BinOp::Lt if lhs_param == Some(0) && rhs_is_one => {
                Some(PeriodicIndexHelperKind::Left)
            }
            BinOp::Ge | BinOp::Gt if rhs_param == Some(0) && lhs_is_one => {
                Some(PeriodicIndexHelperKind::Left)
            }
            BinOp::Ge | BinOp::Gt if lhs_param == Some(0) && rhs_param == Some(1) => {
                Some(PeriodicIndexHelperKind::Right)
            }
            BinOp::Le | BinOp::Lt if lhs_param == Some(1) && rhs_param == Some(0) => {
                Some(PeriodicIndexHelperKind::Right)
            }
            _ => None,
        }
    }

    pub(crate) fn parse_wrap_if_rule(
        fn_ir: &FnIR,
        cond: ValueId,
        then_bb: BlockId,
        else_bb: BlockId,
    ) -> Option<(String, bool, usize)> {
        let then_assign = Self::single_assign_block(fn_ir, then_bb);
        if Self::wrap_trace_enabled() && then_assign.is_none() {
            eprintln!(
                "   [wrap-rule] {}: then_bb {} is not single-assign",
                fn_ir.name, then_bb
            );
        }
        let (then_assign_var, then_assign_src) = then_assign?;
        let else_assign = Self::single_assign_block(fn_ir, else_bb);
        if else_assign.is_some() {
            if Self::wrap_trace_enabled() {
                eprintln!(
                    "   [wrap-rule] {}: else_bb {} has assign",
                    fn_ir.name, else_bb
                );
            }
            return None;
        }

        let (cond_var, op_is_lt, cond_bound_param, cond_bound_is_one) =
            match Self::parse_wrap_cond(fn_ir, cond) {
                Some(v) => v,
                None => {
                    if Self::wrap_trace_enabled() {
                        eprintln!(
                            "   [wrap-rule] {}: cond {} parse failed kind={:?}",
                            fn_ir.name, cond, fn_ir.values[cond].kind
                        );
                    }
                    return None;
                }
            };
        if then_assign_var != cond_var {
            if Self::wrap_trace_enabled() {
                eprintln!(
                    "   [wrap-rule] {}: then dst {} != cond var {}",
                    fn_ir.name, then_assign_var, cond_var
                );
            }
            return None;
        }

        let assign_src_param = Self::value_param_index(fn_ir, then_assign_src);
        let assign_src_is_one = Self::value_is_const_one(fn_ir, then_assign_src);
        if op_is_lt && cond_bound_is_one {
            let Some(p) = assign_src_param else {
                if Self::wrap_trace_enabled() {
                    eprintln!(
                        "   [wrap-rule] {}: lt rule src is not param (src={} kind={:?} origin={:?})",
                        fn_ir.name,
                        then_assign_src,
                        fn_ir.values[then_assign_src].kind,
                        fn_ir.values[then_assign_src].origin_var
                    );
                }
                return None;
            };
            if p >= 2 {
                return Some((cond_var, true, p));
            }
            if Self::wrap_trace_enabled() {
                eprintln!(
                    "   [wrap-rule] {}: lt rule bound param {} < 2",
                    fn_ir.name, p
                );
            }
            return None;
        }
        if !op_is_lt && assign_src_is_one {
            let Some(p) = cond_bound_param else {
                if Self::wrap_trace_enabled() {
                    eprintln!(
                        "   [wrap-rule] {}: gt rule bound is not param (cond={})",
                        fn_ir.name, cond
                    );
                }
                return None;
            };
            if p >= 2 {
                return Some((cond_var, false, p));
            }
            if Self::wrap_trace_enabled() {
                eprintln!(
                    "   [wrap-rule] {}: gt rule bound param {} < 2",
                    fn_ir.name, p
                );
            }
            return None;
        }
        if Self::wrap_trace_enabled() {
            eprintln!(
                "   [wrap-rule] {}: no matching lt/gt rewrite case (op_is_lt={}, bound_is_one={}, src_param={:?}, src_one={}, cond_param={:?})",
                fn_ir.name,
                op_is_lt,
                cond_bound_is_one,
                assign_src_param,
                assign_src_is_one,
                cond_bound_param
            );
        }
        None
    }

    pub(crate) fn single_assign_block(fn_ir: &FnIR, bid: BlockId) -> Option<(String, ValueId)> {
        let mut out: Option<(String, ValueId)> = None;
        for ins in &fn_ir.blocks[bid].instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                return None;
            };
            if out.is_some() {
                return None;
            }
            out = Some((dst.clone(), *src));
        }
        out
    }

    pub(crate) fn parse_wrap_cond(
        fn_ir: &FnIR,
        cond: ValueId,
    ) -> Option<(String, bool, Option<usize>, bool)> {
        let cond = Self::resolve_load_alias_value(fn_ir, cond);
        let ValueKind::Binary { op, lhs, rhs } = &fn_ir.values[cond].kind else {
            return None;
        };
        let lhs_var = Self::value_non_param_var_name(fn_ir, *lhs);
        let rhs_var = Self::value_non_param_var_name(fn_ir, *rhs);
        let lhs_is_one = Self::value_is_const_one(fn_ir, *lhs);
        let rhs_is_one = Self::value_is_const_one(fn_ir, *rhs);
        let lhs_param = Self::value_param_index(fn_ir, *lhs);
        let rhs_param = Self::value_param_index(fn_ir, *rhs);

        let out = match op {
            BinOp::Lt | BinOp::Gt => {
                if let Some(var) = lhs_var {
                    let is_lt = matches!(op, BinOp::Lt);
                    Some((var, is_lt, rhs_param, rhs_is_one))
                } else if let Some(var) = rhs_var {
                    let is_lt = matches!(op, BinOp::Gt);
                    Some((var, is_lt, lhs_param, lhs_is_one))
                } else {
                    None
                }
            }
            _ => None,
        };
        if out.is_none() && Self::wrap_trace_enabled() {
            eprintln!(
                "   [wrap-cond] {} cond={} op={:?} lhs={} kind={:?} origin={:?} rhs={} kind={:?} origin={:?}",
                fn_ir.name,
                cond,
                op,
                lhs,
                fn_ir.values[*lhs].kind,
                fn_ir.values[*lhs].origin_var,
                rhs,
                fn_ir.values[*rhs].kind,
                fn_ir.values[*rhs].origin_var
            );
        }
        out
    }

    pub(crate) fn assignments_match_wrap_sources(
        fn_ir: &FnIR,
        var: &str,
        seed_param: usize,
        bound_param: usize,
    ) -> bool {
        let mut saw_seed = false;
        let mut saw_bound = false;
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                if let Some(p) = Self::value_param_index(fn_ir, *src) {
                    if p == seed_param {
                        saw_seed = true;
                        continue;
                    }
                    if p == bound_param {
                        saw_bound = true;
                        continue;
                    }
                    return false;
                }
                if Self::value_is_const_one(fn_ir, *src) {
                    continue;
                }
                return false;
            }
        }
        saw_seed && saw_bound
    }

    pub(crate) fn return_matches_wrap_expr(fn_ir: &FnIR, x_var: &str, y_var: &str) -> bool {
        let reachable = Self::reachable_blocks(fn_ir);
        let mut return_vals = Vec::new();
        for bb in &fn_ir.blocks {
            if !reachable.contains(&bb.id) {
                continue;
            }
            if let Terminator::Return(Some(v)) = bb.term {
                let v = Self::resolve_load_alias_value(fn_ir, v);
                if matches!(fn_ir.values[v].kind, ValueKind::Const(Lit::Null)) {
                    continue;
                }
                return_vals.push(v);
            }
        }
        if return_vals.len() != 1 {
            return false;
        }
        Self::is_wrap_return_expr(fn_ir, return_vals[0], x_var, y_var)
    }

    pub(crate) fn is_wrap_return_expr(
        fn_ir: &FnIR,
        ret: ValueId,
        x_var: &str,
        y_var: &str,
    ) -> bool {
        let ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } = &fn_ir.values[ret].kind
        else {
            return false;
        };
        Self::is_wrap_return_form(fn_ir, *lhs, *rhs, x_var, y_var)
            || Self::is_wrap_return_form(fn_ir, *rhs, *lhs, x_var, y_var)
    }

    pub(crate) fn is_wrap_return_form(
        fn_ir: &FnIR,
        mul_side: ValueId,
        x_side: ValueId,
        x_var: &str,
        y_var: &str,
    ) -> bool {
        if Self::value_var_name(fn_ir, x_side).as_deref() != Some(x_var) {
            return false;
        }
        let ValueKind::Binary {
            op: BinOp::Mul,
            lhs,
            rhs,
        } = &fn_ir.values[Self::resolve_load_alias_value(fn_ir, mul_side)].kind
        else {
            return false;
        };

        let lhs_is_y = Self::is_y_minus_one(fn_ir, *lhs, y_var);
        let rhs_is_y = Self::is_y_minus_one(fn_ir, *rhs, y_var);
        let lhs_is_w = Self::value_param_index(fn_ir, *lhs) == Some(2);
        let rhs_is_w = Self::value_param_index(fn_ir, *rhs) == Some(2);

        (lhs_is_y && rhs_is_w) || (rhs_is_y && lhs_is_w)
    }

    pub(crate) fn is_y_minus_one(fn_ir: &FnIR, vid: ValueId, y_var: &str) -> bool {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        let ValueKind::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } = &fn_ir.values[v].kind
        else {
            return false;
        };
        Self::value_var_name(fn_ir, *lhs).as_deref() == Some(y_var)
            && Self::value_is_const_one(fn_ir, *rhs)
    }

    pub(crate) fn collect_cube_index_helpers(
        all_fns: &FxHashMap<String, FnIR>,
    ) -> FxHashSet<String> {
        let round_helpers = Self::collect_round_helpers(all_fns);
        let mut helpers = FxHashSet::default();
        let ordered = Self::sorted_fn_names(all_fns);
        for name in ordered {
            let Some(fn_ir) = all_fns.get(&name) else {
                continue;
            };
            if Self::is_cube_index_helper_fn(fn_ir, &round_helpers) {
                helpers.insert(name);
            }
        }
        helpers
    }

    pub(crate) fn collect_floor_helpers(all_fns: &FxHashMap<String, FnIR>) -> FxHashSet<String> {
        let mut helpers = FxHashSet::default();
        for name in Self::sorted_fn_names(all_fns) {
            let Some(fn_ir) = all_fns.get(&name) else {
                continue;
            };
            if Self::is_rr_floor_helper_fn(fn_ir) {
                helpers.insert(name);
            }
        }
        helpers
    }

    pub(crate) fn collect_trivial_clamp_helpers(
        all_fns: &FxHashMap<String, FnIR>,
    ) -> FxHashSet<String> {
        let mut helpers = FxHashSet::default();
        for name in Self::sorted_fn_names(all_fns) {
            let Some(fn_ir) = all_fns.get(&name) else {
                continue;
            };
            if Self::is_trivial_clamp_helper_fn(fn_ir) {
                helpers.insert(name);
            }
        }
        helpers
    }

    pub(crate) fn collect_trivial_abs_helpers(
        all_fns: &FxHashMap<String, FnIR>,
    ) -> FxHashSet<String> {
        let mut helpers = FxHashSet::default();
        for name in Self::sorted_fn_names(all_fns) {
            let Some(fn_ir) = all_fns.get(&name) else {
                continue;
            };
            if Self::is_trivial_abs_helper_fn(fn_ir) {
                helpers.insert(name);
            }
        }
        helpers
    }

    pub(crate) fn collect_unit_index_helpers(
        all_fns: &FxHashMap<String, FnIR>,
    ) -> FxHashSet<String> {
        let mut helpers = FxHashSet::default();
        for name in Self::sorted_fn_names(all_fns) {
            let Some(fn_ir) = all_fns.get(&name) else {
                continue;
            };
            if Self::is_unit_index_helper_fn(fn_ir) {
                helpers.insert(name);
            }
        }
        helpers
    }

    pub(crate) fn collect_trivial_minmax_helpers(
        all_fns: &FxHashMap<String, FnIR>,
    ) -> FxHashMap<String, TrivialMinMaxHelperKind> {
        let mut helpers = FxHashMap::default();
        for name in Self::sorted_fn_names(all_fns) {
            let Some(fn_ir) = all_fns.get(&name) else {
                continue;
            };
            let Some(kind) = Self::trivial_minmax_helper_kind(fn_ir) else {
                continue;
            };
            helpers.insert(name, kind);
        }
        helpers
    }

    pub(crate) fn rewrite_floor_helper_calls(
        all_fns: &mut FxHashMap<String, FnIR>,
        helpers: &FxHashSet<String>,
    ) -> usize {
        let mut rewrites = 0usize;
        for fn_ir in all_fns.values_mut() {
            for v in &mut fn_ir.values {
                let ValueKind::Call {
                    callee,
                    args,
                    names,
                } = &mut v.kind
                else {
                    continue;
                };
                if !helpers.contains(callee.as_str()) || args.len() != 1 {
                    continue;
                }
                *callee = "floor".to_string();
                *names = vec![None];
                rewrites += 1;
            }
        }
        rewrites
    }

    pub(crate) fn rewrite_trivial_clamp_helper_calls(
        all_fns: &mut FxHashMap<String, FnIR>,
        helpers: &FxHashSet<String>,
    ) -> usize {
        let mut rewrites = 0usize;
        for fn_ir in all_fns.values_mut() {
            let mut pending = Vec::new();
            for (vid, value) in fn_ir.values.iter().enumerate() {
                let ValueKind::Call { callee, args, .. } = &value.kind else {
                    continue;
                };
                if !helpers.contains(callee.as_str()) || args.len() != 3 {
                    continue;
                }
                pending.push((vid, args[0], args[1], args[2], value.span, value.facts));
            }
            for (vid, x, lo, hi, span, facts) in pending {
                let max_call = fn_ir.add_value(
                    ValueKind::Call {
                        callee: "pmax".to_string(),
                        args: vec![x, lo],
                        names: vec![None, None],
                    },
                    span,
                    facts,
                    None,
                );
                let value = &mut fn_ir.values[vid];
                value.kind = ValueKind::Call {
                    callee: "pmin".to_string(),
                    args: vec![max_call, hi],
                    names: vec![None, None],
                };
                value.origin_var = None;
                rewrites += 1;
            }
        }
        rewrites
    }

    pub(crate) fn rewrite_trivial_abs_helper_calls(
        all_fns: &mut FxHashMap<String, FnIR>,
        helpers: &FxHashSet<String>,
    ) -> usize {
        let mut rewrites = 0usize;
        for fn_ir in all_fns.values_mut() {
            for value in &mut fn_ir.values {
                let ValueKind::Call {
                    callee,
                    args,
                    names,
                } = &mut value.kind
                else {
                    continue;
                };
                if !helpers.contains(callee.as_str()) || args.len() != 1 {
                    continue;
                }
                *callee = "abs".to_string();
                *names = vec![None];
                rewrites += 1;
            }
        }
        rewrites
    }

    pub(crate) fn rewrite_unit_index_helper_calls(
        all_fns: &mut FxHashMap<String, FnIR>,
        helpers: &FxHashSet<String>,
    ) -> usize {
        let mut rewrites = 0usize;
        for fn_ir in all_fns.values_mut() {
            let original_len = fn_ir.values.len();
            for vid in 0..original_len {
                let (callee, args, span, facts) = match &fn_ir.values[vid].kind {
                    ValueKind::Call { callee, args, .. } if args.len() == 2 => (
                        callee.clone(),
                        args.clone(),
                        fn_ir.values[vid].span,
                        fn_ir.values[vid].facts,
                    ),
                    _ => continue,
                };
                if !helpers.contains(callee.as_str()) {
                    continue;
                }
                let one = fn_ir.add_value(ValueKind::Const(Lit::Float(1.0)), span, facts, None);
                let mul = fn_ir.add_value(
                    ValueKind::Binary {
                        op: BinOp::Mul,
                        lhs: args[0],
                        rhs: args[1],
                    },
                    span,
                    facts,
                    None,
                );
                let floor = fn_ir.add_value(
                    ValueKind::Call {
                        callee: "floor".to_string(),
                        args: vec![mul],
                        names: vec![None],
                    },
                    span,
                    facts,
                    None,
                );
                let plus_one = fn_ir.add_value(
                    ValueKind::Binary {
                        op: BinOp::Add,
                        lhs: one,
                        rhs: floor,
                    },
                    span,
                    facts,
                    None,
                );
                let lower = fn_ir.add_value(
                    ValueKind::Call {
                        callee: "pmax".to_string(),
                        args: vec![plus_one, one],
                        names: vec![None, None],
                    },
                    span,
                    facts,
                    None,
                );
                let value = &mut fn_ir.values[vid];
                value.kind = ValueKind::Call {
                    callee: "pmin".to_string(),
                    args: vec![lower, args[1]],
                    names: vec![None, None],
                };
                value.origin_var = None;
                rewrites += 1;
            }
        }
        rewrites
    }

    pub(crate) fn rewrite_trivial_minmax_helper_calls(
        all_fns: &mut FxHashMap<String, FnIR>,
        helpers: &FxHashMap<String, TrivialMinMaxHelperKind>,
    ) -> usize {
        let mut rewrites = 0usize;
        for fn_ir in all_fns.values_mut() {
            for value in &mut fn_ir.values {
                let ValueKind::Call {
                    callee,
                    args,
                    names,
                } = &mut value.kind
                else {
                    continue;
                };
                let Some(kind) = helpers.get(callee.as_str()).copied() else {
                    continue;
                };
                if args.len() != 2 {
                    continue;
                }
                *callee = match kind {
                    TrivialMinMaxHelperKind::Min => "pmin".to_string(),
                    TrivialMinMaxHelperKind::Max => "pmax".to_string(),
                };
                *names = vec![None, None];
                rewrites += 1;
            }
        }
        rewrites
    }

    pub(crate) fn collect_round_helpers(all_fns: &FxHashMap<String, FnIR>) -> FxHashSet<String> {
        let mut helpers = FxHashSet::default();
        for name in Self::sorted_fn_names(all_fns) {
            let Some(fn_ir) = all_fns.get(&name) else {
                continue;
            };
            if Self::is_rr_round_helper_fn(fn_ir) {
                helpers.insert(name);
            }
        }
        helpers
    }

    pub(crate) fn rewrite_cube_index_helper_calls(
        all_fns: &mut FxHashMap<String, FnIR>,
        helpers: &FxHashSet<String>,
    ) -> usize {
        let mut rewrites = 0usize;
        for fn_ir in all_fns.values_mut() {
            for v in &mut fn_ir.values {
                let ValueKind::Call {
                    callee,
                    args,
                    names,
                } = &mut v.kind
                else {
                    continue;
                };
                if !helpers.contains(callee.as_str()) || args.len() != 4 {
                    continue;
                }
                *callee = "rr_idx_cube_vec_i".to_string();
                *names = vec![None, None, None, None];
                rewrites += 1;
            }
        }
        rewrites
    }
}
