use RR::compiler::{
    CompilerParallelConfig, CompilerParallelMode, CompilerScheduler,
    default_compiler_parallel_config,
};

#[test]
fn compiler_parallel_defaults_are_auto_sized_from_host() {
    let cfg = default_compiler_parallel_config();
    assert_eq!(cfg.mode, CompilerParallelMode::Auto);
    assert_eq!(cfg.threads, 0);
    assert_eq!(cfg.max_jobs, 0);
    assert!(cfg.active_workers() >= 1);

    let scheduler = CompilerScheduler::new(cfg);
    assert_eq!(scheduler.config().mode, CompilerParallelMode::Auto);
    assert_eq!(scheduler.config().threads, 0);
    assert_eq!(scheduler.config().max_jobs, 0);
}

#[test]
fn compiler_parallel_active_workers_respects_threads_and_max_jobs() {
    let cfg = CompilerParallelConfig {
        mode: CompilerParallelMode::On,
        threads: 4,
        min_functions: 1,
        min_fn_ir: 1,
        max_jobs: 2,
    };
    assert_eq!(cfg.effective_threads(), 4);
    assert_eq!(cfg.active_workers(), 2);

    let unclamped = CompilerParallelConfig { max_jobs: 0, ..cfg };
    assert_eq!(unclamped.active_workers(), 4);

    let oversubscribed = CompilerParallelConfig { max_jobs: 8, ..cfg };
    assert_eq!(oversubscribed.active_workers(), 4);
}

#[test]
fn compiler_parallel_on_mode_requires_multiple_jobs() {
    let cfg = CompilerParallelConfig {
        mode: CompilerParallelMode::On,
        threads: 3,
        min_functions: usize::MAX,
        min_fn_ir: usize::MAX,
        max_jobs: 0,
    };
    assert!(!cfg.should_parallelize(1, 10_000));
    assert!(cfg.should_parallelize(2, 0));

    let scheduler = CompilerScheduler::new(cfg);
    assert!(!scheduler.should_parallelize(1, 10_000));
    assert!(scheduler.should_parallelize(2, 0));
}

#[test]
fn compiler_parallel_auto_mode_respects_thresholds() {
    let cfg = CompilerParallelConfig {
        mode: CompilerParallelMode::Auto,
        threads: 4,
        min_functions: 3,
        min_fn_ir: 50,
        max_jobs: 0,
    };
    assert!(!cfg.should_parallelize(2, 100));
    assert!(!cfg.should_parallelize(3, 49));
    assert!(cfg.should_parallelize(3, 50));

    let scheduler = CompilerScheduler::new(cfg);
    assert!(!scheduler.should_parallelize(2, 100));
    assert!(!scheduler.should_parallelize(3, 49));
    assert!(scheduler.should_parallelize(3, 50));
}

#[test]
fn compiler_parallel_single_active_worker_disables_parallelism_even_in_on_mode() {
    let cfg = CompilerParallelConfig {
        mode: CompilerParallelMode::On,
        threads: 4,
        min_functions: 1,
        min_fn_ir: 1,
        max_jobs: 1,
    };
    assert_eq!(cfg.active_workers(), 1);
    assert!(!cfg.should_parallelize(8, 10_000));

    let scheduler = CompilerScheduler::new(cfg);
    assert!(!scheduler.should_parallelize(8, 10_000));
}

#[test]
fn compiler_parallel_off_mode_never_parallelizes() {
    let cfg = CompilerParallelConfig {
        mode: CompilerParallelMode::Off,
        threads: 8,
        min_functions: 1,
        min_fn_ir: 1,
        max_jobs: 0,
    };
    assert!(!cfg.should_parallelize(8, 10_000));

    let scheduler = CompilerScheduler::new(cfg);
    assert!(!scheduler.should_parallelize(8, 10_000));
}
