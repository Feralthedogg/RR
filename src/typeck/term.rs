use crate::hir::def::{SymbolId, Ty};
use crate::syntax::ast::Lit;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TypeTerm {
    Any,
    Never,
    Null,
    Logical,
    Int,
    Double,
    Char,
    Vector(Box<TypeTerm>),
    VectorLen(Box<TypeTerm>, Option<i64>),
    Matrix(Box<TypeTerm>),
    MatrixDim(Box<TypeTerm>, Option<i64>, Option<i64>),
    ArrayDim(Box<TypeTerm>, Vec<Option<i64>>),
    DataFrame(Vec<TypeTerm>),
    DataFrameNamed(Vec<(String, TypeTerm)>),
    NamedList(Vec<(String, TypeTerm)>),
    List(Box<TypeTerm>),
    Boxed(Box<TypeTerm>),
    Option(Box<TypeTerm>),
    Union(Vec<TypeTerm>),
}

impl TypeTerm {
    pub const fn any() -> Self {
        Self::Any
    }

    pub fn is_any(&self) -> bool {
        matches!(self, Self::Any)
    }

    pub fn join(&self, other: &Self) -> Self {
        if self.is_any() {
            return other.clone();
        }
        if other.is_any() {
            return self.clone();
        }

        if self == other {
            return self.clone();
        }

        if let Some(joined) = Self::join_numeric_terms(self, other) {
            return joined;
        }
        if let Some(joined) = Self::join_vector_terms(self, other) {
            return joined;
        }
        if let Some(joined) = Self::join_matrix_terms(self, other) {
            return joined;
        }
        if let Some(joined) = Self::join_record_terms(self, other) {
            return joined;
        }
        if let Some(joined) = Self::join_wrapped_terms(self, other) {
            return joined;
        }
        Self::join_union_or_fallback(self, other)
    }

    fn join_numeric_terms(lhs: &Self, rhs: &Self) -> Option<Self> {
        matches!(
            (lhs, rhs),
            (Self::Int, Self::Double) | (Self::Double, Self::Int)
        )
        .then_some(Self::Double)
    }

    fn join_vector_terms(lhs: &Self, rhs: &Self) -> Option<Self> {
        match (lhs, rhs) {
            (Self::Vector(a), Self::Vector(b)) => Some(Self::Vector(Box::new(a.join(b)))),
            (Self::VectorLen(a, alen), Self::VectorLen(b, blen)) => Some(Self::VectorLen(
                Box::new(a.join(b)),
                if alen == blen { *alen } else { None },
            )),
            (Self::Vector(a), Self::VectorLen(b, _)) | (Self::VectorLen(b, _), Self::Vector(a)) => {
                Some(Self::Vector(Box::new(a.join(b))))
            }
            _ => None,
        }
    }

    fn join_matrix_terms(lhs: &Self, rhs: &Self) -> Option<Self> {
        match (lhs, rhs) {
            (Self::Matrix(a), Self::Matrix(b)) => Some(Self::Matrix(Box::new(a.join(b)))),
            (Self::MatrixDim(a, ar, ac), Self::MatrixDim(b, br, bc)) => Some(Self::MatrixDim(
                Box::new(a.join(b)),
                if ar == br { *ar } else { None },
                if ac == bc { *ac } else { None },
            )),
            (Self::ArrayDim(a, adims), Self::ArrayDim(b, bdims)) if adims.len() == bdims.len() => {
                Some(Self::ArrayDim(
                    Box::new(a.join(b)),
                    adims
                        .iter()
                        .zip(bdims.iter())
                        .map(|(a, b)| if a == b { *a } else { None })
                        .collect(),
                ))
            }
            (Self::Matrix(a), Self::MatrixDim(b, _, _))
            | (Self::MatrixDim(b, _, _), Self::Matrix(a))
            | (Self::Matrix(a), Self::ArrayDim(b, _))
            | (Self::ArrayDim(b, _), Self::Matrix(a))
            | (Self::MatrixDim(a, _, _), Self::ArrayDim(b, _))
            | (Self::ArrayDim(b, _), Self::MatrixDim(a, _, _)) => {
                Some(Self::Matrix(Box::new(a.join(b))))
            }
            _ => None,
        }
    }

