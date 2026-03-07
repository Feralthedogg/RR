use rustc_hash::FxHashMap;

use super::term::TypeTerm;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TyVar(pub u32);

#[derive(Debug, Clone)]
pub enum TypeConstraint {
    Bind(TyVar, TypeTerm),
    Eq(TyVar, TyVar),
    ElementOf { container: TyVar, element: TyVar },
    Unbox { boxed: TyVar, value: TyVar },
}

#[derive(Debug, Default)]
pub struct ConstraintSet {
    next_var: u32,
    constraints: Vec<TypeConstraint>,
    bindings: FxHashMap<TyVar, TypeTerm>,
}

impl ConstraintSet {
    pub fn fresh_var(&mut self) -> TyVar {
        let id = TyVar(self.next_var);
        self.next_var += 1;
        id
    }

    pub fn add(&mut self, c: TypeConstraint) {
        self.constraints.push(c);
    }

    pub fn solve(&mut self) {
        let mut changed = true;
        let mut guard = 0usize;
        let constraints = &self.constraints;
        let bindings = &mut self.bindings;
        while changed && guard < 64 {
            guard += 1;
            changed = false;
            for constraint in constraints {
                if Self::apply_constraint(bindings, constraint) {
                    changed = true;
                }
            }
        }
    }

    pub fn resolve(&self, v: TyVar) -> TypeTerm {
        self.bindings.get(&v).cloned().unwrap_or(TypeTerm::Any)
    }

    fn apply_constraint(
        bindings: &mut FxHashMap<TyVar, TypeTerm>,
        constraint: &TypeConstraint,
    ) -> bool {
        match constraint {
            TypeConstraint::Bind(v, term) => Self::bind_var_ref(bindings, *v, term),
            TypeConstraint::Eq(a, b) => {
                let ta = Self::resolve_in(bindings, *a);
                let tb = Self::resolve_in(bindings, *b);
                let mut changed = false;
                changed |= Self::bind_var_ref(bindings, *a, &tb);
                changed |= Self::bind_var_ref(bindings, *b, &ta);
                changed
            }
            TypeConstraint::ElementOf { container, element } => {
                let container_term = Self::resolve_in(bindings, *container);
                let element_term = container_term.index_element();
                Self::bind_var_ref(bindings, *element, &element_term)
            }
            TypeConstraint::Unbox { boxed, value } => {
                let boxed_term = Self::resolve_in(bindings, *boxed);
                let value_term = boxed_term.unbox();
                Self::bind_var_ref(bindings, *value, &value_term)
            }
        }
    }

    fn resolve_in(bindings: &FxHashMap<TyVar, TypeTerm>, v: TyVar) -> TypeTerm {
        bindings.get(&v).cloned().unwrap_or(TypeTerm::Any)
    }

    fn bind_var_ref(bindings: &mut FxHashMap<TyVar, TypeTerm>, v: TyVar, term: &TypeTerm) -> bool {
        let next = if let Some(prev) = bindings.get(&v) {
            prev.join(term)
        } else {
            term.clone()
        };
        let changed = bindings.get(&v) != Some(&next);
        if changed {
            bindings.insert(v, next);
        }
        changed
    }
}
