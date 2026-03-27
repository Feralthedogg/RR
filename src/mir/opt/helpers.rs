use super::TachyonEngine;
use super::types::{ClampBound, CubeIndexReturnVars};
use crate::mir::*;
use crate::syntax::ast::BinOp;
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PeriodicIndexHelperKind {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TrivialMinMaxHelperKind {
    Min,
    Max,
}

impl TachyonEngine {
    pub(super) fn collect_wrap_index_helpers(
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

    pub(super) fn rewrite_wrap_index_helper_calls(
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

    pub(super) fn collect_periodic_index_helpers(
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

    pub(super) fn rewrite_periodic_index_helper_calls(
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

    pub(super) fn is_wrap_index_helper_fn(fn_ir: &FnIR) -> bool {
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
                    | Instr::StoreIndex3D { .. } => fail!("contains eval/store"),
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

    pub(super) fn periodic_index_helper_kind(fn_ir: &FnIR) -> Option<PeriodicIndexHelperKind> {
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
                    | Instr::StoreIndex3D { .. } => fail!("contains eval/store"),
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

    fn simple_return_value_through_gotos(fn_ir: &FnIR, start: BlockId) -> Option<ValueId> {
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

    fn parse_periodic_index_cond(fn_ir: &FnIR, cond: ValueId) -> Option<PeriodicIndexHelperKind> {
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

    pub(super) fn parse_wrap_if_rule(
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

    pub(super) fn single_assign_block(fn_ir: &FnIR, bid: BlockId) -> Option<(String, ValueId)> {
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

    pub(super) fn parse_wrap_cond(
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

    pub(super) fn assignments_match_wrap_sources(
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

    pub(super) fn return_matches_wrap_expr(fn_ir: &FnIR, x_var: &str, y_var: &str) -> bool {
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

    pub(super) fn is_wrap_return_expr(
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

    pub(super) fn is_wrap_return_form(
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

    pub(super) fn is_y_minus_one(fn_ir: &FnIR, vid: ValueId, y_var: &str) -> bool {
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

    pub(super) fn collect_cube_index_helpers(
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

    pub(super) fn collect_floor_helpers(all_fns: &FxHashMap<String, FnIR>) -> FxHashSet<String> {
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

    pub(super) fn collect_trivial_clamp_helpers(
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

    pub(super) fn collect_trivial_abs_helpers(
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

    pub(super) fn collect_unit_index_helpers(
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

    pub(super) fn collect_trivial_minmax_helpers(
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

    pub(super) fn rewrite_floor_helper_calls(
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

    pub(super) fn rewrite_trivial_clamp_helper_calls(
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

    pub(super) fn rewrite_trivial_abs_helper_calls(
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

    pub(super) fn rewrite_unit_index_helper_calls(
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

    pub(super) fn rewrite_trivial_minmax_helper_calls(
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

    pub(super) fn collect_round_helpers(all_fns: &FxHashMap<String, FnIR>) -> FxHashSet<String> {
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

    pub(super) fn rewrite_cube_index_helper_calls(
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

    pub(super) fn is_cube_index_helper_fn(fn_ir: &FnIR, round_helpers: &FxHashSet<String>) -> bool {
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
                    | Instr::StoreIndex3D { .. } => fail!("contains eval/store"),
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

    pub(super) fn cube_index_return_vars(fn_ir: &FnIR) -> Option<CubeIndexReturnVars> {
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

    pub(super) fn parse_cube_bound(fn_ir: &FnIR, vid: ValueId) -> Option<ClampBound> {
        if Self::value_is_const_one(fn_ir, vid) {
            return Some(ClampBound::ConstOne);
        }
        if Self::value_is_const_six(fn_ir, vid) {
            return Some(ClampBound::ConstSix);
        }
        Self::value_var_name(fn_ir, vid).map(ClampBound::Var)
    }

    pub(super) fn parse_cube_cond(
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

    pub(super) fn cube_bound_matches_value(fn_ir: &FnIR, bound: &ClampBound, vid: ValueId) -> bool {
        match bound {
            ClampBound::ConstOne => Self::value_is_const_one(fn_ir, vid),
            ClampBound::ConstSix => Self::value_is_const_six(fn_ir, vid),
            ClampBound::Var(var) => {
                Self::value_var_name(fn_ir, vid).as_deref() == Some(var.as_str())
            }
        }
    }

    pub(super) fn is_benign_cube_aux_assignment(
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

    pub(super) fn parse_cube_if_rule(
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

    pub(super) fn assignments_match_cube_seed_source(
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

    pub(super) fn parse_trivial_clamp_cond(
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

    pub(super) fn parse_unit_index_if_rule(
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

    pub(super) fn is_trivial_clamp_helper_fn(fn_ir: &FnIR) -> bool {
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
                    | Instr::StoreIndex3D { .. } => fail!("contains eval/store"),
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

    pub(super) fn is_trivial_abs_helper_fn(fn_ir: &FnIR) -> bool {
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
                    | Instr::StoreIndex3D { .. } => fail!("contains eval/store"),
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

    pub(super) fn is_unit_index_helper_fn(fn_ir: &FnIR) -> bool {
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
                    | Instr::StoreIndex3D { .. } => fail!("contains eval/store"),
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

    pub(super) fn trivial_minmax_helper_kind(fn_ir: &FnIR) -> Option<TrivialMinMaxHelperKind> {
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
                    | Instr::StoreIndex3D { .. } => fail!("contains eval/store"),
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

    pub(super) fn is_round_call_of_param(
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

    pub(super) fn is_rr_floor_helper_fn(fn_ir: &FnIR) -> bool {
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
                    | Instr::StoreIndex3D { .. } => fail!("contains eval/store"),
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

    pub(super) fn is_rr_round_helper_fn(fn_ir: &FnIR) -> bool {
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
                    | Instr::StoreIndex3D { .. } => fail!("contains eval/store"),
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

    pub(super) fn returns_param_minus_rem_plus_one_expr(
        fn_ir: &FnIR,
        vid: ValueId,
        param_idx: usize,
        rem_var: &str,
    ) -> bool {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        let ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } = &fn_ir.values[v].kind
        else {
            return false;
        };
        (Self::returns_param_minus_rem_expr(fn_ir, *lhs, param_idx, rem_var)
            && Self::value_is_const_one(fn_ir, *rhs))
            || (Self::returns_param_minus_rem_expr(fn_ir, *rhs, param_idx, rem_var)
                && Self::value_is_const_one(fn_ir, *lhs))
    }

    pub(super) fn returns_param_minus_rem_expr(
        fn_ir: &FnIR,
        vid: ValueId,
        param_idx: usize,
        rem_var: &str,
    ) -> bool {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        let ValueKind::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } = &fn_ir.values[v].kind
        else {
            return false;
        };
        Self::value_param_index(fn_ir, *lhs) == Some(param_idx)
            && Self::value_var_name(fn_ir, *rhs).as_deref() == Some(rem_var)
    }

    pub(super) fn returns_zero_minus_param_expr(
        fn_ir: &FnIR,
        vid: ValueId,
        param_idx: usize,
    ) -> bool {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        let ValueKind::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } = &fn_ir.values[v].kind
        else {
            return false;
        };
        Self::value_is_const_zero(fn_ir, *lhs)
            && Self::value_param_index(fn_ir, *rhs) == Some(param_idx)
    }

    pub(super) fn is_unit_index_seed_expr(fn_ir: &FnIR, vid: ValueId) -> bool {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        let ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } = &fn_ir.values[v].kind
        else {
            return false;
        };
        let (one_side, floor_side) = if Self::value_is_const_one(fn_ir, *lhs) {
            (*lhs, *rhs)
        } else if Self::value_is_const_one(fn_ir, *rhs) {
            (*rhs, *lhs)
        } else {
            return false;
        };
        let _ = one_side;
        let floor_side = Self::resolve_load_alias_value(fn_ir, floor_side);
        let ValueKind::Call { callee, args, .. } = &fn_ir.values[floor_side].kind else {
            return false;
        };
        if callee != "floor" || args.len() != 1 {
            return false;
        }
        let mul = Self::resolve_load_alias_value(fn_ir, args[0]);
        let ValueKind::Binary {
            op: BinOp::Mul,
            lhs,
            rhs,
        } = &fn_ir.values[mul].kind
        else {
            return false;
        };
        (Self::value_param_index(fn_ir, *lhs) == Some(0)
            && Self::value_param_index(fn_ir, *rhs) == Some(1))
            || (Self::value_param_index(fn_ir, *lhs) == Some(1)
                && Self::value_param_index(fn_ir, *rhs) == Some(0))
    }

    pub(super) fn returns_param_plus_minus_one_expr(
        fn_ir: &FnIR,
        vid: ValueId,
        param_idx: usize,
        is_add: bool,
    ) -> bool {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        let ValueKind::Binary { op, lhs, rhs } = &fn_ir.values[v].kind else {
            return false;
        };
        match (is_add, op) {
            (true, BinOp::Add) => {
                (Self::value_param_index(fn_ir, *lhs) == Some(param_idx)
                    && Self::value_is_const_one(fn_ir, *rhs))
                    || (Self::value_param_index(fn_ir, *rhs) == Some(param_idx)
                        && Self::value_is_const_one(fn_ir, *lhs))
            }
            (false, BinOp::Sub) => {
                Self::value_param_index(fn_ir, *lhs) == Some(param_idx)
                    && Self::value_is_const_one(fn_ir, *rhs)
            }
            _ => false,
        }
    }
}