    fn join_record_terms(lhs: &Self, rhs: &Self) -> Option<Self> {
        match (lhs, rhs) {
            (Self::DataFrame(a), Self::DataFrame(b)) if a.len() == b.len() => {
                Some(Self::DataFrame(Self::join_positional_fields(a, b)))
            }
            (Self::DataFrameNamed(a), Self::DataFrameNamed(b))
                if Self::named_fields_match(a, b) =>
            {
                Some(Self::DataFrameNamed(Self::join_named_fields(a, b)))
            }
            (Self::NamedList(a), Self::NamedList(b)) if Self::named_fields_match(a, b) => {
                Some(Self::NamedList(Self::join_named_fields(a, b)))
            }
            (Self::DataFrame(a), Self::DataFrameNamed(b))
            | (Self::DataFrameNamed(b), Self::DataFrame(a))
                if a.len() == b.len() =>
            {
                Some(Self::DataFrame(Self::join_named_rhs_as_positional_fields(
                    a, b,
                )))
            }
            _ => None,
        }
    }

    fn join_wrapped_terms(lhs: &Self, rhs: &Self) -> Option<Self> {
        match (lhs, rhs) {
            (Self::List(a), Self::List(b)) => Some(Self::List(Box::new(a.join(b)))),
            (Self::Boxed(a), Self::Boxed(b)) => Some(Self::Boxed(Box::new(a.join(b)))),
            (Self::Option(a), Self::Option(b)) => Some(Self::Option(Box::new(a.join(b)))),
            _ => None,
        }
    }

    fn join_union_or_fallback(lhs: &Self, rhs: &Self) -> Self {
        match (lhs, rhs) {
            (Self::Union(xs), rhs) => Self::append_to_union(lhs, xs, rhs),
            (lhs, Self::Union(xs)) => Self::prepend_to_union(rhs, lhs, xs),
            (lhs, rhs) => Self::Union(vec![lhs.clone(), rhs.clone()]),
        }
    }

    fn join_positional_fields(lhs: &[Self], rhs: &[Self]) -> Vec<Self> {
        lhs.iter().zip(rhs.iter()).map(|(x, y)| x.join(y)).collect()
    }

    fn named_fields_match(lhs: &[(String, Self)], rhs: &[(String, Self)]) -> bool {
        lhs.len() == rhs.len()
            && lhs
                .iter()
                .zip(rhs.iter())
                .all(|((left_name, _), (right_name, _))| left_name == right_name)
    }

    fn join_named_fields(lhs: &[(String, Self)], rhs: &[(String, Self)]) -> Vec<(String, Self)> {
        lhs.iter()
            .zip(rhs.iter())
            .map(|((name, x), (_, y))| (name.clone(), x.join(y)))
            .collect()
    }

    fn join_named_rhs_as_positional_fields(lhs: &[Self], rhs: &[(String, Self)]) -> Vec<Self> {
        lhs.iter()
            .zip(rhs.iter())
            .map(|(x, (_, y))| x.join(y))
            .collect()
    }

    fn append_to_union(union_term: &Self, members: &[Self], rhs: &Self) -> Self {
        if members.iter().any(|term| term == rhs) {
            union_term.clone()
        } else {
            let mut out = members.to_vec();
            out.push(rhs.clone());
            Self::Union(out)
        }
    }

    fn prepend_to_union(union_term: &Self, lhs: &Self, members: &[Self]) -> Self {
        if members.iter().any(|term| term == lhs) {
            union_term.clone()
        } else {
            let mut out = vec![lhs.clone()];
            out.extend(members.to_vec());
            Self::Union(out)
        }
    }

    pub fn compatible_with(&self, got: &Self) -> bool {
        self.compatible_with_inner(got, true)
    }

