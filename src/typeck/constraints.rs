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
        while changed && guard < 64 {
            guard += 1;
            changed = false;
            let constraint_count = self.constraints.len();
            for idx in 0..constraint_count {
                let c = self.constraints[idx].clone();
                if self.apply(c) {
                    changed = true;
                }
            }
        }
    }

    pub fn resolve(&self, v: TyVar) -> TypeTerm {
        self.bindings.get(&v).cloned().unwrap_or(TypeTerm::Any)
    }

    fn apply(&mut self, c: TypeConstraint) -> bool {
        match c {
            TypeConstraint::Bind(v, term) => self.bind_var(v, term),
            TypeConstraint::Eq(a, b) => {
                let ta = self.resolve(a);
                let tb = self.resolve(b);
                let mut changed = false;
                changed |= self.bind_var(a, tb.clone());
                changed |= self.bind_var(b, ta);
                changed
            }
            TypeConstraint::ElementOf { container, element } => {
                let c = self.resolve(container);
                self.bind_var(element, c.index_element())
            }
            TypeConstraint::Unbox { boxed, value } => {
                let b = self.resolve(boxed);
                self.bind_var(value, b.unbox())
            }
        }
    }

    fn bind_var(&mut self, v: TyVar, term: TypeTerm) -> bool {
        let next = if let Some(prev) = self.bindings.get(&v) {
            prev.join(&term)
        } else {
            term
        };
        let changed = self.bindings.get(&v) != Some(&next);
        if changed {
            self.bindings.insert(v, next);
        }
        changed
    }
}
