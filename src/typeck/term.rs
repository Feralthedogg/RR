use crate::hir::def::Ty;
use crate::syntax::ast::Lit;

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
            | (Self::Matrix(a), Self::Matrix(b))
            | (Self::List(a), Self::List(b))
            | (Self::Boxed(a), Self::Boxed(b))
            | (Self::Option(a), Self::Option(b)) => a.compatible_with_inner(b, false),
            (Self::Union(arms), rhs) => arms
                .iter()
                .any(|a| a.compatible_with_inner(rhs, allow_numeric_widen)),
            _ => false,
        }
    }

    pub fn index_element(&self) -> Self {
        match self {
            Self::Vector(inner) | Self::Matrix(inner) | Self::List(inner) | Self::Option(inner) => {
                inner.as_ref().clone()
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
        Ty::List(inner) => TypeTerm::List(Box::new(from_hir_ty(inner))),
        Ty::Box(inner) => TypeTerm::Boxed(Box::new(from_hir_ty(inner))),
        Ty::Option(inner) => TypeTerm::Option(Box::new(from_hir_ty(inner))),
        Ty::Result(ok, err) => TypeTerm::Union(vec![from_hir_ty(ok), from_hir_ty(err)]),
        Ty::Union(xs) => TypeTerm::Union(xs.iter().map(from_hir_ty).collect()),
        Ty::DataFrame(_) => TypeTerm::Any,
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
