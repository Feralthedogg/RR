pub use crate::mir::flow::Facts;
pub use crate::syntax::ast::{BinOp, Lit, UnaryOp};
use crate::typeck::{TypeState, TypeTerm};
use crate::utils::Span;

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
    pub span: Span,
    pub params: Vec<VarId>,
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
        Self {
            name,
            span: Span::default(),
            params,
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
}