    fn compatible_with_inner(&self, got: &Self, allow_numeric_widen: bool) -> bool {
        if self.is_any() || got.is_any() {
            return true;
        }
        if self == got {
            return true;
        }
        match (self, got) {
            // Numeric widening.
            (Self::Double, Self::Int) if allow_numeric_widen => true,
            (Self::Vector(a), Self::Vector(b))
            | (Self::Vector(a), Self::VectorLen(b, _))
            | (Self::VectorLen(a, _), Self::Vector(b))
            | (Self::Boxed(a), Self::Boxed(b))
            | (Self::Option(a), Self::Option(b)) => a.compatible_with_inner(b, false),
            (Self::List(a), Self::List(b)) => a.compatible_with_inner(b, false),
            (Self::VectorLen(a, alen), Self::VectorLen(b, blen)) => {
                a.compatible_with_inner(b, false)
                    && (alen.is_none() || blen.is_none() || alen == blen)
            }
            (Self::Matrix(a), Self::Matrix(b))
            | (Self::Matrix(a), Self::MatrixDim(b, _, _))
            | (Self::MatrixDim(a, _, _), Self::Matrix(b))
            | (Self::Matrix(a), Self::ArrayDim(b, _))
            | (Self::ArrayDim(a, _), Self::Matrix(b)) => a.compatible_with_inner(b, false),
            (Self::MatrixDim(a, ar, ac), Self::MatrixDim(b, br, bc)) => {
                a.compatible_with_inner(b, false)
                    && (ar.is_none() || br.is_none() || ar == br)
                    && (ac.is_none() || bc.is_none() || ac == bc)
            }
            (Self::ArrayDim(a, adims), Self::ArrayDim(b, bdims)) if adims.len() == bdims.len() => {
                a.compatible_with_inner(b, false)
                    && adims
                        .iter()
                        .zip(bdims.iter())
                        .all(|(a, b)| a.is_none() || b.is_none() || a == b)
            }
            (Self::MatrixDim(a, ar, ac), Self::ArrayDim(b, bdims))
            | (Self::ArrayDim(b, bdims), Self::MatrixDim(a, ar, ac)) => {
                a.compatible_with_inner(b, false)
                    && (ar.is_none() || bdims.is_empty() || *ar == bdims[0])
                    && (ac.is_none() || bdims.get(1).is_none() || *ac == bdims[1])
            }
            (Self::DataFrame(a), Self::DataFrame(b)) if a.len() == b.len() => a
                .iter()
                .zip(b.iter())
                .all(|(x, y)| x.compatible_with_inner(y, false)),
            (Self::DataFrameNamed(a), Self::DataFrameNamed(b))
                if a.len() == b.len()
                    && a.iter().zip(b.iter()).all(|((an, _), (bn, _))| an == bn) =>
            {
                a.iter()
                    .zip(b.iter())
                    .all(|((_, x), (_, y))| x.compatible_with_inner(y, false))
            }
            (Self::DataFrame(a), Self::DataFrameNamed(b))
            | (Self::DataFrameNamed(b), Self::DataFrame(a))
                if a.len() == b.len() =>
            {
                a.iter()
                    .zip(b.iter())
                    .all(|(x, (_, y))| x.compatible_with_inner(y, false))
            }
            (Self::NamedList(a), Self::NamedList(b))
                if a.len() == b.len()
                    && a.iter().zip(b.iter()).all(|((an, _), (bn, _))| an == bn) =>
            {
                a.iter()
                    .zip(b.iter())
                    .all(|((_, x), (_, y))| x.compatible_with_inner(y, false))
            }
            (Self::List(a), Self::NamedList(b)) | (Self::NamedList(b), Self::List(a)) => {
                b.iter().all(|(_, y)| a.compatible_with_inner(y, false))
            }
            (Self::Union(arms), rhs) => arms
                .iter()
                .any(|a| a.compatible_with_inner(rhs, allow_numeric_widen)),
            _ => false,
        }
    }

    pub fn index_element(&self) -> Self {
        match self {
            Self::Vector(inner)
            | Self::VectorLen(inner, _)
            | Self::Matrix(inner)
            | Self::MatrixDim(inner, _, _)
            | Self::ArrayDim(inner, _)
            | Self::List(inner)
            | Self::Option(inner) => inner.as_ref().clone(),
            Self::DataFrame(cols) => {
                let mut out = TypeTerm::Any;
                for col in cols {
                    out = out.join(col);
                }
                out
            }
            Self::DataFrameNamed(cols) => {
                let mut out = TypeTerm::Any;
                for (_, col) in cols {
                    out = out.join(col);
                }
                out
            }
            Self::NamedList(fields) => {
                let mut out = TypeTerm::Any;
                for (_, field) in fields {
                    out = out.join(field);
                }
                out
            }
            Self::Union(arms) => {
                let mut out = TypeTerm::Any;
                for arm in arms {
                    out = out.join(&arm.index_element());
                }
                out
            }
            _ => Self::Any,
        }
    }

    pub fn field_value(&self) -> Self {
        self.field_value_named(None)
    }

