use super::{CompileMode, OptLevel, ParallelConfig, fn_emit_cache_salt, stable_hash_bytes};
use crate::error::{InternalCompilerError, Stage};
use crate::typeck::TypeConfig;
use rustc_hash::{FxHashMap, FxHashSet};

pub(crate) fn compile_output_cache_salt() -> u64 {
    let payload = format!(
        "rr-compile-output-cache-salt-v3|{}|{}",
        fn_emit_cache_salt(),
        crate::runtime::R_RUNTIME,
    );
    stable_hash_bytes(payload.as_bytes())
}

#[derive(Clone, Copy)]
pub(crate) struct OutputCacheKeyOptions {
    pub(crate) opt_level: OptLevel,
    pub(crate) direct_builtin_call_map: bool,
    pub(crate) preserve_all_defs: bool,
    pub(crate) compile_mode: CompileMode,
}

pub(crate) fn peephole_output_cache_key(
    raw_output: &str,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
    options: OutputCacheKeyOptions,
) -> String {
    let mut pure_names = pure_user_calls.iter().cloned().collect::<Vec<_>>();
    pure_names.sort();
    let mut fresh_names = fresh_user_calls.iter().cloned().collect::<Vec<_>>();
    fresh_names.sort();
    let payload = format!(
        "rr-peephole-v2|{}|{}|{}|{}|{}|{:?}|{:?}|{}",
        compile_output_cache_salt(),
        options.opt_level.label(),
        options.direct_builtin_call_map,
        options.preserve_all_defs,
        options.compile_mode.as_str(),
        pure_names,
        fresh_names,
        raw_output,
    );
    format!("{:016x}", stable_hash_bytes(payload.as_bytes()))
}

pub(crate) fn optimized_fragment_output_cache_key(
    emitted_fragment: &str,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
    options: OutputCacheKeyOptions,
) -> String {
    let mut pure_names = pure_user_calls.iter().cloned().collect::<Vec<_>>();
    pure_names.sort();
    let mut fresh_names = fresh_user_calls.iter().cloned().collect::<Vec<_>>();
    fresh_names.sort();
    let payload = format!(
        "rr-opt-frag-v2|{}|{}|{}|{}|{}|{:?}|{:?}|{}",
        compile_output_cache_salt(),
        options.opt_level.label(),
        options.direct_builtin_call_map,
        options.preserve_all_defs,
        options.compile_mode.as_str(),
        pure_names,
        fresh_names,
        emitted_fragment,
    );
    format!("{:016x}", stable_hash_bytes(payload.as_bytes()))
}

pub(crate) fn optimized_assembly_cache_key(
    raw_output: &str,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
    options: OutputCacheKeyOptions,
) -> String {
    let mut pure_names = pure_user_calls.iter().cloned().collect::<Vec<_>>();
    pure_names.sort();
    let mut fresh_names = fresh_user_calls.iter().cloned().collect::<Vec<_>>();
    fresh_names.sort();
    let payload = format!(
        "rr-opt-asm-v2|{}|{}|{}|{}|{}|{:?}|{:?}|{}",
        compile_output_cache_salt(),
        options.opt_level.label(),
        options.direct_builtin_call_map,
        options.preserve_all_defs,
        options.compile_mode.as_str(),
        pure_names,
        fresh_names,
        raw_output,
    );
    format!("{:016x}", stable_hash_bytes(payload.as_bytes()))
}

pub(crate) fn raw_rewrite_output_cache_key(
    raw_output: &str,
    opt_level: OptLevel,
    pure_user_calls: &FxHashSet<String>,
    preserve_all_defs: bool,
    compile_mode: CompileMode,
) -> String {
    let mut pure_names = pure_user_calls.iter().cloned().collect::<Vec<_>>();
    pure_names.sort();
    let payload = format!(
        "rr-raw-rewrite-v2|{}|{}|{}|{}|{:?}|{}",
        compile_output_cache_salt(),
        opt_level.label(),
        preserve_all_defs,
        compile_mode.as_str(),
        pure_names,
        raw_output,
    );
    format!("{:016x}", stable_hash_bytes(payload.as_bytes()))
}

