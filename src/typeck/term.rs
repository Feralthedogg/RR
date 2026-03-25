use crate::hir::def::{SymbolId, Ty};
use crate::syntax::ast::Lit;
use rustc_hash::FxHashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeTerm {
    Any,
    Never,
    Null,
    Logical,
    Int,
    Double,
    Char,
    Vector(Box<TypeTerm>),
    Matrix(Box<TypeTerm>),
    MatrixDim(Box<TypeTerm>, Option<i64>, Option<i64>),
    DataFrame(Vec<TypeTerm>),
    DataFrameNamed(Vec<(String, TypeTerm)>),
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

        match (self, other) {
            (Self::Int, Self::Double) | (Self::Double, Self::Int) => Self::Double,
            (Self::Vector(a), Self::Vector(b)) => Self::Vector(Box::new(a.join(b))),
            (Self::Matrix(a), Self::Matrix(b)) => Self::Matrix(Box::new(a.join(b))),
            (Self::MatrixDim(a, ar, ac), Self::MatrixDim(b, br, bc)) => Self::MatrixDim(
                Box::new(a.join(b)),
                if ar == br { *ar } else { None },
                if ac == bc { *ac } else { None },
            ),
            (Self::Matrix(a), Self::MatrixDim(b, _, _))
            | (Self::MatrixDim(b, _, _), Self::Matrix(a)) => Self::Matrix(Box::new(a.join(b))),
            (Self::DataFrame(a), Self::DataFrame(b)) if a.len() == b.len() => {
                Self::DataFrame(a.iter().zip(b.iter()).map(|(x, y)| x.join(y)).collect())
            }
            (Self::DataFrameNamed(a), Self::DataFrameNamed(b))
                if a.len() == b.len()
                    && a.iter().zip(b.iter()).all(|((an, _), (bn, _))| an == bn) =>
            {
                Self::DataFrameNamed(
                    a.iter()
                        .zip(b.iter())
                        .map(|((name, x), (_, y))| (name.clone(), x.join(y)))
                        .collect(),
                )
            }
            (Self::DataFrame(a), Self::DataFrameNamed(b))
            | (Self::DataFrameNamed(b), Self::DataFrame(a))
                if a.len() == b.len() =>
            {
                Self::DataFrame(
                    a.iter()
                        .zip(b.iter())
                        .map(|(x, (_, y))| x.join(y))
                        .collect(),
                )
            }
            (Self::List(a), Self::List(b)) => Self::List(Box::new(a.join(b))),
            (Self::Boxed(a), Self::Boxed(b)) => Self::Boxed(Box::new(a.join(b))),
            (Self::Option(a), Self::Option(b)) => Self::Option(Box::new(a.join(b))),
            (Self::Union(xs), rhs) => {
                if xs.iter().any(|x| x == rhs) {
                    self.clone()
                } else {
                    let mut out = xs.clone();
                    out.push(rhs.clone());
                    Self::Union(out)
                }
            }
            (lhs, Self::Union(xs)) => {
                if xs.iter().any(|x| x == lhs) {
                    other.clone()
                } else {
                    let mut out = vec![lhs.clone()];
                    out.extend(xs.clone());
                    Self::Union(out)
                }
            }
            (lhs, rhs) => Self::Union(vec![lhs.clone(), rhs.clone()]),
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
            | (Self::List(a), Self::List(b))
            | (Self::Boxed(a), Self::Boxed(b))
            | (Self::Option(a), Self::Option(b)) => a.compatible_with_inner(b, false),
            (Self::Matrix(a), Self::Matrix(b))
            | (Self::Matrix(a), Self::MatrixDim(b, _, _))
            | (Self::MatrixDim(a, _, _), Self::Matrix(b)) => a.compatible_with_inner(b, false),
            (Self::MatrixDim(a, ar, ac), Self::MatrixDim(b, br, bc)) => {
                a.compatible_with_inner(b, false)
                    && (ar.is_none() || br.is_none() || ar == br)
                    && (ac.is_none() || bc.is_none() || ac == bc)
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
            (Self::Union(arms), rhs) => arms
                .iter()
                .any(|a| a.compatible_with_inner(rhs, allow_numeric_widen)),
            _ => false,
        }
    }

    pub fn index_element(&self) -> Self {
        match self {
            Self::Vector(inner)
            | Self::Matrix(inner)
            | Self::MatrixDim(inner, _, _)
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
            Self::DataFrameNamed(_) => true,
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
