use rr::error::{RRCode, RRException, Stage};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

struct ScopedCompileCacheOverride {
    previous: Option<std::ffi::OsString>,
    temp_root: PathBuf,
}

struct ScopedProfileUseOverride {
    previous: Option<std::ffi::OsString>,
}

impl Drop for ScopedProfileUseOverride {
    fn drop(&mut self) {
        // SAFETY: Safe alternatives cannot express scoped process-env mutation.
        // The CLI applies `RR_PROFILE_USE` only around one synchronous compile
        // invocation and restores the previous value before returning.
        unsafe {
            if let Some(previous) = self.previous.as_ref() {
                env::set_var("RR_PROFILE_USE", previous);
            } else {
                env::remove_var("RR_PROFILE_USE");
            }
        }
    }
}

impl Drop for ScopedCompileCacheOverride {
    fn drop(&mut self) {
        // SAFETY: No safe alternatives exist for scoped process-env overrides;
        // the CLI runs this synchronously around one cold compile and the guard
        // restores the previous process environment before leaving the CLI path.
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

pub(crate) fn with_compile_cache_override<T>(
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
    // SAFETY: No safe alternatives exist for scoped process-env overrides;
    // this cold-compile CLI path mutates the process environment only until
    // `ScopedCompileCacheOverride` restores the previous value.
    unsafe {
        env::set_var("RR_INCREMENTAL_CACHE_DIR", &temp_root);
    }
    let _guard = ScopedCompileCacheOverride {
        previous,
        temp_root,
    };
    f()
}

pub(crate) fn with_profile_use_override<T>(
    profile_use: Option<&str>,
    f: impl FnOnce() -> Result<T, RRException>,
) -> Result<T, RRException> {
    let Some(profile_use) = profile_use else {
        return f();
    };
    let previous = env::var_os("RR_PROFILE_USE");
    // SAFETY: Safe alternatives cannot express scoped process-env mutation.
    // The mutation is limited to this synchronous compile call and restored by
    // `ScopedProfileUseOverride`.
    unsafe {
        env::set_var("RR_PROFILE_USE", profile_use);
    }
    let _guard = ScopedProfileUseOverride { previous };
    f()
}
