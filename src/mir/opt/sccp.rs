use crate::mir::*;
use crate::syntax::ast::Lit;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;

#[derive(Clone, Debug, PartialEq)]
enum Lattice {
    Top, // Undefined
    Constant(Lit),
    Bottom, // Overdefined
}

type ExecutableEdge = (BlockId, BlockId);
type SolveResult = (
    FxHashMap<ValueId, Lattice>,
    FxHashSet<BlockId>,
    FxHashSet<ExecutableEdge>,
);

pub struct MirSCCP;

impl Default for MirSCCP {
    fn default() -> Self {
        Self::new()
    }
}

impl MirSCCP {
    pub fn new() -> Self {
        Self
    }

    pub fn optimize(&self, fn_ir: &mut FnIR) -> bool {
        let (lattice, executable_blocks, _executable_edges) = self.solve(fn_ir);
        self.apply_results(fn_ir, &lattice, &executable_blocks)
    }

    fn solve(&self, fn_ir: &FnIR) -> SolveResult {
        let mut lattice = FxHashMap::default();
        let mut executable_edges = FxHashSet::default(); // FxHashSet<(from, to)>
        let mut executable_blocks = FxHashSet::default();
        let mut flow_worklist = VecDeque::new();
        let mut ssa_worklist = VecDeque::new();

        // Initial state
        flow_worklist.push_back((fn_ir.entry, fn_ir.entry)); // Mock edge for entry

        // Initial lattice state: Constants are Constant, others are Top
        for (id, val) in fn_ir.values.iter().enumerate() {
            if let ValueKind::Const(lit) = &val.kind {
                lattice.insert(id, Lattice::Constant(lit.clone()));
                ssa_worklist.push_back(id);
            }
        }

        // Maps value to its users
        let mut users: FxHashMap<ValueId, Vec<User>> = FxHashMap::default();
        self.build_user_map(fn_ir, &mut users);

        while !flow_worklist.is_empty() || !ssa_worklist.is_empty() {
            if let Some((from, to)) = flow_worklist.pop_front() {
                if executable_edges.insert((from, to)) {
                    let newly_executable_block = executable_blocks.insert(to);
                    if newly_executable_block {
                        self.visit_block(
                            to,
                            fn_ir,
                            &mut lattice,
                            &executable_edges,
                            &mut flow_worklist,
                            &mut ssa_worklist,
                        );
                    }

                    // Re-evaluate Phis in the 'to' block because a new incoming edge is executable
                    for (id, val) in fn_ir.values.iter().enumerate() {
                        if val.phi_block == Some(to)
                            && let ValueKind::Phi { args } = &val.kind
                        {
                            // Basic check: does this Phi have an argument from 'from'?
                            if args.iter().any(|(_, p)| *p == from) {
                                self.visit_value(
                                    id,
                                    fn_ir,
                                    &mut lattice,
                                    &executable_edges,
                                    &mut ssa_worklist,
                                );
                            }
                        }
                    }
                }
            } else if let Some(val_id) = ssa_worklist.pop_front()
                && let Some(user_list) = users.get(&val_id)
            {
                for user in user_list {
                    match user {
                        User::Block(bid) => {
                            if executable_blocks.contains(bid) {
                                self.visit_block(
                                    *bid,
                                    fn_ir,
                                    &mut lattice,
                                    &executable_edges,
                                    &mut flow_worklist,
                                    &mut ssa_worklist,
                                );
                            }
                        }
                        User::Value(target_val) => {
                            self.visit_value(
                                *target_val,
                                fn_ir,
                                &mut lattice,
                                &executable_edges,
                                &mut ssa_worklist,
                            );
                        }
                    }
                }
            }
        }

        (lattice, executable_blocks, executable_edges)
    }

    #[cfg(test)]
    fn solve_for_test(&self, fn_ir: &FnIR) -> (FxHashMap<ValueId, Lattice>, FxHashSet<BlockId>) {
        let (lattice, executable_blocks, _edges) = self.solve(fn_ir);
        (lattice, executable_blocks)
    }

    fn visit_block(
        &self,
        bid: BlockId,
        fn_ir: &FnIR,
        lattice: &mut FxHashMap<ValueId, Lattice>,
        executable_edges: &FxHashSet<(BlockId, BlockId)>,
        flow_worklist: &mut VecDeque<(BlockId, BlockId)>,
        ssa_worklist: &mut VecDeque<ValueId>,
    ) {
        let block = &fn_ir.blocks[bid];

        for instr in &block.instrs {
            match instr {
                Instr::Assign { src, .. } | Instr::Eval { val: src, .. } => {
                    self.visit_value(*src, fn_ir, lattice, executable_edges, ssa_worklist);
                }
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    self.visit_value(*base, fn_ir, lattice, executable_edges, ssa_worklist);
                    self.visit_value(*idx, fn_ir, lattice, executable_edges, ssa_worklist);
                    self.visit_value(*val, fn_ir, lattice, executable_edges, ssa_worklist);
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    self.visit_value(*base, fn_ir, lattice, executable_edges, ssa_worklist);
                    self.visit_value(*r, fn_ir, lattice, executable_edges, ssa_worklist);
                    self.visit_value(*c, fn_ir, lattice, executable_edges, ssa_worklist);
                    self.visit_value(*val, fn_ir, lattice, executable_edges, ssa_worklist);
                }
            }
        }

