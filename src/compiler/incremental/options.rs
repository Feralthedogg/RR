use super::*;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IncrementalOptions {
    pub enabled: bool,
    pub auto: bool,
    pub phase1: bool,
    pub phase2: bool,
    pub phase3: bool,
    pub strict_verify: bool,
}

impl IncrementalOptions {
    pub fn disabled() -> Self {
        Self::default()
    }

    pub fn auto() -> Self {
        Self {
            enabled: true,
            auto: true,
            phase1: false,
            phase2: false,
            phase3: false,
            strict_verify: false,
        }
    }

    pub fn phase1_only() -> Self {
        Self {
            enabled: true,
            auto: false,
            phase1: true,
            phase2: false,
            phase3: false,
            strict_verify: false,
        }
    }

    pub fn all_phases() -> Self {
        Self {
            enabled: true,
            auto: false,
            phase1: true,
            phase2: true,
            phase3: true,
            strict_verify: false,
        }
    }

    pub fn resolve(self, has_session: bool) -> Self {
        if !self.enabled || !self.auto {
            return self;
        }
        Self {
            enabled: true,
            auto: false,
            phase1: true,
            phase2: true,
            phase3: has_session,
            strict_verify: self.strict_verify,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IncrementalStats {
    pub phase1_artifact_hit: bool,
    pub phase2_emit_hits: usize,
    pub phase2_emit_misses: usize,
    pub phase3_memory_hit: bool,
    pub strict_verification_checked: bool,
    pub strict_verification_passed: bool,
    pub miss_reasons: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct IncrementalSession {
    pub(crate) phase3_artifacts: FxHashMap<String, CachedArtifact>,
}

#[derive(Clone, Debug, Default)]
pub struct IncrementalCompileOutput {
    pub r_code: String,
    pub source_map: Vec<MapEntry>,
    pub stats: IncrementalStats,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CachedArtifact {
    pub(crate) r_code: String,
    pub(crate) source_map: Vec<MapEntry>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum StrictArtifactTier {
    Phase1Disk,
    Phase3Memory,
}

pub(crate) fn required_artifact_cache_key<'a>(
    cache_key: Option<&'a String>,
    context: &str,
) -> RR<&'a String> {
    cache_key.ok_or_else(|| {
        InternalCompilerError::new(
            Stage::Codegen,
            format!(
                "incremental artifact cache key missing while {}: pipeline invariant violated",
                context
            ),
        )
        .into_exception()
    })
}

impl StrictArtifactTier {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Phase1Disk => "phase1 disk artifact",
            Self::Phase3Memory => "phase3 memory artifact",
        }
    }
}
