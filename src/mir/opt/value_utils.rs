use super::*;

impl TachyonEngine {
    pub(super) fn value_var_name(fn_ir: &FnIR, vid: ValueId) -> Option<String> {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        if let ValueKind::Load { var } = &fn_ir.values[v].kind {
            return Some(var.clone());
        }
        fn_ir.values[v].origin_var.clone()
    }

    pub(super) fn value_non_param_var_name(fn_ir: &FnIR, vid: ValueId) -> Option<String> {
        let var = Self::value_var_name(fn_ir, vid)?;
        if Self::param_index_for_var(fn_ir, &var).is_some() {
            None
        } else {
            Some(var)
        }
    }

    pub(super) fn param_index_for_var(fn_ir: &FnIR, var: &str) -> Option<usize> {
        if let Some(idx) = fn_ir.params.iter().position(|p| p == var) {
            return Some(idx);
        }
        if let Some(stripped) = var.strip_prefix(".arg_") {
            return fn_ir.params.iter().position(|p| p == stripped);
        }
        if let Some(stripped) = var.strip_prefix("arg_") {
            return fn_ir.params.iter().position(|p| p == stripped);
        }
        None
    }

    pub(super) fn value_param_index(fn_ir: &FnIR, vid: ValueId) -> Option<usize> {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        match fn_ir.values[v].kind {
            ValueKind::Param { index } => Some(index),
            _ => {
                let var = fn_ir.values[v].origin_var.as_deref()?;
                Self::param_index_for_var(fn_ir, var)
            }
        }
    }

    pub(super) fn value_is_const_one(fn_ir: &FnIR, vid: ValueId) -> bool {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        match fn_ir.values[v].kind {
            ValueKind::Const(Lit::Int(n)) => n == 1,
            ValueKind::Const(Lit::Float(f)) => (f - 1.0).abs() < f64::EPSILON,
            _ => false,
        }
    }

    pub(super) fn value_is_const_six(fn_ir: &FnIR, vid: ValueId) -> bool {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        match fn_ir.values[v].kind {
            ValueKind::Const(Lit::Int(n)) => n == 6,
            ValueKind::Const(Lit::Float(f)) => (f - 6.0).abs() < f64::EPSILON,
            _ => false,
        }
    }

    pub(super) fn value_is_const_half(fn_ir: &FnIR, vid: ValueId) -> bool {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        match fn_ir.values[v].kind {
            ValueKind::Const(Lit::Float(f)) => (f - 0.5).abs() < f64::EPSILON,
            _ => false,
        }
    }

    pub(super) fn resolve_load_alias_value(fn_ir: &FnIR, vid: ValueId) -> ValueId {
        fn unique_assign_source(fn_ir: &FnIR, var: &str) -> Option<ValueId> {
            let mut src: Option<ValueId> = None;
            for bb in &fn_ir.blocks {
                for ins in &bb.instrs {
                    let Instr::Assign { dst, src: s, .. } = ins else {
                        continue;
                    };
                    if dst != var {
                        continue;
                    }
                    match src {
                        None => src = Some(*s),
                        Some(prev) if prev == *s => {}
                        Some(_) => return None,
                    }
                }
            }
            src
        }

        let mut cur = vid;
        let mut seen = FxHashSet::default();
        while seen.insert(cur) {
            if let ValueKind::Load { var } = &fn_ir.values[cur].kind
                && let Some(src) = unique_assign_source(fn_ir, var)
            {
                cur = src;
                continue;
            }
            break;
        }
        cur
    }

    pub(super) fn flatten_assoc_binop(
        fn_ir: &FnIR,
        vid: ValueId,
        op: BinOp,
        out: &mut Vec<ValueId>,
    ) {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        match &fn_ir.values[v].kind {
            ValueKind::Binary { op: bop, lhs, rhs } if *bop == op => {
                Self::flatten_assoc_binop(fn_ir, *lhs, op, out);
                Self::flatten_assoc_binop(fn_ir, *rhs, op, out);
            }
            _ => out.push(v),
        }
    }

    pub(super) fn parse_var_minus_one(fn_ir: &FnIR, vid: ValueId) -> Option<String> {
        let v = Self::resolve_load_alias_value(fn_ir, vid);
        let ValueKind::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } = &fn_ir.values[v].kind
        else {
            return None;
        };
        if !Self::value_is_const_one(fn_ir, *rhs) {
            return None;
        }
        Self::value_var_name(fn_ir, *lhs)
    }

    pub(super) fn block_assignments(fn_ir: &FnIR, bid: BlockId) -> Option<Vec<(String, ValueId)>> {
        let mut out = Vec::new();
        for ins in &fn_ir.blocks[bid].instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                return None;
            };
            out.push((dst.clone(), *src));
        }
        Some(out)
    }
}