        match &block.term {
            Terminator::Goto(target) => {
                flow_worklist.push_back((bid, *target));
            }
            Terminator::If {
                cond,
                then_bb,
                else_bb,
            } => {
                self.visit_value(*cond, fn_ir, lattice, executable_edges, ssa_worklist);
                match lattice.get(cond).cloned().unwrap_or(Lattice::Top) {
                    Lattice::Constant(Lit::Bool(true)) => {
                        flow_worklist.push_back((bid, *then_bb));
                    }
                    Lattice::Constant(Lit::Bool(false)) => {
                        flow_worklist.push_back((bid, *else_bb));
                    }
                    Lattice::Bottom => {
                        flow_worklist.push_back((bid, *then_bb));
                        flow_worklist.push_back((bid, *else_bb));
                    }
                    // Unknown or non-bool constant: conservatively keep both edges executable.
                    _ => {
                        flow_worklist.push_back((bid, *then_bb));
                        flow_worklist.push_back((bid, *else_bb));
                    }
                }
            }
            Terminator::Return(Some(v)) => {
                self.visit_value(*v, fn_ir, lattice, executable_edges, ssa_worklist);
            }
            Terminator::Return(None) | Terminator::Unreachable => {}
        }
    }

    fn visit_value(
        &self,
        val_id: ValueId,
        fn_ir: &FnIR,
        lattice: &mut FxHashMap<ValueId, Lattice>,
        executable_edges: &FxHashSet<(BlockId, BlockId)>,
        ssa_worklist: &mut VecDeque<ValueId>,
    ) {
        let val = &fn_ir.values[val_id];
        let old_state = lattice.get(&val_id).cloned().unwrap_or(Lattice::Top);

        let new_state = match &val.kind {
            ValueKind::Const(lit) => Lattice::Constant(lit.clone()),
            ValueKind::Binary { op, lhs, rhs } => {
                // Ensure operands are evaluated so nested expressions don't stay Top forever.
                self.visit_value(*lhs, fn_ir, lattice, executable_edges, ssa_worklist);
                self.visit_value(*rhs, fn_ir, lattice, executable_edges, ssa_worklist);
                let l = self.ensure_non_top(*lhs, fn_ir, lattice, ssa_worklist);
                let r = self.ensure_non_top(*rhs, fn_ir, lattice, ssa_worklist);
                self.eval_binary(*op, l, r)
            }
            ValueKind::Unary { op, rhs } => {
                self.visit_value(*rhs, fn_ir, lattice, executable_edges, ssa_worklist);
                let r = self.ensure_non_top(*rhs, fn_ir, lattice, ssa_worklist);
                self.eval_unary(*op, r)
            }
            ValueKind::Phi { args } => {
                let mut merged = Lattice::Top;
                let mut executable_incoming = 0usize;
                let mut saw_top = false;
                if let Some(phi_bb) = fn_ir.values[val_id].phi_block {
                    // Only merge values from executable incoming edges.
                    for (arg_val, pred_blk) in args {
                        if !executable_edges.contains(&(*pred_blk, phi_bb)) {
                            continue;
                        }
                        executable_incoming += 1;
                        let state = self.ensure_non_top(*arg_val, fn_ir, lattice, ssa_worklist);
                        if matches!(state, Lattice::Top) {
                            saw_top = true;
                        }
                        merged = self.meet(&merged, &state);
                        if matches!(merged, Lattice::Bottom) {
                            break;
                        }
                    }
                } else {
                    // Conservative: merge all incoming values when block ownership is unknown.
                    for (arg_val, _) in args {
                        executable_incoming += 1;
                        let state = self.ensure_non_top(*arg_val, fn_ir, lattice, ssa_worklist);
                        if matches!(state, Lattice::Top) {
                            saw_top = true;
                        }
                        merged = self.meet(&merged, &state);
                        if matches!(merged, Lattice::Bottom) {
                            break;
                        }
                    }
                }
                // Multiple executable inputs with unresolved (Top) input should not stay constant.
                if executable_incoming > 1 && saw_top {
                    merged = Lattice::Bottom;
                }
                merged
            }
            ValueKind::Len { base } => {
                self.eval_len(*base, fn_ir, lattice, executable_edges, ssa_worklist)
            }
            ValueKind::Index1D { base, idx, .. } => {
                self.eval_index1d(*base, *idx, fn_ir, lattice, executable_edges, ssa_worklist)
            }
            ValueKind::Call { callee, args, .. } => {
                self.eval_call(callee, args, fn_ir, lattice, executable_edges, ssa_worklist)
            }
            _ => Lattice::Bottom,
        };

        if old_state != new_state {
            lattice.insert(val_id, new_state);
            ssa_worklist.push_back(val_id);
        }
    }

    fn eval_binary(&self, op: BinOp, l: Lattice, r: Lattice) -> Lattice {
        match (l, r) {
            (Lattice::Constant(Lit::Int(lv)), Lattice::Constant(Lit::Int(rv))) => match op {
                // Never panic in optimizer: overflow means "not safely foldable".
                BinOp::Add => match lv.checked_add(rv) {
                    Some(v) => Lattice::Constant(Lit::Int(v)),
                    None => Lattice::Bottom,
                },
                BinOp::Sub => match lv.checked_sub(rv) {
                    Some(v) => Lattice::Constant(Lit::Int(v)),
                    None => Lattice::Bottom,
                },
                BinOp::Mul => match lv.checked_mul(rv) {
                    Some(v) => Lattice::Constant(Lit::Int(v)),
                    None => Lattice::Bottom,
                },
                BinOp::Div => match lv.checked_div(rv) {
                    Some(v) => Lattice::Constant(Lit::Int(v)),
                    None => Lattice::Bottom,
                },
                BinOp::Mod => match lv.checked_rem(rv) {
                    Some(v) => Lattice::Constant(Lit::Int(v)),
                    None => Lattice::Bottom,
                },
                BinOp::Lt => Lattice::Constant(Lit::Bool(lv < rv)),
                BinOp::Le => Lattice::Constant(Lit::Bool(lv <= rv)),
                BinOp::Gt => Lattice::Constant(Lit::Bool(lv > rv)),
                BinOp::Ge => Lattice::Constant(Lit::Bool(lv >= rv)),
                BinOp::Eq => Lattice::Constant(Lit::Bool(lv == rv)),
                BinOp::Ne => Lattice::Constant(Lit::Bool(lv != rv)),
                _ => Lattice::Bottom,
            },
            (Lattice::Bottom, _) | (_, Lattice::Bottom) => Lattice::Bottom,
            (Lattice::Top, _) | (_, Lattice::Top) => Lattice::Top,
            _ => Lattice::Bottom,
        }
    }

    fn eval_unary(&self, op: UnaryOp, r: Lattice) -> Lattice {
        match r {
            Lattice::Constant(Lit::Bool(v)) => {
                if matches!(op, UnaryOp::Not) {
                    Lattice::Constant(Lit::Bool(!v))
                } else {
                    Lattice::Bottom
                }
            }
            Lattice::Bottom => Lattice::Bottom,
            Lattice::Top => Lattice::Top,
            _ => Lattice::Bottom,
        }
    }

    fn meet(&self, old: &Lattice, new: &Lattice) -> Lattice {
        match (old, new) {
            (Lattice::Top, x) | (x, Lattice::Top) => x.clone(),
            (Lattice::Bottom, _) | (_, Lattice::Bottom) => Lattice::Bottom,
            (Lattice::Constant(c1), Lattice::Constant(c2)) => {
                if c1 == c2 {
                    Lattice::Constant(c1.clone())
                } else {
                    Lattice::Bottom
                }
            }
        }
    }

    fn eval_len(
        &self,
        base: ValueId,
        fn_ir: &FnIR,
        lattice: &mut FxHashMap<ValueId, Lattice>,
        executable_edges: &FxHashSet<(BlockId, BlockId)>,
        ssa_worklist: &mut VecDeque<ValueId>,
    ) -> Lattice {
        self.visit_value(base, fn_ir, lattice, executable_edges, ssa_worklist);
        match self.try_const_len(base, fn_ir, lattice) {
            Some(n) => Lattice::Constant(Lit::Int(n)),
            None => match lattice.get(&base).cloned().unwrap_or(Lattice::Top) {
                Lattice::Top => Lattice::Top,
                _ => Lattice::Bottom,
            },
        }
    }

    fn eval_index1d(
        &self,
        base: ValueId,
        idx: ValueId,
        fn_ir: &FnIR,
        lattice: &mut FxHashMap<ValueId, Lattice>,
        executable_edges: &FxHashSet<(BlockId, BlockId)>,
        ssa_worklist: &mut VecDeque<ValueId>,
    ) -> Lattice {
        self.visit_value(base, fn_ir, lattice, executable_edges, ssa_worklist);
        self.visit_value(idx, fn_ir, lattice, executable_edges, ssa_worklist);

        let base_state = lattice.get(&base).cloned().unwrap_or(Lattice::Top);
        let idx_state = lattice.get(&idx).cloned().unwrap_or(Lattice::Top);
        if matches!(base_state, Lattice::Top) || matches!(idx_state, Lattice::Top) {
            return Lattice::Top;
        }

        if let Some(i) = self.const_index_value(&idx_state)
            && let Some(v) = self.try_const_index(base, i, fn_ir, lattice)
        {
            return Lattice::Constant(v);
        }

        Lattice::Bottom
    }

    fn eval_call(
        &self,
        callee: &str,
        args: &[ValueId],
        fn_ir: &FnIR,
        lattice: &mut FxHashMap<ValueId, Lattice>,
        executable_edges: &FxHashSet<(BlockId, BlockId)>,
        ssa_worklist: &mut VecDeque<ValueId>,
    ) -> Lattice {
        let mut arg_states = Vec::with_capacity(args.len());
        for arg in args {
            self.visit_value(*arg, fn_ir, lattice, executable_edges, ssa_worklist);
            arg_states.push(lattice.get(arg).cloned().unwrap_or(Lattice::Top));
        }

        if arg_states.iter().any(|s| matches!(s, Lattice::Top)) {
            return Lattice::Top;
        }

        if let Some(v) = self.try_eval_builtin_call(callee, &arg_states) {
            return Lattice::Constant(v);
        }

        Lattice::Bottom
    }

    fn try_const_len(
        &self,
        id: ValueId,
        fn_ir: &FnIR,
        lattice: &FxHashMap<ValueId, Lattice>,
    ) -> Option<i64> {
        let state = lattice.get(&id).cloned().unwrap_or(Lattice::Top);
        if let Lattice::Constant(lit) = state {
            return Some(match lit {
                Lit::Null => 0,
                _ => 1,
            });
        }

        match &fn_ir.values[id].kind {
            ValueKind::Range { start, end } => {
                let s = self.const_int_from_value(*start, fn_ir, lattice)?;
                let e = self.const_int_from_value(*end, fn_ir, lattice)?;
                Self::checked_range_len(s, e)
            }
            ValueKind::Indices { base } => self.try_const_len(*base, fn_ir, lattice),
            ValueKind::Call { callee, args, .. } if (callee == "c" || callee == "list") => {
                i64::try_from(args.len()).ok()
            }
            ValueKind::Call { callee, args, .. } if callee == "seq_along" && args.len() == 1 => {
                self.try_const_len(args[0], fn_ir, lattice)
            }
            ValueKind::Call { callee, args, .. } if callee == "seq_len" && args.len() == 1 => {
                let n = self.const_int_like_from_value(args[0], fn_ir, lattice)?;
                if n >= 0 { Some(n) } else { None }
            }
            _ => None,
        }
    }

    fn try_const_index(
        &self,
        base: ValueId,
        index1: i64,
        fn_ir: &FnIR,
        lattice: &FxHashMap<ValueId, Lattice>,
    ) -> Option<Lit> {
        if index1 < 1 {
            return None;
        }
        match &fn_ir.values[base].kind {
            ValueKind::Range { start, end } => {
                let s = self.const_int_from_value(*start, fn_ir, lattice)?;
                let e = self.const_int_from_value(*end, fn_ir, lattice)?;
                let v = Self::checked_range_at_1based(s, e, index1)?;
                Some(Lit::Int(v))
            }
            ValueKind::Indices { base: src } => {
                let len = self.try_const_len(*src, fn_ir, lattice)?;
                if index1 > len {
                    return None;
                }
                Some(Lit::Int(index1.checked_sub(1)?))
            }
            ValueKind::Call { callee, args, .. }
                if (callee == "c" || callee == "list") && !args.is_empty() =>
            {
                let idx0 = usize::try_from(index1.checked_sub(1)?).ok()?;
                if idx0 >= args.len() {
                    return None;
                }
                self.const_lit_from_value(args[idx0], fn_ir, lattice)
            }
            ValueKind::Call { callee, args, .. } if callee == "seq_len" && args.len() == 1 => {
                let n = self.const_int_like_from_value(args[0], fn_ir, lattice)?;
                if n < 0 || index1 > n {
                    return None;
                }
                Some(Lit::Int(index1))
            }
            _ => None,
        }
    }

    fn try_eval_builtin_call(&self, callee: &str, args: &[Lattice]) -> Option<Lit> {
        let nums: Option<Vec<f64>> = args.iter().map(Self::const_numeric).collect();
        match callee {
            "abs" if args.len() == 1 => {
                let x = Self::const_numeric(&args[0])?;
                Some(Self::lit_from_f64(x.abs()))
            }
            "sqrt" if args.len() == 1 => {
                let x = Self::const_numeric(&args[0])?;
                if x < 0.0 {
                    None
                } else {
                    Some(Self::lit_from_f64(x.sqrt()))
                }
            }
            "sin" if args.len() == 1 => {
                Some(Self::lit_from_f64(Self::const_numeric(&args[0])?.sin()))
            }
            "cos" if args.len() == 1 => {
                Some(Self::lit_from_f64(Self::const_numeric(&args[0])?.cos()))
            }
            "tan" if args.len() == 1 => {
                Some(Self::lit_from_f64(Self::const_numeric(&args[0])?.tan()))
            }
            "asin" if args.len() == 1 => {
                Some(Self::lit_from_f64(Self::const_numeric(&args[0])?.asin()))
            }
            "acos" if args.len() == 1 => {
                Some(Self::lit_from_f64(Self::const_numeric(&args[0])?.acos()))
            }
            "atan" if args.len() == 1 => {
                Some(Self::lit_from_f64(Self::const_numeric(&args[0])?.atan()))
            }
            "atan2" if args.len() == 2 => {
                let y = Self::const_numeric(&args[0])?;
                let x = Self::const_numeric(&args[1])?;
                Some(Self::lit_from_f64(y.atan2(x)))
            }
            "sinh" if args.len() == 1 => {
                Some(Self::lit_from_f64(Self::const_numeric(&args[0])?.sinh()))
            }
            "cosh" if args.len() == 1 => {
                Some(Self::lit_from_f64(Self::const_numeric(&args[0])?.cosh()))
            }
            "tanh" if args.len() == 1 => {
                Some(Self::lit_from_f64(Self::const_numeric(&args[0])?.tanh()))
            }
            "log" if args.len() == 1 => {
                let x = Self::const_numeric(&args[0])?;
                if x <= 0.0 {
                    None
                } else {
                    Some(Self::lit_from_f64(x.ln()))
                }
            }
            "log10" if args.len() == 1 => {
                let x = Self::const_numeric(&args[0])?;
                if x <= 0.0 {
                    None
                } else {
                    Some(Self::lit_from_f64(x.log10()))
                }
            }
            "log2" if args.len() == 1 => {
                let x = Self::const_numeric(&args[0])?;
                if x <= 0.0 {
                    None
                } else {
                    Some(Self::lit_from_f64(x.log2()))
                }
            }
            "exp" if args.len() == 1 => {
                Some(Self::lit_from_f64(Self::const_numeric(&args[0])?.exp()))
            }
            "floor" if args.len() == 1 => {
                Some(Self::lit_from_f64(Self::const_numeric(&args[0])?.floor()))
            }
            "ceiling" if args.len() == 1 => {
                Some(Self::lit_from_f64(Self::const_numeric(&args[0])?.ceil()))
            }
            "trunc" if args.len() == 1 => {
                Some(Self::lit_from_f64(Self::const_numeric(&args[0])?.trunc()))
            }
            "round" if args.len() == 1 => {
                Some(Self::lit_from_f64(Self::const_numeric(&args[0])?.round()))
            }
            "round" if args.len() == 2 => {
                let x = Self::const_numeric(&args[0])?;
                let digits = Self::const_numeric(&args[1])?;
                let p = 10f64.powf(digits);
                Some(Self::lit_from_f64((x * p).round() / p))
            }
            "sign" if args.len() == 1 => {
                let x = Self::const_numeric(&args[0])?;
                Some(Self::lit_from_f64(x.signum()))
            }
            "sum" => {
                let ns = nums?;
                Some(Self::lit_from_f64(ns.iter().sum()))
            }
            "mean" if !args.is_empty() => {
                let ns = nums?;
                Some(Self::lit_from_f64(ns.iter().sum::<f64>() / ns.len() as f64))
            }
            "min" if !args.is_empty() => {
                let ns = nums?;
                Some(Self::lit_from_f64(
                    ns.into_iter().fold(f64::INFINITY, |a, b| a.min(b)),
                ))
            }
            "max" if !args.is_empty() => {
                let ns = nums?;
                Some(Self::lit_from_f64(
                    ns.into_iter().fold(f64::NEG_INFINITY, |a, b| a.max(b)),
                ))
            }
            "pmin" if !args.is_empty() => {
                let ns = nums?;
                Some(Self::lit_from_f64(
                    ns.into_iter().fold(f64::INFINITY, |a, b| a.min(b)),
                ))
            }
            "pmax" if !args.is_empty() => {
                let ns = nums?;
                Some(Self::lit_from_f64(
                    ns.into_iter().fold(f64::NEG_INFINITY, |a, b| a.max(b)),
                ))
            }
            _ => None,
        }
    }

    fn const_lit_from_value(
        &self,
        id: ValueId,
        fn_ir: &FnIR,
        lattice: &FxHashMap<ValueId, Lattice>,
    ) -> Option<Lit> {
        if let Some(Lattice::Constant(lit)) = lattice.get(&id) {
            return Some(lit.clone());
        }
        if let ValueKind::Const(lit) = &fn_ir.values[id].kind {
            return Some(lit.clone());
        }
        None
    }

    fn const_int_from_value(
        &self,
        id: ValueId,
        fn_ir: &FnIR,
        lattice: &FxHashMap<ValueId, Lattice>,
    ) -> Option<i64> {
        let lit = self.const_lit_from_value(id, fn_ir, lattice)?;
        match lit {
            Lit::Int(i) => Some(i),
            _ => None,
        }
    }

    fn const_int_like_from_value(
        &self,
        id: ValueId,
        fn_ir: &FnIR,
        lattice: &FxHashMap<ValueId, Lattice>,
    ) -> Option<i64> {
        let lit = self.const_lit_from_value(id, fn_ir, lattice)?;
        match lit {
            Lit::Int(i) => Some(i),
            Lit::Float(f) => Self::float_to_i64_exact(f),
            _ => None,
        }
    }

    fn const_index_value(&self, state: &Lattice) -> Option<i64> {
        match state {
            Lattice::Constant(Lit::Int(i)) => Some(*i),
            Lattice::Constant(Lit::Float(f)) => Self::float_to_i64_exact(*f),
            _ => None,
        }
    }

    fn const_numeric(state: &Lattice) -> Option<f64> {
        match state {
            Lattice::Constant(Lit::Int(i)) => Some(*i as f64),
            Lattice::Constant(Lit::Float(f)) => Some(*f),
            _ => None,
        }
    }

    fn lit_from_f64(v: f64) -> Lit {
        let rounded = v.round();
        if (v - rounded).abs() < 1e-12 {
            match Self::float_to_i64_exact(rounded) {
                Some(i) => Lit::Int(i),
                None => Lit::Float(v),
            }
        } else {
            Lit::Float(v)
        }
    }

    fn float_to_i64_exact(f: f64) -> Option<i64> {
        if !f.is_finite() {
            return None;
        }
        if (f.fract()).abs() >= 1e-12 {
            return None;
        }
        if f < i64::MIN as f64 || f > i64::MAX as f64 {
            return None;
        }
        Some(f as i64)
    }

    fn checked_range_len(start: i64, end: i64) -> Option<i64> {
        let diff = i128::from(end) - i128::from(start);
        let abs = if diff < 0 { -diff } else { diff };
        let len = abs.checked_add(1)?;
        i64::try_from(len).ok()
    }

    fn checked_range_at_1based(start: i64, end: i64, index1: i64) -> Option<i64> {
        if index1 < 1 {
            return None;
        }
        let len = Self::checked_range_len(start, end)?;
        if index1 > len {
            return None;
        }
        let offset = index1.checked_sub(1)?;
        if end >= start {
            start.checked_add(offset)
        } else {
            start.checked_sub(offset)
        }
    }

    fn ensure_non_top(
        &self,
        val_id: ValueId,
        fn_ir: &FnIR,
        lattice: &mut FxHashMap<ValueId, Lattice>,
        ssa_worklist: &mut VecDeque<ValueId>,
    ) -> Lattice {
        let mut state = lattice.get(&val_id).cloned().unwrap_or(Lattice::Top);
        if matches!(state, Lattice::Top) {
            match &fn_ir.values[val_id].kind {
                ValueKind::Const(lit) => {
                    state = Lattice::Constant(lit.clone());
                    lattice.insert(val_id, state.clone());
                    ssa_worklist.push_back(val_id);
                }
                ValueKind::Load { .. } | ValueKind::Param { .. } => {
                    state = Lattice::Bottom;
                    lattice.insert(val_id, state.clone());
                    ssa_worklist.push_back(val_id);
                }
                _ => {}
            }
        }
        state
    }

    fn apply_results(
        &self,
        fn_ir: &mut FnIR,
        lattice: &FxHashMap<ValueId, Lattice>,
        executable: &FxHashSet<BlockId>,
    ) -> bool {
        let mut changed = false;

        for (id, state) in lattice {
            if let Lattice::Constant(lit) = state {
                let val = &mut fn_ir.values[*id];
                if !matches!(val.kind, ValueKind::Const(_)) {
                    val.kind = ValueKind::Const(lit.clone());
                    changed = true;
                }
            }
        }

        for bid in 0..fn_ir.blocks.len() {
            if !executable.contains(&bid) {
                continue;
            }

            let mut new_term = None;
            {
                let block = &fn_ir.blocks[bid];
                if let Terminator::If {
                    cond,
                    then_bb,
                    else_bb,
                } = &block.term
                    && let Some(state) = lattice.get(cond)
                    && let Lattice::Constant(Lit::Bool(c)) = state
                {
                    new_term = Some(Terminator::Goto(if *c { *then_bb } else { *else_bb }));
                }
            }

            if let Some(term) = new_term {
                fn_ir.blocks[bid].term = term;
                changed = true;
            }
        }

        changed
    }

    fn build_user_map(&self, fn_ir: &FnIR, users: &mut FxHashMap<ValueId, Vec<User>>) {
        for blk in &fn_ir.blocks {
            match &blk.term {
                Terminator::If { cond, .. } => {
                    users.entry(*cond).or_default().push(User::Block(blk.id));
                }
                Terminator::Return(Some(v)) => {
                    users.entry(*v).or_default().push(User::Block(blk.id));
                }
                _ => {}
            }
            for instr in &blk.instrs {
                match instr {
                    Instr::Assign { src, .. } | Instr::Eval { val: src, .. } => {
                        users.entry(*src).or_default().push(User::Block(blk.id));
                    }
                    Instr::StoreIndex1D { base, idx, val, .. } => {
                        users.entry(*base).or_default().push(User::Block(blk.id));
                        users.entry(*idx).or_default().push(User::Block(blk.id));
                        users.entry(*val).or_default().push(User::Block(blk.id));
                    }
                    Instr::StoreIndex2D {
                        base, r, c, val, ..
                    } => {
                        users.entry(*base).or_default().push(User::Block(blk.id));
                        users.entry(*r).or_default().push(User::Block(blk.id));
                        users.entry(*c).or_default().push(User::Block(blk.id));
                        users.entry(*val).or_default().push(User::Block(blk.id));
                    }
                }
            }
        }
        for (id, val) in fn_ir.values.iter().enumerate() {
            match &val.kind {
                ValueKind::Binary { lhs, rhs, .. } => {
                    users.entry(*lhs).or_default().push(User::Value(id));
                    users.entry(*rhs).or_default().push(User::Value(id));
                }
                ValueKind::Unary { rhs, .. } => {
                    users.entry(*rhs).or_default().push(User::Value(id));
                }
                ValueKind::Phi { args } => {
                    for (arg, _) in args {
                        users.entry(*arg).or_default().push(User::Value(id));
                    }
                }
                ValueKind::Len { base } | ValueKind::Indices { base } => {
                    users.entry(*base).or_default().push(User::Value(id));
                }
                ValueKind::Call { args, .. } => {
                    for arg in args {
                        users.entry(*arg).or_default().push(User::Value(id));
                    }
                }
                ValueKind::Range { start, end } => {
                    users.entry(*start).or_default().push(User::Value(id));
                    users.entry(*end).or_default().push(User::Value(id));
                }
                ValueKind::Index1D { base, idx, .. } => {
                    users.entry(*base).or_default().push(User::Value(id));
                    users.entry(*idx).or_default().push(User::Value(id));
                }
                ValueKind::Index2D { base, r, c } => {
                    users.entry(*base).or_default().push(User::Value(id));
                    users.entry(*r).or_default().push(User::Value(id));
                    users.entry(*c).or_default().push(User::Value(id));
                }
                _ => {}
            }
        }
    }
}

