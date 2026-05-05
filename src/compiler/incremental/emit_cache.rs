use super::*;
pub(crate) struct DiskFnEmitCache {
    pub(crate) root: PathBuf,
}

pub(crate) struct CachedCodeMapArtifactMeta {
    pub(crate) content_hash: u64,
}

#[path = "emit_cache/paths_impl.rs"]
mod paths_impl;
#[path = "emit_cache/trait_impl.rs"]
mod trait_impl;
