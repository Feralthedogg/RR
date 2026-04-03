pub use crate::mir::flow::Facts;
pub use crate::syntax::ast::{BinOp, Lit, UnaryOp};
use crate::typeck::{TypeState, TypeTerm};
use crate::utils::Span;
use rustc_hash::FxHashMap;

impl Span {
    pub fn dummy() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapeStatus {
    Local,   // Safe, does not escape
    Escaped, // Escapes function (args, return, globals)
    Unknown, // Default/Analysis pending
}

pub type BlockId = usize;
pub type ValueId = usize;
pub type VarId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteropTier {
    Hybrid,
    Opaque,
}

impl InteropTier {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hybrid => "hybrid",
            Self::Opaque => "opaque",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteropReasonKind {
    DynamicBuiltin,
    PackageCall,
    TidyHelper,
}

impl InteropReasonKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DynamicBuiltin => "dynamic-builtin",
            Self::PackageCall => "package-call",
            Self::TidyHelper => "tidy-helper",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteropReason {
    pub tier: InteropTier,
    pub kind: InteropReasonKind,
    pub callee: Box<str>,
    pub package: Option<Box<str>>,
    pub symbol: Option<Box<str>>,
    pub why: Box<str>,
    pub suggestion: Option<Box<str>>,
}

impl InteropReason {
    pub fn new(
        tier: InteropTier,
        kind: InteropReasonKind,
        callee: impl Into<String>,
        why: impl Into<String>,
        suggestion: Option<impl Into<String>>,
    ) -> Self {
        let callee = callee.into();
        let (package, symbol) = callee
            .split_once("::")
            .map(|(pkg, sym)| {
                (
                    Some(pkg.to_string().into_boxed_str()),
                    Some(sym.to_string().into_boxed_str()),
                )
            })
            .unwrap_or((None, None));
        Self {
            tier,
            kind,
            callee: callee.into_boxed_str(),
            package,
            symbol,
            why: why.into().into_boxed_str(),
            suggestion: suggestion.map(Into::into).map(String::into_boxed_str),
        }
    }

