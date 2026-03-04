#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimTy {
    Any,
    Null,
    Logical,
    Int,
    Double,
    Char,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShapeTy {
    Unknown,
    Scalar,
    Vector,
    Matrix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NaTy {
    Maybe,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LenSym(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeState {
    pub prim: PrimTy,
    pub shape: ShapeTy,
    pub na: NaTy,
    pub len_sym: Option<LenSym>,
}

impl TypeState {
    pub const fn unknown() -> Self {
        Self {
            prim: PrimTy::Any,
            shape: ShapeTy::Unknown,
            na: NaTy::Maybe,
            len_sym: None,
        }
    }

    pub const fn null() -> Self {
        Self {
            prim: PrimTy::Null,
            shape: ShapeTy::Scalar,
            na: NaTy::Never,
            len_sym: None,
        }
    }

    pub const fn scalar(prim: PrimTy, non_na: bool) -> Self {
        Self {
            prim,
            shape: ShapeTy::Scalar,
            na: if non_na { NaTy::Never } else { NaTy::Maybe },
            len_sym: None,
        }
    }

    pub const fn vector(prim: PrimTy, non_na: bool) -> Self {
        Self {
            prim,
            shape: ShapeTy::Vector,
            na: if non_na { NaTy::Never } else { NaTy::Maybe },
            len_sym: None,
        }
    }

    pub const fn matrix(prim: PrimTy, non_na: bool) -> Self {
        Self {
            prim,
            shape: ShapeTy::Matrix,
            na: if non_na { NaTy::Never } else { NaTy::Maybe },
            len_sym: None,
        }
    }

    pub fn with_len(mut self, len_sym: Option<LenSym>) -> Self {
        self.len_sym = len_sym;
        self
    }

    pub fn is_unknown(self) -> bool {
        self.prim == PrimTy::Any || self.shape == ShapeTy::Unknown
    }

    pub fn is_logical_scalar_non_na(self) -> bool {
        self.prim == PrimTy::Logical && self.shape == ShapeTy::Scalar && self.na == NaTy::Never
    }

    pub fn is_int_scalar_non_na(self) -> bool {
        self.prim == PrimTy::Int && self.shape == ShapeTy::Scalar && self.na == NaTy::Never
    }

    pub fn is_numeric_vector(self) -> bool {
        self.shape == ShapeTy::Vector && matches!(self.prim, PrimTy::Int | PrimTy::Double)
    }

    pub fn join(self, other: Self) -> Self {
        if self.is_unknown() {
            return other;
        }
        if other.is_unknown() {
            return self;
        }

        let prim = match (self.prim, other.prim) {
            (a, b) if a == b => a,
            (PrimTy::Int, PrimTy::Double) | (PrimTy::Double, PrimTy::Int) => PrimTy::Double,
            _ => PrimTy::Any,
        };

        let shape = if self.shape == other.shape {
            self.shape
        } else {
            ShapeTy::Unknown
        };

        let na = if self.na == NaTy::Never && other.na == NaTy::Never {
            NaTy::Never
        } else {
            NaTy::Maybe
        };

        let len_sym = if self.len_sym.is_some() && self.len_sym == other.len_sym {
            self.len_sym
        } else {
            None
        };

        Self {
            prim,
            shape,
            na,
            len_sym,
        }
    }
}
