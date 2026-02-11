use crate::utils::Span;

#[derive(Debug, Clone)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    Let {
        name: String,
        ty_hint: Option<String>,
        init: Option<Expr>,
    },
    Assign {
        target: LValue,
        value: Expr,
    },
    FnDecl {
        name: String,
        params: Vec<FnParam>,
        ret_ty_hint: Option<String>,
        body: Block,
    }, // Global fn
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
    Import(String), // import "path"
    Export(FnDecl), // export fn
}

#[derive(Debug, Clone)]
pub struct FnParam {
    pub name: String,
    pub ty_hint: Option<String>,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<FnParam>,
    pub ret_ty_hint: Option<String>,
    pub body: Block,
    pub public: bool, // export
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct LValue {
    pub kind: LValueKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum LValueKind {
    Name(String),
    Index { base: Expr, idx: Vec<Expr> }, // x[i], m[i,j]
    Field { base: Expr, name: String },   // obj.x
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Lit(Lit),
    Name(String),

    Unary {
        op: UnaryOp,
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
        ret_ty_hint: Option<String>,
        body: Block,
    }, // fn(x, y) { ... }

    Call {
        callee: Box<Expr>,
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

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pat: Pattern,
    pub guard: Option<Box<Expr>>,
    pub body: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

impl Pattern {
    pub fn span(&self) -> Span {
        self.span
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
