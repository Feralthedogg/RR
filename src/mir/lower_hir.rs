//! HIR-to-MIR lowering with sealed-SSA construction.
//!
//! This module turns structured HIR into the MIR form consumed by validation,
//! optimization, and codegen while preserving user-visible control/data flow.

use crate::error::{InternalCompilerError, RR, RRException, Stage};
use crate::hir::def as hir;
use crate::mir::flow::Facts;
use crate::mir::*;
use crate::syntax::ast::{BinOp, Lit};
use crate::typeck::solver::{hir_ty_to_type_state, hir_ty_to_type_term_with_symbols};
use crate::utils::{Span, did_you_mean};
use rustc_hash::{FxHashMap, FxHashSet};

#[path = "lower_hir/loops.rs"]
mod loops;
#[path = "lower_hir/matching.rs"]
mod matching;

#[derive(Clone, Copy)]
struct LoopTargets {
    break_bb: BlockId,
    continue_bb: BlockId,
    continue_step: Option<(hir::LocalId, ValueId)>,
}
pub struct MirLowerer<'a> {
    fn_ir: FnIR,

    // SSA construction state.
    curr_block: BlockId,

    // Current definitions per block (sealed SSA construction).
    defs: FxHashMap<BlockId, FxHashMap<hir::LocalId, ValueId>>,

    // Deferred phi operands for unsealed blocks.
    incomplete_phis: FxHashMap<BlockId, Vec<(hir::LocalId, ValueId)>>,
    sealed_blocks: FxHashSet<BlockId>,
    // Predecessor map for SSA reads.
    preds: FxHashMap<BlockId, Vec<BlockId>>,

    // Name mapping for codegen.
    var_names: FxHashMap<hir::LocalId, String>,

    // Symbol table (borrowed from caller).
    symbols: &'a FxHashMap<hir::SymbolId, String>,
    known_functions: &'a FxHashMap<String, usize>,
    loop_stack: Vec<LoopTargets>,
    tidy_mask_depth: usize,
}

impl<'a> MirLowerer<'a> {
    // Most math/data builtins are treated as intrinsics by later passes.
    // Only the small scalar-indexing group is allowed to shadow base R names.
    fn allow_user_builtin_shadowing(name: &str) -> bool {
        matches!(name, "length" | "floor" | "round" | "ceiling" | "trunc")
    }

