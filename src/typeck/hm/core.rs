use super::*;
use crate::hir::def::{
    HirArg, HirBinOp, HirBlock, HirCall, HirExpr, HirFn, HirForIter, HirItem, HirLValue, HirLit,
    HirProgram, HirStmt, HirUnOp, LocalId, SymbolId, Ty,
};
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum HmTy {
    Var(u32),
    Int,
    Double,
    Logical,
    Char,
    Null,
    Vector(Box<HmTy>),
    Matrix(Box<HmTy>),
    List(Box<HmTy>),
    Record(Vec<(String, HmTy)>),
    Function(Vec<HmTy>, Box<HmTy>),
    Any,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scheme {
    pub vars: Vec<u32>,
    pub ty: HmTy,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Subst {
    pub map: FxHashMap<u32, HmTy>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Constraint {
    pub lhs: HmTy,
    pub rhs: HmTy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnifyError {
    Occurs { var: u32, ty: HmTy },
    Mismatch { lhs: HmTy, rhs: HmTy },
}

impl Subst {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(&self, ty: &HmTy) -> HmTy {
        match ty {
            HmTy::Var(var) => self
                .map
                .get(var)
                .map(|bound| self.apply(bound))
                .unwrap_or(HmTy::Var(*var)),
            HmTy::Vector(inner) => HmTy::Vector(Box::new(self.apply(inner))),
            HmTy::Matrix(inner) => HmTy::Matrix(Box::new(self.apply(inner))),
            HmTy::List(inner) => HmTy::List(Box::new(self.apply(inner))),
            HmTy::Record(fields) => HmTy::Record(
                fields
                    .iter()
                    .map(|(name, ty)| (name.clone(), self.apply(ty)))
                    .collect(),
            ),
            HmTy::Function(params, ret) => HmTy::Function(
                params.iter().map(|param| self.apply(param)).collect(),
                Box::new(self.apply(ret)),
            ),
            HmTy::Int | HmTy::Double | HmTy::Logical | HmTy::Char | HmTy::Null | HmTy::Any => {
                ty.clone()
            }
        }
    }
}

pub fn unify(lhs: &HmTy, rhs: &HmTy, subst: &mut Subst) -> Result<(), UnifyError> {
    let lhs = subst.apply(lhs);
    let rhs = subst.apply(rhs);
    match (lhs, rhs) {
        (HmTy::Any, _) | (_, HmTy::Any) => Ok(()),
        (HmTy::Var(var), ty) | (ty, HmTy::Var(var)) => bind_var(var, ty, subst),
        (HmTy::Int, HmTy::Int)
        | (HmTy::Double, HmTy::Double)
        | (HmTy::Logical, HmTy::Logical)
        | (HmTy::Char, HmTy::Char)
        | (HmTy::Null, HmTy::Null) => Ok(()),
        (HmTy::Vector(lhs), HmTy::Vector(rhs))
        | (HmTy::Matrix(lhs), HmTy::Matrix(rhs))
        | (HmTy::List(lhs), HmTy::List(rhs)) => unify(&lhs, &rhs, subst),
        (HmTy::Function(lhs_params, lhs_ret), HmTy::Function(rhs_params, rhs_ret))
            if lhs_params.len() == rhs_params.len() =>
        {
            for (lhs, rhs) in lhs_params.iter().zip(rhs_params.iter()) {
                unify(lhs, rhs, subst)?;
            }
            unify(&lhs_ret, &rhs_ret, subst)
        }
        (HmTy::Record(lhs_fields), HmTy::Record(rhs_fields))
            if lhs_fields.len() == rhs_fields.len()
                && lhs_fields
                    .iter()
                    .zip(rhs_fields.iter())
                    .all(|((lhs_name, _), (rhs_name, _))| lhs_name == rhs_name) =>
        {
            for ((_, lhs_ty), (_, rhs_ty)) in lhs_fields.iter().zip(rhs_fields.iter()) {
                unify(lhs_ty, rhs_ty, subst)?;
            }
            Ok(())
        }
        (lhs, rhs) => Err(UnifyError::Mismatch { lhs, rhs }),
    }
}

pub(crate) fn bind_var(var: u32, ty: HmTy, subst: &mut Subst) -> Result<(), UnifyError> {
    if ty == HmTy::Var(var) {
        return Ok(());
    }
    if occurs_in(var, &ty, subst) {
        return Err(UnifyError::Occurs { var, ty });
    }
    subst.map.insert(var, ty);
    Ok(())
}

pub(crate) fn occurs_in(var: u32, ty: &HmTy, subst: &Subst) -> bool {
    match subst.apply(ty) {
        HmTy::Var(other) => var == other,
        HmTy::Vector(inner) | HmTy::Matrix(inner) | HmTy::List(inner) => {
            occurs_in(var, &inner, subst)
        }
        HmTy::Record(fields) => fields.iter().any(|(_, field)| occurs_in(var, field, subst)),
        HmTy::Function(params, ret) => {
            params.iter().any(|param| occurs_in(var, param, subst)) || occurs_in(var, &ret, subst)
        }
        HmTy::Int | HmTy::Double | HmTy::Logical | HmTy::Char | HmTy::Null | HmTy::Any => false,
    }
}

#[derive(Clone, Debug)]
pub struct InferEnv {
    pub(crate) next_var: u32,
    pub(crate) locals: FxHashMap<LocalId, Scheme>,
    pub(crate) globals: FxHashMap<String, Scheme>,
    pub(crate) subst: Subst,
    pub(crate) return_tys: Vec<HmTy>,
}

impl InferEnv {
    pub fn new(globals: FxHashMap<String, Scheme>) -> Self {
        Self {
            next_var: 0,
            locals: FxHashMap::default(),
            globals,
            subst: Subst::new(),
            return_tys: Vec::new(),
        }
    }

    pub fn fresh_var(&mut self) -> HmTy {
        let var = self.next_var;
        self.next_var += 1;
        HmTy::Var(var)
    }

    pub fn add_constraint(&mut self, constraint: Constraint) {
        let _ = unify(&constraint.lhs, &constraint.rhs, &mut self.subst);
    }

    pub(crate) fn apply_fn(
        &mut self,
        f: &mut HirFn,
        symbols: &FxHashMap<SymbolId, String>,
    ) -> Scheme {
        let mut param_tys = Vec::with_capacity(f.params.len());
        for idx in 0..f.params.len() {
            let param_ty = f.params[idx]
                .ty
                .as_ref()
                .and_then(hm_ty_from_hir_ty)
                .unwrap_or_else(|| self.fresh_var());

            if let Some(default) = f.params[idx].default.as_ref() {
                let default_ty = self.infer_expr(default, symbols);
                let _ = unify(&param_ty, &default_ty, &mut self.subst);
                if f.params[idx].ty.is_none()
                    && let Some(hint) = hm_ty_to_hir_ty(&self.subst.apply(&default_ty))
                {
                    f.params[idx].ty = Some(hint);
                    f.params[idx].ty_inferred = true;
                }
            }

            if let Some(local) = param_local_id(f, symbols, f.params[idx].name) {
                self.locals.insert(
                    local,
                    Scheme {
                        vars: Vec::new(),
                        ty: param_ty.clone(),
                    },
                );
            }
            param_tys.push(param_ty);
        }

        let body_ty = self.infer_block(&mut f.body, symbols);
        let mut ret_ty = self.subst.apply(&body_ty);
        for explicit_ret in &self.return_tys {
            ret_ty = hm_join(&ret_ty, &self.subst.apply(explicit_ret));
        }
        ret_ty = self.subst.apply(&ret_ty);

        if let Some(explicit) = f.ret_ty.as_ref().and_then(hm_ty_from_hir_ty) {
            let _ = unify(&ret_ty, &explicit, &mut self.subst);
            ret_ty = self.subst.apply(&explicit);
        } else if ret_hint_safe_for_fn(f)
            && let Some(hint) = hm_ty_to_hir_ty(&ret_ty)
        {
            f.ret_ty = Some(hint);
            f.ret_ty_inferred = true;
        }

        for (param, param_ty) in f.params.iter_mut().zip(param_tys.iter()) {
            if param.ty.is_none()
                && let Some(hint) = hm_ty_to_hir_ty(&self.subst.apply(param_ty))
            {
                param.ty = Some(hint);
                param.ty_inferred = true;
            }
        }

        let fn_ty = HmTy::Function(
            param_tys
                .iter()
                .map(|param_ty| self.subst.apply(param_ty))
                .collect(),
            Box::new(ret_ty),
        );
        close_scheme(self.subst.apply(&fn_ty))
    }

    pub(crate) fn infer_block(
        &mut self,
        block: &mut HirBlock,
        symbols: &FxHashMap<SymbolId, String>,
    ) -> HmTy {
        let saved_locals = self.locals.clone();
        let mut last = HmTy::Null;
        for stmt in &mut block.stmts {
            last = self.infer_stmt(stmt, symbols);
        }
        self.locals = saved_locals;
        self.subst.apply(&last)
    }

    pub(crate) fn infer_stmt(
        &mut self,
        stmt: &mut HirStmt,
        symbols: &FxHashMap<SymbolId, String>,
    ) -> HmTy {
        match stmt {
            HirStmt::Let {
                local, ty, init, ..
            } => {
                let init_ty = init
                    .as_ref()
                    .map(|expr| self.infer_expr(expr, symbols))
                    .unwrap_or(HmTy::Null);
                let final_ty = if let Some(explicit) = ty.as_ref().and_then(hm_ty_from_hir_ty) {
                    let _ = unify(&explicit, &init_ty, &mut self.subst);
                    self.subst.apply(&explicit)
                } else {
                    let inferred = self.subst.apply(&init_ty);
                    if let Some(hint) = hm_ty_to_hir_ty(&inferred) {
                        *ty = Some(hint);
                    }
                    inferred
                };
                let scheme = if init.as_ref().is_some_and(is_generalizable_expr) {
                    self.generalize(final_ty.clone())
                } else {
                    Scheme {
                        vars: Vec::new(),
                        ty: final_ty.clone(),
                    }
                };
                self.locals.insert(*local, scheme);
                HmTy::Null
            }
            HirStmt::Assign { target, value, .. } => {
                let value_ty = self.infer_expr(value, symbols);
                self.infer_lvalue(target, symbols);
                if let HirLValue::Local(local) = target {
                    self.locals.insert(
                        *local,
                        Scheme {
                            vars: Vec::new(),
                            ty: value_ty.clone(),
                        },
                    );
                }
                HmTy::Null
            }
            HirStmt::If {
                cond,
                then_blk,
                else_blk,
                ..
            } => {
                let cond_ty = self.infer_expr(cond, symbols);
                self.add_constraint(Constraint {
                    lhs: cond_ty,
                    rhs: HmTy::Logical,
                });
                let then_ty = self.infer_scoped_block(then_blk, symbols);
                let else_ty = else_blk
                    .as_mut()
                    .map(|blk| self.infer_scoped_block(blk, symbols))
                    .unwrap_or(HmTy::Null);
                hm_join(&then_ty, &else_ty)
            }
            HirStmt::While { cond, body, .. } => {
                let cond_ty = self.infer_expr(cond, symbols);
                self.add_constraint(Constraint {
                    lhs: cond_ty,
                    rhs: HmTy::Logical,
                });
                self.infer_scoped_block(body, symbols);
                HmTy::Null
            }
            HirStmt::For { iter, body, .. } => {
                self.infer_for_iter(iter, symbols);
                self.infer_scoped_block(body, symbols);
                HmTy::Null
            }
            HirStmt::Return { value, .. } => {
                let ret_ty = value
                    .as_ref()
                    .map(|expr| self.infer_expr(expr, symbols))
                    .unwrap_or(HmTy::Null);
                self.return_tys.push(ret_ty.clone());
                ret_ty
            }
            HirStmt::Break { .. } | HirStmt::Next { .. } | HirStmt::UnsafeRBlock { .. } => {
                HmTy::Null
            }
            HirStmt::Expr { expr, .. } => self.infer_expr(expr, symbols),
        }
    }

    pub(crate) fn infer_scoped_block(
        &mut self,
        block: &mut HirBlock,
        symbols: &FxHashMap<SymbolId, String>,
    ) -> HmTy {
        let saved_locals = self.locals.clone();
        let ty = self.infer_block(block, symbols);
        self.locals = saved_locals;
        ty
    }

    pub(crate) fn infer_lvalue(
        &mut self,
        target: &mut HirLValue,
        symbols: &FxHashMap<SymbolId, String>,
    ) {
        match target {
            HirLValue::Local(_) => {}
            HirLValue::Index { base, index } => {
                self.infer_expr(base, symbols);
                for idx in index {
                    self.infer_expr(idx, symbols);
                }
            }
            HirLValue::Field { base, .. } => {
                self.infer_expr(base, symbols);
            }
        }
    }

    pub(crate) fn infer_for_iter(
        &mut self,
        iter: &mut HirForIter,
        symbols: &FxHashMap<SymbolId, String>,
    ) {
        match iter {
            HirForIter::Range {
                var, start, end, ..
            } => {
                self.infer_expr(start, symbols);
                self.infer_expr(end, symbols);
                self.locals.insert(
                    *var,
                    Scheme {
                        vars: Vec::new(),
                        ty: HmTy::Int,
                    },
                );
            }
            HirForIter::SeqLen { var, len } => {
                self.infer_expr(len, symbols);
                self.locals.insert(
                    *var,
                    Scheme {
                        vars: Vec::new(),
                        ty: HmTy::Int,
                    },
                );
            }
            HirForIter::SeqAlong { var, xs } => {
                self.infer_expr(xs, symbols);
                self.locals.insert(
                    *var,
                    Scheme {
                        vars: Vec::new(),
                        ty: HmTy::Int,
                    },
                );
            }
        }
    }

    pub(crate) fn infer_expr(
        &mut self,
        expr: &HirExpr,
        symbols: &FxHashMap<SymbolId, String>,
    ) -> HmTy {
        match expr {
            HirExpr::Local(local) => self
                .locals
                .get(local)
                .cloned()
                .map(|scheme| self.instantiate(&scheme))
                .unwrap_or(HmTy::Any),
            HirExpr::Global(sym, _) => symbol_name(symbols, *sym)
                .and_then(|name| self.globals.get(&name).cloned())
                .map(|scheme| self.instantiate(&scheme))
                .unwrap_or(HmTy::Any),
            HirExpr::Lit(lit) => hm_ty_from_lit(lit),
            HirExpr::Call(call) => self.infer_call(call, symbols),
            HirExpr::TidyCall(_) => HmTy::Any,
            HirExpr::Index { base, index } => {
                let base_ty = self.infer_expr(base, symbols);
                for idx in index {
                    self.infer_expr(idx, symbols);
                }
                match self.subst.apply(&base_ty) {
                    HmTy::Vector(inner) | HmTy::Matrix(inner) | HmTy::List(inner) => *inner,
                    _ => HmTy::Any,
                }
            }
            HirExpr::Field { base, name } => {
                let base_ty = self.infer_expr(base, symbols);
                let field_name = symbol_name(symbols, *name).unwrap_or_else(|| name.0.to_string());
                match self.subst.apply(&base_ty) {
                    HmTy::Record(fields) => fields
                        .into_iter()
                        .find_map(|(field, ty)| (field == field_name).then_some(ty))
                        .unwrap_or(HmTy::Any),
                    _ => HmTy::Any,
                }
            }
            HirExpr::Block(block) => {
                let mut block = block.clone();
                self.infer_scoped_block(&mut block, symbols)
            }
            HirExpr::IfExpr {
                cond,
                then_expr,
                else_expr,
            } => {
                let cond_ty = self.infer_expr(cond, symbols);
                self.add_constraint(Constraint {
                    lhs: cond_ty,
                    rhs: HmTy::Logical,
                });
                let then_ty = self.infer_expr(then_expr, symbols);
                let else_ty = self.infer_expr(else_expr, symbols);
                hm_join(&then_ty, &else_ty)
            }
            HirExpr::Match { scrut, arms } => {
                self.infer_expr(scrut, symbols);
                arms.iter()
                    .map(|arm| self.infer_expr(&arm.body, symbols))
                    .fold(HmTy::Any, |acc, ty| {
                        if acc == HmTy::Any {
                            ty
                        } else {
                            hm_join(&acc, &ty)
                        }
                    })
            }
            HirExpr::Some(inner)
            | HirExpr::Ok(inner)
            | HirExpr::Err(inner)
            | HirExpr::Try(inner) => self.infer_expr(inner, symbols),
            HirExpr::None => HmTy::Null,
            HirExpr::Unary { op, expr } => {
                let ty = self.infer_expr(expr, symbols);
                match op {
                    HirUnOp::Not => {
                        self.add_constraint(Constraint {
                            lhs: ty,
                            rhs: HmTy::Logical,
                        });
                        HmTy::Logical
                    }
                    HirUnOp::Neg => numeric_unary_result(&self.subst.apply(&ty)),
                }
            }
            HirExpr::Binary { op, lhs, rhs } => {
                let lhs = self.infer_expr(lhs, symbols);
                let rhs = self.infer_expr(rhs, symbols);
                self.infer_binary(op, lhs, rhs)
            }
            HirExpr::ListLit(fields) => HmTy::Record(
                fields
                    .iter()
                    .map(|(name, value)| {
                        (
                            symbol_name(symbols, *name).unwrap_or_else(|| name.0.to_string()),
                            self.infer_expr(value, symbols),
                        )
                    })
                    .collect(),
            ),
            HirExpr::VectorLit(values) => {
                let elem = values.iter().fold(HmTy::Any, |acc, value| {
                    let value_ty = self.infer_expr(value, symbols);
                    if acc == HmTy::Any {
                        value_ty
                    } else {
                        hm_join(&acc, &value_ty)
                    }
                });
                HmTy::Vector(Box::new(elem))
            }
            HirExpr::Range { start, end } => {
                self.infer_expr(start, symbols);
                self.infer_expr(end, symbols);
                HmTy::Vector(Box::new(HmTy::Int))
            }
            HirExpr::Unquote(inner) => self.infer_expr(inner, symbols),
            HirExpr::Column(_) => HmTy::Any,
        }
    }

    pub(crate) fn infer_call(
        &mut self,
        call: &HirCall,
        symbols: &FxHashMap<SymbolId, String>,
    ) -> HmTy {
        let args = call
            .args
            .iter()
            .map(|arg| match arg {
                HirArg::Pos(expr) => self.infer_expr(expr, symbols),
                HirArg::Named { value, .. } => self.infer_expr(value, symbols),
            })
            .collect::<Vec<_>>();

        if let HirExpr::Global(sym, _) = call.callee.as_ref()
            && let Some(name) = symbol_name(symbols, *sym)
        {
            if let Some(ret) = infer_builtin_call_hint(name.as_str(), &args) {
                return ret;
            }
            if let Some(scheme) = self.globals.get(&name).cloned() {
                return self.apply_function_scheme(scheme, &args);
            }
        }

        let callee_ty = self.infer_expr(&call.callee, symbols);
        match self.subst.apply(&callee_ty) {
            HmTy::Function(params, ret) if params.len() == args.len() => {
                for (param, arg) in params.iter().zip(args.iter()) {
                    let _ = unify(param, arg, &mut self.subst);
                }
                self.subst.apply(&ret)
            }
            _ => HmTy::Any,
        }
    }

    pub(crate) fn apply_function_scheme(&mut self, scheme: Scheme, args: &[HmTy]) -> HmTy {
        match self.instantiate(&scheme) {
            HmTy::Function(params, ret) if params.len() == args.len() => {
                for (param, arg) in params.iter().zip(args.iter()) {
                    let _ = unify(param, arg, &mut self.subst);
                }
                self.subst.apply(&ret)
            }
            _ => HmTy::Any,
        }
    }

    pub(crate) fn infer_binary(&mut self, op: &HirBinOp, lhs: HmTy, rhs: HmTy) -> HmTy {
        match op {
            HirBinOp::Add | HirBinOp::Sub | HirBinOp::Mul => {
                self.constrain_numeric_pair(&lhs, &rhs, false);
                binary_result(op.clone(), &self.subst.apply(&lhs), &self.subst.apply(&rhs))
            }
            HirBinOp::Div | HirBinOp::Mod => {
                self.constrain_numeric_pair(&lhs, &rhs, true);
                binary_result(op.clone(), &self.subst.apply(&lhs), &self.subst.apply(&rhs))
            }
            HirBinOp::And | HirBinOp::Or => {
                self.constrain_logical_pair(&lhs, &rhs);
                binary_result(op.clone(), &self.subst.apply(&lhs), &self.subst.apply(&rhs))
            }
            HirBinOp::Eq
            | HirBinOp::Ne
            | HirBinOp::Lt
            | HirBinOp::Le
            | HirBinOp::Gt
            | HirBinOp::Ge
            | HirBinOp::MatMul => {
                binary_result(op.clone(), &self.subst.apply(&lhs), &self.subst.apply(&rhs))
            }
        }
    }

    pub(crate) fn constrain_logical_pair(&mut self, lhs: &HmTy, rhs: &HmTy) {
        let lhs_applied = self.subst.apply(lhs);
        let rhs_applied = self.subst.apply(rhs);
        let target = logical_join(&lhs_applied, &rhs_applied);

        let _ = unify(lhs, &target, &mut self.subst);
        let _ = unify(rhs, &target, &mut self.subst);
    }

    pub(crate) fn constrain_numeric_pair(&mut self, lhs: &HmTy, rhs: &HmTy, widen_int_vars: bool) {
        let lhs = self.subst.apply(lhs);
        let rhs = self.subst.apply(rhs);
        match (&lhs, &rhs) {
            (HmTy::Var(_), ty) if is_numeric_scalar(ty) => {
                let _ = unify(
                    &lhs,
                    &numeric_param_constraint(ty, widen_int_vars),
                    &mut self.subst,
                );
            }
            (ty, HmTy::Var(_)) if is_numeric_scalar(ty) => {
                let _ = unify(
                    &rhs,
                    &numeric_param_constraint(ty, widen_int_vars),
                    &mut self.subst,
                );
            }
            (HmTy::Vector(lhs_inner), HmTy::Vector(rhs_inner))
            | (HmTy::Matrix(lhs_inner), HmTy::Matrix(rhs_inner)) => {
                self.constrain_numeric_pair(lhs_inner, rhs_inner, widen_int_vars);
            }
            (HmTy::Vector(inner) | HmTy::Matrix(inner), ty)
            | (ty, HmTy::Vector(inner) | HmTy::Matrix(inner))
                if is_numeric_scalar(ty) || matches!(ty, HmTy::Var(_)) =>
            {
                self.constrain_numeric_pair(inner, ty, widen_int_vars);
            }
            _ => {}
        }
    }

    pub(crate) fn instantiate(&mut self, scheme: &Scheme) -> HmTy {
        let replacements = scheme
            .vars
            .iter()
            .map(|var| (*var, self.fresh_var()))
            .collect::<FxHashMap<_, _>>();
        instantiate_ty(&scheme.ty, &replacements)
    }

    pub(crate) fn generalize(&self, ty: HmTy) -> Scheme {
        let ty = self.subst.apply(&ty);
        let mut free = FxHashSet::default();
        collect_free_vars(&ty, &mut free);
        for scheme in self.locals.values() {
            let mut env_free = FxHashSet::default();
            collect_free_vars(&self.subst.apply(&scheme.ty), &mut env_free);
            for var in env_free {
                free.remove(&var);
            }
        }
        Scheme {
            vars: sorted_vars(free),
            ty,
        }
    }
}

pub fn apply_hm_hints(
    program: &mut HirProgram,
    symbols: &FxHashMap<SymbolId, String>,
) -> FxHashMap<String, Scheme> {
    let mut globals = collect_global_schemes(program, symbols);

    for _ in 0..2 {
        for module in &mut program.modules {
            for item in &mut module.items {
                if let HirItem::Fn(f) = item {
                    let mut env = InferEnv::new(globals.clone());
                    let scheme = env.apply_fn(f, symbols);
                    update_function_scheme(&mut globals, f, symbols, scheme);
                }
            }
        }
    }

    for module in &mut program.modules {
        let mut env = InferEnv::new(globals.clone());
        for item in &mut module.items {
            if let HirItem::Stmt(stmt) = item {
                env.infer_stmt(stmt, symbols);
            }
        }
    }

    globals
}

pub(crate) fn collect_global_schemes(
    program: &HirProgram,
    symbols: &FxHashMap<SymbolId, String>,
) -> FxHashMap<String, Scheme> {
    let mut globals = builtin_schemes();
    for module in &program.modules {
        for item in &module.items {
            if let HirItem::Fn(f) = item {
                let params = f
                    .params
                    .iter()
                    .map(|param| {
                        param
                            .ty
                            .as_ref()
                            .and_then(hm_ty_from_hir_ty)
                            .unwrap_or(HmTy::Any)
                    })
                    .collect::<Vec<_>>();
                let ret = f
                    .ret_ty
                    .as_ref()
                    .and_then(hm_ty_from_hir_ty)
                    .unwrap_or(HmTy::Any);
                let scheme = Scheme {
                    vars: Vec::new(),
                    ty: HmTy::Function(params, Box::new(ret)),
                };
                globals.insert(format!("Sym_{}", f.name.0), scheme.clone());
                if let Some(name) = symbol_name(symbols, f.name) {
                    globals.insert(name, scheme);
                }
            }
        }
    }
    globals
}

pub(crate) fn update_function_scheme(
    globals: &mut FxHashMap<String, Scheme>,
    f: &HirFn,
    symbols: &FxHashMap<SymbolId, String>,
    scheme: Scheme,
) {
    globals.insert(format!("Sym_{}", f.name.0), scheme.clone());
    if let Some(name) = symbol_name(symbols, f.name) {
        globals.insert(name, scheme);
    }
}

pub(crate) fn hm_ty_from_hir_ty(ty: &Ty) -> Option<HmTy> {
    match ty {
        Ty::Any | Ty::Never | Ty::Union(_) | Ty::Option(_) | Ty::Result(_, _) => Some(HmTy::Any),
        Ty::Null => Some(HmTy::Null),
        Ty::Logical => Some(HmTy::Logical),
        Ty::Int => Some(HmTy::Int),
        Ty::Double => Some(HmTy::Double),
        Ty::Char => Some(HmTy::Char),
        Ty::Vector(inner) => hm_ty_from_hir_ty(inner).map(|ty| HmTy::Vector(Box::new(ty))),
        Ty::Matrix(inner) => hm_ty_from_hir_ty(inner).map(|ty| HmTy::Matrix(Box::new(ty))),
        Ty::List(inner) => hm_ty_from_hir_ty(inner).map(|ty| HmTy::List(Box::new(ty))),
        Ty::Box(inner) => hm_ty_from_hir_ty(inner),
        Ty::DataFrame(_) => None,
    }
}

pub(crate) fn hm_ty_to_hir_ty(ty: &HmTy) -> Option<Ty> {
    match ty {
        HmTy::Int => Some(Ty::Int),
        HmTy::Double => Some(Ty::Double),
        HmTy::Logical => Some(Ty::Logical),
        HmTy::Char => Some(Ty::Char),
        HmTy::Null => Some(Ty::Null),
        HmTy::Vector(inner) => hm_ty_to_hir_ty(inner).map(|ty| Ty::Vector(Box::new(ty))),
        HmTy::Matrix(inner) => hm_ty_to_hir_ty(inner).map(|ty| Ty::Matrix(Box::new(ty))),
        HmTy::List(inner) => hm_ty_to_hir_ty(inner).map(|ty| Ty::List(Box::new(ty))),
        HmTy::Var(_) | HmTy::Function(_, _) | HmTy::Record(_) | HmTy::Any => None,
    }
}

pub(crate) fn hm_ty_from_lit(lit: &HirLit) -> HmTy {
    match lit {
        HirLit::Int(_) => HmTy::Int,
        HirLit::Double(_) => HmTy::Double,
        HirLit::Char(_) => HmTy::Char,
        HirLit::Bool(_) => HmTy::Logical,
        HirLit::NA => HmTy::Any,
        HirLit::Null => HmTy::Null,
    }
}

pub(crate) fn hm_join(lhs: &HmTy, rhs: &HmTy) -> HmTy {
    match (lhs, rhs) {
        (HmTy::Any, other) | (other, HmTy::Any) => other.clone(),
        (lhs, rhs) if lhs == rhs => lhs.clone(),
        (HmTy::Int, HmTy::Double) | (HmTy::Double, HmTy::Int) => HmTy::Double,
        (HmTy::Vector(lhs), HmTy::Vector(rhs)) => HmTy::Vector(Box::new(hm_join(lhs, rhs))),
        (HmTy::Matrix(lhs), HmTy::Matrix(rhs)) => HmTy::Matrix(Box::new(hm_join(lhs, rhs))),
        (HmTy::List(lhs), HmTy::List(rhs)) => HmTy::List(Box::new(hm_join(lhs, rhs))),
        _ => HmTy::Any,
    }
}

pub(crate) fn numeric_unary_result(ty: &HmTy) -> HmTy {
    match ty {
        HmTy::Int => HmTy::Int,
        HmTy::Double => HmTy::Double,
        _ => HmTy::Any,
    }
}

pub(crate) fn binary_result(op: HirBinOp, lhs: &HmTy, rhs: &HmTy) -> HmTy {
    match op {
        HirBinOp::Add | HirBinOp::Sub | HirBinOp::Mul | HirBinOp::Mod => numeric_join(lhs, rhs),
        HirBinOp::Div => match (lhs, rhs) {
            (HmTy::Int | HmTy::Double, HmTy::Int | HmTy::Double) => HmTy::Double,
            _ => HmTy::Any,
        },
        HirBinOp::MatMul => HmTy::Matrix(Box::new(HmTy::Double)),
        HirBinOp::And | HirBinOp::Or => logical_join(lhs, rhs),
        HirBinOp::Eq | HirBinOp::Ne | HirBinOp::Lt | HirBinOp::Le | HirBinOp::Gt | HirBinOp::Ge => {
            logical_join(lhs, rhs)
        }
    }
}

pub(crate) fn logical_join(lhs: &HmTy, rhs: &HmTy) -> HmTy {
    if is_matrixish(lhs) || is_matrixish(rhs) {
        HmTy::Matrix(Box::new(HmTy::Logical))
    } else if is_vectorish(lhs) || is_vectorish(rhs) {
        HmTy::Vector(Box::new(HmTy::Logical))
    } else {
        HmTy::Logical
    }
}

pub(crate) fn numeric_join(lhs: &HmTy, rhs: &HmTy) -> HmTy {
    match (lhs, rhs) {
        (HmTy::Int, HmTy::Int) => HmTy::Int,
        (HmTy::Int | HmTy::Double, HmTy::Int | HmTy::Double) => HmTy::Double,
        (HmTy::Vector(lhs), HmTy::Vector(rhs)) => HmTy::Vector(Box::new(numeric_join(lhs, rhs))),
        (HmTy::Vector(lhs), rhs) | (rhs, HmTy::Vector(lhs)) => {
            HmTy::Vector(Box::new(numeric_join(lhs, rhs)))
        }
        _ => HmTy::Any,
    }
}

pub(crate) fn is_numeric_scalar(ty: &HmTy) -> bool {
    matches!(ty, HmTy::Int | HmTy::Double)
}

pub(crate) fn numeric_param_constraint(ty: &HmTy, widen_int_vars: bool) -> HmTy {
    match ty {
        HmTy::Int if widen_int_vars => HmTy::Double,
        HmTy::Int => HmTy::Int,
        HmTy::Double => HmTy::Double,
        _ => HmTy::Any,
    }
}

pub(crate) fn infer_builtin_call_hint(callee: &str, args: &[HmTy]) -> Option<HmTy> {
    let callee = callee.strip_prefix("base::").unwrap_or(callee);
    match callee {
        "length" | "nrow" | "ncol" => Some(HmTy::Int),
        "seq_len" | "seq_along" => Some(HmTy::Vector(Box::new(HmTy::Int))),
        "is.na" | "is.finite" => Some(logical_like_first_arg(args.first())),
        "isTRUE" | "isFALSE" => Some(HmTy::Logical),
        "which" => Some(HmTy::Vector(Box::new(HmTy::Int))),
        "numeric" | "double" => Some(HmTy::Vector(Box::new(HmTy::Double))),
        "integer" => Some(HmTy::Vector(Box::new(HmTy::Int))),
        "logical" => Some(HmTy::Vector(Box::new(HmTy::Logical))),
        "character" => Some(HmTy::Vector(Box::new(HmTy::Char))),
        "c" => Some(HmTy::Vector(Box::new(args.iter().fold(
            HmTy::Any,
            |acc, ty| {
                if acc == HmTy::Any {
                    ty.clone()
                } else {
                    hm_join(&acc, ty)
                }
            },
        )))),
        "rep" | "rep.int" => {
            let elem = match args.first() {
                Some(HmTy::Vector(inner)) | Some(HmTy::Matrix(inner)) | Some(HmTy::List(inner)) => {
                    inner.as_ref().clone()
                }
                Some(ty) => ty.clone(),
                None => HmTy::Any,
            };
            Some(HmTy::Vector(Box::new(elem)))
        }
        "sum" | "prod" | "min" | "max" => Some(numeric_join_all(args)),
        "mean" => Some(HmTy::Double),
        "abs" | "pmax" | "pmin" => Some(vectorize_if_needed(numeric_join_all(args), args)),
        _ => None,
    }
}

pub(crate) fn logical_like_first_arg(first: Option<&HmTy>) -> HmTy {
    match first {
        Some(ty) if is_vectorish(ty) => HmTy::Vector(Box::new(HmTy::Logical)),
        _ => HmTy::Logical,
    }
}

pub(crate) fn is_vectorish(ty: &HmTy) -> bool {
    matches!(ty, HmTy::Vector(_) | HmTy::Matrix(_))
}

pub(crate) fn is_matrixish(ty: &HmTy) -> bool {
    matches!(ty, HmTy::Matrix(_))
}

pub(crate) fn numeric_join_all(args: &[HmTy]) -> HmTy {
    args.iter().fold(HmTy::Any, |acc, ty| {
        if acc == HmTy::Any {
            numeric_unary_result(ty)
        } else {
            numeric_join(&acc, ty)
        }
    })
}
pub(crate) fn vectorize_if_needed(elem: HmTy, args: &[HmTy]) -> HmTy {
    if args
        .iter()
        .any(|ty| matches!(ty, HmTy::Vector(_) | HmTy::Matrix(_)))
    {
        HmTy::Vector(Box::new(elem))
    } else {
        elem
    }
}

pub(crate) fn builtin_schemes() -> FxHashMap<String, Scheme> {
    FxHashMap::default()
}

pub(crate) fn instantiate_ty(ty: &HmTy, replacements: &FxHashMap<u32, HmTy>) -> HmTy {
    match ty {
        HmTy::Var(var) => replacements.get(var).cloned().unwrap_or(HmTy::Var(*var)),
        HmTy::Vector(inner) => HmTy::Vector(Box::new(instantiate_ty(inner, replacements))),
        HmTy::Matrix(inner) => HmTy::Matrix(Box::new(instantiate_ty(inner, replacements))),
        HmTy::List(inner) => HmTy::List(Box::new(instantiate_ty(inner, replacements))),
        HmTy::Record(fields) => HmTy::Record(
            fields
                .iter()
                .map(|(name, ty)| (name.clone(), instantiate_ty(ty, replacements)))
                .collect(),
        ),
        HmTy::Function(params, ret) => HmTy::Function(
            params
                .iter()
                .map(|param| instantiate_ty(param, replacements))
                .collect(),
            Box::new(instantiate_ty(ret, replacements)),
        ),
        HmTy::Int | HmTy::Double | HmTy::Logical | HmTy::Char | HmTy::Null | HmTy::Any => {
            ty.clone()
        }
    }
}

pub(crate) fn collect_free_vars(ty: &HmTy, out: &mut FxHashSet<u32>) {
    match ty {
        HmTy::Var(var) => {
            out.insert(*var);
        }
        HmTy::Vector(inner) | HmTy::Matrix(inner) | HmTy::List(inner) => {
            collect_free_vars(inner, out)
        }
        HmTy::Record(fields) => {
            for (_, ty) in fields {
                collect_free_vars(ty, out);
            }
        }
        HmTy::Function(params, ret) => {
            for param in params {
                collect_free_vars(param, out);
            }
            collect_free_vars(ret, out);
        }
        HmTy::Int | HmTy::Double | HmTy::Logical | HmTy::Char | HmTy::Null | HmTy::Any => {}
    }
}

pub(crate) fn close_scheme(ty: HmTy) -> Scheme {
    let mut free = FxHashSet::default();
    collect_free_vars(&ty, &mut free);
    Scheme {
        vars: sorted_vars(free),
        ty,
    }
}

pub(crate) fn sorted_vars(vars: FxHashSet<u32>) -> Vec<u32> {
    let mut vars = vars.into_iter().collect::<Vec<_>>();
    vars.sort_unstable();
    vars
}
