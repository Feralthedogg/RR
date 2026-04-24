use std::path::PathBuf;

#[derive(Default)]
pub(super) struct WatchState {
    pub(super) last_successful_snapshot: Option<Vec<(PathBuf, u64)>>,
    pub(super) last_successful_fingerprint: Option<u64>,
    pub(super) last_successful_output_hash: Option<u64>,
    last_announced_fingerprint: Option<u64>,
    reported_idle_wait: bool,
}

impl WatchState {
    pub(super) fn output_hash(&self) -> Option<u64> {
        self.last_successful_output_hash
    }

    pub(super) fn unchanged_and_output_current(
        &self,
        fingerprint: u64,
        output_current: bool,
    ) -> bool {
        self.last_successful_fingerprint == Some(fingerprint) && output_current
    }

    pub(super) fn should_report_idle_wait(&mut self) -> bool {
        if self.reported_idle_wait {
            false
        } else {
            self.reported_idle_wait = true;
            true
        }
    }

    pub(super) fn resume_after_rebuild_candidate(&mut self) {
        self.reported_idle_wait = false;
    }

    pub(super) fn should_announce_change(&mut self, fingerprint: u64) -> bool {
        if self.last_announced_fingerprint == Some(fingerprint) {
            false
        } else {
            self.last_announced_fingerprint = Some(fingerprint);
            true
        }
    }

    pub(super) fn output_was_modified_or_missing(
        &self,
        fingerprint: u64,
        output_current: bool,
    ) -> bool {
        self.last_successful_fingerprint == Some(fingerprint)
            && self.last_successful_output_hash.is_some()
            && !output_current
    }

    pub(super) fn record_success(
        &mut self,
        snapshot: Vec<(PathBuf, u64)>,
        fingerprint: u64,
        output_hash: u64,
    ) {
        self.last_successful_snapshot = Some(snapshot);
        self.last_successful_fingerprint = Some(fingerprint);
        self.last_successful_output_hash = Some(output_hash);
    }
}
