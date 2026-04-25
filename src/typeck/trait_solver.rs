use crate::error::{RR, RRCode, RRException, Stage};
use crate::hir::def::HirTypeRef;
use crate::utils::Span;
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraitObligation {
    pub trait_name: String,
    pub for_ty: HirTypeRef,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraitImplHeader {
    pub trait_name: String,
    pub for_ty: HirTypeRef,
    pub type_params: Vec<String>,
    pub public: bool,
    pub span: Span,
}

impl TraitImplHeader {
    pub fn type_param_set(&self) -> FxHashSet<String> {
        self.type_params.iter().cloned().collect()
    }

    pub fn is_generic(&self) -> bool {
        !self.type_params.is_empty()
            || type_ref_contains_type_param(&self.for_ty, &self.type_param_set())
    }
}

pub fn type_ref_contains_type_param(ty: &HirTypeRef, type_params: &FxHashSet<String>) -> bool {
    match ty {
        HirTypeRef::Named(name) => type_params.contains(name),
        HirTypeRef::Generic { args, .. } => args
            .iter()
            .any(|arg| type_ref_contains_type_param(arg, type_params)),
    }
}

pub fn bind_trait_type_param(
    subst: &mut FxHashMap<String, HirTypeRef>,
    type_param: &str,
    actual: HirTypeRef,
    span: Span,
) -> RR<()> {
    if let Some(prev) = subst.get(type_param) {
        if prev != &actual {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "generic type parameter '{}' inferred as both '{}' and '{}'",
                    type_param,
                    prev.key(),
                    actual.key()
                ),
            )
            .at(span));
        }
    } else {
        subst.insert(type_param.to_string(), actual);
    }
    Ok(())
}

