#![allow(dead_code)]

use rr::codegen::mir_emit::MirEmitter;
use rr::hir::def::{HirItem, HirProgram, ModuleId};
use rr::hir::desugar::Desugarer;
use rr::hir::lower::Lowerer;
use rr::mir::lower_hir::MirLowerer;
use rr::mir::opt::TachyonEngine;
use rr::mir::{self, FnIR};
use rr::syntax::parse::Parser;
use rr::typeck::TypeConfig;
use rr::typeck::solver::analyze_program;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

pub const MAX_INPUT_BYTES: usize = 16 * 1024;

pub fn decode_source(data: &[u8]) -> Option<&str> {
    if data.is_empty() || data.len() > MAX_INPUT_BYTES {
        return None;
    }
    std::str::from_utf8(data).ok()
}

pub fn source_variants(src: &str) -> Vec<String> {
    let mut out = Vec::with_capacity(4);
    out.push(src.to_string());
    if src.contains("\r\n") {
        out.push(src.replace("\r\n", "\n"));
    }
    if !src.ends_with('\n') {
        out.push(format!("{src}\n"));
    }
    out.push(format!("fn __fuzz_entry() {{\n{}\n}}\n", src));

    let mut seen = FxHashSet::default();
    out.retain(|s| seen.insert(s.clone()));
    out
}

pub fn stable_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

pub fn temp_case_root(namespace: &str, seed: u64) -> PathBuf {
    std::env::temp_dir()
        .join("rr-fuzz")
        .join(namespace)
        .join(format!("{seed:016x}"))
}

pub struct ScopedEnvVar {
    key: &'static str,
    prev: Option<String>,
}