    fn function_name_suggestion_candidates() -> &'static [&'static str] {
        &[
            "length",
            "seq_along",
            "seq_len",
            "c",
            "list",
            "sum",
            "mean",
            "var",
            "prod",
            "min",
            "max",
            "abs",
            "sqrt",
            "sin",
            "cos",
            "tan",
            "asin",
            "acos",
            "atan",
            "atan2",
            "sinh",
            "cosh",
            "tanh",
            "log",
            "log10",
            "log2",
            "exp",
            "sign",
            "gamma",
            "lgamma",
            "floor",
            "ceiling",
            "trunc",
            "round",
            "pmax",
            "pmin",
            "print",
            "paste",
            "paste0",
            "sprintf",
            "cat",
            "names",
            "rownames",
            "colnames",
            "sort",
            "order",
            "match",
            "unique",
            "duplicated",
            "anyDuplicated",
            "any",
            "all",
            "which",
            "is.na",
            "is.finite",
            "numeric",
            "character",
            "logical",
            "integer",
            "double",
            "rep",
            "rep.int",
            "vector",
            "matrix",
            "dim",
            "dimnames",
            "nrow",
            "ncol",
            "colSums",
            "rowSums",
            "crossprod",
            "tcrossprod",
            "t",
            "diag",
            "rbind",
            "cbind",
            "library",
            "require",
            "plot",
            "lines",
            "legend",
            "png",
            "dev.off",
            "eval",
            "parse",
            "get",
            "assign",
            "exists",
            "mget",
            "rm",
            "ls",
            "parent.frame",
            "environment",
            "sys.frame",
            "sys.call",
            "do.call",
        ]
    }

    fn suggest_function_name(&self, name: &str) -> Option<String> {
        did_you_mean(
            name,
            self.known_functions.keys().cloned().chain(
                Self::function_name_suggestion_candidates()
                    .iter()
                    .map(|name| (*name).to_string()),
            ),
        )
    }

    fn render_default_lit(lit: &hir::HirLit) -> String {
        match lit {
            hir::HirLit::Int(i) => format!("{i}L"),
            hir::HirLit::Double(f) => {
                let mut rendered = f.to_string();
                if f.is_finite() && !rendered.contains(['.', 'e', 'E']) {
                    rendered.push_str(".0");
                }
                rendered
            }
            hir::HirLit::Char(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
            hir::HirLit::Bool(true) => "TRUE".to_string(),
            hir::HirLit::Bool(false) => "FALSE".to_string(),
            hir::HirLit::NA => "NA".to_string(),
            hir::HirLit::Null => "NULL".to_string(),
        }
    }

    fn render_default_unop(op: &hir::HirUnOp) -> &'static str {
        match op {
            hir::HirUnOp::Not => "!",
            hir::HirUnOp::Neg => "-",
        }
    }

    fn render_default_binop(op: &hir::HirBinOp) -> &'static str {
        match op {
            hir::HirBinOp::Add => "+",
            hir::HirBinOp::Sub => "-",
            hir::HirBinOp::Mul => "*",
            hir::HirBinOp::Div => "/",
            hir::HirBinOp::Mod => "%%",
            hir::HirBinOp::MatMul => "%*%",
            hir::HirBinOp::And => "&",
            hir::HirBinOp::Or => "|",
            hir::HirBinOp::Eq => "==",
            hir::HirBinOp::Ne => "!=",
            hir::HirBinOp::Lt => "<",
            hir::HirBinOp::Le => "<=",
            hir::HirBinOp::Gt => ">",
            hir::HirBinOp::Ge => ">=",
        }
    }

    fn render_default_arg(&self, arg: &hir::HirArg) -> RR<String> {
        match arg {
            hir::HirArg::Pos(expr) => self.render_default_expr(expr),
            hir::HirArg::Named { name, value } => {
                let rendered_name = self
                    .symbols
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| format!("arg_{}", name.0));
                Ok(format!(
                    "{rendered_name} = {}",
                    self.render_default_expr(value)?
                ))
            }
        }
    }

    fn render_default_expr(&self, expr: &hir::HirExpr) -> RR<String> {
        match expr {
            hir::HirExpr::Local(local) => self.var_names.get(local).cloned().ok_or_else(|| {
                InternalCompilerError::new(
                    Stage::Mir,
                    format!(
                        "missing local name while rendering default argument: {:?}",
                        local
                    ),
                )
                .into_exception()
            }),
            hir::HirExpr::Global(sym, _) => Ok(self
                .symbols
                .get(sym)
                .cloned()
                .unwrap_or_else(|| format!("Sym_{}", sym.0))),
            hir::HirExpr::Lit(lit) => Ok(Self::render_default_lit(lit)),
            hir::HirExpr::Unary { op, expr } => Ok(format!(
                "{}({})",
                Self::render_default_unop(op),
                self.render_default_expr(expr)?
            )),
            hir::HirExpr::Binary { op, lhs, rhs } => Ok(format!(
                "({} {} {})",
                self.render_default_expr(lhs)?,
                Self::render_default_binop(op),
                self.render_default_expr(rhs)?
            )),
            hir::HirExpr::Call(call) => {
                let callee = self.render_default_expr(call.callee.as_ref())?;
                let args = call
                    .args
                    .iter()
                    .map(|arg| self.render_default_arg(arg))
                    .collect::<RR<Vec<_>>>()?;
                Ok(format!("{callee}({})", args.join(", ")))
            }
            hir::HirExpr::Index { base, index } => {
                let base = self.render_default_expr(base)?;
                let index = index
                    .iter()
                    .map(|expr| self.render_default_expr(expr))
                    .collect::<RR<Vec<_>>>()?;
                Ok(format!("{base}[{}]", index.join(", ")))
            }
            hir::HirExpr::Field { base, name } => {
                let base = self.render_default_expr(base)?;
                let name = self
                    .symbols
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| format!("field_{}", name.0));
                Ok(format!("{base}[[\"{name}\"]]"))
            }
            hir::HirExpr::IfExpr {
                cond,
                then_expr,
                else_expr,
            } => Ok(format!(
                "if ({}) {} else {}",
                self.render_default_expr(cond)?,
                self.render_default_expr(then_expr)?,
                self.render_default_expr(else_expr)?
            )),
            hir::HirExpr::ListLit(fields) => {
                let rendered = fields
                    .iter()
                    .map(|(name, value)| {
                        let rendered_name = self
                            .symbols
                            .get(name)
                            .cloned()
                            .unwrap_or_else(|| format!("field_{}", name.0));
                        Ok(format!(
                            "{rendered_name} = {}",
                            self.render_default_expr(value)?
                        ))
                    })
                    .collect::<RR<Vec<_>>>()?;
                Ok(format!("list({})", rendered.join(", ")))
            }
            hir::HirExpr::VectorLit(values) => {
                let rendered = values
                    .iter()
                    .map(|value| self.render_default_expr(value))
                    .collect::<RR<Vec<_>>>()?;
                Ok(format!("c({})", rendered.join(", ")))
            }
            hir::HirExpr::Range { start, end } => Ok(format!(
                "{}:{}",
                self.render_default_expr(start)?,
                self.render_default_expr(end)?
            )),
            unsupported => Err(RRException::new(
                "RR.SemanticError",
                crate::error::RRCode::E1002,
                crate::error::Stage::Mir,
                format!(
                    "default argument expression is not yet supported in MIR lowering: {:?}",
                    unsupported
                ),
            )),
        }
    }

    pub fn new(
        name: String,
        params: Vec<String>,
        var_names: FxHashMap<hir::LocalId, String>,
        symbols: &'a FxHashMap<hir::SymbolId, String>,
        known_functions: &'a FxHashMap<String, usize>,
    ) -> Self {
        let mut fn_ir = FnIR::new(name, params.clone());
        let entry = fn_ir.add_block();
        let body_head = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = body_head;

        // Init defs map for entry
        let mut defs = FxHashMap::default();
        defs.insert(entry, FxHashMap::default());
        defs.insert(body_head, FxHashMap::default());

        Self {
            fn_ir,
            curr_block: entry,
            defs,
            incomplete_phis: FxHashMap::default(),
            sealed_blocks: FxHashSet::default(),
            preds: FxHashMap::default(),
            var_names,
            symbols,
            known_functions,
            loop_stack: Vec::new(),
            tidy_mask_depth: 0,
        }
    }

    fn with_tidy_mask<T>(&mut self, f: impl FnOnce(&mut Self) -> RR<T>) -> RR<T> {
        self.tidy_mask_depth += 1;
        let out = f(self);
        self.tidy_mask_depth -= 1;
        out
    }

    fn in_tidy_mask(&self) -> bool {
        self.tidy_mask_depth > 0
    }

    // Core Helpers
    fn add_pred(&mut self, target: BlockId, pred: BlockId) {
        self.preds.entry(target).or_default().push(pred);
    }

    // Standardize Value Addition
    fn add_value(&mut self, kind: ValueKind, span: Span) -> ValueId {
        let vid = self.fn_ir.add_value(kind, span, Facts::empty(), None);
        self.annotate_new_value(vid);
        vid
    }

    fn add_value_with_name(
        &mut self,
        kind: ValueKind,
        span: Span,
        var_name: Option<String>,
    ) -> ValueId {
        let vid = self.fn_ir.add_value(kind, span, Facts::empty(), var_name);
        self.annotate_new_value(vid);
        vid
    }

    fn annotate_new_value(&mut self, vid: ValueId) {
        match &self.fn_ir.values[vid].kind {
            ValueKind::Call { callee, .. } => {
                if let Some(kind) = builtin_kind_for_name(callee) {
                    self.fn_ir
                        .set_call_semantics(vid, CallSemantics::Builtin(kind));
                    match kind {
                        BuiltinKind::SeqAlong
                        | BuiltinKind::SeqLen
                        | BuiltinKind::C
                        | BuiltinKind::Numeric
                        | BuiltinKind::Character
                        | BuiltinKind::Logical
                        | BuiltinKind::Integer
                        | BuiltinKind::Double
                        | BuiltinKind::Rep
                        | BuiltinKind::RepInt
                        | BuiltinKind::Vector => {
                            self.fn_ir
                                .set_memory_layout_hint(vid, MemoryLayoutHint::Dense1D);
                        }
                        BuiltinKind::Matrix
                        | BuiltinKind::Transpose
                        | BuiltinKind::Diag
                        | BuiltinKind::Rbind
                        | BuiltinKind::Cbind
                        | BuiltinKind::Crossprod
                        | BuiltinKind::Tcrossprod => {
                            self.fn_ir
                                .set_memory_layout_hint(vid, MemoryLayoutHint::ColumnMajor2D);
                        }
                        BuiltinKind::Array => {
                            self.fn_ir
                                .set_memory_layout_hint(vid, MemoryLayoutHint::ColumnMajorND);
                        }
                        _ => {}
                    }
                } else if callee == "rr_call_closure" {
                    self.fn_ir
                        .set_call_semantics(vid, CallSemantics::ClosureDispatch);
                } else if callee.starts_with("rr_") {
                    self.fn_ir
                        .set_call_semantics(vid, CallSemantics::RuntimeHelper);
                } else {
                    self.fn_ir
                        .set_call_semantics(vid, CallSemantics::UserDefined);
                }
            }
            ValueKind::Len { .. } | ValueKind::Range { .. } | ValueKind::Indices { .. } => {
                self.fn_ir
                    .set_memory_layout_hint(vid, MemoryLayoutHint::Dense1D);
            }
            _ => {}
        }
    }

    // Core Helpers
    fn define_var_at(
        &mut self,
        block: BlockId,
        var: hir::LocalId,
        val: ValueId,
        emit_assign: bool,
    ) {
        let name = self.var_names.get(&var).cloned();
        if let Some(n) = name {
            if emit_assign {
                self.fn_ir.blocks[block].instrs.push(Instr::Assign {
                    dst: n.clone(),
                    src: val,
                    span: Span::default(),
                });
                let mismatched_origin = self
                    .fn_ir
                    .values
                    .get(val)
                    .and_then(|v| v.origin_var.as_ref())
                    .map(|orig| orig != &n)
                    .unwrap_or(false);
                let is_phi_value = self
                    .fn_ir
                    .values
                    .get(val)
                    .map(|v| matches!(v.kind, ValueKind::Phi { .. }))
                    .unwrap_or(false);

                let def_val = if mismatched_origin || is_phi_value {
                    self.add_value_with_name(
                        ValueKind::Load { var: n.clone() },
                        Span::default(),
                        Some(n),
                    )
                } else {
                    if let Some(v) = self.fn_ir.values.get_mut(val)
                        && v.origin_var.is_none()
                    {
                        v.origin_var = Some(n);
                    }
                    val
                };
                self.defs.entry(block).or_default().insert(var, def_val);
                return;
            }

            if let Some(v) = self.fn_ir.values.get_mut(val)
                && v.origin_var.is_none()
            {
                v.origin_var = Some(n);
            }
        }

        self.defs.entry(block).or_default().insert(var, val);
    }

    fn write_var(&mut self, var: hir::LocalId, val: ValueId) {
        self.define_var_at(self.curr_block, var, val, true);
    }

    fn read_var(&mut self, var: hir::LocalId, block: BlockId) -> RR<ValueId> {
        if let Some(m) = self.defs.get(&block)
            && let Some(&v) = m.get(&var)
        {
            return Ok(v);
        }
        // Not found in local, look in predecessors
        self.read_var_recursive(var, block)
    }

    // Sealed Block SSA Construction (Braun et al.)

    fn seal_block(&mut self, block: BlockId) -> RR<()> {
        if self.sealed_blocks.contains(&block) {
            return Ok(());
        }

        // Resolve incomplete Phis
        if let Some(incomplete) = self.incomplete_phis.remove(&block) {
            for (var, phi_val) in incomplete {
                self.add_phi_operands(block, var, phi_val)?;
            }
        }

        self.sealed_blocks.insert(block);
        Ok(())
    }

    fn read_var_recursive(&mut self, var: hir::LocalId, block: BlockId) -> RR<ValueId> {
        if !self.sealed_blocks.contains(&block) {
            // Create a placeholder phi and resolve operands when the block is sealed.
            let phi = self.add_phi_placeholder(block, Span::default());
            self.incomplete_phis
                .entry(block)
                .or_default()
                .push((var, phi));
            // Define the SSA name for this block without emitting an assignment.
            self.define_var_at(block, var, phi, false);
            return Ok(phi);
        }

        let preds = self.preds.get(&block).cloned().unwrap_or_default();
        if preds.is_empty() {
            let var_name = self
                .var_names
                .get(&var)
                .cloned()
                .unwrap_or_else(|| format!("local#{}", var.0));
            Err(crate::error::RRException::new(
                "RR.SemanticError",
                crate::error::RRCode::E1001,
                crate::error::Stage::Mir,
                format!("undefined variable '{}'", var_name),
            )
            .at(Span::default())
            .push_frame(
                "mir::lower_hir::read_var_recursive/2",
                Some(Span::default()),
            )
            .note("Declare the variable with let before use."))
        } else if preds.len() == 1 {
            // Optimize: No phi needed, just look in pred
            self.read_var(var, preds[0])
        } else {
            // Multiple predecessors require a phi.
            let phi = self.add_phi_placeholder(block, Span::default());
            // Break cycles with a Phi placeholder, but don't emit an assignment yet.
            self.define_var_at(block, var, phi, false);
            self.add_phi_operands(block, var, phi)?;
            Ok(phi)
        }
    }

    fn add_phi_operands(&mut self, block: BlockId, var: hir::LocalId, phi_val: ValueId) -> RR<()> {
        // Collect operands from all preds
        let preds = self.preds.get(&block).cloned().unwrap_or_default();
        let mut new_args = Vec::new();
        for pred in preds {
            let val = self.read_var(var, pred)?;
            new_args.push((val, pred));
        }

        if let Some(src) = self.trivial_phi_source(phi_val, &new_args, &mut FxHashSet::default()) {
            self.defs.entry(block).or_default().insert(var, src);
            let src_val = self.fn_ir.values[src].clone();
            if let Some(dst) = self.fn_ir.values.get_mut(phi_val) {
                dst.kind = src_val.kind;
                dst.facts = src_val.facts;
                dst.value_ty = src_val.value_ty;
                dst.value_term = src_val.value_term;
                if dst.origin_var.is_none() {
                    dst.origin_var = src_val.origin_var;
                }
                dst.phi_block = None;
                dst.escape = src_val.escape;
            }
            return Ok(());
        }

        // Update Phi instruction
        if let Some(val) = self.fn_ir.values.get_mut(phi_val) {
            if let ValueKind::Phi { ref mut args } = val.kind {
                *args = new_args;
            } else {
                return Err(InternalCompilerError::new(
                    Stage::Mir,
                    format!("Value {} is not a Phi during SSA sealing", phi_val),
                )
                .into_exception());
            }
        } else {
            return Err(InternalCompilerError::new(
                Stage::Mir,
                format!("Value {} not found during SSA sealing", phi_val),
            )
            .into_exception());
        }

        Ok(())
    }

    fn trivial_phi_source(
        &self,
        phi_val: ValueId,
        args: &[(ValueId, BlockId)],
        seen: &mut FxHashSet<ValueId>,
    ) -> Option<ValueId> {
        if !seen.insert(phi_val) {
            return None;
        }
        let mut candidate = None;
        for (arg, pred) in args {
            if *arg == phi_val {
                continue;
            }
            let resolved = match &self.fn_ir.values[*arg].kind {
                ValueKind::Phi { args: nested } => {
                    self.trivial_phi_source(*arg, nested, seen).unwrap_or(*arg)
                }
                _ => *arg,
            };
            let resolved = self.canonicalize_phi_arg_for_pred(*pred, resolved);
            match candidate {
                None => candidate = Some(resolved),
                Some(prev) if prev == resolved => {}
                Some(_) => return None,
            }
        }
        candidate
    }

    fn canonicalize_phi_arg_for_pred(&self, pred: BlockId, mut value: ValueId) -> ValueId {
        let mut seen = FxHashSet::default();
        while seen.insert(value) {
            let ValueKind::Load { var } = &self.fn_ir.values[value].kind else {
                break;
            };
            let Some(next) = self.fn_ir.blocks[pred]
                .instrs
                .iter()
                .rev()
                .find_map(|instr| match instr {
                    Instr::Assign { dst, src, .. } if dst == var => Some(*src),
                    _ => None,
                })
            else {
                break;
            };
            value = next;
        }
        value
    }

    fn add_phi_placeholder(&mut self, _block: BlockId, span: Span) -> ValueId {
        let id = self.add_value(ValueKind::Phi { args: vec![] }, span);
        if let Some(v) = self.fn_ir.values.get_mut(id) {
            v.phi_block = Some(_block);
        }
        id
    }

    // Call update: terminate must track preds

    // Proof correspondence:
    // `proof/lean/RRProofs/LoweringSubset.lean`,
    // `proof/lean/RRProofs/LoweringIfPhiSubset.lean`,
    // `proof/lean/RRProofs/PipelineBlockEnvSubset.lean`,
    // `proof/lean/RRProofs/PipelineFnEnvSubset.lean`,
    // `proof/lean/RRProofs/PipelineFnCfgSubset.lean`,
    // and the Coq `Lowering*` / `Pipeline*Subset` companions model reduced
    // slices of this source-to-MIR lowering entry point.
    pub fn lower_fn(mut self, f: hir::HirFn) -> RR<FnIR> {
        self.fn_ir.span = f.span;
        self.fn_ir.user_name = self.symbols.get(&f.name).cloned();
        self.fn_ir.param_default_r_exprs = f
            .params
            .iter()
            .map(|p| {
                p.default
                    .as_ref()
                    .map(|expr| self.render_default_expr(expr))
                    .transpose()
            })
            .collect::<RR<Vec<_>>>()?;
        self.fn_ir.param_spans = f.params.iter().map(|p| p.span).collect();
        self.fn_ir.param_ty_hints = f
            .params
            .iter()
            .map(|p| {
                p.ty.as_ref()
                    .map(hir_ty_to_type_state)
                    .unwrap_or(crate::typeck::TypeState::unknown())
            })
            .collect();
        self.fn_ir.param_term_hints = f
            .params
            .iter()
            .map(|p| {
                p.ty.as_ref()
                    .map(|ty| hir_ty_to_type_term_with_symbols(ty, self.symbols))
                    .unwrap_or(crate::typeck::TypeTerm::Any)
            })
            .collect();
        self.fn_ir.param_hint_spans = f
            .params
            .iter()
            .map(|p| p.ty.as_ref().map(|_| p.span))
            .collect();
        self.fn_ir.ret_ty_hint = f.ret_ty.as_ref().map(hir_ty_to_type_state);
        self.fn_ir.ret_term_hint = f
            .ret_ty
            .as_ref()
            .map(|ty| hir_ty_to_type_term_with_symbols(ty, self.symbols));
        self.fn_ir.ret_hint_span = f.ret_ty.as_ref().map(|_| f.span);

        // 1. Bind parameters in the entry block
        for (i, param) in f.params.iter().enumerate() {
            let param_name = self.symbols.get(&param.name).cloned().ok_or_else(|| {
                InternalCompilerError::new(
                    Stage::Mir,
                    format!(
                        "missing parameter symbol during MIR lowering: {:?}",
                        param.name
                    ),
                )
                .into_exception()
            })?; // Clone early to avoid borrow conflict
            if let Some((&local_id, _)) =
                self.var_names.iter().find(|(_, name)| **name == param_name)
            {
                let local_param_name = self.unique_param_local_name(&param_name, local_id);
                self.var_names.insert(local_id, local_param_name.clone());
                // Initialize parameter Value
                let param_val = self.add_value(ValueKind::Param { index: i }, param.span);
                // Parameter writes always target an internal local copy to avoid accidental
                // mutation/aliasing of the visible argument symbol in generated R.
                if let Some(v) = self.fn_ir.values.get_mut(param_val) {
                    v.origin_var = Some(local_param_name);
                }
                // Write to the variable (this also emits Instr::Assign in entry block)
                self.write_var(local_id, param_val);
            }
        }

        // 2. Transition from Entry to Body Head
        let entry_bb = self.fn_ir.entry;
        let head_bb = self.fn_ir.body_head;
        self.add_pred(head_bb, entry_bb);
        self.terminate(Terminator::Goto(head_bb));
        self.curr_block = head_bb;
        self.seal_block(head_bb)?;

        // 3. Lower Body
        let ret_val = self.lower_block(f.body)?;

        // Implicit return if not terminated
        if !self.is_terminated(self.curr_block) {
            self.fn_ir.blocks[self.curr_block].term = Terminator::Return(Some(ret_val));
        }

        Ok(self.fn_ir)
    }

    fn lower_block(&mut self, blk: hir::HirBlock) -> RR<ValueId> {
        let mut last_val = self.add_void_val(blk.span);
        let len = blk.stmts.len();

        for (i, stmt) in blk.stmts.into_iter().enumerate() {
            if let hir::HirStmt::Expr { expr, span } = stmt {
                let val = self.lower_expr(expr)?;
                if i < len - 1 {
                    // Non-tail expression statements are evaluated for effects.
                    self.fn_ir.blocks[self.curr_block]
                        .instrs
                        .push(Instr::Eval { val, span });
                    last_val = self.add_void_val(span);
                } else {
                    last_val = val;
                }
            } else {
                self.lower_stmt(stmt)?;
                last_val = self.add_void_val(blk.span);
            }
        }
        Ok(last_val)
    }

    fn lower_block_effects(&mut self, blk: hir::HirBlock) -> RR<()> {
        for stmt in blk.stmts {
            match stmt {
                hir::HirStmt::Expr { expr, span } => {
                    let val = self.lower_expr(expr)?;
                    self.fn_ir.blocks[self.curr_block]
                        .instrs
                        .push(Instr::Eval { val, span });
                }
                other => {
                    self.lower_stmt(other)?;
                }
            }
        }
        Ok(())
    }

    fn lower_stmt(&mut self, stmt: hir::HirStmt) -> RR<()> {
        match stmt {
            hir::HirStmt::Let {
                local, init, span, ..
            } => {
                let val = if let Some(e) = init {
                    self.lower_expr(e)?
                } else {
                    self.add_null_val(span) // Default init
                };
                self.write_var(local, val);
            }
            hir::HirStmt::Assign {
                target,
                value,
                span,
            } => {
                let v = self.lower_expr(value)?;
                match target {
                    hir::HirLValue::Local(l) => self.write_var(l, v),
                    hir::HirLValue::Index { base, index } => {
                        let base_id = self.lower_expr(base)?;
                        let mut ids = Vec::with_capacity(index.len());
                        for idx_expr in index {
                            ids.push(self.lower_expr(idx_expr)?);
                        }
                        match ids.as_slice() {
                            [idx] => {
                                self.fn_ir.blocks[self.curr_block].instrs.push(
                                    Instr::StoreIndex1D {
                                        base: base_id,
                                        idx: *idx,
                                        val: v,
                                        is_safe: false,
                                        is_na_safe: false,
                                        is_vector: false,
                                        span,
                                    },
                                );
                            }
                            [r, c] => {
                                self.fn_ir.blocks[self.curr_block].instrs.push(
                                    Instr::StoreIndex2D {
                                        base: base_id,
                                        r: *r,
                                        c: *c,
                                        val: v,
                                        span,
                                    },
                                );
                            }
                            [i, j, k] => {
                                self.fn_ir.blocks[self.curr_block].instrs.push(
                                    Instr::StoreIndex3D {
                                        base: base_id,
                                        i: *i,
                                        j: *j,
                                        k: *k,
                                        val: v,
                                        span,
                                    },
                                );
                            }
                            _ => {
                                return Err(crate::error::RRException::new(
                                    "RR.SemanticError",
                                    crate::error::RRCode::E1002,
                                    crate::error::Stage::Mir,
                                    "Only 1D/2D/3D indexing is supported",
                                ));
                            }
                        }
                    }
                    hir::HirLValue::Field { base, name } => {
                        let field_name = self
                            .symbols
                            .get(&name)
                            .cloned()
                            .unwrap_or_else(|| format!("field_{}", name.0));
                        let base_clone = base.clone();
                        let base_id = self.lower_expr(base)?;
                        let set_id = self.add_value(
                            ValueKind::FieldSet {
                                base: base_id,
                                field: field_name,
                                value: v,
                            },
                            span,
                        );
                        match base_clone {
                            hir::HirExpr::Local(lid) => {
                                self.write_var(lid, set_id);
                            }
                            hir::HirExpr::Global(sym, _) => {
                                if let Some(dst_name) = self.symbols.get(&sym).cloned() {
                                    self.fn_ir.blocks[self.curr_block]
                                        .instrs
                                        .push(Instr::Assign {
                                            dst: dst_name,
                                            src: set_id,
                                            span,
                                        });
                                } else {
                                    self.fn_ir.blocks[self.curr_block]
                                        .instrs
                                        .push(Instr::Eval { val: set_id, span });
                                }
                            }
                            _ => {
                                // Fallback: preserve side effect when base isn't a writable symbol.
                                self.fn_ir.blocks[self.curr_block]
                                    .instrs
                                    .push(Instr::Eval { val: set_id, span });
                            }
                        }
                    }
                }
            }
            hir::HirStmt::Expr { expr, .. } => {
                self.lower_expr(expr)?;
            }
            hir::HirStmt::Return { value, span: _span } => {
                let v = if let Some(e) = value {
                    Some(self.lower_expr(e)?)
                } else {
                    None
                };
                self.terminate_and_detach(Terminator::Return(v));
            }
            hir::HirStmt::If {
                cond,
                then_blk,
                else_blk,
                span: _span,
            } => {
                let cond_val = self.lower_expr(cond)?;
                let pre_if_bb = self.curr_block;

                let then_bb = self.fn_ir.add_block();
                let else_bb = self.fn_ir.add_block();
                let join_bb = self.fn_ir.add_block();

                self.terminate(Terminator::If {
                    cond: cond_val,
                    then_bb,
                    else_bb,
                });

                // Then branch
                self.add_pred(then_bb, pre_if_bb);
                self.curr_block = then_bb;
                self.seal_block(then_bb)?;
                self.lower_block_effects(then_blk)?;
                if !self.is_terminated(self.curr_block) {
                    self.add_pred(join_bb, self.curr_block);
                    self.terminate(Terminator::Goto(join_bb));
                }

                // Else branch
                self.add_pred(else_bb, pre_if_bb);
                self.curr_block = else_bb;
                self.seal_block(else_bb)?;
                if let Some(eb) = else_blk {
                    self.lower_block_effects(eb)?;
                }
                if !self.is_terminated(self.curr_block) {
                    self.add_pred(join_bb, self.curr_block);
                    self.terminate(Terminator::Goto(join_bb));
                }

                self.curr_block = join_bb;
                self.seal_block(join_bb)?;
            }
            hir::HirStmt::While {
                cond,
                body,
                span: _span,
            } => {
                let header_bb = self.fn_ir.add_block();
                let body_bb = self.fn_ir.add_block();
                let exit_bb = self.fn_ir.add_block();

                self.add_pred(header_bb, self.curr_block);
                self.terminate(Terminator::Goto(header_bb));

                self.curr_block = header_bb;
                let cond_val = self.lower_expr(cond)?;
                self.terminate(Terminator::If {
                    cond: cond_val,
                    then_bb: body_bb,
                    else_bb: exit_bb,
                });

                self.add_pred(body_bb, header_bb);
                self.curr_block = body_bb;
                self.seal_block(body_bb)?;
                self.loop_stack.push(LoopTargets {
                    break_bb: exit_bb,
                    continue_bb: header_bb,
                    continue_step: None,
                });
                self.lower_block_effects(body)?;
                self.loop_stack.pop();
                let curr_reachable = self
                    .preds
                    .get(&self.curr_block)
                    .map(|ps| !ps.is_empty())
                    .unwrap_or(false);
                if !self.is_terminated(self.curr_block) && curr_reachable {
                    self.add_pred(header_bb, self.curr_block);
                    self.terminate(Terminator::Goto(header_bb));
                }

                self.seal_block(header_bb)?;
                self.add_pred(exit_bb, header_bb);
                self.curr_block = exit_bb;
                self.seal_block(exit_bb)?;
            }
            hir::HirStmt::For { iter, body, span } => {
                self.lower_for(iter, body, span)?;
            }
            hir::HirStmt::Break { span } => {
                if let Some(targets) = self.loop_stack.last().copied() {
                    self.add_pred(targets.break_bb, self.curr_block);
                    self.terminate_and_detach(Terminator::Goto(targets.break_bb));
                } else {
                    return Err(crate::error::RRException::new(
                        "RR.SemanticError",
                        crate::error::RRCode::E1002,
                        crate::error::Stage::Mir,
                        "break used outside of a loop".to_string(),
                    )
                    .at(span));
                }
            }
            hir::HirStmt::Next { span } => {
                if let Some(targets) = self.loop_stack.last().copied() {
                    if let Some((var, iv)) = targets.continue_step {
                        let one = self.add_int_val(1, span);
                        let next_iv = self.add_value(
                            ValueKind::Binary {
                                op: BinOp::Add,
                                lhs: iv,
                                rhs: one,
                            },
                            span,
                        );
                        self.write_var(var, next_iv);
                    }
                    self.add_pred(targets.continue_bb, self.curr_block);
                    self.terminate_and_detach(Terminator::Goto(targets.continue_bb));
                } else {
                    return Err(crate::error::RRException::new(
                        "RR.SemanticError",
                        crate::error::RRCode::E1002,
                        crate::error::Stage::Mir,
                        "next used outside of a loop".to_string(),
                    )
                    .at(span));
                }
            }
        }
        Ok(())
    }

    fn lower_expr(&mut self, expr: hir::HirExpr) -> RR<ValueId> {
        // println!("DEBUG: Lowering Expr: {:?}", expr);
        match expr {
            hir::HirExpr::Lit(l) => {
                let al = match l {
                    hir::HirLit::Int(i) => Lit::Int(i),
                    hir::HirLit::Double(f) => Lit::Float(f),
                    hir::HirLit::Char(s) => Lit::Str(s),
                    hir::HirLit::Bool(b) => Lit::Bool(b),
                    hir::HirLit::NA => Lit::Na,
                    hir::HirLit::Null => Lit::Null,
                };
                Ok(self.add_value(ValueKind::Const(al), Span::default()))
            }
            hir::HirExpr::Local(l) => self.read_var(l, self.curr_block),
            hir::HirExpr::Global(sym, span) => {
                let raw_name = self
                    .symbols
                    .get(&sym)
                    .cloned()
                    .unwrap_or_else(|| format!("Sym_{}", sym.0));
                if self.in_tidy_mask() && Self::should_lower_as_tidy_symbol(&raw_name) {
                    return Ok(self.add_value(
                        ValueKind::RSymbol {
                            name: raw_name.clone(),
                        },
                        span,
                    ));
                }
                let name = if self.known_functions.contains_key(&raw_name) {
                    format!("Sym_{}", sym.0)
                } else {
                    raw_name
                };
                Ok(self.add_value_with_name(
                    ValueKind::Load { var: name.clone() },
                    span,
                    Some(name),
                ))
            }
            hir::HirExpr::Unary { op, expr } => {
                let rhs = self.lower_expr(*expr)?;
                let op = match op {
                    hir::HirUnOp::Not => crate::syntax::ast::UnaryOp::Not,
                    hir::HirUnOp::Neg => crate::syntax::ast::UnaryOp::Neg,
                };
                Ok(self.add_value(ValueKind::Unary { op, rhs }, Span::default()))
            }
            hir::HirExpr::Binary { op, lhs, rhs } => {
                let l = self.lower_expr(*lhs)?;
                let r = self.lower_expr(*rhs)?;
                let op = self.map_binop(op);
                Ok(self.add_value(ValueKind::Binary { op, lhs: l, rhs: r }, Span::default()))
            }
            hir::HirExpr::Field { base, name } => {
                let b = self.lower_expr(*base)?;
                let field_name = self
                    .symbols
                    .get(&name)
                    .cloned()
                    .unwrap_or_else(|| format!("field_{}", name.0));
                Ok(self.add_value(
                    ValueKind::FieldGet {
                        base: b,
                        field: field_name,
                    },
                    Span::default(),
                ))
            }
            hir::HirExpr::Index { base, index } => {
                let span = Span::default();
                let base_id = self.lower_expr(*base)?;
                let mut ids = Vec::with_capacity(index.len());
                for idx_expr in index {
                    ids.push(self.lower_expr(idx_expr)?);
                }
                match ids.as_slice() {
                    [idx] => Ok(self.add_value(
                        ValueKind::Index1D {
                            base: base_id,
                            idx: *idx,
                            is_safe: false,
                            is_na_safe: false,
                        },
                        span,
                    )),
                    [r, c] => Ok(self.add_value(
                        ValueKind::Index2D {
                            base: base_id,
                            r: *r,
                            c: *c,
                        },
                        span,
                    )),
                    [i, j, k] => Ok(self.add_value(
                        ValueKind::Index3D {
                            base: base_id,
                            i: *i,
                            j: *j,
                            k: *k,
                        },
                        span,
                    )),
                    _ => Err(crate::error::RRException::new(
                        "RR.SemanticError",
                        crate::error::RRCode::E1002,
                        crate::error::Stage::Mir,
                        "Only 1D/2D/3D indexing is supported",
                    )),
                }
            }
            hir::HirExpr::Block(blk) => self.lower_block(blk),

            hir::HirExpr::Call(hir::HirCall { callee, args, span }) => {
                let tidy_mask_args = match callee.as_ref() {
                    hir::HirExpr::Global(sym, _) => self
                        .symbols
                        .get(sym)
                        .is_some_and(|name| Self::is_tidy_data_mask_call(name)),
                    _ => false,
                };
                let mut v_args = Vec::new();
                let mut arg_names: Vec<Option<String>> = Vec::new();
                for arg in args {
                    match arg {
                        hir::HirArg::Pos(e) => {
                            let lowered = if tidy_mask_args {
                                self.with_tidy_mask(|lowerer| lowerer.lower_expr(e))?
                            } else {
                                self.lower_expr(e)?
                            };
                            v_args.push(lowered);
                            arg_names.push(None);
                        }
                        hir::HirArg::Named { name, value } => {
                            let lowered = if tidy_mask_args {
                                self.with_tidy_mask(|lowerer| lowerer.lower_expr(value))?
                            } else {
                                self.lower_expr(value)?
                            };
                            v_args.push(lowered);
                            let n = self
                                .symbols
                                .get(&name)
                                .cloned()
                                .unwrap_or_else(|| format!("arg_{}", name.0));
                            arg_names.push(Some(n));
                        }
                    }
                }

                match callee.as_ref() {
                    hir::HirExpr::Global(sym, _) => {
                        if let Some(name) = self.symbols.get(sym) {
                            if name.starts_with("rr_") {
                                return Ok(self.add_value(
                                    ValueKind::Call {
                                        callee: name.clone(),
                                        args: v_args,
                                        names: arg_names,
                                    },
                                    span,
                                ));
                            }
                            if Self::allow_user_builtin_shadowing(name)
                                && let Some(expected) = self.known_functions.get(name)
                            {
                                let _ = expected;
                                return Ok(self.add_value(
                                    ValueKind::Call {
                                        callee: format!("Sym_{}", sym.0),
                                        args: v_args,
                                        names: arg_names,
                                    },
                                    span,
                                ));
                            }
                            if name == "length" {
                                if v_args.len() != 1 {
                                    return Err(crate::error::RRException::new(
                                        "RR.SemanticError",
                                        crate::error::RRCode::E1002,
                                        crate::error::Stage::Mir,
                                        format!(
                                            "builtin '{}' expects 1 argument, got {}",
                                            name,
                                            v_args.len()
                                        ),
                                    )
                                    .at(span));
                                }
                                return Ok(self.add_value(ValueKind::Len { base: v_args[0] }, span));
                            }
                            // Known builtins should keep their original names.
                            if matches!(
                                name.as_str(),
                                "seq_along"
                                    | "seq_len"
                                    | "c"
                                    | "list"
                                    | "sum"
                                    | "mean"
                                    | "var"
                                    | "prod"
                                    | "min"
                                    | "max"
                                    | "abs"
                                    | "sqrt"
                                    | "sin"
                                    | "cos"
                                    | "tan"
                                    | "asin"
                                    | "acos"
                                    | "atan"
                                    | "atan2"
                                    | "sinh"
                                    | "cosh"
                                    | "tanh"
                                    | "log"
                                    | "log10"
                                    | "log2"
                                    | "exp"
                                    | "sign"
                                    | "gamma"
                                    | "lgamma"
                                    | "floor"
                                    | "ceiling"
                                    | "trunc"
                                    | "round"
                                    | "pmax"
                                    | "pmin"
                                    | "print"
                                    | "paste"
                                    | "paste0"
                                    | "sprintf"
                                    | "cat"
                                    | "names"
                                    | "rownames"
                                    | "colnames"
                                    | "sort"
                                    | "order"
                                    | "match"
                                    | "unique"
                                    | "duplicated"
                                    | "anyDuplicated"
                                    | "any"
                                    | "all"
                                    | "which"
                                    | "is.na"
                                    | "is.finite"
                                    | "numeric"
                                    | "character"
                                    | "logical"
                                    | "integer"
                                    | "double"
                                    | "rep"
                                    | "rep.int"
                                    | "vector"
                                    | "matrix"
                                    | "dim"
                                    | "dimnames"
                                    | "nrow"
                                    | "ncol"
                                    | "colSums"
                                    | "rowSums"
                                    | "crossprod"
                                    | "tcrossprod"
                                    | "t"
                                    | "diag"
                                    | "rbind"
                                    | "cbind"
                            ) {
                                return Ok(self.add_value(
                                    ValueKind::Call {
                                        callee: name.clone(),
                                        args: v_args,
                                        names: arg_names,
                                    },
                                    span,
                                ));
                            }
                            if Self::is_dynamic_fallback_builtin(name) {
                                self.fn_ir
                                    .mark_hybrid_interop(Self::hybrid_interop_reason(name));
                                return Ok(self.add_value(
                                    ValueKind::Call {
                                        callee: name.clone(),
                                        args: v_args,
                                        names: arg_names,
                                    },
                                    span,
                                ));
                            }
                            if Self::is_namespaced_r_call(name) {
                                if !Self::is_supported_package_call(name) {
                                    self.fn_ir.mark_opaque_interop_reason(
                                        Self::opaque_package_reason(name),
                                    );
                                }
                                return Ok(self.add_value(
                                    ValueKind::Call {
                                        callee: name.clone(),
                                        args: v_args,
                                        names: arg_names,
                                    },
                                    span,
                                ));
                            }
                            if self.in_tidy_mask() && Self::is_tidy_helper_call(name) {
                                if !Self::is_supported_tidy_helper_call(name) {
                                    self.fn_ir.mark_opaque_interop_reason(
                                        Self::opaque_tidy_helper_reason(name),
                                    );
                                }
                                return Ok(self.add_value(
                                    ValueKind::Call {
                                        callee: name.clone(),
                                        args: v_args,
                                        names: arg_names,
                                    },
                                    span,
                                ));
                            }
                            if let Some(expected) = self.known_functions.get(name) {
                                let _ = expected;
                                return Ok(self.add_value(
                                    ValueKind::Call {
                                        callee: format!("Sym_{}", sym.0),
                                        args: v_args,
                                        names: arg_names,
                                    },
                                    span,
                                ));
                            }
                            let mut err = crate::error::RRException::new(
                                "RR.SemanticError",
                                crate::error::RRCode::E1001,
                                crate::error::Stage::Mir,
                                format!("undefined function '{}'", name),
                            )
                            .at(span)
                            .note("Define or import the function before calling it.");
                            if let Some(suggestion) = self.suggest_function_name(name) {
                                err = err.help(suggestion);
                            }
                            return Err(err);
                        }
                        Err(crate::error::RRException::new(
                            "RR.SemanticError",
                            crate::error::RRCode::E1001,
                            crate::error::Stage::Mir,
                            "invalid unresolved callee symbol".to_string(),
                        )
                        .at(span))
                    }
                    _ => {
                        let callee_val = self.lower_expr(callee.as_ref().clone())?;
                        let mut dyn_args = Vec::with_capacity(v_args.len() + 1);
                        dyn_args.push(callee_val);
                        dyn_args.extend(v_args);
                        let mut dyn_names = Vec::with_capacity(arg_names.len() + 1);
                        dyn_names.push(None);
                        dyn_names.extend(arg_names);
                        Ok(self.add_value(
                            ValueKind::Call {
                                callee: "rr_call_closure".to_string(),
                                args: dyn_args,
                                names: dyn_names,
                            },
                            span,
                        ))
                    }
                }
            }
            hir::HirExpr::IfExpr {
                cond,
                then_expr,
                else_expr,
            } => {
                let cond_val = self.lower_expr(*cond)?;

                let then_bb = self.fn_ir.add_block();
                let else_bb = self.fn_ir.add_block();
                let merge_bb = self.fn_ir.add_block();

                self.add_pred(then_bb, self.curr_block);
                self.add_pred(else_bb, self.curr_block);

                self.terminate(Terminator::If {
                    cond: cond_val,
                    then_bb,
                    else_bb,
                });

                // Then Branch
                self.curr_block = then_bb;
                // Seal Then? Only 1 pred.
                self.seal_block(then_bb)?;
                let then_val = self.lower_expr(*then_expr)?;
                if !self.is_terminated(then_bb) {
                    self.add_pred(merge_bb, self.curr_block);
                    self.terminate(Terminator::Goto(merge_bb));
                }
                let then_end_bb = self.curr_block;

                // Else Branch
                self.curr_block = else_bb;
                self.seal_block(else_bb)?;
                let else_val = self.lower_expr(*else_expr)?;
                if !self.is_terminated(else_bb) {
                    self.add_pred(merge_bb, self.curr_block);
                    self.terminate(Terminator::Goto(merge_bb));
                }
                let else_end_bb = self.curr_block;

                // Merge Branch
                self.curr_block = merge_bb;
                self.seal_block(merge_bb)?;

                // Phi for result value
                let phi_val = self.add_value(
                    ValueKind::Phi {
                        args: vec![(then_val, then_end_bb), (else_val, else_end_bb)],
                    },
                    Span::default(),
                );
                if let Some(v) = self.fn_ir.values.get_mut(phi_val) {
                    v.phi_block = Some(merge_bb);
                }

                Ok(phi_val)
            }
            hir::HirExpr::VectorLit(elems) => {
                let mut vals = Vec::new();
                for e in elems {
                    vals.push(self.lower_expr(e)?);
                }
                // Lower vector literals via R's `c(...)` constructor.
                let names = vec![None; vals.len()];
                Ok(self.add_value(
                    ValueKind::Call {
                        callee: "c".to_string(),
                        args: vals,
                        names,
                    },
                    Span::default(),
                ))
            }
            hir::HirExpr::ListLit(fields) => {
                let mut vals = Vec::new();
                for (sym, e) in fields {
                    let field = self
                        .symbols
                        .get(&sym)
                        .cloned()
                        .unwrap_or_else(|| format!("field_{}", sym.0));
                    vals.push((field, self.lower_expr(e)?));
                }
                Ok(self.add_value(ValueKind::RecordLit { fields: vals }, Span::default()))
            }
            hir::HirExpr::Range { start, end } => {
                let s = self.lower_expr(*start)?;
                let e = self.lower_expr(*end)?;
                Ok(self.add_value(ValueKind::Range { start: s, end: e }, Span::default()))
            }
            hir::HirExpr::Try(inner) => {
                // RR v1: try-postfix lowers to value propagation in MIR.
                // Runtime error propagation is still handled by R semantics.
                self.lower_expr(*inner)
            }
            hir::HirExpr::Match { scrut, arms } => self.lower_match_expr(*scrut, arms),
            hir::HirExpr::Column(name) => {
                Ok(self.add_value(ValueKind::RSymbol { name }, Span::default()))
            }
            hir::HirExpr::Unquote(e) => self.lower_expr(*e),
            _ => Err(crate::error::RRException::new(
                "RR.SemanticError",
                crate::error::RRCode::E1002,
                crate::error::Stage::Mir,
                format!("unsupported expression in MIR lowering: {:?}", expr),
            )
            .at(Span::default())
            .push_frame("mir::lower_hir::lower_expr/1", Some(Span::default()))),
        }
    }

    // Helpers

    fn add_void_val(&mut self, span: Span) -> ValueId {
        self.add_value(ValueKind::Const(Lit::Null), span)
    }

    fn add_null_val(&mut self, span: Span) -> ValueId {
        self.add_value(ValueKind::Const(Lit::Null), span)
    }

    fn add_bool_val(&mut self, b: bool, span: Span) -> ValueId {
        self.add_value(ValueKind::Const(Lit::Bool(b)), span)
    }

    fn add_int_val(&mut self, n: i64, span: Span) -> ValueId {
        self.add_value(ValueKind::Const(Lit::Int(n)), span)
    }

    fn add_bin_bool(&mut self, op: BinOp, lhs: ValueId, rhs: ValueId, span: Span) -> ValueId {
        self.add_value(ValueKind::Binary { op, lhs, rhs }, span)
    }

    fn add_call_value(&mut self, callee: &str, args: Vec<ValueId>, span: Span) -> ValueId {
        let names = vec![None; args.len()];
        self.add_value(
            ValueKind::Call {
                callee: callee.to_string(),
                args,
                names,
            },
            span,
        )
    }

    fn symbol_name(&self, sym: hir::SymbolId) -> String {
        self.symbols
            .get(&sym)
            .cloned()
            .unwrap_or_else(|| format!("field_{}", sym.0))
    }

    fn terminate_and_detach(&mut self, term: Terminator) {
        let from = self.curr_block;
        self.terminate(term);
        let dead_bb = self.fn_ir.add_block();
        if let Some(defs_here) = self.defs.get(&from).cloned() {
            self.defs.insert(dead_bb, defs_here);
        } else {
            self.defs.insert(dead_bb, FxHashMap::default());
        }
        self.curr_block = dead_bb;
    }

    fn terminate(&mut self, term: Terminator) {
        self.fn_ir.blocks[self.curr_block].term = term;
    }

    fn is_terminated(&self, b: BlockId) -> bool {
        !matches!(self.fn_ir.blocks[b].term, Terminator::Unreachable)
    }

    fn map_binop(&self, op: hir::HirBinOp) -> BinOp {
        match op {
            hir::HirBinOp::Add => BinOp::Add,
            hir::HirBinOp::Sub => BinOp::Sub,
            hir::HirBinOp::Mul => BinOp::Mul,
            hir::HirBinOp::Div => BinOp::Div,
            hir::HirBinOp::Mod => BinOp::Mod,
            hir::HirBinOp::MatMul => BinOp::MatMul,
            hir::HirBinOp::Eq => BinOp::Eq,
            hir::HirBinOp::Ne => BinOp::Ne,
            hir::HirBinOp::Lt => BinOp::Lt,
            hir::HirBinOp::Le => BinOp::Le,
            hir::HirBinOp::Gt => BinOp::Gt,
            hir::HirBinOp::Ge => BinOp::Ge,
            hir::HirBinOp::And => BinOp::And,
            hir::HirBinOp::Or => BinOp::Or,
            // HirBinOp might have more variants?
        }
    }

    fn is_dynamic_fallback_builtin(name: &str) -> bool {
        crate::mir::semantics::call_model::is_dynamic_fallback_builtin(name)
    }

    fn is_namespaced_r_call(name: &str) -> bool {
        crate::mir::semantics::call_model::is_namespaced_r_call(name)
    }

    fn is_tidy_data_mask_call(name: &str) -> bool {
        crate::mir::semantics::call_model::is_tidy_data_mask_call(name)
    }

    fn is_tidy_helper_call(name: &str) -> bool {
        crate::mir::semantics::call_model::is_tidy_helper_call(name)
    }

    fn is_supported_package_call(name: &str) -> bool {
        crate::mir::semantics::call_model::is_supported_package_call(name)
    }

    fn is_supported_tidy_helper_call(name: &str) -> bool {
        crate::mir::semantics::call_model::is_supported_tidy_helper_call(name)
    }

    fn should_lower_as_tidy_symbol(name: &str) -> bool {
        !name.starts_with("rr_")
            && !Self::is_namespaced_r_call(name)
            && !Self::is_dynamic_fallback_builtin(name)
            && !Self::is_tidy_helper_call(name)
            && !matches!(
                name,
                "seq_along"
                    | "seq_len"
                    | "c"
                    | "list"
                    | "sum"
                    | "mean"
                    | "var"
                    | "prod"
                    | "min"
                    | "max"
                    | "abs"
                    | "sqrt"
                    | "sin"
                    | "cos"
                    | "tan"
                    | "asin"
                    | "acos"
                    | "atan"
                    | "atan2"
                    | "sinh"
                    | "cosh"
                    | "tanh"
                    | "log"
                    | "log10"
                    | "log2"
                    | "exp"
                    | "sign"
                    | "gamma"
                    | "lgamma"
                    | "floor"
                    | "ceiling"
                    | "trunc"
                    | "round"
                    | "pmax"
                    | "pmin"
                    | "print"
                    | "paste"
                    | "paste0"
                    | "sprintf"
                    | "cat"
                    | "names"
                    | "rownames"
                    | "colnames"
                    | "sort"
                    | "order"
                    | "match"
                    | "unique"
                    | "duplicated"
                    | "anyDuplicated"
                    | "any"
                    | "all"
                    | "which"
                    | "is.na"
                    | "is.finite"
                    | "numeric"
                    | "character"
                    | "logical"
                    | "integer"
                    | "double"
                    | "rep"
                    | "rep.int"
                    | "vector"
                    | "matrix"
                    | "dim"
                    | "dimnames"
                    | "nrow"
                    | "ncol"
                    | "colSums"
                    | "rowSums"
                    | "crossprod"
                    | "tcrossprod"
                    | "t"
                    | "diag"
                    | "rbind"
                    | "cbind"
            )
    }

    fn hybrid_interop_reason(name: &str) -> InteropReason {
        let (why, suggestion) = match name {
            "library" | "require" => (
                "package attachment mutates the runtime search path and cannot be proven stable at compile-time",
                Some(
                    "prefer `import r \"pkg\"`, `import r { ... } from \"pkg\"`, or `import r * as ns from \"pkg\"` for namespace-only access",
                ),
            ),
            "plot" | "lines" | "legend" | "png" | "dev.off" => (
                "unqualified plotting call depends on runtime package attachment and search-path resolution",
                Some(
                    "prefer namespaced R imports so the call lowers to `pkg::symbol(...)` directly",
                ),
            ),
            "eval" | "parse" => (
                "call evaluates code dynamically, so RR cannot stabilize the callee or its argument semantics ahead of time",
                Some(
                    "avoid runtime code construction or isolate it behind a dedicated dynamic boundary",
                ),
            ),
            "get" | "assign" | "exists" | "mget" | "rm" | "ls" => (
                "call reads or mutates environments dynamically, so symbol resolution depends on runtime state",
                Some(
                    "prefer explicit RR bindings or namespaced package imports when the target is known",
                ),
            ),
            "parent.frame" | "environment" | "sys.frame" | "sys.call" | "do.call" => (
                "call depends on runtime stack or environment state that RR cannot model statically",
                Some("pass the callee and arguments explicitly through RR values where possible"),
            ),
            _ => (
                "call uses a dynamic runtime feature that RR cannot reduce to stable direct interop",
                None,
            ),
        };
        InteropReason::new(
            InteropTier::Hybrid,
            InteropReasonKind::DynamicBuiltin,
            name,
            why,
            suggestion,
        )
    }

    fn opaque_package_reason(name: &str) -> InteropReason {
        InteropReason::new(
            InteropTier::Opaque,
            InteropReasonKind::PackageCall,
            name,
            "package call is preserved exactly, but RR has no dedicated semantic model for this symbol",
            Some(
                "keep the call namespaced or add this symbol to the direct interop surface if RR should reason about it",
            ),
        )
    }

    fn opaque_tidy_helper_reason(name: &str) -> InteropReason {
        InteropReason::new(
            InteropTier::Opaque,
            InteropReasonKind::TidyHelper,
            name,
            "tidy helper is forwarded as-is because RR does not model its selector semantics directly",
            Some(
                "prefer supported tidy helpers or add this helper to the direct tidy interop surface",
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trivial_phi_is_folded_during_sealing_for_pred_local_load_aliases() {
        let symbols = FxHashMap::default();
        let known_functions = FxHashMap::default();
        let mut var_names = FxHashMap::default();
        let local_x = hir::LocalId(0);
        var_names.insert(local_x, "x".to_string());

        let mut lowerer = MirLowerer::new(
            "seal_phi_alias".to_string(),
            vec![],
            var_names,
            &symbols,
            &known_functions,
        );

        let left = lowerer.fn_ir.body_head;
        let right = lowerer.fn_ir.add_block();
        let merge = lowerer.fn_ir.add_block();
        lowerer.defs.entry(right).or_default();
        lowerer.defs.entry(merge).or_default();

        let one = lowerer.fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let load_left = lowerer.fn_ir.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let load_right = lowerer.fn_ir.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = lowerer.add_phi_placeholder(merge, Span::default());

        lowerer.fn_ir.blocks[left].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: one,
            span: Span::default(),
        });
        lowerer.fn_ir.blocks[left].term = Terminator::Goto(merge);
        lowerer.fn_ir.blocks[right].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: one,
            span: Span::default(),
        });
        lowerer.fn_ir.blocks[right].term = Terminator::Goto(merge);
        lowerer.preds.insert(merge, vec![left, right]);
        lowerer
            .defs
            .entry(left)
            .or_default()
            .insert(local_x, load_left);
        lowerer
            .defs
            .entry(right)
            .or_default()
            .insert(local_x, load_right);

        lowerer.add_phi_operands(merge, local_x, phi).unwrap();

        assert!(
            !matches!(lowerer.fn_ir.values[phi].kind, ValueKind::Phi { .. }),
            "sealing should fold load-alias phi inputs that resolve to the same predecessor-local assignment"
        );
        assert_eq!(
            lowerer
                .defs
                .get(&merge)
                .and_then(|m| m.get(&local_x))
                .copied(),
            Some(one)
        );
    }
}