pub fn infer_trait_type_subst(
    type_params: &FxHashSet<String>,
    pattern: &HirTypeRef,
    actual: &HirTypeRef,
    subst: &mut FxHashMap<String, HirTypeRef>,
    span: Span,
) -> RR<bool> {
    match pattern {
        HirTypeRef::Named(name) if type_params.contains(name) => {
            bind_trait_type_param(subst, name, actual.clone(), span)?;
            Ok(true)
        }
        HirTypeRef::Named(name) => {
            Ok(matches!(actual, HirTypeRef::Named(actual_name) if actual_name == name))
        }
        HirTypeRef::Generic { base, args } => {
            let HirTypeRef::Generic {
                base: actual_base,
                args: actual_args,
            } = actual
            else {
                return Ok(false);
            };
            if base != actual_base || args.len() != actual_args.len() {
                return Ok(false);
            }
            for (pattern_arg, actual_arg) in args.iter().zip(actual_args) {
                if !infer_trait_type_subst(type_params, pattern_arg, actual_arg, subst, span)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
    }
}

pub fn trait_impl_patterns_overlap(a: &TraitImplHeader, b: &TraitImplHeader) -> bool {
    if a.trait_name != b.trait_name {
        return false;
    }
    let a_params = a.type_param_set();
    let b_params = b.type_param_set();
    type_patterns_may_overlap(&a.for_ty, &a_params, &b.for_ty, &b_params)
}

pub fn trait_impl_overlap_is_allowed_specialization(
    existing: &TraitImplHeader,
    new_header: &TraitImplHeader,
) -> bool {
    trait_impl_patterns_overlap(existing, new_header)
        && (trait_impl_is_more_specific(existing, new_header)
            || trait_impl_is_more_specific(new_header, existing))
}

pub fn trait_impl_is_more_specific(a: &TraitImplHeader, b: &TraitImplHeader) -> bool {
    if a.trait_name != b.trait_name || !trait_impl_patterns_overlap(a, b) {
        return false;
    }
    let a_params = a.type_param_set();
    let b_params = b.type_param_set();
    type_pattern_is_instance_of(&a.for_ty, &a_params, &b.for_ty, &b_params)
        && !type_pattern_is_instance_of(&b.for_ty, &b_params, &a.for_ty, &a_params)
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct PatternVar {
    side: u8,
    name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PatternTerm {
    Var(PatternVar),
    Named(String),
    Generic {
        base: String,
        args: Vec<PatternTerm>,
    },
}

impl PatternTerm {
    fn from_type_ref(ty: &HirTypeRef, params: &FxHashSet<String>, side: u8) -> Self {
        match ty {
            HirTypeRef::Named(name) if params.contains(name) => PatternTerm::Var(PatternVar {
                side,
                name: name.clone(),
            }),
            HirTypeRef::Named(name) => PatternTerm::Named(name.clone()),
            HirTypeRef::Generic { base, args } => PatternTerm::Generic {
                base: base.clone(),
                args: args
                    .iter()
                    .map(|arg| PatternTerm::from_type_ref(arg, params, side))
                    .collect(),
            },
        }
    }
}

fn resolve_pattern_term(
    term: &PatternTerm,
    subst: &FxHashMap<PatternVar, PatternTerm>,
) -> PatternTerm {
    match term {
        PatternTerm::Var(var) => subst
            .get(var)
            .map(|next| resolve_pattern_term(next, subst))
            .unwrap_or_else(|| term.clone()),
        PatternTerm::Named(_) => term.clone(),
        PatternTerm::Generic { base, args } => PatternTerm::Generic {
            base: base.clone(),
            args: args
                .iter()
                .map(|arg| resolve_pattern_term(arg, subst))
                .collect(),
        },
    }
}

fn pattern_occurs(
    needle: &PatternVar,
    term: &PatternTerm,
    subst: &FxHashMap<PatternVar, PatternTerm>,
) -> bool {
    match resolve_pattern_term(term, subst) {
        PatternTerm::Var(var) => &var == needle,
        PatternTerm::Named(_) => false,
        PatternTerm::Generic { args, .. } => {
            args.iter().any(|arg| pattern_occurs(needle, arg, subst))
        }
    }
}

fn unify_pattern_terms(
    lhs: &PatternTerm,
    rhs: &PatternTerm,
    subst: &mut FxHashMap<PatternVar, PatternTerm>,
) -> bool {
    let lhs = resolve_pattern_term(lhs, subst);
    let rhs = resolve_pattern_term(rhs, subst);
    match (lhs, rhs) {
        (PatternTerm::Var(lhs_var), PatternTerm::Var(rhs_var)) if lhs_var == rhs_var => true,
        (PatternTerm::Var(var), term) | (term, PatternTerm::Var(var)) => {
            if pattern_occurs(&var, &term, subst) {
                return false;
            }
            subst.insert(var, term);
            true
        }
        (PatternTerm::Named(lhs), PatternTerm::Named(rhs)) => lhs == rhs,
        (
            PatternTerm::Generic {
                base: lhs_base,
                args: lhs_args,
            },
            PatternTerm::Generic {
                base: rhs_base,
                args: rhs_args,
            },
        ) => {
            lhs_base == rhs_base
                && lhs_args.len() == rhs_args.len()
                && lhs_args
                    .iter()
                    .zip(rhs_args.iter())
                    .all(|(lhs_arg, rhs_arg)| unify_pattern_terms(lhs_arg, rhs_arg, subst))
        }
        _ => false,
    }
}

fn bind_pattern_key(bindings: &mut FxHashMap<String, String>, param: &str, key: String) -> bool {
    if let Some(prev) = bindings.get(param) {
        prev == &key
    } else {
        bindings.insert(param.to_string(), key);
        true
    }
}

fn type_pattern_key(ty: &HirTypeRef) -> String {
    ty.key()
}

fn type_pattern_is_instance_of(
    specific: &HirTypeRef,
    _specific_params: &FxHashSet<String>,
    general: &HirTypeRef,
    general_params: &FxHashSet<String>,
) -> bool {
    fn go(
        specific: &HirTypeRef,
        general: &HirTypeRef,
        general_params: &FxHashSet<String>,
        bindings: &mut FxHashMap<String, String>,
    ) -> bool {
        match general {
            HirTypeRef::Named(name) if general_params.contains(name) => {
                bind_pattern_key(bindings, name, type_pattern_key(specific))
            }
            HirTypeRef::Named(general_name) => {
                matches!(specific, HirTypeRef::Named(specific_name) if specific_name == general_name)
            }
            HirTypeRef::Generic {
                base: general_base,
                args: general_args,
            } => {
                let HirTypeRef::Generic {
                    base: specific_base,
                    args: specific_args,
                } = specific
                else {
                    return false;
                };
                general_base == specific_base
                    && general_args.len() == specific_args.len()
                    && specific_args
                        .iter()
                        .zip(general_args)
                        .all(|(specific_arg, general_arg)| {
                            go(specific_arg, general_arg, general_params, bindings)
                        })
            }
        }
    }

    let mut bindings = FxHashMap::default();
    go(specific, general, general_params, &mut bindings)
}

pub fn type_patterns_may_overlap(
    a: &HirTypeRef,
    a_params: &FxHashSet<String>,
    b: &HirTypeRef,
    b_params: &FxHashSet<String>,
) -> bool {
    let lhs = PatternTerm::from_type_ref(a, a_params, 0);
    let rhs = PatternTerm::from_type_ref(b, b_params, 1);
    let mut subst = FxHashMap::default();
    unify_pattern_terms(&lhs, &rhs, &mut subst)
}

#[derive(Default)]
pub struct TraitSolver {
    impls: Vec<TraitImplHeader>,
}

impl TraitSolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_impl(&mut self, header: TraitImplHeader) -> RR<()> {
        for existing in &self.impls {
            if trait_impl_patterns_overlap(existing, &header) {
                if trait_impl_overlap_is_allowed_specialization(existing, &header) {
                    continue;
                }
                return Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Lower,
                    format!(
                        "overlapping impl of trait '{}' for '{}' conflicts with existing impl for '{}'",
                        header.trait_name,
                        header.for_ty.key(),
                        existing.for_ty.key()
                    ),
                )
                .at(header.span));
            }
        }
        self.impls.push(header);
        Ok(())
    }

    pub fn solve(&self, obligation: &TraitObligation) -> RR<bool> {
        let mut exact_matches = 0usize;
        let mut generic_matches = Vec::new();
        for header in &self.impls {
            if header.trait_name != obligation.trait_name {
                continue;
            }
            let mut subst = FxHashMap::default();
            if infer_trait_type_subst(
                &header.type_param_set(),
                &header.for_ty,
                &obligation.for_ty,
                &mut subst,
                obligation.span,
            )? {
                if header.is_generic() {
                    generic_matches.push(header);
                } else {
                    exact_matches += 1;
                }
            }
        }
        let matches = if exact_matches > 0 {
            exact_matches
        } else {
            (0..generic_matches.len())
                .filter(|candidate_idx| {
                    !(0..generic_matches.len()).any(|other_idx| {
                        other_idx != *candidate_idx
                            && trait_impl_is_more_specific(
                                generic_matches[other_idx],
                                generic_matches[*candidate_idx],
                            )
                    })
                })
                .count()
        };
        if matches > 1 {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Lower,
                format!(
                    "ambiguous impls satisfy trait '{}' for '{}'",
                    obligation.trait_name,
                    obligation.for_ty.key()
                ),
            )
            .at(obligation.span));
        }
        Ok(matches == 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(name: &str) -> HirTypeRef {
        HirTypeRef::Named(name.to_string())
    }

    fn generic(base: &str, args: Vec<HirTypeRef>) -> HirTypeRef {
        HirTypeRef::Generic {
            base: base.to_string(),
            args,
        }
    }

    fn params(names: &[&str]) -> FxHashSet<String> {
        names.iter().map(|name| (*name).to_string()).collect()
    }

    #[test]
    fn overlap_respects_repeated_type_parameters() {
        let repeated = generic("Pair", vec![named("T"), named("T")]);
        let mixed = generic("Pair", vec![named("int"), named("float")]);
        assert!(!type_patterns_may_overlap(
            &repeated,
            &params(&["T"]),
            &mixed,
            &FxHashSet::default()
        ));
    }

    #[test]
    fn overlap_allows_consistent_repeated_type_parameters() {
        let repeated = generic("Pair", vec![named("T"), named("T")]);
        let same = generic("Pair", vec![named("int"), named("int")]);
        assert!(type_patterns_may_overlap(
            &repeated,
            &params(&["T"]),
            &same,
            &FxHashSet::default()
        ));
    }
}