pub(crate) fn fn_emit_cache_key(
    fn_ir: &crate::mir::def::FnIR,
    opt_level: OptLevel,
    type_cfg: TypeConfig,
    parallel_cfg: ParallelConfig,
    compile_mode: CompileMode,
    seq_len_param_end_slots: Option<&FxHashMap<usize, usize>>,
) -> String {
    let mut seq_len_summary: Vec<(usize, usize)> = seq_len_param_end_slots
        .map(|slots| slots.iter().map(|(param, end)| (*param, *end)).collect())
        .unwrap_or_default();
    seq_len_summary.sort_unstable();
    let payload = format!(
        "rr-fn-emit-v4|{}|{}|{}|{}|{}|{}|{}|{}|{}|{:?}|{:?}",
        fn_ir.name,
        opt_level.label(),
        type_cfg.mode.as_str(),
        type_cfg.native_backend.as_str(),
        parallel_cfg.mode.as_str(),
        parallel_cfg.backend.as_str(),
        parallel_cfg.threads,
        compile_mode.as_str(),
        fn_emit_cache_salt(),
        fn_ir,
        seq_len_summary,
    );
    format!("{:016x}", stable_hash_bytes(payload.as_bytes()))
}

pub(crate) struct SourceAnalysisOutput {
    pub(crate) desugared_hir: crate::hir::def::HirProgram,
    pub(crate) global_symbols: FxHashMap<crate::hir::def::SymbolId, String>,
}

pub(crate) type FnSlot = usize;

pub(crate) struct FnUnit {
    pub(crate) name: String,
    pub(crate) ir: Option<crate::mir::def::FnIR>,
    pub(crate) is_public: bool,
    pub(crate) is_top_level: bool,
}

pub(crate) struct ProgramIR {
    pub(crate) fns: Vec<FnUnit>,
    pub(crate) by_name: FxHashMap<String, FnSlot>,
    pub(crate) emit_order: Vec<FnSlot>,
    pub(crate) emit_roots: Vec<FnSlot>,
    pub(crate) top_level_calls: Vec<FnSlot>,
}

impl ProgramIR {
    pub(crate) fn from_parts(
        mut all_fns: FxHashMap<String, crate::mir::def::FnIR>,
        emit_order_names: Vec<String>,
        emit_root_names: Vec<String>,
        top_level_call_names: Vec<String>,
        meta_by_name: FxHashMap<String, (bool, bool)>,
    ) -> crate::error::RR<Self> {
        let mut fns = Vec::new();
        let mut by_name = FxHashMap::default();
        for name in &emit_order_names {
            let Some(fn_ir) = all_fns.remove(name.as_str()) else {
                return Err(InternalCompilerError::new(
                    Stage::Mir,
                    format!("ProgramIR IR missing function '{}'", name),
                )
                .into_exception());
            };
            let Some((is_public, is_top_level)) = meta_by_name.get(name).copied() else {
                return Err(InternalCompilerError::new(
                    Stage::Mir,
                    format!("ProgramIR metadata missing function '{}'", name),
                )
                .into_exception());
            };
            let slot = fns.len();
            fns.push(FnUnit {
                name: name.clone(),
                ir: Some(fn_ir),
                is_public,
                is_top_level,
            });
            by_name.insert(name.clone(), slot);
        }
        let mut remaining_names: Vec<String> = all_fns
            .keys()
            .filter(|name| !by_name.contains_key(*name))
            .cloned()
            .collect();
        remaining_names.sort();
        for name in remaining_names {
            let Some(fn_ir) = all_fns.remove(name.as_str()) else {
                return Err(InternalCompilerError::new(
                    Stage::Mir,
                    format!("ProgramIR IR missing late-bound function '{}'", name),
                )
                .into_exception());
            };
            let (is_public, is_top_level) = meta_by_name
                .get(&name)
                .copied()
                .unwrap_or((false, name.starts_with("Sym_top_")));
            let slot = fns.len();
            fns.push(FnUnit {
                name: name.clone(),
                ir: Some(fn_ir),
                is_public,
                is_top_level,
            });
            by_name.insert(name, slot);
        }

        let map_slots = |names: Vec<String>,
                         label: &str,
                         by_name: &FxHashMap<String, FnSlot>|
         -> crate::error::RR<Vec<FnSlot>> {
            names
                .into_iter()
                .map(|name| {
                    by_name.get(&name).copied().ok_or_else(|| {
                        InternalCompilerError::new(
                            Stage::Mir,
                            format!("ProgramIR {} references missing function '{}'", label, name),
                        )
                        .into_exception()
                    })
                })
                .collect()
        };
        let emit_order = map_slots(emit_order_names, "emit order", &by_name)?;
        let emit_roots = map_slots(emit_root_names, "emit roots", &by_name)?;
        let top_level_calls = map_slots(top_level_call_names, "top-level call list", &by_name)?;

        Ok(Self {
            fns,
            by_name,
            emit_order,
            emit_roots,
            top_level_calls,
        })
    }

