use crate::utils::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TypeExpr {
    Named(String),
    Generic { base: String, args: Vec<TypeExpr> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StmtKind {
    Let {
        name: String,
        ty_hint: Option<TypeExpr>,
        init: Option<Expr>,
    },
    Assign {
        target: LValue,
        value: Expr,
    },
    FnDecl {
        name: String,
        type_params: Vec<String>,
        params: Vec<FnParam>,
        ret_ty_hint: Option<TypeExpr>,
        where_bounds: Vec<TraitBound>,
        body: Block,
    }, // Global fn
    TraitDecl(TraitDecl),
    ImplDecl(ImplDecl),
    If {
        cond: Expr,
        then_blk: Block,
        else_blk: Option<Block>,
    },
    While {
        cond: Expr,
        body: Block,
    },
    For {
        var: String,
        iter: Expr,
        body: Block,
    },
    Return {
        value: Option<Expr>,
    },
    Break,
    Next,
    ExprStmt {
        expr: Expr,
    },
    Expr(Expr),
    Import {
        source: ImportSource,
        path: String,
        spec: ImportSpec,
    }, // import "path" | import r "pkg" | import r default from "pkg" | import r { foo as bar } from "pkg" | import r * as ns from "pkg"
    Export(FnDecl), // export fn
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportSource {
    Module,
    RPackage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImportSpec {
    Glob,
    Named(Vec<ImportBinding>),
    Namespace(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportBinding {
    pub imported: String,
    pub local: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FnParam {
    pub name: String,
    pub ty_hint: Option<TypeExpr>,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FnDecl {
    pub name: String,
    pub type_params: Vec<String>,
    pub params: Vec<FnParam>,
    pub ret_ty_hint: Option<TypeExpr>,
    pub where_bounds: Vec<TraitBound>,
    pub body: Block,
    pub public: bool, // export
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitDecl {
    pub name: String,
    pub type_params: Vec<String>,
    pub supertraits: Vec<String>,
    pub where_bounds: Vec<TraitBound>,
    #[serde(default)]
    pub assoc_types: Vec<TraitAssocType>,
    #[serde(default)]
    pub assoc_consts: Vec<TraitAssocConst>,
    pub methods: Vec<TraitMethodSig>,
    #[serde(default)]
    pub public: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplDecl {
    pub trait_name: String,
    pub type_params: Vec<String>,
    #[serde(default)]
    pub negative: bool,
    pub for_ty: TypeExpr,
    pub where_bounds: Vec<TraitBound>,
    #[serde(default)]
    pub assoc_types: Vec<ImplAssocType>,
    #[serde(default)]
    pub assoc_consts: Vec<ImplAssocConst>,
    pub methods: Vec<FnDecl>,
    #[serde(default)]
    pub public: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraitBound {
    pub type_name: String,
    pub trait_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitAssocType {
    pub name: String,
    #[serde(default)]
    pub type_params: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitAssocConst {
    pub name: String,
    pub ty_hint: TypeExpr,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplAssocType {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplAssocConst {
    pub name: String,
    pub ty_hint: TypeExpr,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitMethodSig {
    pub name: String,
    pub params: Vec<FnParam>,
    pub ret_ty_hint: Option<TypeExpr>,
    pub where_bounds: Vec<TraitBound>,
    pub default_body: Option<Block>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LValue {
    pub kind: LValueKind,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LValueKind {
    Name(String),
    Index { base: Expr, idx: Vec<Expr> }, // x[i], m[i,j]
    Field { base: Expr, name: String },   // obj.x
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExprKind {
    Lit(Lit),
    Name(String),

    Unary {
        op: UnaryOp,
        rhs: Box<Expr>,
    },
    Formula {
        lhs: Option<Box<Expr>>,
        rhs: Box<Expr>,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },

    Range {
        a: Box<Expr>,
        b: Box<Expr>,
    }, // a..b

    Lambda {
        params: Vec<FnParam>,
        ret_ty_hint: Option<TypeExpr>,
        body: Block,
    }, // fn(x, y) { ... }

    Call {
        callee: Box<Expr>,
        type_args: Vec<TypeExpr>,
        args: Vec<Expr>,
    },
    NamedArg {
        name: String,
        value: Box<Expr>,
    }, // only valid inside Call args
    Index {
        base: Box<Expr>,
        idx: Vec<Expr>,
    },
    Field {
        base: Box<Expr>,
        name: String,
    },

    VectorLit(Vec<Expr>),
    RecordLit(Vec<(String, Expr)>),

    // Pipe logic is handled during parsing to nested Calls, but if we keep it in AST:
    Pipe {
        lhs: Box<Expr>,
        rhs_call: Box<Expr>,
    },

    // v6.0 Features
    Try {
        expr: Box<Expr>,
    }, // expr?
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    ColRef(String),     // @col
    Unquote(Box<Expr>), // ^expr
    Column(String),     // @name
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchArm {
    pub pat: Pattern,
    pub guard: Option<Box<Expr>>,
    pub body: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

impl Pattern {
    pub fn span(&self) -> Span {
        self.span
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternKind {
    Wild,
    Lit(Lit),
    Bind(String),
    List {
        items: Vec<Pattern>,
        rest: Option<String>,
    }, // [a, b, ..rest]
    Record {
        fields: Vec<(String, Pattern)>,
    }, // {a: x, b: 1}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Lit {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Null,
    Na,
}

impl Eq for Lit {}
impl std::hash::Hash for Lit {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Lit::Int(i) => i.hash(state),
            Lit::Float(f) => f.to_bits().hash(state),
            Lit::Str(s) => s.hash(state),
            Lit::Bool(b) => b.hash(state),
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnaryOp {
    Neg,
    Not,
    Formula,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    MatMul,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}
