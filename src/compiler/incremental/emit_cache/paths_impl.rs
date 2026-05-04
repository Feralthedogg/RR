use super::*;
impl DiskFnEmitCache {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }
    pub(crate) fn paths(&self, key: &str) -> (PathBuf, PathBuf) {
        (
            self.root.join(format!("{}.Rfn", key)),
            self.root.join(format!("{}.map", key)),
        )
    }
    pub(crate) fn function_emit_meta_path(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.fn.meta", key))
    }
    pub(crate) fn peephole_paths(&self, key: &str) -> (PathBuf, PathBuf) {
        (
            self.root.join(format!("{}.Rpee", key)),
            self.root.join(format!("{}.linemap", key)),
        )
    }
    pub(crate) fn peephole_meta_path(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.pee.meta", key))
    }
    pub(crate) fn raw_rewrite_path(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.Rraw", key))
    }
    pub(crate) fn raw_rewrite_meta_path(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.raw.meta", key))
    }
    pub(crate) fn optimized_fragment_paths(&self, key: &str) -> (PathBuf, PathBuf) {
        (
            self.root.join(format!("{}.Roptfn", key)),
            self.root.join(format!("{}.optmap", key)),
        )
    }
    pub(crate) fn optimized_fragment_meta_path(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.optfrag.meta", key))
    }
    pub(crate) fn optimized_assembly_safe_path(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.optok", key))
    }
    pub(crate) fn optimized_assembly_artifact_paths(&self, key: &str) -> (PathBuf, PathBuf) {
        (
            self.root.join(format!("{}.Roptasm", key)),
            self.root.join(format!("{}.optasm.map", key)),
        )
    }
    pub(crate) fn optimized_assembly_meta_path(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.optasm.meta", key))
    }
    pub(crate) fn optimized_assembly_source_map_path(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.optfinal.map", key))
    }
    pub(crate) fn optimized_raw_assembly_safe_path(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.optrawok", key))
    }
    pub(crate) fn optimized_peephole_assembly_safe_path(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.optpeeok", key))
    }
}
