use crate::utils::Span;
use rustc_hash::FxHashMap;

// ----- IDs -----
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ModuleId(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FnId(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LocalId(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SymbolId(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

// ----- Types (Gradual) -----
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Ty {
    Any,
    Never,
    Null,
    Logical,
    Int,
    Double,
    Char,
    Vector(Box<Ty>),
    Matrix(Box<Ty>),
    List(Box<Ty>),
    Box(Box<Ty>),
    DataFrame(Vec<(SymbolId, Ty)>), // schema: col -> type
    Option(Box<Ty>),                // Some/None
    Result(Box<Ty>, Box<Ty>),       // Ok/Err
    Union(Vec<Ty>),                 // merge types
}

// ----- Program Structure -----
#[derive(Clone, Debug)]
pub struct HirProgram {
    pub modules: Vec<HirModule>,
}

#[derive(Clone, Debug)]
pub struct HirModule {
    pub id: ModuleId,
    pub path: Vec<SymbolId>,
    pub items: Vec<HirItem>,
}

#[derive(Clone, Debug)]
pub enum HirItem {
    Import(HirImport),
    Fn(HirFn),
    Export(SymbolId),
    TypeAlias { name: SymbolId, ty: Ty, span: Span },
    Stmt(HirStmt),
}

#[derive(Clone, Debug)]
pub struct HirImport {
    pub module: String, // path
    pub spec: HirImportSpec,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum HirImportSpec {
    Glob,
    Names(Vec<SymbolId>),
}

// ----- Functions -----
#[derive(Clone, Debug)]
pub struct HirFn {
    pub id: FnId,
    pub name: SymbolId,
    pub params: Vec<HirParam>,
    pub has_varargs: bool,
    pub ret_ty: Option<Ty>,
    pub body: HirBlock,
    pub attrs: HirFnAttrs,
    pub span: Span,
    pub local_names: FxHashMap<LocalId, String>,
    pub public: bool, // export modifier
}

#[derive(Clone, Debug)]
pub struct HirFnAttrs {
    pub inline_hint: InlineHint, // default/always/never
    pub tidy_safe: bool,         // NSE-friendly (default false)
}

#[derive(Clone, Debug)]
pub enum InlineHint {
    Default,
    Always,
    Never,
}

#[derive(Clone, Debug)]
pub struct HirParam {
    pub name: SymbolId,
    pub ty: Option<Ty>,
    pub default: Option<HirExpr>,
    pub span: Span,
}

// ----- Blocks / Statements -----
#[derive(Clone, Debug)]
pub struct HirBlock {
    pub stmts: Vec<HirStmt>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum HirStmt {
    Let {
        local: LocalId,
        name: SymbolId,
        ty: Option<Ty>,
        init: Option<HirExpr>,
        span: Span,
    },
    Assign {
        target: HirLValue,
        value: HirExpr,
        span: Span,
    },
    If {
        cond: HirExpr,
        then_blk: HirBlock,
        else_blk: Option<HirBlock>,
        span: Span,
    },
    While {
        cond: HirExpr,
        body: HirBlock,
        span: Span,
    },
    For {
        iter: HirForIter,
        body: HirBlock,
        span: Span,
    }, // canonicalized early if possible
    Return {
        value: Option<HirExpr>,
        span: Span,
    },
    Break {
        span: Span,
    },
    Next {
        span: Span,
    },
    Expr {
        expr: HirExpr,
        span: Span,
    },
}

#[derive(Clone, Debug)]
pub enum HirForIter {
    Range {
        var: LocalId,
        start: HirExpr,
        end: HirExpr,
        inclusive: bool,
    }, // e.g. 1..=n
    SeqLen {
        var: LocalId,
        len: HirExpr,
    }, // seq_len(n)
    SeqAlong {
        var: LocalId,
        xs: HirExpr,
    }, // seq_along(x)
}

// LValues
#[derive(Clone, Debug)]
pub enum HirLValue {
    Local(LocalId),
    Index { base: HirExpr, index: Vec<HirExpr> }, // y[i], m[i,j]
    Field { base: HirExpr, name: SymbolId },      // obj.x
}

// ----- Expressions -----
#[derive(Clone, Debug)]
pub enum HirExpr {
    Local(LocalId),
    Global(SymbolId, Span),
    Lit(HirLit),

    Call(HirCall),         // normal call
    TidyCall(HirTidyCall), // dplyr pipeline node (special)

    Index {
        base: Box<HirExpr>,
        index: Vec<HirExpr>,
    },
    Field {
        base: Box<HirExpr>,
        name: SymbolId,
    },

    Block(HirBlock), // { stmt; expr }

    IfExpr {
        cond: Box<HirExpr>,
        then_expr: Box<HirExpr>,
        else_expr: Box<HirExpr>,
    },

    // Match kept for potential exhaustive checking before desugaring,
    // or as intermediate node.
    Match {
        scrut: Box<HirExpr>,
        arms: Vec<HirMatchArm>,
    },

    // Option/Result constructors
    Some(Box<HirExpr>),
    None,
    Ok(Box<HirExpr>),
    Err(Box<HirExpr>),

    // Try operator
    Try(Box<HirExpr>),

    Unary {
        op: HirUnOp,
        expr: Box<HirExpr>,
    },
    Binary {
        op: HirBinOp,
        lhs: Box<HirExpr>,
        rhs: Box<HirExpr>,
    },

    ListLit(Vec<(SymbolId, HirExpr)>),
    VectorLit(Vec<HirExpr>),
    Range {
        start: Box<HirExpr>,
        end: Box<HirExpr>,
    },

    // v6.0 Phase 3
    Unquote(Box<HirExpr>), // ^expr
    Column(String),        // @name
}

#[derive(Clone, Debug)]
pub struct HirCall {
    pub callee: Box<HirExpr>,
    pub args: Vec<HirArg>, // named args retained for resolution
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum HirArg {
    Pos(HirExpr),
    Named { name: SymbolId, value: HirExpr },
}

// Match
#[derive(Clone, Debug)]
pub struct HirMatchArm {
    pub pat: HirPat,
    pub guard: Option<HirExpr>,
    pub body: HirExpr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum HirPat {
    Wild,
    Lit(HirLit),
    Bind {
        name: SymbolId,
        local: LocalId,
    },
    Or(Vec<HirPat>),

    List {
        items: Vec<HirPat>,
        rest: Option<(SymbolId, LocalId)>,
    },
    Record {
        fields: Vec<(SymbolId, HirPat)>,
    }, // {x: pat, y: pat}
}

// Tidy
#[derive(Clone, Debug)]
pub struct HirTidyCall {
    pub input: Box<HirExpr>, // df (or result of previous tidy op)
    pub verb: TidyVerb,      // filter/mutate/...
    pub args: Vec<TidyArg>,  // expressions with @col/^unquote inside
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum TidyVerb {
    Filter,
    Mutate,
    Select,
    Summarise,
    Arrange,
    GroupBy,
    Rename,
}

#[derive(Clone, Debug)]
pub enum TidyArg {
    Expr(TidyExpr),            // filter(@age > 20)
    Named(SymbolId, TidyExpr), // mutate(height_m = @h / 100)
}

#[derive(Clone, Debug)]
pub enum TidyExpr {
    Lit(HirLit),
    Ident(SymbolId), // helper functions like starts_with
    Col(SymbolId),   // @col
    Env(LocalId),    // ^var (after resolution)
    Unary {
        op: HirUnOp,
        expr: Box<TidyExpr>,
    },
    Binary {
        op: HirBinOp,
        lhs: Box<TidyExpr>,
        rhs: Box<TidyExpr>,
    },
    Call {
        callee: Box<TidyExpr>,
        args: Vec<TidyExpr>,
    },
}

#[derive(Clone, Debug)]
pub enum HirLit {
    Int(i64),
    Double(f64),
    Char(String),
    Bool(bool),
    NA,
    Null,
}

#[derive(Clone, Debug)]
pub enum HirUnOp {
    Not,
    Neg,
}

#[derive(Clone, Debug)]
pub enum HirBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    MatMul,
    And,
    Or,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}
