use crate::mir::*;
use rustc_hash::FxHashMap;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interval {
    pub min: i64,
    pub max: i64,
}

impl Interval {
    // We treat i64::MIN/MAX as Infinity for safety,
    // effectively "Unbounded" if we hit them.
    pub const TOP: Interval = Interval {
        min: i64::MIN,
        max: i64::MAX,
    };
    pub const BOTTOM: Interval = Interval {
        min: i64::MAX,
        max: i64::MIN,
    }; // Empty set representation

    pub fn new(min: i64, max: i64) -> Self {
        Self { min, max }
    }

    pub fn point(val: i64) -> Self {
        Self { min: val, max: val }
    }

    pub fn join(&self, other: &Interval) -> Interval {
        if *self == Self::BOTTOM {
            return *other;
        }
        if *other == Self::BOTTOM {
            return *self;
        }

        Interval {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    pub fn add(&self, other: &Interval) -> Interval {
        if *self == Self::BOTTOM || *other == Self::BOTTOM {
            return Self::BOTTOM;
        }
        // Saturating add to avoid overflow panics
        let min = self.min.saturating_add(other.min);
        let max = self.max.saturating_add(other.max);
        Interval { min, max }
    }

    pub fn sub(&self, other: &Interval) -> Interval {
        if *self == Self::BOTTOM || *other == Self::BOTTOM {
            return Self::BOTTOM;
        }
        // [a, b] - [c, d] = [a - d, b - c]
        let min = self.min.saturating_sub(other.max);
        let max = self.max.saturating_sub(other.min);
        Interval { min, max }
    }

    pub fn contains(&self, val: i64) -> bool {
        val >= self.min && val <= self.max
    }

    pub fn mul(&self, other: &Interval) -> Interval {
        if *self == Self::BOTTOM || *other == Self::BOTTOM {
            return Self::BOTTOM;
        }
        // [a, b] * [c, d] = [min(ac, ad, bc, bd), max(ac, ad, bc, bd)]
        let a = self.min;
        let b = self.max;
        let c = other.min;
        let d = other.max;

        // Handle potential overflow with saturating_mul
        let ac = a.saturating_mul(c);
        let ad = a.saturating_mul(d);
        let bc = b.saturating_mul(c);
        let bd = b.saturating_mul(d);

        let min = ac.min(ad).min(bc).min(bd);
        let max = ac.max(ad).max(bc).max(bd);

        Interval { min, max }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Facts {
    pub flags: u32,
    pub interval: Interval,
}

impl Facts {
    pub const NONE: u32 = 0;

    // Core Types
    pub const INT_SCALAR: u32 = 1 << 0;
    pub const BOOL_SCALAR: u32 = 1 << 1;

    // Properties
    pub const NON_NA: u32 = 1 << 2;
    pub const NON_NEG: u32 = 1 << 3;
    pub const IN_BOUNDS: u32 = 1 << 4;
    pub const ONE_BASED: u32 = 1 << 5;
    pub const IS_VECTOR: u32 = 1 << 6;

    pub fn new(flags: u32, interval: Interval) -> Self {
        Self { flags, interval }
    }

    pub fn empty() -> Self {
        Self {
            flags: 0,
            interval: Interval::BOTTOM,
        } // Start at Bottom for analysis
    }

    pub fn has(&self, flag: u32) -> bool {
        (self.flags & flag) == flag
    }

    pub fn add(&mut self, flag: u32) {
        self.flags |= flag;
        if flag == Self::NON_NEG && self.interval == Interval::BOTTOM {
            // If we just set flag, maybe init interval?
            // But usually interval drives flags.
        }
    }

    pub fn join(&self, other: &Self) -> Self {
        let flags = self.flags & other.flags; // Intersection of flags (Must have)
        let interval = self.interval.join(&other.interval);

        let mut f = Facts { flags, interval };
        // Re-derive flags from interval
        if f.interval.min >= 0 {
            f.add(Self::NON_NEG);
        }
        f
    }
}

pub struct DataflowSolver;

impl Default for DataflowSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl DataflowSolver {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze_function(fn_ir: &FnIR) -> FxHashMap<ValueId, Facts> {
        let mut facts: FxHashMap<ValueId, Facts> = FxHashMap::default();
        let mut worklist: VecDeque<ValueId> = VecDeque::new();

        // 1. Initialize
        for val in &fn_ir.values {
            worklist.push_back(val.id);
            // Seed constants early; others start at Bottom.
            let seed = match &val.kind {
                ValueKind::Const(l) => {
                    let mut f = Facts::empty();
                    f.add(Facts::NON_NA);
                    match l {
                        crate::syntax::ast::Lit::Int(i) => {
                            f.add(Facts::INT_SCALAR);
                            f.interval = Interval::point(*i);
                        }
                        crate::syntax::ast::Lit::Float(v) => {
                            if *v >= 0.0 {
                                f.add(Facts::NON_NEG);
                            }
                            // Interval for float? No, currently integer only.
                            f.interval = Interval::TOP;
                        }
                        crate::syntax::ast::Lit::Bool(_) => {
                            f.add(Facts::BOOL_SCALAR);
                            f.interval = Interval::new(0, 1);
                        }
                        _ => {
                            f.interval = Interval::TOP;
                        }
                    }
                    f
                }
                // All others start at Bottom (Empty)
                _ => Facts::empty(),
            };
            facts.insert(val.id, seed);
        }

        // 2. Iterate
        while let Some(vid) = worklist.pop_front() {
            let val = &fn_ir.values[vid];
            let old_facts = facts.get(&vid).cloned().unwrap_or(Facts::empty());

            let new_facts = Self::transfer(val, &facts, fn_ir);

            if new_facts != old_facts {
                facts.insert(vid, new_facts);
            }
        }

        // Iterative Sweep fallback (since we didn't implement Use-Def chains)
        let mut changed = true;
        while changed {
            changed = false;
            for i in 0..fn_ir.values.len() {
                let vid = i;
                let val = &fn_ir.values[vid];
                let old_facts = facts.get(&vid).cloned().unwrap_or(Facts::empty());
                let new_facts = Self::transfer(val, &facts, fn_ir);

                if new_facts != old_facts {
                    facts.insert(vid, new_facts);
                    changed = true;
                }
            }
        }

        facts
    }

    fn transfer(val: &Value, facts: &FxHashMap<ValueId, Facts>, _fn_ir: &FnIR) -> Facts {
        // Assume Val starts as Empty (Bottom) and we build it up from inputs.
        // Except if inputs are Bottom, result might be Bottom.

        match &val.kind {
            ValueKind::Const(_) => {
                // Constants are static, handled in init. But valid to re-return.
                facts.get(&val.id).cloned().unwrap_or(Facts::empty())
            }

            ValueKind::Phi { args } => {
                // Join all inputs.
                if args.is_empty() {
                    return Facts::empty();
                }

                // Handle empty set issues: if all inputs are Bottom, result is Bottom.
                // Initialize with first arg's facts
                let first_id = args[0].0;
                let mut acc = facts.get(&first_id).cloned().unwrap_or(Facts::empty());

                for (arg_id, _) in args.iter().skip(1) {
                    let f = facts.get(arg_id).cloned().unwrap_or(Facts::empty());
                    acc = acc.join(&f);
                }
                acc
            }

            ValueKind::Binary { op, lhs, rhs } => {
                let lf = facts.get(lhs).cloned().unwrap_or(Facts::empty());
                let rf = facts.get(rhs).cloned().unwrap_or(Facts::empty());

                let mut res = Facts::empty();
                if lf.has(Facts::IS_VECTOR) || rf.has(Facts::IS_VECTOR) {
                    res.add(Facts::IS_VECTOR);
                }

                // Propagate Int Scalar
                if lf.has(Facts::INT_SCALAR) && rf.has(Facts::INT_SCALAR) {
                    res.add(Facts::INT_SCALAR);

                    // Interval Arithmetic
                    let ival = match op {
                        crate::syntax::ast::BinOp::Add => lf.interval.add(&rf.interval),
                        crate::syntax::ast::BinOp::Sub => lf.interval.sub(&rf.interval),
                        crate::syntax::ast::BinOp::Mul => lf.interval.mul(&rf.interval),
                        // Div/etc
                        _ => Interval::TOP,
                    };
                    res.interval = ival;
                } else {
                    res.interval = Interval::TOP;
                }

                // Re-derive
                if res.interval.min >= 0 {
                    res.add(Facts::NON_NEG);
                }

                res
            }

            ValueKind::Unary {
                op: crate::syntax::ast::UnaryOp::Neg,
                rhs,
            } => {
                let rf = facts.get(rhs).cloned().unwrap_or(Facts::empty());
                let mut res = Facts::empty();
                if rf.has(Facts::IS_VECTOR) {
                    res.add(Facts::IS_VECTOR);
                }
                if rf.has(Facts::INT_SCALAR) {
                    res.add(Facts::INT_SCALAR);
                    //Negate interval: [-max, -min]
                    // Carefil with overflow
                    let min = rf.interval.max.saturating_neg();
                    let max = rf.interval.min.saturating_neg();
                    res.interval = Interval::new(min, max); // Note swapped min/max
                }
                res
            }

            ValueKind::Len { .. } => {
                let mut f = Facts::empty();
                f.add(Facts::INT_SCALAR | Facts::NON_NEG | Facts::NON_NA);
                f.interval = Interval::new(0, i64::MAX); // Length is >= 0
                f
            }

            ValueKind::Indices { base: _ } => {
                let mut f = Facts::empty();
                f.add(Facts::INT_SCALAR | Facts::NON_NEG | Facts::NON_NA);
                f.add(Facts::IS_VECTOR);
                // Indices(x) is [0, len(x)-1] in zero-based RR.
                f.interval = Interval::new(0, i64::MAX);
                f
            }

            ValueKind::Range { start, end } => {
                let mut f = Facts::empty();
                f.add(Facts::INT_SCALAR | Facts::IS_VECTOR);
                let sf = facts.get(start).cloned().unwrap_or(Facts::empty());
                let ef = facts.get(end).cloned().unwrap_or(Facts::empty());

                // Interval of elements is [s.min, e.max]
                f.interval = Interval::new(sf.interval.min, ef.interval.max);
                f
            }

            ValueKind::Index1D { .. } => {
                // Result of load.
                // Could be anything.
                Facts::empty()
            }

            _ => Facts::empty(),
        }
    }
}
