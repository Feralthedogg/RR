use RR::error::{RRCode, RRException, Stage};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

struct ScopedCompileCacheOverride {
    previous: Option<std::ffi::OsString>,
    temp_root: PathBuf,
}

impl Drop for ScopedCompileCacheOverride {
    fn drop(&mut self) {
        // SAFETY: The CLI runs this override synchronously around a single
        // compile invocation. We restore the previous process environment
        // immediately afterward.
        unsafe {
            if let Some(previous) = self.previous.as_ref() {
                env::set_var("RR_INCREMENTAL_CACHE_DIR", previous);
            } else {
                env::remove_var("RR_INCREMENTAL_CACHE_DIR");
            }
        }
        let _ = fs::remove_dir_all(&self.temp_root);
    }
}

pub(super) fn with_compile_cache_override<T>(
    cold_compile: bool,
    f: impl FnOnce() -> Result<T, RRException>,
) -> Result<T, RRException> {
    if !cold_compile {
        return f();
    }
    static COLD_CACHE_COUNTER: AtomicUsize = AtomicUsize::new(0);
    let seq = COLD_CACHE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_root = env::temp_dir().join(format!("rr-cold-compile-{}-{}", std::process::id(), seq));
    let _ = fs::remove_dir_all(&temp_root);
    if let Err(err) = fs::create_dir_all(&temp_root) {
        return Err(RRException::new(
            "RR.RuntimeError",
            RRCode::E2001,
            Stage::Runner,
            format!(
                "failed to create cold compile cache directory '{}': {}",
                temp_root.display(),
                err
            ),
        )
        .help("retry without --cold, or point TMPDIR at a writable location"));
    }
    let previous = env::var_os("RR_INCREMENTAL_CACHE_DIR");
    // SAFETY: The CLI applies this override only for the duration of one
    // compile call and restores the previous value immediately after.
    unsafe {
        env::set_var("RR_INCREMENTAL_CACHE_DIR", &temp_root);
    }
    let _guard = ScopedCompileCacheOverride {
        previous,
        temp_root,
    };
    f()
}
