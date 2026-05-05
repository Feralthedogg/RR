use super::*;
impl TachyonEngine {
    pub(crate) fn returns_param_minus_rem_plus_one_expr(
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

    pub(crate) fn returns_param_minus_rem_expr(
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

    pub(crate) fn returns_zero_minus_param_expr(
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

    pub(crate) fn is_unit_index_seed_expr(fn_ir: &FnIR, vid: ValueId) -> bool {
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

    pub(crate) fn returns_param_plus_minus_one_expr(
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