#[derive(Clone, Debug)]
enum User {
    Block(BlockId),
    Value(ValueId),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::ast::BinOp;
    use crate::utils::Span;

    fn build_loop_phi_ir() -> (FnIR, ValueId, ValueId, BlockId) {
        let mut fn_ir = FnIR::new("phi_loop".to_string(), vec![]);
        let entry = fn_ir.add_block();
        let header = fn_ir.add_block();
        let latch = fn_ir.add_block();
        let exit = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = header;

        fn_ir.blocks[entry].term = Terminator::Goto(header);
        fn_ir.blocks[latch].term = Terminator::Goto(header);

        let c0 = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let c1 = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let c10 = fn_ir.add_value(
            ValueKind::Const(Lit::Int(10)),
            Span::default(),
            Facts::empty(),
            None,
        );

        let phi_i = fn_ir.add_value(
            ValueKind::Phi {
                args: vec![(c0, entry), (c0, latch)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.values[phi_i].phi_block = Some(header);

        let v_next = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: phi_i,
                rhs: c1,
            },
            Span::default(),
            Facts::empty(),
            None,
        );

        if let ValueKind::Phi { args } = &mut fn_ir.values[phi_i].kind {
            args[1] = (v_next, latch);
        }

        let cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Lt,
                lhs: phi_i,
                rhs: c10,
            },
            Span::default(),
            Facts::empty(),
            None,
        );

