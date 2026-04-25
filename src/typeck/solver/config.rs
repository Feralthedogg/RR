#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeMode {
    Strict,
    Gradual,
}

impl TypeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::Gradual => "gradual",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeBackend {
    Off,
    Optional,
    Required,
}

impl NativeBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Optional => "optional",
            Self::Required => "required",
        }
    }
}

impl std::str::FromStr for TypeMode {
    type Err = ();

    fn from_str(v: &str) -> Result<Self, Self::Err> {
        match v.trim().to_ascii_lowercase().as_str() {
            "strict" => Ok(Self::Strict),
            "gradual" => Ok(Self::Gradual),
            _ => Err(()),
        }
    }
}

impl std::str::FromStr for NativeBackend {
    type Err = ();

    fn from_str(v: &str) -> Result<Self, Self::Err> {
        match v.trim().to_ascii_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "optional" => Ok(Self::Optional),
            "required" => Ok(Self::Required),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TypeConfig {
    pub mode: TypeMode,
    pub native_backend: NativeBackend,
}

impl Default for TypeConfig {
    fn default() -> Self {
        Self {
            mode: TypeMode::Strict,
            native_backend: NativeBackend::Off,
        }
    }
}

fn from_hir_ty(ty: &Ty) -> TypeState {
    match ty {
        Ty::Any => TypeState::unknown(),
        Ty::Null => TypeState::null(),
        Ty::Logical => TypeState::scalar(PrimTy::Logical, true),
        Ty::Int => TypeState::scalar(PrimTy::Int, true),
        Ty::Double => TypeState::scalar(PrimTy::Double, true),
        Ty::Char => TypeState::scalar(PrimTy::Char, true),
        Ty::Vector(inner) => TypeState::vector(from_hir_ty(inner).prim, true),
        Ty::Matrix(inner) => TypeState::matrix(from_hir_ty(inner).prim, true),
        Ty::List(_) => TypeState::vector(PrimTy::Any, false),
        Ty::Box(inner) => from_hir_ty(inner),
        Ty::DataFrame(_) => TypeState::matrix(PrimTy::Any, false),
        Ty::Union(xs) => xs
            .iter()
            .map(from_hir_ty)
            .fold(TypeState::unknown(), TypeState::join),
        Ty::Option(inner) => {
            let mut inner_ty = from_hir_ty(inner);
            inner_ty.na = NaTy::Maybe;
            inner_ty
        }
        Ty::Result(ok, err) => from_hir_ty(ok).join(from_hir_ty(err)),
        Ty::Never => TypeState::unknown(),
    }
}

fn from_hir_ty_term(ty: &Ty) -> TypeTerm {
    term_from_hir_ty(ty)
}

fn lit_type(lit: &crate::syntax::ast::Lit) -> TypeState {
    match lit {
        crate::syntax::ast::Lit::Int(_) => TypeState::scalar(PrimTy::Int, true),
        crate::syntax::ast::Lit::Float(_) => TypeState::scalar(PrimTy::Double, true),
        crate::syntax::ast::Lit::Bool(_) => TypeState::scalar(PrimTy::Logical, true),
        crate::syntax::ast::Lit::Str(_) => TypeState::scalar(PrimTy::Char, true),
        crate::syntax::ast::Lit::Null => TypeState::null(),
        crate::syntax::ast::Lit::Na => TypeState::scalar(PrimTy::Any, false),
    }
}

fn normalize_call_numeric_shape(args: &[TypeState]) -> ShapeTy {
    let has_matrix = args.iter().any(|a| a.shape == ShapeTy::Matrix);
    let has_vector = args.iter().any(|a| a.shape == ShapeTy::Vector);
    match (has_matrix, has_vector) {
        (true, true) => ShapeTy::Unknown,
        (true, false) => ShapeTy::Matrix,
        (false, true) => ShapeTy::Vector,
        (false, false) => ShapeTy::Scalar,
    }
}

fn type_state_from_term(term: &TypeTerm) -> TypeState {
    match term {
        TypeTerm::Any | TypeTerm::Never => TypeState::unknown(),
        TypeTerm::Null => TypeState::null(),
        TypeTerm::Logical => TypeState::scalar(PrimTy::Logical, false),
        TypeTerm::Int => TypeState::scalar(PrimTy::Int, false),
        TypeTerm::Double => TypeState::scalar(PrimTy::Double, false),
        TypeTerm::Char => TypeState::scalar(PrimTy::Char, false),
        TypeTerm::Vector(inner) => TypeState::vector(type_state_from_term(inner).prim, false),
        TypeTerm::VectorLen(inner, _) => TypeState::vector(type_state_from_term(inner).prim, false),
        TypeTerm::Matrix(inner) => TypeState::matrix(type_state_from_term(inner).prim, false),
        TypeTerm::MatrixDim(inner, _, _) => {
            TypeState::matrix(type_state_from_term(inner).prim, false)
        }
        TypeTerm::ArrayDim(inner, dims) => {
            let prim = type_state_from_term(inner).prim;
            if dims.len() <= 1 {
                TypeState::vector(prim, false)
            } else {
                TypeState::matrix(prim, false)
            }
        }
        TypeTerm::DataFrame(cols) => {
            let prim = cols
                .iter()
                .map(type_state_from_term)
                .fold(TypeState::unknown(), TypeState::join)
                .prim;
            TypeState::matrix(prim, false)
        }
        TypeTerm::DataFrameNamed(cols) => {
            let prim = cols
                .iter()
                .map(|(_, term)| type_state_from_term(term))
                .fold(TypeState::unknown(), TypeState::join)
                .prim;
            TypeState::matrix(prim, false)
        }
        TypeTerm::NamedList(_) | TypeTerm::List(_) => TypeState::vector(PrimTy::Any, false),
        TypeTerm::Boxed(inner) => type_state_from_term(inner),
        TypeTerm::Option(inner) => {
            let mut inner_ty = type_state_from_term(inner);
            inner_ty.na = NaTy::Maybe;
            inner_ty
        }
        TypeTerm::Union(xs) => xs
            .iter()
            .map(type_state_from_term)
            .fold(TypeState::unknown(), TypeState::join),
    }
}

fn refine_type_with_term(ty: TypeState, term: &TypeTerm) -> TypeState {
    let term_ty = type_state_from_term(term);
    let mut out = ty.join(term_ty);
    if out.len_sym.is_none() {
        out.len_sym = ty.len_sym.or(term_ty.len_sym);
    }
    out
}

fn promoted_numeric_prim(lhs: PrimTy, rhs: PrimTy) -> PrimTy {
    match (lhs, rhs) {
        (PrimTy::Int, PrimTy::Int) => PrimTy::Int,
        (PrimTy::Int, PrimTy::Double)
        | (PrimTy::Double, PrimTy::Int)
        | (PrimTy::Double, PrimTy::Double) => PrimTy::Double,
        (PrimTy::Any, other) | (other, PrimTy::Any) => other,
        _ => PrimTy::Any,
    }
}
