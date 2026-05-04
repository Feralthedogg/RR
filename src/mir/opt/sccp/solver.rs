use super::*;
impl MirSCCP {
    pub fn new() -> Self {
        Self
    }

    pub fn optimize(&self, fn_ir: &mut FnIR) -> bool {
        let (lattice, executable_blocks, _executable_edges) = self.solve(fn_ir);
        self.apply_results(fn_ir, &lattice, &executable_blocks)
    }

    pub(crate) fn solve(&self, fn_ir: &FnIR) -> SolveResult {
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
    pub(crate) fn solve_for_test(
        &self,
        fn_ir: &FnIR,
    ) -> (FxHashMap<ValueId, Lattice>, FxHashSet<BlockId>) {
        let (lattice, executable_blocks, _edges) = self.solve(fn_ir);
        (lattice, executable_blocks)
    }

    pub(crate) fn visit_block(
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
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    self.visit_value(*base, fn_ir, lattice, executable_edges, ssa_worklist);
                    self.visit_value(*i, fn_ir, lattice, executable_edges, ssa_worklist);
                    self.visit_value(*j, fn_ir, lattice, executable_edges, ssa_worklist);
                    self.visit_value(*k, fn_ir, lattice, executable_edges, ssa_worklist);
                    self.visit_value(*val, fn_ir, lattice, executable_edges, ssa_worklist);
                }
                Instr::UnsafeRBlock { .. } => {}
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

    pub(crate) fn visit_value(
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

    pub(crate) fn eval_binary(&self, op: BinOp, l: Lattice, r: Lattice) -> Lattice {
        match (l, r) {
            (Lattice::Bottom, _) | (_, Lattice::Bottom) => Lattice::Bottom,
            (Lattice::Top, _) | (_, Lattice::Top) => Lattice::Top,
            (Lattice::Constant(lhs), Lattice::Constant(rhs)) => {
                Self::eval_const_binary(op, &lhs, &rhs)
                    .map(Lattice::Constant)
                    .unwrap_or(Lattice::Bottom)
            }
        }
    }

    pub(crate) fn eval_unary(&self, op: UnaryOp, r: Lattice) -> Lattice {
        match r {
            Lattice::Constant(Lit::Int(v)) => match op {
                UnaryOp::Neg => v
                    .checked_neg()
                    .map(|negated| Lattice::Constant(Lit::Int(negated)))
                    .unwrap_or(Lattice::Bottom),
                _ => Lattice::Bottom,
            },
            Lattice::Constant(Lit::Float(v)) => match op {
                UnaryOp::Neg if v.is_finite() => Lattice::Constant(Lit::Float(-v)),
                _ => Lattice::Bottom,
            },
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

    pub(crate) fn eval_const_binary(op: BinOp, lhs: &Lit, rhs: &Lit) -> Option<Lit> {
        crate::mir::const_fold::eval_binary_const(op, lhs, rhs)
    }

    pub(crate) fn meet(&self, old: &Lattice, new: &Lattice) -> Lattice {
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

    pub(crate) fn eval_len(
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

    pub(crate) fn eval_index1d(
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

        if self.base_value_has_store_effects(fn_ir, base) {
            return Lattice::Bottom;
        }

        if let Some(i) = self.const_index_value(&idx_state)
            && let Some(v) = self.try_const_index(base, i, fn_ir, lattice)
        {
            return Lattice::Constant(v);
        }

        Lattice::Bottom
    }

    pub(crate) fn base_value_has_store_effects(&self, fn_ir: &FnIR, base: ValueId) -> bool {
        let direct_origin = fn_ir.values.get(base).and_then(|v| v.origin_var.as_deref());
        for block in &fn_ir.blocks {
            for instr in &block.instrs {
                let stored_base = match instr {
                    Instr::StoreIndex1D { base, .. }
                    | Instr::StoreIndex2D { base, .. }
                    | Instr::StoreIndex3D { base, .. } => *base,
                    Instr::Assign { .. } | Instr::Eval { .. } | Instr::UnsafeRBlock { .. } => {
                        continue;
                    }
                };
                if stored_base == base {
                    return true;
                }
                if direct_origin.is_some()
                    && fn_ir.values[stored_base].origin_var.as_deref() == direct_origin
                {
                    return true;
                }
            }
        }
        false
    }

    pub(crate) fn eval_call(
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

    pub(crate) fn try_const_len(
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

    pub(crate) fn try_const_index(
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

    pub(crate) fn try_eval_builtin_call(&self, callee: &str, args: &[Lattice]) -> Option<Lit> {
        Self::try_eval_unary_numeric_builtin(callee, args)
            .or_else(|| Self::try_eval_binary_numeric_builtin(callee, args))
            .or_else(|| Self::try_eval_numeric_reduction_builtin(callee, args))
    }

    pub(crate) fn try_eval_unary_numeric_builtin(callee: &str, args: &[Lattice]) -> Option<Lit> {
        if args.len() != 1 {
            return None;
        }
        let x = Self::const_numeric(&args[0])?;
        let value = match callee {
            "abs" => x.abs(),
            "sqrt" if x >= 0.0 => x.sqrt(),
            "sin" => x.sin(),
            "cos" => x.cos(),
            "tan" => x.tan(),
            "asin" => x.asin(),
            "acos" => x.acos(),
            "atan" => x.atan(),
            "sinh" => x.sinh(),
            "cosh" => x.cosh(),
            "tanh" => x.tanh(),
            "log" if x > 0.0 => x.ln(),
            "log10" if x > 0.0 => x.log10(),
            "log2" if x > 0.0 => x.log2(),
            "exp" => x.exp(),
            "floor" => x.floor(),
            "ceiling" => x.ceil(),
            "trunc" => x.trunc(),
            "round" => x.round(),
            "sign" if x > 0.0 => 1.0,
            "sign" if x < 0.0 => -1.0,
            "sign" => 0.0,
            _ => return None,
        };
        Self::float_lit(value, false)
    }

    pub(crate) fn try_eval_binary_numeric_builtin(callee: &str, args: &[Lattice]) -> Option<Lit> {
        match callee {
            "atan2" if args.len() == 2 => {
                let y = Self::const_numeric(&args[0])?;
                let x = Self::const_numeric(&args[1])?;
                Self::float_lit(y.atan2(x), false)
            }
            "round" if args.len() == 2 => {
                let x = Self::const_numeric(&args[0])?;
                let digits = Self::const_numeric(&args[1])?;
                let p = 10f64.powf(digits);
                Self::float_lit((x * p).round() / p, false)
            }
            _ => None,
        }
    }

    pub(crate) fn try_eval_numeric_reduction_builtin(
        callee: &str,
        args: &[Lattice],
    ) -> Option<Lit> {
        let nums: Vec<f64> = args
            .iter()
            .map(Self::const_numeric)
            .collect::<Option<_>>()?;
        match callee {
            "sum" => Self::float_lit(nums.iter().sum(), false),
            "mean" if !nums.is_empty() => {
                Self::float_lit(nums.iter().sum::<f64>() / nums.len() as f64, false)
            }
            "min" | "pmin" if !nums.is_empty() => {
                Self::float_lit(nums.into_iter().fold(f64::INFINITY, |a, b| a.min(b)), false)
            }
            "max" | "pmax" if !nums.is_empty() => Self::float_lit(
                nums.into_iter().fold(f64::NEG_INFINITY, |a, b| a.max(b)),
                false,
            ),
            _ => None,
        }
    }

    pub(crate) fn const_lit_from_value(
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

    pub(crate) fn const_int_from_value(
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

    pub(crate) fn const_int_like_from_value(
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

    pub(crate) fn const_index_value(&self, state: &Lattice) -> Option<i64> {
        match state {
            Lattice::Constant(Lit::Int(i)) => Some(*i),
            Lattice::Constant(Lit::Float(f)) => Self::float_to_i64_exact(*f),
            _ => None,
        }
    }

    pub(crate) fn const_numeric(state: &Lattice) -> Option<f64> {
        match state {
            Lattice::Constant(Lit::Int(i)) => Some(*i as f64),
            Lattice::Constant(Lit::Float(f)) => Some(*f),
            _ => None,
        }
    }

    pub(crate) fn float_lit(v: f64, force_float: bool) -> Option<Lit> {
        if !v.is_finite() {
            return None;
        }
        if force_float {
            return Some(Lit::Float(v));
        }
        let rounded = v.round();
        if (v - rounded).abs() < 1e-12 {
            match Self::float_to_i64_exact(rounded) {
                Some(i) => Some(Lit::Int(i)),
                None => Some(Lit::Float(v)),
            }
        } else {
            Some(Lit::Float(v))
        }
    }

    pub(crate) fn float_to_i64_exact(f: f64) -> Option<i64> {
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

    pub(crate) fn checked_range_len(start: i64, end: i64) -> Option<i64> {
        let diff = i128::from(end) - i128::from(start);
        let abs = if diff < 0 { -diff } else { diff };
        let len = abs.checked_add(1)?;
        i64::try_from(len).ok()
    }

    pub(crate) fn checked_range_at_1based(start: i64, end: i64, index1: i64) -> Option<i64> {
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

    pub(crate) fn ensure_non_top(
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

    pub(crate) fn apply_results(
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
}
