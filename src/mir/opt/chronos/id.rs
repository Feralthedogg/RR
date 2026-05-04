use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::mir::opt) enum ChronosPassId {
    IndexCanonicalize,
    RecordCallSpecialize,
    RecordReturnSpecialize,
    SimplifyCfg,
    Sccp,
    Intrinsics,
    Gvn,
    Simplify,
    TypeSpecialize,
    Inline,
    Outline,
    Poly,
    Vectorize,
    Unroll,
    Tco,
    LoopOpt,
    Licm,
    Sroa,
    Dce,
    DeSsa,
    CopyCleanup,
    FreshAlias,
    FreshAlloc,
    Bce,
}

impl ChronosPassId {
    pub(in crate::mir::opt) const fn timing_name(self) -> &'static str {
        match self {
            Self::IndexCanonicalize => "index_canonicalize",
            Self::RecordCallSpecialize => "sroa_record_call_specialize",
            Self::RecordReturnSpecialize => "sroa_record_return_specialize",
            Self::SimplifyCfg => "simplify_cfg",
            Self::Sccp => "sccp",
            Self::Intrinsics => "intrinsics",
            Self::Gvn => "gvn",
            Self::Simplify => "simplify",
            Self::TypeSpecialize => "type_specialize",
            Self::Inline => "inline",
            Self::Outline => "outline",
            Self::Poly => "poly",
            Self::Vectorize => "vectorize",
            Self::Unroll => "unroll",
            Self::Tco => "tco",
            Self::LoopOpt => "loop_opt",
            Self::Licm => "licm",
            Self::Sroa => "sroa",
            Self::Dce => "dce",
            Self::DeSsa => "de_ssa",
            Self::CopyCleanup => "copy_cleanup",
            Self::FreshAlias => "fresh_alias",
            Self::FreshAlloc => "fresh_alloc",
            Self::Bce => "bce",
        }
    }
}