    pub fn summary(&self) -> String {
        let mut parts = vec![
            format!("tier={}", self.tier.as_str()),
            format!("kind={}", self.kind.as_str()),
            format!("call={}", self.callee),
            format!("why={}", self.why),
        ];
        if let Some(pkg) = &self.package {
            parts.push(format!("package={pkg}"));
        }
        if let Some(sym) = &self.symbol {
            parts.push(format!("symbol={sym}"));
        }
        if let Some(suggestion) = &self.suggestion {
            parts.push(format!("suggestion={suggestion}"));
        }
        parts.join(" | ")
    }
}

#[derive(Debug, Clone)]
pub struct FnIR {
    pub name: String,
    pub user_name: Option<String>,
    pub span: Span,
    pub params: Vec<VarId>,
    pub param_default_r_exprs: Vec<Option<String>>,
    pub param_spans: Vec<Span>,
    pub param_ty_hints: Vec<TypeState>,
    pub param_term_hints: Vec<TypeTerm>,
    pub param_hint_spans: Vec<Option<Span>>,
    pub ret_ty_hint: Option<TypeState>,
    pub ret_term_hint: Option<TypeTerm>,
    pub ret_hint_span: Option<Span>,
    pub inferred_ret_ty: TypeState,
    pub inferred_ret_term: TypeTerm,
    pub blocks: Vec<Block>, // indices are BlockIds
    pub values: Vec<Value>, // indices are ValueIds. SSA-like values.
    pub entry: BlockId,
    pub body_head: BlockId, // Actual start after entry prologue
    // Hybrid fallback: this function uses dynamic patterns that are not statically optimizable.
    pub unsupported_dynamic: bool,
    pub fallback_reasons: Vec<String>,
    pub hybrid_interop_reasons: Vec<InteropReason>,
    // Opaque interop: function contains package/runtime calls RR can preserve and execute,
    // but not reason about deeply enough for aggressive optimization.
    pub opaque_interop: bool,
    pub opaque_reasons: Vec<String>,
    pub opaque_interop_reasons: Vec<InteropReason>,
    pub call_semantics: FxHashMap<ValueId, CallSemantics>,
    pub memory_layout_hints: FxHashMap<ValueId, MemoryLayoutHint>,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub id: BlockId,
    pub instrs: Vec<Instr>,
    pub term: Terminator,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Terminator {
    Goto(BlockId),
    If {
        cond: ValueId,
        then_bb: BlockId,
        else_bb: BlockId,
    },
    Return(Option<ValueId>),
    Unreachable,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instr {
    // Standard Assignment: x <- val
    Assign {
        dst: VarId,
        src: ValueId,
        span: Span,
    },
    // Evaluate value for side-effects
    Eval {
        val: ValueId,
        span: Span,
    },

    // Memory Store: x[i] <- val
    StoreIndex1D {
        base: ValueId,
        idx: ValueId,
        val: ValueId,
        is_safe: bool,
        is_na_safe: bool,
        is_vector: bool,
        span: Span,
    },
    // Memory Store: x[r, c] <- val
    StoreIndex2D {
        base: ValueId,
        r: ValueId,
        c: ValueId,
        val: ValueId,
        span: Span,
    },
    // Memory Store: x[i, j, k] <- val
    StoreIndex3D {
        base: ValueId,
        i: ValueId,
        j: ValueId,
        k: ValueId,
        val: ValueId,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub struct Value {
    pub id: ValueId,
    pub kind: ValueKind,
    pub span: Span,                 // Originating source
    pub facts: Facts,               // Type/Range facts
    pub value_ty: TypeState,        // Static type facts
    pub value_term: TypeTerm,       // Structural generic type facts
    pub origin_var: Option<VarId>,  // Original variable name (if any)
    pub phi_block: Option<BlockId>, // Owning block for Phi values
    pub escape: EscapeStatus,       // Optimization: Escape Analysis result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntrinsicOp {
    VecAddF64,
    VecSubF64,
    VecMulF64,
    VecDivF64,
    VecAbsF64,
    VecLogF64,
    VecSqrtF64,
    VecPmaxF64,
    VecPminF64,
    VecSumF64,
    VecMeanF64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinKind {
    Length,
    SeqAlong,
    SeqLen,
    Sum,
    Mean,
    Var,
    Prod,
    Min,
    Max,
    Abs,
    Sqrt,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Atan2,
    Sinh,
    Cosh,
    Tanh,
    Log,
    Log10,
    Log2,
    Exp,
    Sign,
    Gamma,
    Lgamma,
    Floor,
    Ceiling,
    Trunc,
    Round,
    Pmax,
    Pmin,
    IsNa,
    IsFinite,
    Numeric,
    Character,
    Logical,
    Integer,
    Double,
    Rep,
    RepInt,
    Vector,
    Matrix,
    Array,
    Dim,
    Dimnames,
    Nrow,
    Ncol,
    ColSums,
    RowSums,
    Crossprod,
    Tcrossprod,
    Transpose,
    Diag,
    Rbind,
    Cbind,
    C,
    List,
}

impl BuiltinKind {
    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::Length => "length",
            Self::SeqAlong => "seq_along",
            Self::SeqLen => "seq_len",
            Self::Sum => "sum",
            Self::Mean => "mean",
            Self::Var => "var",
            Self::Prod => "prod",
            Self::Min => "min",
            Self::Max => "max",
            Self::Abs => "abs",
            Self::Sqrt => "sqrt",
            Self::Sin => "sin",
            Self::Cos => "cos",
            Self::Tan => "tan",
            Self::Asin => "asin",
            Self::Acos => "acos",
            Self::Atan => "atan",
            Self::Atan2 => "atan2",
            Self::Sinh => "sinh",
            Self::Cosh => "cosh",
            Self::Tanh => "tanh",
            Self::Log => "log",
            Self::Log10 => "log10",
            Self::Log2 => "log2",
            Self::Exp => "exp",
            Self::Sign => "sign",
            Self::Gamma => "gamma",
            Self::Lgamma => "lgamma",
            Self::Floor => "floor",
            Self::Ceiling => "ceiling",
            Self::Trunc => "trunc",
            Self::Round => "round",
            Self::Pmax => "pmax",
            Self::Pmin => "pmin",
            Self::IsNa => "is.na",
            Self::IsFinite => "is.finite",
            Self::Numeric => "numeric",
            Self::Character => "character",
            Self::Logical => "logical",
            Self::Integer => "integer",
            Self::Double => "double",
            Self::Rep => "rep",
            Self::RepInt => "rep.int",
            Self::Vector => "vector",
            Self::Matrix => "matrix",
            Self::Array => "array",
            Self::Dim => "dim",
            Self::Dimnames => "dimnames",
            Self::Nrow => "nrow",
            Self::Ncol => "ncol",
            Self::ColSums => "colSums",
            Self::RowSums => "rowSums",
            Self::Crossprod => "crossprod",
            Self::Tcrossprod => "tcrossprod",
            Self::Transpose => "t",
            Self::Diag => "diag",
            Self::Rbind => "rbind",
            Self::Cbind => "cbind",
            Self::C => "c",
            Self::List => "list",
        }
    }

    pub fn is_floor_like(self) -> bool {
        matches!(self, Self::Floor | Self::Ceiling | Self::Trunc)
    }

    pub fn is_minmax(self) -> bool {
        matches!(self, Self::Min | Self::Max)
    }

    pub fn is_reducer(self) -> bool {
        matches!(
            self,
            Self::Sum | Self::Mean | Self::Var | Self::Prod | Self::Min | Self::Max
        )
    }
}

pub fn builtin_kind_for_name(name: &str) -> Option<BuiltinKind> {
    match name.strip_prefix("base::").unwrap_or(name) {
        "length" => Some(BuiltinKind::Length),
        "seq_along" => Some(BuiltinKind::SeqAlong),
        "seq_len" => Some(BuiltinKind::SeqLen),
        "sum" => Some(BuiltinKind::Sum),
        "mean" => Some(BuiltinKind::Mean),
        "var" => Some(BuiltinKind::Var),
        "prod" => Some(BuiltinKind::Prod),
        "min" => Some(BuiltinKind::Min),
        "max" => Some(BuiltinKind::Max),
        "abs" => Some(BuiltinKind::Abs),
        "sqrt" => Some(BuiltinKind::Sqrt),
        "sin" => Some(BuiltinKind::Sin),
        "cos" => Some(BuiltinKind::Cos),
        "tan" => Some(BuiltinKind::Tan),
        "asin" => Some(BuiltinKind::Asin),
        "acos" => Some(BuiltinKind::Acos),
        "atan" => Some(BuiltinKind::Atan),
        "atan2" => Some(BuiltinKind::Atan2),
        "sinh" => Some(BuiltinKind::Sinh),
        "cosh" => Some(BuiltinKind::Cosh),
        "tanh" => Some(BuiltinKind::Tanh),
        "log" => Some(BuiltinKind::Log),
        "log10" => Some(BuiltinKind::Log10),
        "log2" => Some(BuiltinKind::Log2),
        "exp" => Some(BuiltinKind::Exp),
        "sign" => Some(BuiltinKind::Sign),
        "gamma" => Some(BuiltinKind::Gamma),
        "lgamma" => Some(BuiltinKind::Lgamma),
        "floor" => Some(BuiltinKind::Floor),
        "ceiling" => Some(BuiltinKind::Ceiling),
        "trunc" => Some(BuiltinKind::Trunc),
        "round" => Some(BuiltinKind::Round),
        "pmax" => Some(BuiltinKind::Pmax),
        "pmin" => Some(BuiltinKind::Pmin),
        "is.na" => Some(BuiltinKind::IsNa),
        "is.finite" => Some(BuiltinKind::IsFinite),
        "numeric" => Some(BuiltinKind::Numeric),
        "character" => Some(BuiltinKind::Character),
        "logical" => Some(BuiltinKind::Logical),
        "integer" => Some(BuiltinKind::Integer),
        "double" => Some(BuiltinKind::Double),
        "rep" => Some(BuiltinKind::Rep),
        "rep.int" => Some(BuiltinKind::RepInt),
        "vector" => Some(BuiltinKind::Vector),
        "matrix" => Some(BuiltinKind::Matrix),
        "array" => Some(BuiltinKind::Array),
        "dim" => Some(BuiltinKind::Dim),
        "dimnames" => Some(BuiltinKind::Dimnames),
        "nrow" => Some(BuiltinKind::Nrow),
        "ncol" => Some(BuiltinKind::Ncol),
        "colSums" => Some(BuiltinKind::ColSums),
        "rowSums" => Some(BuiltinKind::RowSums),
        "crossprod" => Some(BuiltinKind::Crossprod),
        "tcrossprod" => Some(BuiltinKind::Tcrossprod),
        "t" => Some(BuiltinKind::Transpose),
        "diag" => Some(BuiltinKind::Diag),
        "rbind" => Some(BuiltinKind::Rbind),
        "cbind" => Some(BuiltinKind::Cbind),
        "c" => Some(BuiltinKind::C),
        "list" => Some(BuiltinKind::List),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallSemantics {
    Builtin(BuiltinKind),
    RuntimeHelper,
    ClosureDispatch,
    UserDefined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryLayoutHint {
    Dense1D,
    ColumnMajor2D,
    ColumnMajorND,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValueKind {
    // Constants
    Const(Lit),

    // SSA Phi Node
    // Merges values from predecessor blocks.
    Phi {
        args: Vec<(ValueId, BlockId)>,
    },

    // Primitives (First-class canonical values)
    Param {
        index: usize,
    }, // Function parameter
    Len {
        base: ValueId,
    }, // length(x)
    Indices {
        base: ValueId,
    }, // 0..len(x)-1
    Range {
        start: ValueId,
        end: ValueId,
    }, // start..end

    // Operations
    Binary {
        op: BinOp,
        lhs: ValueId,
        rhs: ValueId,
    },
    Unary {
        op: UnaryOp,
        rhs: ValueId,
    },

    // Generic Call (User functions, unknown builtins)
    // `names` keeps optional argument labels for `callee(name = value, ...)`.
    Call {
        callee: String,
        args: Vec<ValueId>,
        names: Vec<Option<String>>,
    },

    RecordLit {
        fields: Vec<(String, ValueId)>,
    },
    FieldGet {
        base: ValueId,
        field: String,
    },
    FieldSet {
        base: ValueId,
        field: String,
        value: ValueId,
    },

    Intrinsic {
        op: IntrinsicOp,
        args: Vec<ValueId>,
    },

    // Access (Load is implicit via ValueId, this is for calculating offsets/pointers if needed?)
    Index1D {
        base: ValueId,
        idx: ValueId,
        is_safe: bool,
        is_na_safe: bool,
    },
    Index2D {
        base: ValueId,
        r: ValueId,
        c: ValueId,
    },
    Index3D {
        base: ValueId,
        i: ValueId,
        j: ValueId,
        k: ValueId,
    },

    // Explicit Load from Variable (Critical for TCO/ParallelCopy cycle breaking)
    Load {
        var: VarId,
    },

    // Raw symbol preserved for tidy-eval style package interop.
    // Emits as a bare symbol in generated R, e.g. `trend` not `"trend"`.
    RSymbol {
        name: String,
    },
}

impl FnIR {
    pub fn new(name: String, params: Vec<VarId>) -> Self {
        let param_ty_hints = vec![TypeState::unknown(); params.len()];
        let param_term_hints = vec![TypeTerm::Any; params.len()];
        let param_default_r_exprs = vec![None; params.len()];
        Self {
            name,
            user_name: None,
            span: Span::default(),
            params,
            param_default_r_exprs,
            param_spans: Vec::new(),
            param_ty_hints,
            param_term_hints,
            param_hint_spans: Vec::new(),
            ret_ty_hint: None,
            ret_term_hint: None,
            ret_hint_span: None,
            inferred_ret_ty: TypeState::unknown(),
            inferred_ret_term: TypeTerm::Any,
            blocks: Vec::new(),
            values: Vec::new(),
            entry: 0,
            body_head: 0,
            unsupported_dynamic: false,
            fallback_reasons: Vec::new(),
            hybrid_interop_reasons: Vec::new(),
            opaque_interop: false,
            opaque_reasons: Vec::new(),
            opaque_interop_reasons: Vec::new(),
            call_semantics: FxHashMap::default(),
            memory_layout_hints: FxHashMap::default(),
        }
    }

    pub fn add_value(
        &mut self,
        kind: ValueKind,
        span: Span,
        facts: Facts,
        origin_var: Option<VarId>,
    ) -> ValueId {
        let id = self.values.len();
        self.values.push(Value {
            id,
            kind,
            span,
            facts,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            origin_var,
            phi_block: None,
            escape: EscapeStatus::Unknown,
        });
        id
    }

    pub fn add_block(&mut self) -> BlockId {
        let id = self.blocks.len();
        self.blocks.push(Block {
            id,
            instrs: Vec::new(),
            // Set to a real terminator when the block is finalized.
            term: Terminator::Unreachable,
        });
        id
    }

    pub fn mark_unsupported_dynamic(&mut self, reason: String) {
        self.unsupported_dynamic = true;
        if !self.fallback_reasons.iter().any(|r| r == &reason) {
            self.fallback_reasons.push(reason);
        }
    }

    pub fn mark_hybrid_interop(&mut self, reason: InteropReason) {
        self.unsupported_dynamic = true;
        if !self.hybrid_interop_reasons.iter().any(|r| r == &reason) {
            self.fallback_reasons.push(reason.summary());
            self.hybrid_interop_reasons.push(reason);
        }
    }

    pub fn mark_opaque_interop(&mut self, reason: String) {
        self.opaque_interop = true;
        if !self.opaque_reasons.iter().any(|r| r == &reason) {
            self.opaque_reasons.push(reason);
        }
    }

    pub fn mark_opaque_interop_reason(&mut self, reason: InteropReason) {
        self.opaque_interop = true;
        if !self.opaque_interop_reasons.iter().any(|r| r == &reason) {
            self.opaque_reasons.push(reason.summary());
            self.opaque_interop_reasons.push(reason);
        }
    }

    pub fn requires_conservative_optimization(&self) -> bool {
        self.unsupported_dynamic || self.opaque_interop
    }

    pub fn set_call_semantics(&mut self, value: ValueId, semantics: CallSemantics) {
        self.call_semantics.insert(value, semantics);
    }

    pub fn call_semantics(&self, value: ValueId) -> Option<CallSemantics> {
        self.call_semantics.get(&value).copied()
    }

    pub fn set_memory_layout_hint(&mut self, value: ValueId, layout: MemoryLayoutHint) {
        self.memory_layout_hints.insert(value, layout);
    }

    pub fn memory_layout_hint(&self, value: ValueId) -> Option<MemoryLayoutHint> {
        self.memory_layout_hints.get(&value).copied()
    }
}