    pub fn field_value_named(&self, name: Option<&str>) -> Self {
        match self {
            Self::DataFrame(cols) => {
                let mut out = TypeTerm::Any;
                for col in cols {
                    out = out.join(col);
                }
                out
            }
            Self::DataFrameNamed(cols) => {
                if let Some(name) = name
                    && let Some((_, term)) = cols.iter().find(|(field, _)| field == name)
                {
                    return term.clone();
                }
                let mut out = TypeTerm::Any;
                for (_, col) in cols {
                    out = out.join(col);
                }
                out
            }
            Self::NamedList(fields) => {
                if let Some(name) = name
                    && let Some((_, term)) = fields.iter().find(|(field, _)| field == name)
                {
                    return term.clone();
                }
                let mut out = TypeTerm::Any;
                for (_, field) in fields {
                    out = out.join(field);
                }
                out
            }
            Self::Union(arms) => {
                let mut out = TypeTerm::Any;
                for arm in arms {
                    out = out.join(&arm.field_value_named(name));
                }
                out
            }
            _ => Self::Any,
        }
    }

    pub fn has_exact_named_fields(&self) -> bool {
        match self {
            Self::DataFrameNamed(_) | Self::NamedList(_) => true,
            Self::Union(arms) => !arms.is_empty() && arms.iter().all(Self::has_exact_named_fields),
            _ => false,
        }
    }

    pub fn exact_field_value(&self, name: &str) -> Option<Self> {
        match self {
            Self::DataFrameNamed(cols) => cols
                .iter()
                .find(|(field, _)| field == name)
                .map(|(_, term)| term.clone()),
            Self::NamedList(fields) => fields
                .iter()
                .find(|(field, _)| field == name)
                .map(|(_, term)| term.clone()),
            Self::Union(arms) => {
                let mut out: Option<Self> = None;
                for arm in arms {
                    let term = arm.exact_field_value(name)?;
                    out = Some(match out {
                        None => term,
                        Some(prev) => prev.join(&term),
                    });
                }
                out
            }
            _ => None,
        }
    }

    pub fn updated_field_value_named(&self, name: &str, value: &Self) -> Self {
        match self {
            Self::DataFrameNamed(cols) => {
                let mut out = cols.clone();
                if let Some((_, term)) = out.iter_mut().find(|(field, _)| field == name) {
                    *term = value.clone();
                } else {
                    out.push((name.to_string(), value.clone()));
                }
                Self::DataFrameNamed(out)
            }
            Self::DataFrame(cols) => {
                let mut out = cols.clone();
                if out.is_empty() {
                    out.push(value.clone());
                } else {
                    for term in &mut out {
                        *term = term.join(value);
                    }
                }
                Self::DataFrame(out)
            }
            Self::NamedList(fields) => {
                let mut out = fields.clone();
                if let Some((_, term)) = out.iter_mut().find(|(field, _)| field == name) {
                    *term = value.clone();
                } else {
                    out.push((name.to_string(), value.clone()));
                }
                Self::NamedList(out)
            }
            Self::Union(arms) => Self::Union(
                arms.iter()
                    .map(|arm| arm.updated_field_value_named(name, value))
                    .collect(),
            ),
            _ => self.clone(),
        }
    }

    pub fn unbox(&self) -> Self {
        match self {
            Self::Boxed(inner) => inner.as_ref().clone(),
            Self::Union(arms) => {
                let mut out = TypeTerm::Any;
                for arm in arms {
                    out = out.join(&arm.unbox());
                }
                out
            }
            _ => Self::Any,
        }
    }

    pub fn matrix_parts(&self) -> Option<(&TypeTerm, Option<i64>, Option<i64>)> {
        match self {
            Self::Matrix(inner) => Some((inner.as_ref(), None, None)),
            Self::MatrixDim(inner, rows, cols) => Some((inner.as_ref(), *rows, *cols)),
            Self::ArrayDim(inner, dims) => Some((
                inner.as_ref(),
                dims.first().copied().flatten(),
                dims.get(1).copied().flatten(),
            )),
            _ => None,
        }
    }

    pub fn vector_parts(&self) -> Option<(&TypeTerm, Option<i64>)> {
        match self {
            Self::Vector(inner) => Some((inner.as_ref(), None)),
            Self::VectorLen(inner, len) => Some((inner.as_ref(), *len)),
            _ => None,
        }
    }
}

