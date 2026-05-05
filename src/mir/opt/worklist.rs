use super::*;

#[derive(Debug, Default)]
pub(crate) struct ValueWorklist {
    stack: Vec<ValueId>,
    seen: FxHashSet<ValueId>,
}

impl ValueWorklist {
    pub(crate) fn seeded(root: ValueId) -> Self {
        let mut worklist = Self::default();
        worklist.push(root);
        worklist
    }

    pub(crate) fn push(&mut self, value: ValueId) {
        if self.seen.insert(value) {
            self.stack.push(value);
        }
    }

    pub(crate) fn pop(&mut self) -> Option<ValueId> {
        self.stack.pop()
    }
}

pub(crate) fn collect_value_dependencies_iterative(
    fn_ir: &FnIR,
    root: ValueId,
) -> FxHashSet<ValueId> {
    let mut out = FxHashSet::default();
    let mut worklist = ValueWorklist::seeded(root);
    while let Some(value) = worklist.pop() {
        if !out.insert(value) {
            continue;
        }
        let Some(row) = fn_ir.values.get(value) else {
            continue;
        };
        for dep in value_dependencies(&row.kind) {
            worklist.push(dep);
        }
    }
    out
}
