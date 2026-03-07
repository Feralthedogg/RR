use crate::mir::*;
use crate::syntax::ast::{BinOp, Lit};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SymbolicBound {
    NegInf,
    PosInf,
    Const(i64),
    VarPlus(ValueId, i64),
    LenOf(ValueId, i64),
}

impl SymbolicBound {
    pub fn is_const(&self) -> bool {
        matches!(self, Self::Const(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeInterval {
    pub lo: SymbolicBound,
    pub hi: SymbolicBound,
}

impl RangeInterval {
    pub fn top() -> Self {
        Self {
            lo: SymbolicBound::NegInf,
            hi: SymbolicBound::PosInf,
        }
    }

    pub fn bottom() -> Self {
        // Technically lo > hi, but we just use special values
        Self {
            lo: SymbolicBound::PosInf,
            hi: SymbolicBound::NegInf,
        }
    }

    pub fn join(&self, other: &Self) -> Self {
        if self == &Self::bottom() {
            return other.clone();
        }
        if other == &Self::bottom() {
            return self.clone();
        }

        let lo = match (&self.lo, &other.lo) {
            (SymbolicBound::Const(a), SymbolicBound::Const(b)) => {
                SymbolicBound::Const((*a).min(*b))
            }
            (SymbolicBound::LenOf(a, off1), SymbolicBound::LenOf(b, off2)) if a == b => {
                SymbolicBound::LenOf(*a, (*off1).min(*off2))
            }
            (SymbolicBound::VarPlus(a, off1), SymbolicBound::VarPlus(b, off2)) if a == b => {
                SymbolicBound::VarPlus(*a, (*off1).min(*off2))
            }
            (a, b) if a == b => a.clone(),
            _ => SymbolicBound::NegInf,
        };

        let hi = match (&self.hi, &other.hi) {
            (SymbolicBound::Const(a), SymbolicBound::Const(b)) => {
                SymbolicBound::Const((*a).max(*b))
            }
            (SymbolicBound::LenOf(a, off1), SymbolicBound::LenOf(b, off2)) if a == b => {
                SymbolicBound::LenOf(*a, (*off1).max(*off2))
            }
            (SymbolicBound::VarPlus(a, off1), SymbolicBound::VarPlus(b, off2)) if a == b => {
                SymbolicBound::VarPlus(*a, (*off1).max(*off2))
            }
            (a, b) if a == b => a.clone(),
            _ => SymbolicBound::PosInf,
        };

        Self { lo, hi }
    }

    /// Proves if this interval is within [1, length(base)] (R 1-based indexing).
    pub fn proves_in_bounds(&self, base: ValueId) -> bool {
        // lo >= 1
        let lo_safe = match &self.lo {
            SymbolicBound::Const(n) => *n >= 1,
            SymbolicBound::LenOf(_, off) => *off >= 1, // length(x) + off >= 1
            _ => false,
        };

        // hi <= length(base)
        let hi_safe = match &self.hi {
            SymbolicBound::Const(_) => false, // We usually don't know length at compile time
            SymbolicBound::LenOf(b, off) => *b == base && *off <= 0, // e.g., length(x), length(x)-1
            _ => false,
        };

        lo_safe && hi_safe
    }
}

#[derive(Debug, Clone)]
pub struct RangeFacts {
    pub values: HashMap<ValueId, RangeInterval>,
}

impl Default for RangeFacts {
    fn default() -> Self {
        Self::new()
    }
}

impl RangeFacts {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn get(&self, vid: ValueId) -> RangeInterval {
        self.values
            .get(&vid)
            .cloned()
            .unwrap_or(RangeInterval::top())
    }

    pub fn set(&mut self, vid: ValueId, interval: RangeInterval) -> bool {
        let old = self.get(vid);
        if old != interval {
            self.values.insert(vid, interval);
            true
        } else {
            false
        }
    }

    pub fn join(&mut self, other: &Self) -> bool {
        let mut changed = false;
        // Optimization: only iterate over keys in 'other'
        for (&vid, other_intv) in &other.values {
            let self_intv = self.get(vid);
            let joined = self_intv.join(other_intv);
            if joined != self_intv {
                self.values.insert(vid, joined);
                changed = true;
            }
        }
        changed
    }
}

pub fn analyze_ranges(fn_ir: &FnIR) -> Vec<RangeFacts> {
    let mut bb_facts = vec![RangeFacts::new(); fn_ir.blocks.len()];
    let mut worklist = VecDeque::new();
    let mut current_facts = RangeFacts::new();
    let mut then_facts = RangeFacts::new();
    let mut else_facts = RangeFacts::new();

    // Init entry block
    worklist.push_back(fn_ir.entry);

    // Seed parameters and constants
    let mut initial_facts = RangeFacts::new();
    for (id, val) in fn_ir.values.iter().enumerate() {
        if let ValueKind::Const(Lit::Int(n)) = &val.kind {
            initial_facts.set(
                id,
                RangeInterval {
                    lo: SymbolicBound::Const(*n),
                    hi: SymbolicBound::Const(*n),
                },
            );
        }
    }
    bb_facts[fn_ir.entry] = initial_facts;

    let mut iterations = HashMap::new();

    while let Some(bid) = worklist.pop_front() {
        *iterations.entry(bid).or_insert(0) += 1;

        current_facts.clone_from(&bb_facts[bid]);
        transfer_block(bid, fn_ir, &mut current_facts);

        // Propagate to successors
        let block = &fn_ir.blocks[bid];
        match &block.term {
            Terminator::Goto(target) => {
                if bb_facts[*target].join(&current_facts) {
                    worklist.push_back(*target);
                }
            }
            Terminator::If {
                cond,
                then_bb,
                else_bb,
            } => {
                // Then branch: try to narrow
                then_facts.clone_from(&current_facts);
                narrow_facts(&mut then_facts, *cond, true, fn_ir);
                if bb_facts[*then_bb].join(&then_facts) {
                    worklist.push_back(*then_bb);
                }

                // Else branch: try to narrow
                else_facts.clone_from(&current_facts);
                narrow_facts(&mut else_facts, *cond, false, fn_ir);
                if bb_facts[*else_bb].join(&else_facts) {
                    worklist.push_back(*else_bb);
                }
            }
            _ => {}
        }

        // Safety: Widening if we iterate too much on a single block
        if iterations[&bid] > 10 {
            for intv in bb_facts[bid].values.values_mut() {
                // Aggressive widening: set to Top if not stable
                // (In a real implementation, we'd be more selective)
                *intv = RangeInterval::top();
            }
        }
    }

    bb_facts
}

fn transfer_block(bid: BlockId, fn_ir: &FnIR, facts: &mut RangeFacts) {
    let block = &fn_ir.blocks[bid];
    for instr in &block.instrs {
        transfer_instr(instr, &fn_ir.values, facts);
    }
}

pub fn transfer_instr(instr: &Instr, values: &[Value], facts: &mut RangeFacts) {
    match instr {
        Instr::Assign { src, .. } | Instr::Eval { val: src, .. } => {
            ensure_value_range(*src, values, facts);
        }
        Instr::StoreIndex1D { base, idx, val, .. } => {
            ensure_value_range(*base, values, facts);
            ensure_value_range(*idx, values, facts);
            ensure_value_range(*val, values, facts);
        }
        Instr::StoreIndex2D {
            base, r, c, val, ..
        } => {
            ensure_value_range(*base, values, facts);
            ensure_value_range(*r, values, facts);
            ensure_value_range(*c, values, facts);
            ensure_value_range(*val, values, facts);
        }
    }
}

pub fn ensure_value_range(vid: ValueId, values: &[Value], facts: &mut RangeFacts) -> RangeInterval {
    let mut seen = HashSet::new();
    ensure_value_range_inner(vid, values, facts, &mut seen)
}

fn ensure_value_range_inner(
    vid: ValueId,
    values: &[Value],
    facts: &mut RangeFacts,
    seen: &mut HashSet<ValueId>,
) -> RangeInterval {
    if let Some(existing) = facts.values.get(&vid) {
        return existing.clone();
    }
    if !seen.insert(vid) {
        // Recursive cycle (e.g. loop Phi self-edge): conservatively Top.
        return RangeInterval::top();
    }

    let interval = match &values[vid].kind {
        ValueKind::Const(Lit::Int(n)) => RangeInterval {
            lo: SymbolicBound::Const(*n),
            hi: SymbolicBound::Const(*n),
        },
        ValueKind::Len { base } => RangeInterval {
            lo: SymbolicBound::LenOf(*base, 0),
            hi: SymbolicBound::LenOf(*base, 0),
        },
        ValueKind::Indices { base } => RangeInterval {
            lo: SymbolicBound::Const(0),
            hi: SymbolicBound::LenOf(*base, -1),
        },
        ValueKind::Range { start, end } => {
            let s = ensure_value_range_inner(*start, values, facts, seen);
            let e = ensure_value_range_inner(*end, values, facts, seen);
            RangeInterval { lo: s.lo, hi: e.hi }
        }
        ValueKind::Binary { op, lhs, rhs } => {
            let li = ensure_value_range_inner(*lhs, values, facts, seen);
            let ri = ensure_value_range_inner(*rhs, values, facts, seen);
            eval_binary(*op, &li, &ri)
        }
        ValueKind::Unary { .. } => RangeInterval::top(),
        ValueKind::Phi { args } => {
            let mut acc = RangeInterval::bottom();
            for (arg_val, _pred) in args {
                let arg_intv = ensure_value_range_inner(*arg_val, values, facts, seen);
                acc = acc.join(&arg_intv);
            }
            acc
        }
        ValueKind::Param { .. }
        | ValueKind::Call { .. }
        | ValueKind::Intrinsic { .. }
        | ValueKind::Index1D { .. }
        | ValueKind::Index2D { .. }
        | ValueKind::Load { .. }
        | ValueKind::Const(_) => RangeInterval::top(),
    };

    seen.remove(&vid);
    if interval != RangeInterval::top() {
        facts.values.insert(vid, interval.clone());
    }
    interval
}

fn eval_binary(op: BinOp, l: &RangeInterval, r: &RangeInterval) -> RangeInterval {
    match op {
        BinOp::Add => {
            let lo = add_bound(&l.lo, &r.lo, true);
            let hi = add_bound(&l.hi, &r.hi, false);
            RangeInterval { lo, hi }
        }
        BinOp::Sub => {
            let lo = sub_bound(&l.lo, &r.hi, true);
            let hi = sub_bound(&l.hi, &r.lo, false);
            RangeInterval { lo, hi }
        }
        _ => RangeInterval::top(),
    }
}

fn narrow_facts(facts: &mut RangeFacts, cond_id: ValueId, is_then: bool, fn_ir: &FnIR) {
    let cond = &fn_ir.values[cond_id];
    if let ValueKind::Binary { op, lhs, rhs } = &cond.kind {
        ensure_value_range(*lhs, &fn_ir.values, facts);
        ensure_value_range(*rhs, &fn_ir.values, facts);
        let left_intv = facts.get(*lhs);
        let right_intv = facts.get(*rhs);

        match (op, is_then) {
            // i <= rhs
            (BinOp::Le, true) | (BinOp::Gt, false) => {
                let new_hi = bound_min(&left_intv.hi, &right_intv.hi);
                facts.set(
                    *lhs,
                    RangeInterval {
                        lo: left_intv.lo.clone(),
                        hi: new_hi,
                    },
                );
            }
            // i < rhs  => i <= rhs - 1
            (BinOp::Lt, true) | (BinOp::Ge, false) => {
                let rhs_hi = right_intv.hi.shift(-1);
                let new_hi = bound_min(&left_intv.hi, &rhs_hi);
                facts.set(
                    *lhs,
                    RangeInterval {
                        lo: left_intv.lo.clone(),
                        hi: new_hi,
                    },
                );
            }
            // i >= rhs
            (BinOp::Ge, true) | (BinOp::Lt, false) => {
                let new_lo = bound_max(&left_intv.lo, &right_intv.lo);
                facts.set(
                    *lhs,
                    RangeInterval {
                        lo: new_lo,
                        hi: left_intv.hi.clone(),
                    },
                );
            }
            // i > rhs => i >= rhs + 1
            (BinOp::Gt, true) | (BinOp::Le, false) => {
                let rhs_lo = right_intv.lo.shift(1);
                let new_lo = bound_max(&left_intv.lo, &rhs_lo);
                facts.set(
                    *lhs,
                    RangeInterval {
                        lo: new_lo,
                        hi: left_intv.hi.clone(),
                    },
                );
            }
            _ => {}
        }
    }
}

fn add_bound(a: &SymbolicBound, b: &SymbolicBound, is_lo: bool) -> SymbolicBound {
    match (a, b) {
        (SymbolicBound::Const(x), SymbolicBound::Const(y)) => x
            .checked_add(*y)
            .map(SymbolicBound::Const)
            .unwrap_or_else(|| unknown_bound(is_lo)),
        (SymbolicBound::LenOf(base, off), SymbolicBound::Const(c))
        | (SymbolicBound::Const(c), SymbolicBound::LenOf(base, off)) => off
            .checked_add(*c)
            .map(|sum| SymbolicBound::LenOf(*base, sum))
            .unwrap_or_else(|| unknown_bound(is_lo)),
        (SymbolicBound::VarPlus(v, off), SymbolicBound::Const(c))
        | (SymbolicBound::Const(c), SymbolicBound::VarPlus(v, off)) => off
            .checked_add(*c)
            .map(|sum| SymbolicBound::VarPlus(*v, sum))
            .unwrap_or_else(|| unknown_bound(is_lo)),
        _ => unknown_bound(is_lo),
    }
}

fn sub_bound(a: &SymbolicBound, b: &SymbolicBound, is_lo: bool) -> SymbolicBound {
    match (a, b) {
        (SymbolicBound::Const(x), SymbolicBound::Const(y)) => x
            .checked_sub(*y)
            .map(SymbolicBound::Const)
            .unwrap_or_else(|| unknown_bound(is_lo)),
        (SymbolicBound::LenOf(base, off), SymbolicBound::Const(c)) => off
            .checked_sub(*c)
            .map(|diff| SymbolicBound::LenOf(*base, diff))
            .unwrap_or_else(|| unknown_bound(is_lo)),
        (SymbolicBound::VarPlus(v, off), SymbolicBound::Const(c)) => off
            .checked_sub(*c)
            .map(|diff| SymbolicBound::VarPlus(*v, diff))
            .unwrap_or_else(|| unknown_bound(is_lo)),
        // (v + a) - (v + b) -> const (a - b)
        (SymbolicBound::VarPlus(v1, off1), SymbolicBound::VarPlus(v2, off2)) if v1 == v2 => off1
            .checked_sub(*off2)
            .map(SymbolicBound::Const)
            .unwrap_or_else(|| unknown_bound(is_lo)),
        (SymbolicBound::LenOf(base1, off1), SymbolicBound::LenOf(base2, off2))
            if base1 == base2 =>
        {
            off1.checked_sub(*off2)
                .map(SymbolicBound::Const)
                .unwrap_or_else(|| unknown_bound(is_lo))
        }
        _ => unknown_bound(is_lo),
    }
}

fn bound_min(current: &SymbolicBound, candidate: &SymbolicBound) -> SymbolicBound {
    match (current, candidate) {
        (SymbolicBound::Const(a), SymbolicBound::Const(b)) => SymbolicBound::Const((*a).min(*b)),
        (SymbolicBound::LenOf(a, off1), SymbolicBound::LenOf(b, off2)) if a == b => {
            SymbolicBound::LenOf(*a, (*off1).min(*off2))
        }
        (SymbolicBound::VarPlus(a, off1), SymbolicBound::VarPlus(b, off2)) if a == b => {
            SymbolicBound::VarPlus(*a, (*off1).min(*off2))
        }
        (SymbolicBound::PosInf, b) => b.clone(),
        _ => current.clone(),
    }
}

fn bound_max(current: &SymbolicBound, candidate: &SymbolicBound) -> SymbolicBound {
    match (current, candidate) {
        (SymbolicBound::Const(a), SymbolicBound::Const(b)) => SymbolicBound::Const((*a).max(*b)),
        (SymbolicBound::LenOf(a, off1), SymbolicBound::LenOf(b, off2)) if a == b => {
            SymbolicBound::LenOf(*a, (*off1).max(*off2))
        }
        (SymbolicBound::VarPlus(a, off1), SymbolicBound::VarPlus(b, off2)) if a == b => {
            SymbolicBound::VarPlus(*a, (*off1).max(*off2))
        }
        (SymbolicBound::NegInf, b) => b.clone(),
        _ => current.clone(),
    }
}

impl SymbolicBound {
    fn shift(&self, delta: i64) -> SymbolicBound {
        match self {
            SymbolicBound::Const(n) => n
                .checked_add(delta)
                .map(SymbolicBound::Const)
                .unwrap_or_else(|| overflow_bound_from_delta(delta)),
            SymbolicBound::LenOf(b, off) => off
                .checked_add(delta)
                .map(|sum| SymbolicBound::LenOf(*b, sum))
                .unwrap_or_else(|| overflow_bound_from_delta(delta)),
            SymbolicBound::VarPlus(v, off) => off
                .checked_add(delta)
                .map(|sum| SymbolicBound::VarPlus(*v, sum))
                .unwrap_or_else(|| overflow_bound_from_delta(delta)),
            SymbolicBound::NegInf => SymbolicBound::NegInf,
            SymbolicBound::PosInf => SymbolicBound::PosInf,
        }
    }
}

fn unknown_bound(is_lo: bool) -> SymbolicBound {
    if is_lo {
        SymbolicBound::NegInf
    } else {
        SymbolicBound::PosInf
    }
}

fn overflow_bound_from_delta(delta: i64) -> SymbolicBound {
    if delta < 0 {
        SymbolicBound::NegInf
    } else {
        SymbolicBound::PosInf
    }
}