    pub(crate) fn names_for_slots(&self, slots: &[FnSlot]) -> Vec<String> {
        slots
            .iter()
            .filter_map(|slot| self.fns.get(*slot))
            .map(|unit| unit.name.clone())
            .collect()
    }

    pub(crate) fn emit_order_names(&self) -> Vec<String> {
        self.names_for_slots(&self.emit_order)
    }

    pub(crate) fn emit_root_names(&self) -> Vec<String> {
        self.names_for_slots(&self.emit_roots)
    }

    pub(crate) fn top_level_call_names(&self) -> Vec<String> {
        self.names_for_slots(&self.top_level_calls)
    }

    pub(crate) fn get(&self, name: &str) -> Option<&crate::mir::def::FnIR> {
        let slot = self.by_name.get(name).copied()?;
        self.get_slot(slot)
    }

    pub(crate) fn get_slot(&self, slot: FnSlot) -> Option<&crate::mir::def::FnIR> {
        self.fns.get(slot)?.ir.as_ref()
    }

    pub(crate) fn contains_name(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    pub(crate) fn all_slots(&self) -> impl Iterator<Item = FnSlot> + '_ {
        0..self.fns.len()
    }

    pub(crate) fn take_all_fns_map(
        &mut self,
    ) -> crate::error::RR<FxHashMap<String, crate::mir::def::FnIR>> {
        let mut out = FxHashMap::default();
        for unit in &mut self.fns {
            let Some(fn_ir) = unit.ir.take() else {
                return Err(InternalCompilerError::new(
                    Stage::Mir,
                    format!("ProgramIR function '{}' already taken", unit.name),
                )
                .into_exception());
            };
            out.insert(unit.name.clone(), fn_ir);
        }
        Ok(out)
    }

    pub(crate) fn restore_all_fns_map(
        &mut self,
        mut all_fns: FxHashMap<String, crate::mir::def::FnIR>,
    ) -> crate::error::RR<()> {
        for unit in &mut self.fns {
            let Some(fn_ir) = all_fns.remove(unit.name.as_str()) else {
                return Err(InternalCompilerError::new(
                    Stage::Mir,
                    format!("ProgramIR restore missing function '{}'", unit.name),
                )
                .into_exception());
            };
            unit.ir = Some(fn_ir);
        }
        if !all_fns.is_empty() {
            let mut leftovers: Vec<String> = all_fns.keys().cloned().collect();
            leftovers.sort();
            for name in leftovers {
                let Some(fn_ir) = all_fns.remove(name.as_str()) else {
                    return Err(InternalCompilerError::new(
                        Stage::Mir,
                        format!("ProgramIR restore missing late-bound function '{}'", name),
                    )
                    .into_exception());
                };
                let slot = self.fns.len();
                self.fns.push(FnUnit {
                    name: name.clone(),
                    ir: Some(fn_ir),
                    is_public: false,
                    is_top_level: name.starts_with("Sym_top_"),
                });
                self.by_name.insert(name, slot);
                self.emit_order.push(slot);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests.rs"]
pub(crate) mod tests;