pub fn from_hir_ty(ty: &Ty) -> TypeTerm {
    match ty {
        Ty::Any => TypeTerm::Any,
        Ty::Never => TypeTerm::Never,
        Ty::Null => TypeTerm::Null,
        Ty::Logical => TypeTerm::Logical,
        Ty::Int => TypeTerm::Int,
        Ty::Double => TypeTerm::Double,
        Ty::Char => TypeTerm::Char,
        Ty::Vector(inner) => TypeTerm::Vector(Box::new(from_hir_ty(inner))),
        Ty::Matrix(inner) => TypeTerm::Matrix(Box::new(from_hir_ty(inner))),
        Ty::List(inner) => TypeTerm::List(Box::new(from_hir_ty(inner))),
        Ty::Box(inner) => TypeTerm::Boxed(Box::new(from_hir_ty(inner))),
        Ty::DataFrame(cols) => {
            TypeTerm::DataFrame(cols.iter().map(|(_, ty)| from_hir_ty(ty)).collect())
        }
        Ty::Option(inner) => TypeTerm::Option(Box::new(from_hir_ty(inner))),
        Ty::Result(ok, err) => TypeTerm::Union(vec![from_hir_ty(ok), from_hir_ty(err)]),
        Ty::Union(xs) => TypeTerm::Union(xs.iter().map(from_hir_ty).collect()),
    }
}

pub fn from_hir_ty_with_symbols(ty: &Ty, symbols: &FxHashMap<SymbolId, String>) -> TypeTerm {
    match ty {
        Ty::DataFrame(cols) => TypeTerm::DataFrameNamed(
            cols.iter()
                .map(|(sym, ty)| {
                    (
                        symbols
                            .get(sym)
                            .cloned()
                            .unwrap_or_else(|| format!("field_{}", sym.0)),
                        from_hir_ty_with_symbols(ty, symbols),
                    )
                })
                .collect(),
        ),
        Ty::Vector(inner) => TypeTerm::Vector(Box::new(from_hir_ty_with_symbols(inner, symbols))),
        Ty::Matrix(inner) => TypeTerm::Matrix(Box::new(from_hir_ty_with_symbols(inner, symbols))),
        Ty::List(inner) => TypeTerm::List(Box::new(from_hir_ty_with_symbols(inner, symbols))),
        Ty::Box(inner) => TypeTerm::Boxed(Box::new(from_hir_ty_with_symbols(inner, symbols))),
        Ty::Option(inner) => TypeTerm::Option(Box::new(from_hir_ty_with_symbols(inner, symbols))),
        Ty::Result(ok, err) => TypeTerm::Union(vec![
            from_hir_ty_with_symbols(ok, symbols),
            from_hir_ty_with_symbols(err, symbols),
        ]),
        Ty::Union(xs) => TypeTerm::Union(
            xs.iter()
                .map(|ty| from_hir_ty_with_symbols(ty, symbols))
                .collect(),
        ),
        _ => from_hir_ty(ty),
    }
}

pub fn from_lit(lit: &Lit) -> TypeTerm {
    match lit {
        Lit::Int(_) => TypeTerm::Int,
        Lit::Float(_) => TypeTerm::Double,
        Lit::Bool(_) => TypeTerm::Logical,
        Lit::Str(_) => TypeTerm::Char,
        Lit::Null => TypeTerm::Null,
        Lit::Na => TypeTerm::Any,
    }
}

#[cfg(test)]
mod tests {
    use super::TypeTerm;

    #[test]
    fn joining_matrix_with_unknown_dims_drops_dimensional_precision() {
        let plain = TypeTerm::Matrix(Box::new(TypeTerm::Int));
        let dimensioned = TypeTerm::MatrixDim(Box::new(TypeTerm::Int), Some(2), Some(3));

        assert_eq!(plain.join(&dimensioned), plain);
        assert_eq!(dimensioned.join(&plain), plain);
    }

    #[test]
    fn joining_dimensioned_matrices_keeps_only_shared_dims() {
        let lhs = TypeTerm::MatrixDim(Box::new(TypeTerm::Int), Some(2), Some(3));
        let rhs = TypeTerm::MatrixDim(Box::new(TypeTerm::Int), Some(2), Some(4));

        assert_eq!(
            lhs.join(&rhs),
            TypeTerm::MatrixDim(Box::new(TypeTerm::Int), Some(2), None)
        );
    }
}