        fn_ir.blocks[header].term = Terminator::If {
            cond,
            then_bb: latch,
            else_bb: exit,
        };
        fn_ir.blocks[exit].term = Terminator::Return(Some(phi_i));
        (fn_ir, phi_i, cond, header)
    }

    #[test]
    fn test_meet_rules() {
        let sccp = MirSCCP::new();
        let top = Lattice::Top;
        let bot = Lattice::Bottom;
        let c1 = Lattice::Constant(Lit::Int(1));
        let c2 = Lattice::Constant(Lit::Int(2));

        assert_eq!(sccp.meet(&top, &c1), c1);
        assert_eq!(sccp.meet(&top, &bot), bot);
        assert_eq!(sccp.meet(&bot, &c1), bot);
        assert_eq!(sccp.meet(&bot, &top), bot);
        assert_eq!(sccp.meet(&c1, &c1), c1);
        assert_eq!(sccp.meet(&c1, &c2), Lattice::Bottom);
    }

    #[test]
    fn test_phi_lowering_in_loop() {
        let (fn_ir, phi_i, cond, header) = build_loop_phi_ir();
        let sccp = MirSCCP::new();
        let (lattice, executable_blocks) = sccp.solve_for_test(&fn_ir);

        assert_eq!(lattice.get(&phi_i), Some(&Lattice::Bottom));
        assert_eq!(lattice.get(&cond), Some(&Lattice::Bottom));
        assert!(executable_blocks.contains(&header));
    }

    #[test]
    fn test_dead_branch_removal() {
        let mut fn_ir = FnIR::new("branch_prune".to_string(), vec![]);
        let entry = fn_ir.add_block();
        let header = fn_ir.add_block();
        let then_bb = fn_ir.add_block();
        let else_bb = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = header;

        fn_ir.blocks[entry].term = Terminator::Goto(header);

        let c1000 = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1000)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let c0 = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Gt,
                lhs: c1000,
                rhs: c0,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let alive = fn_ir.add_value(
            ValueKind::Const(Lit::Str("Alive".to_string())),
            Span::default(),
            Facts::empty(),
            None,
        );
        let dead = fn_ir.add_value(
            ValueKind::Const(Lit::Str("Dead".to_string())),
            Span::default(),
            Facts::empty(),
            None,
        );

        fn_ir.blocks[header].term = Terminator::If {
            cond,
            then_bb,
            else_bb,
        };
        fn_ir.blocks[then_bb].term = Terminator::Return(Some(alive));
        fn_ir.blocks[else_bb].term = Terminator::Return(Some(dead));

        let sccp = MirSCCP::new();
        let (_lattice, executable_blocks) = sccp.solve_for_test(&fn_ir);
        assert!(!executable_blocks.contains(&else_bb));

        let mut optimized = fn_ir.clone();
        let changed = sccp.optimize(&mut optimized);
        assert!(changed);
        assert!(matches!(optimized.blocks[header].term, Terminator::Goto(t) if t == then_bb));
    }

    #[test]
    fn test_phi_with_executable_top_input_drops_to_bottom() {
        let mut fn_ir = FnIR::new("phi_top_input".to_string(), vec!["n".to_string()]);
        let entry = fn_ir.add_block();
        let header = fn_ir.add_block();
        let latch = fn_ir.add_block();
        let exit = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = header;

        fn_ir.blocks[entry].term = Terminator::Goto(header);
        fn_ir.blocks[latch].term = Terminator::Goto(header);

        let c0 = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let c1 = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let n = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("n".to_string()),
        );

        let phi_i = fn_ir.add_value(
            ValueKind::Phi {
                args: vec![(c0, entry), (c0, latch)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.values[phi_i].phi_block = Some(header);

        // Leave the backedge expression unresolved in early iterations (depends on phi itself).
        let next = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: phi_i,
                rhs: c1,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        if let ValueKind::Phi { args } = &mut fn_ir.values[phi_i].kind {
            args[1] = (next, latch);
        }

        let cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Lt,
                lhs: phi_i,
                rhs: n,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[header].term = Terminator::If {
            cond,
            then_bb: latch,
            else_bb: exit,
        };
        fn_ir.blocks[exit].term = Terminator::Return(Some(phi_i));

        let sccp = MirSCCP::new();
        let (lattice, _executable_blocks) = sccp.solve_for_test(&fn_ir);
        assert_eq!(lattice.get(&phi_i), Some(&Lattice::Bottom));
    }

    #[test]
    fn test_len_seq_along_constant_base() {
        let mut fn_ir = FnIR::new("len_seq".to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let n = fn_ir.add_value(
            ValueKind::Const(Lit::Int(5)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let seq = fn_ir.add_value(
            ValueKind::Call {
                callee: "seq_along".to_string(),
                args: vec![n],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let len = fn_ir.add_value(
            ValueKind::Len { base: seq },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].term = Terminator::Return(Some(len));

        let sccp = MirSCCP::new();
        let mut opt = fn_ir.clone();
        assert!(sccp.optimize(&mut opt));
        assert!(matches!(
            opt.values[len].kind,
            ValueKind::Const(Lit::Int(1))
        ));
    }

    #[test]
    fn test_index_range_constant_fold() {
        let mut fn_ir = FnIR::new("index_range".to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let c1 = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let c3 = fn_ir.add_value(
            ValueKind::Const(Lit::Int(3)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let c2 = fn_ir.add_value(
            ValueKind::Const(Lit::Int(2)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let r = fn_ir.add_value(
            ValueKind::Range { start: c1, end: c3 },
            Span::default(),
            Facts::empty(),
            None,
        );
        let idx = fn_ir.add_value(
            ValueKind::Index1D {
                base: r,
                idx: c2,
                is_safe: false,
                is_na_safe: false,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].term = Terminator::Return(Some(idx));

        let sccp = MirSCCP::new();
        let mut opt = fn_ir.clone();
        assert!(sccp.optimize(&mut opt));
        assert!(matches!(
            opt.values[idx].kind,
            ValueKind::Const(Lit::Int(2))
        ));
    }

    #[test]
    fn test_call_sum_constant_fold() {
        let mut fn_ir = FnIR::new("sum_const".to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let a = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let b = fn_ir.add_value(
            ValueKind::Const(Lit::Int(2)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let c = fn_ir.add_value(
            ValueKind::Const(Lit::Int(3)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let sum = fn_ir.add_value(
            ValueKind::Call {
                callee: "sum".to_string(),
                args: vec![a, b, c],
                names: vec![None, None, None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].term = Terminator::Return(Some(sum));

        let sccp = MirSCCP::new();
        let mut opt = fn_ir.clone();
        assert!(sccp.optimize(&mut opt));
        assert!(matches!(
            opt.values[sum].kind,
            ValueKind::Const(Lit::Int(6))
        ));
    }

    #[test]
    fn test_len_of_c_literal_constant_fold() {
        let mut fn_ir = FnIR::new("len_c_const".to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let a = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let b = fn_ir.add_value(
            ValueKind::Const(Lit::Int(2)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let c = fn_ir.add_value(
            ValueKind::Const(Lit::Int(3)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let vecv = fn_ir.add_value(
            ValueKind::Call {
                callee: "c".to_string(),
                args: vec![a, b, c],
                names: vec![None, None, None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let len = fn_ir.add_value(
            ValueKind::Len { base: vecv },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].term = Terminator::Return(Some(len));

        let sccp = MirSCCP::new();
        let mut opt = fn_ir.clone();
        assert!(sccp.optimize(&mut opt));
        assert!(matches!(
            opt.values[len].kind,
            ValueKind::Const(Lit::Int(3))
        ));
    }

    #[test]
    fn test_index_seq_len_constant_fold() {
        let mut fn_ir = FnIR::new("index_seq_len".to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let n = fn_ir.add_value(
            ValueKind::Const(Lit::Int(5)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let idx = fn_ir.add_value(
            ValueKind::Const(Lit::Int(2)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let seq = fn_ir.add_value(
            ValueKind::Call {
                callee: "seq_len".to_string(),
                args: vec![n],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let at = fn_ir.add_value(
            ValueKind::Index1D {
                base: seq,
                idx,
                is_safe: false,
                is_na_safe: false,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].term = Terminator::Return(Some(at));

        let sccp = MirSCCP::new();
        let mut opt = fn_ir.clone();
        assert!(sccp.optimize(&mut opt));
        assert!(matches!(opt.values[at].kind, ValueKind::Const(Lit::Int(2))));
    }

    #[test]
    fn test_call_log10_constant_fold() {
        let mut fn_ir = FnIR::new("log10_const".to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let ten = fn_ir.add_value(
            ValueKind::Const(Lit::Int(10)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let log = fn_ir.add_value(
            ValueKind::Call {
                callee: "log10".to_string(),
                args: vec![ten],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].term = Terminator::Return(Some(log));

        let sccp = MirSCCP::new();
        let mut opt = fn_ir.clone();
        assert!(sccp.optimize(&mut opt));
        assert!(matches!(
            opt.values[log].kind,
            ValueKind::Const(Lit::Int(1))
        ));
    }

    #[test]
    fn test_div_overflow_is_not_folded() {
        let mut fn_ir = FnIR::new("div_overflow".to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let min_i64 = fn_ir.add_value(
            ValueKind::Const(Lit::Int(i64::MIN)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let neg_one = fn_ir.add_value(
            ValueKind::Const(Lit::Int(-1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let div = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Div,
                lhs: min_i64,
                rhs: neg_one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].term = Terminator::Return(Some(div));

        let sccp = MirSCCP::new();
        let mut opt = fn_ir.clone();
        let _ = sccp.optimize(&mut opt);
        assert!(
            !matches!(opt.values[div].kind, ValueKind::Const(_)),
            "overflowing i64::MIN / -1 must stay runtime-evaluated"
        );
    }

    #[test]
    fn test_range_len_overflow_is_not_folded() {
        let mut fn_ir = FnIR::new("range_len_overflow".to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let start = fn_ir.add_value(
            ValueKind::Const(Lit::Int(i64::MIN)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let end = fn_ir.add_value(
            ValueKind::Const(Lit::Int(i64::MAX)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let range = fn_ir.add_value(
            ValueKind::Range { start, end },
            Span::default(),
            Facts::empty(),
            None,
        );
        let len = fn_ir.add_value(
            ValueKind::Len { base: range },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].term = Terminator::Return(Some(len));

        let sccp = MirSCCP::new();
        let mut opt = fn_ir.clone();
        let _ = sccp.optimize(&mut opt);
        assert!(
            !matches!(opt.values[len].kind, ValueKind::Const(_)),
            "range length overflow must not fold to an invalid constant"
        );
    }
}