impl ScopedEnvVar {
    pub fn set(key: &'static str, value: Option<&str>) -> Self {
        let prev = std::env::var(key).ok();
        // SAFETY: libFuzzer runs each target in a single process/threaded loop
        // here, and these guards restore the previous environment before exit.
        unsafe {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        Self { key, prev }
    }
}

impl Drop for ScopedEnvVar {
    fn drop(&mut self) {
        // SAFETY: restores the previous process environment for the same
        // single-threaded fuzz execution described above.
        unsafe {
            match &self.prev {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }
}

pub fn build_mir(src: &str) -> Option<FxHashMap<String, FnIR>> {
    let mut parser = Parser::new(src);
    let ast = parser.parse_program().ok()?;

    let mut lowerer = Lowerer::new();
    let hir_mod = lowerer.lower_module(ast, ModuleId(0)).ok()?;
    let symbols = lowerer.into_symbols();

    let mut known_fn_arities: FxHashMap<String, usize> = FxHashMap::default();
    for item in &hir_mod.items {
        if let HirItem::Fn(f) = item
            && let Some(name) = symbols.get(&f.name).cloned()
        {
            known_fn_arities.insert(name, f.params.len());
        }
    }

    let mut desugarer = Desugarer::new();
    let desugared = desugarer
        .desugar_program(HirProgram {
            modules: vec![hir_mod],
        })
        .ok()?;

    let mut all_fns = FxHashMap::default();
    for module in desugared.modules {
        let mut top_level_stmts = Vec::new();
        for item in module.items {
            match item {
                HirItem::Fn(f) => {
                    let fn_name = format!("Sym_{}", f.name.0);
                    let params: Vec<String> = f
                        .params
                        .iter()
                        .filter_map(|p| symbols.get(&p.name).cloned())
                        .collect();
                    if params.len() != f.params.len() {
                        continue;
                    }
                    let var_names = f.local_names.clone().into_iter().collect();
                    let mir_lowerer = MirLowerer::new(
                        fn_name.clone(),
                        params,
                        var_names,
                        &symbols,
                        &known_fn_arities,
                    );
                    if let Ok(fn_ir) = mir_lowerer.lower_fn(f) {
                        all_fns.insert(fn_name, fn_ir);
                    }
                }
                HirItem::Stmt(s) => top_level_stmts.push(s),
                _ => {}
            }
        }
        if !top_level_stmts.is_empty() {
            let top_name = format!("Sym_top_{}", module.id.0);
            let top_fn = rr::hir::def::HirFn {
                id: rr::hir::def::FnId(1_000_000 + module.id.0),
                name: rr::hir::def::SymbolId(1_000_000 + module.id.0),
                params: Vec::new(),
                type_params: Vec::new(),
                where_bounds: Vec::new(),
                has_varargs: false,
                ret_ty: None,
                ret_ty_inferred: false,
                body: rr::hir::def::HirBlock {
                    stmts: top_level_stmts,
                    span: rr::utils::Span::default(),
                },
                attrs: rr::hir::def::HirFnAttrs {
                    inline_hint: rr::hir::def::InlineHint::Never,
                    tidy_safe: false,
                },
                span: rr::utils::Span::default(),
                local_names: FxHashMap::default(),
                public: false,
            };
            let mir_lowerer = MirLowerer::new(
                top_name.clone(),
                Vec::new(),
                FxHashMap::default(),
                &symbols,
                &known_fn_arities,
            );
            if let Ok(fn_ir) = mir_lowerer.lower_fn(top_fn) {
                all_fns.insert(top_name, fn_ir);
            }
        }
    }

    Some(all_fns)
}

fn fn_ir_size(fn_ir: &FnIR) -> usize {
    let instrs: usize = fn_ir.blocks.iter().map(|b| b.instrs.len()).sum();
    fn_ir.values.len() + instrs
}

pub fn assert_emittable_or_rejected(all_fns: &FxHashMap<String, FnIR>) {
    let mut names: Vec<String> = all_fns.keys().cloned().collect();
    names.sort();
    for name in names {
        let fn_ir = all_fns.get(&name).expect("function key must exist");
        match mir::verify::verify_ir(fn_ir) {
            Ok(()) => {
                let mut emitter = MirEmitter::new();
                let _ = emitter.emit(fn_ir).unwrap_or_else(|e| {
                    panic!("codegen failed for verified function {}: {:?}", name, e)
                });
            }
            Err(err) => {
                assert!(
                    fn_ir.unsupported_dynamic,
                    "invalid MIR must be marked unsupported_dynamic: {} ({})",
                    name, err
                );
            }
        }
    }
}

pub fn emit_snapshot_for_verified(all_fns: &FxHashMap<String, FnIR>) -> Vec<(String, String)> {
    let mut names: Vec<String> = all_fns.keys().cloned().collect();
    names.sort();
    let mut out = Vec::new();
    for name in names {
        let fn_ir = all_fns.get(&name).expect("function key must exist");
        match mir::verify::verify_ir(fn_ir) {
            Ok(()) => {
                let mut emitter = MirEmitter::new();
                let (code, _map) = emitter
                    .emit(fn_ir)
                    .unwrap_or_else(|e| panic!("codegen snapshot failed for {}: {:?}", name, e));
                out.push((name, code));
            }
            Err(_) => {
                assert!(
                    fn_ir.unsupported_dynamic,
                    "invalid MIR must be marked unsupported_dynamic for snapshot"
                );
            }
        }
    }
    out
}

pub fn run_full_pipeline(all_fns: &FxHashMap<String, FnIR>, cfg: TypeConfig) {
    // Type analysis path (strict/gradual), including expected error surfaces.
    let mut typed = all_fns.clone();
    let _ = analyze_program(&mut typed, cfg);
    let _ = mir::semantics::validate_program(&typed);
    let _ = mir::semantics::validate_runtime_safety(&typed);

    // Full optimization path.
    let mut optimized = typed.clone();
    TachyonEngine::new().run_program_with_stats(&mut optimized);
    let _ = mir::semantics::validate_program(&optimized);
    let _ = mir::semantics::validate_runtime_safety(&optimized);
    assert_emittable_or_rejected(&optimized);

    // O0 stabilization path.
    let mut stabilized = typed.clone();
    TachyonEngine::new().stabilize_for_codegen(&mut stabilized);
    let _ = mir::semantics::validate_program(&stabilized);
    let _ = mir::semantics::validate_runtime_safety(&stabilized);
    assert_emittable_or_rejected(&stabilized);

    // Determinism check on manageable cases: same input + same config => same emitted output.
    let total_ir: usize = typed.values().map(fn_ir_size).sum();
    if total_ir <= 700 {
        let mut run_a = typed.clone();
        let mut run_b = typed;
        let engine = TachyonEngine::new();
        engine.run_program_with_stats(&mut run_a);
        engine.run_program_with_stats(&mut run_b);
        let snap_a = emit_snapshot_for_verified(&run_a);
        let snap_b = emit_snapshot_for_verified(&run_b);
        assert_eq!(snap_a, snap_b, "non-deterministic optimizer/codegen output");
    }
}
