use super::*;
use crate::utils::Span;
use rustc_hash::FxHashMap;

pub struct MirInliner {
    pub(crate) policy: InlinePolicy,
}
pub(crate) type InlineCall = (String, Vec<ValueId>, ValueId, Option<VarId>, Span);

#[derive(Default)]
pub(crate) struct InlineMap {
    pub(crate) v: FxHashMap<ValueId, ValueId>,
    pub(crate) b: FxHashMap<BlockId, BlockId>,
    pub(crate) vars: FxHashMap<VarId, VarId>,
    pub(crate) inline_tag: String,
}

pub(crate) fn copy_cloned_value_metadata(caller: &mut FnIR, new_id: ValueId, source: &Value) {
    let cloned = &mut caller.values[new_id];
    cloned.value_ty = source.value_ty;
    cloned.value_term = source.value_term.clone();
    cloned.escape = source.escape;
}

impl InlineMap {
    pub(crate) fn map_var(&mut self, old: &VarId) -> VarId {
        if let Some(mapped) = self.vars.get(old) {
            return mapped.clone();
        }
        let new_name = format!("inlined_{}_{}", self.inline_tag, old);
        self.vars.insert(old.clone(), new_name.clone());
        new_name
    }
}

impl Default for MirInliner {
    fn default() -> Self {
        Self::new()
    }
}
