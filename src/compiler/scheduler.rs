use rayon::prelude::*;
use rayon::{ThreadPool, ThreadPoolBuilder};
use rustc_hash::FxHashMap;
use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompilerParallelMode {
    Off,
    Auto,
    On,
}

impl CompilerParallelMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Auto => "auto",
            Self::On => "on",
        }
    }
}

impl std::str::FromStr for CompilerParallelMode {
    type Err = ();

    fn from_str(v: &str) -> Result<Self, Self::Err> {
        match v.trim().to_ascii_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "auto" => Ok(Self::Auto),
            "on" | "required" => Ok(Self::On),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CompilerParallelStage {
    MirLowering,
    TypeAnalysis,
    TachyonAlways,
    TachyonHeavy,
    TachyonInlineCleanup,
    TachyonFreshAlias,
    TachyonDeSsa,
    Emit,
}

impl CompilerParallelStage {
    pub const fn label(self) -> &'static str {
        match self {
            Self::MirLowering => "mir_lowering",
            Self::TypeAnalysis => "type_analysis",
            Self::TachyonAlways => "tachyon_always",
            Self::TachyonHeavy => "tachyon_heavy",
            Self::TachyonInlineCleanup => "tachyon_inline_cleanup",
            Self::TachyonFreshAlias => "tachyon_fresh_alias",
            Self::TachyonDeSsa => "tachyon_de_ssa",
            Self::Emit => "emit",
        }
    }

    const fn enabled_by_default(self) -> bool {
        matches!(
            self,
            Self::MirLowering
                | Self::TypeAnalysis
                | Self::TachyonAlways
                | Self::TachyonHeavy
                | Self::TachyonInlineCleanup
                | Self::TachyonFreshAlias
                | Self::TachyonDeSsa
                | Self::Emit
        )
    }

    fn all() -> &'static [Self] {
        const ALL: &[CompilerParallelStage] = &[
            CompilerParallelStage::MirLowering,
            CompilerParallelStage::TypeAnalysis,
            CompilerParallelStage::TachyonAlways,
            CompilerParallelStage::TachyonHeavy,
            CompilerParallelStage::TachyonInlineCleanup,
            CompilerParallelStage::TachyonFreshAlias,
            CompilerParallelStage::TachyonDeSsa,
            CompilerParallelStage::Emit,
        ];
        ALL
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum CompilerParallelDecisionReason {
    StageDisabled,
    ModeOff,
    SingleWorker,
    SingleJob,
    BelowMinFunctions,
    BelowMinFnIr,
    EnabledAuto,
    EnabledOn,
}

impl CompilerParallelDecisionReason {
    const fn label(self) -> &'static str {
        match self {
            Self::StageDisabled => "stage_disabled",
            Self::ModeOff => "mode_off",
            Self::SingleWorker => "single_worker",
            Self::SingleJob => "single_job",
            Self::BelowMinFunctions => "below_min_functions",
            Self::BelowMinFnIr => "below_min_fn_ir",
            Self::EnabledAuto => "enabled_auto",
            Self::EnabledOn => "enabled_on",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct CompilerParallelDecision {
    parallelize: bool,
    reason: CompilerParallelDecisionReason,
}

#[derive(Clone, Debug, Default)]
pub struct CompilerParallelStageProfile {
    pub stage: String,
    pub invocations: usize,
    pub parallel_invocations: usize,
    pub serial_invocations: usize,
    pub total_jobs: usize,
    pub total_ir: usize,
    pub max_jobs: usize,
    pub max_ir: usize,
    pub reason_counts: BTreeMap<String, usize>,
}

impl CompilerParallelStageProfile {
    fn new(stage: CompilerParallelStage) -> Self {
        Self {
            stage: stage.label().to_string(),
            ..Self::default()
        }
    }

    fn record(&mut self, decision: CompilerParallelDecision, job_count: usize, total_ir: usize) {
        self.invocations += 1;
        self.total_jobs += job_count;
        self.total_ir += total_ir;
        self.max_jobs = self.max_jobs.max(job_count);
        self.max_ir = self.max_ir.max(total_ir);
        if decision.parallelize {
            self.parallel_invocations += 1;
        } else {
            self.serial_invocations += 1;
        }
        *self
            .reason_counts
            .entry(decision.reason.label().to_string())
            .or_insert(0) += 1;
    }
}

#[derive(Clone, Debug, Default)]
pub struct CompilerParallelProfile {
    pub mode: String,
    pub configured_threads: usize,
    pub active_workers: usize,
    pub max_jobs: usize,
    pub min_functions: usize,
    pub min_fn_ir: usize,
    pub stages: Vec<CompilerParallelStageProfile>,
}

impl CompilerParallelProfile {
    pub fn stage(&self, label: &str) -> Option<&CompilerParallelStageProfile> {
        self.stages.iter().find(|stage| stage.stage == label)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CompilerParallelConfig {
    pub mode: CompilerParallelMode,
    pub threads: usize,
    pub min_functions: usize,
    pub min_fn_ir: usize,
    pub max_jobs: usize,
}

impl Default for CompilerParallelConfig {
    fn default() -> Self {
        Self {
            mode: CompilerParallelMode::Auto,
            threads: 0,
            min_functions: 2,
            min_fn_ir: 128,
            max_jobs: 0,
        }
    }
}

pub fn default_compiler_parallel_config() -> CompilerParallelConfig {
    CompilerParallelConfig::default()
}

impl CompilerParallelConfig {
    pub fn effective_threads(self) -> usize {
        if self.threads > 0 {
            return self.threads.max(1);
        }
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .max(1)
    }

    pub fn should_parallelize(self, job_count: usize, total_ir: usize) -> bool {
        if self.active_workers() <= 1 {
            return false;
        }
        match self.mode {
            CompilerParallelMode::Off => false,
            CompilerParallelMode::On => job_count > 1,
            CompilerParallelMode::Auto => {
                job_count >= self.min_functions && total_ir >= self.min_fn_ir
            }
        }
    }

    pub fn active_workers(self) -> usize {
        let threads = self.effective_threads();
        if self.max_jobs == 0 {
            threads
        } else {
            threads.min(self.max_jobs.max(1))
        }
    }

    fn decision_for_stage(
        self,
        stage: CompilerParallelStage,
        job_count: usize,
        total_ir: usize,
    ) -> CompilerParallelDecision {
        if !stage.enabled_by_default() {
            return CompilerParallelDecision {
                parallelize: false,
                reason: CompilerParallelDecisionReason::StageDisabled,
            };
        }
        if self.active_workers() <= 1 {
            return CompilerParallelDecision {
                parallelize: false,
                reason: CompilerParallelDecisionReason::SingleWorker,
            };
        }
        match self.mode {
            CompilerParallelMode::Off => CompilerParallelDecision {
                parallelize: false,
                reason: CompilerParallelDecisionReason::ModeOff,
            },
            CompilerParallelMode::On => {
                if job_count <= 1 {
                    CompilerParallelDecision {
                        parallelize: false,
                        reason: CompilerParallelDecisionReason::SingleJob,
                    }
                } else {
                    CompilerParallelDecision {
                        parallelize: true,
                        reason: CompilerParallelDecisionReason::EnabledOn,
                    }
                }
            }
            CompilerParallelMode::Auto => {
                if job_count <= 1 {
                    CompilerParallelDecision {
                        parallelize: false,
                        reason: CompilerParallelDecisionReason::SingleJob,
                    }
                } else if job_count < self.min_functions {
                    CompilerParallelDecision {
                        parallelize: false,
                        reason: CompilerParallelDecisionReason::BelowMinFunctions,
                    }
                } else if total_ir < self.min_fn_ir {
                    CompilerParallelDecision {
                        parallelize: false,
                        reason: CompilerParallelDecisionReason::BelowMinFnIr,
                    }
                } else {
                    CompilerParallelDecision {
                        parallelize: true,
                        reason: CompilerParallelDecisionReason::EnabledAuto,
                    }
                }
            }
        }
    }
}

pub struct CompilerScheduler {
    cfg: CompilerParallelConfig,
    pool: Option<Arc<ThreadPool>>,
    stats: Arc<Mutex<BTreeMap<CompilerParallelStage, CompilerParallelStageProfile>>>,
}

impl CompilerScheduler {
    pub fn new(cfg: CompilerParallelConfig) -> Self {
        let pool = if matches!(cfg.mode, CompilerParallelMode::Off) || cfg.effective_threads() <= 1
        {
            None
        } else {
            shared_pool_for_threads(cfg.active_workers())
        };
        Self {
            cfg,
            pool,
            stats: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    pub fn config(&self) -> CompilerParallelConfig {
        self.cfg
    }

    pub fn should_parallelize(&self, job_count: usize, total_ir: usize) -> bool {
        self.pool.is_some() && self.cfg.should_parallelize(job_count, total_ir)
    }

    pub fn should_parallelize_stage(
        &self,
        stage: CompilerParallelStage,
        job_count: usize,
        total_ir: usize,
    ) -> bool {
        self.pool.is_some()
            && self
                .cfg
                .decision_for_stage(stage, job_count, total_ir)
                .parallelize
    }

    fn worker_count(&self) -> usize {
        self.cfg.active_workers()
    }

    fn decision_for_stage(
        &self,
        stage: CompilerParallelStage,
        job_count: usize,
        total_ir: usize,
    ) -> CompilerParallelDecision {
        let cfg_decision = self.cfg.decision_for_stage(stage, job_count, total_ir);
        let decision = if cfg_decision.parallelize && self.pool.is_none() {
            CompilerParallelDecision {
                parallelize: false,
                reason: CompilerParallelDecisionReason::SingleWorker,
            }
        } else {
            cfg_decision
        };
        let mut guard = lock_or_recover(&self.stats);
        guard
            .entry(stage)
            .or_insert_with(|| CompilerParallelStageProfile::new(stage))
            .record(decision, job_count, total_ir);
        decision
    }

    pub fn profile_snapshot(&self) -> CompilerParallelProfile {
        let guard = lock_or_recover(&self.stats);
        let mut stages = Vec::with_capacity(CompilerParallelStage::all().len());
        for stage in CompilerParallelStage::all() {
            stages.push(
                guard
                    .get(stage)
                    .cloned()
                    .unwrap_or_else(|| CompilerParallelStageProfile::new(*stage)),
            );
        }
        CompilerParallelProfile {
            mode: self.cfg.mode.as_str().to_string(),
            configured_threads: self.cfg.threads,
            active_workers: self.cfg.active_workers(),
            max_jobs: self.cfg.max_jobs,
            min_functions: self.cfg.min_functions,
            min_fn_ir: self.cfg.min_fn_ir,
            stages,
        }
    }

    pub fn install<R, F>(&self, job_count: usize, total_ir: usize, f: F) -> R
    where
        R: Send,
        F: FnOnce() -> R + Send,
    {
        if !self.should_parallelize(job_count, total_ir) {
            f()
        } else if let Some(pool) = self.pool.as_deref() {
            pool.install(f)
        } else {
            f()
        }
    }

    pub fn install_stage<R, F>(
        &self,
        stage: CompilerParallelStage,
        job_count: usize,
        total_ir: usize,
        f: F,
    ) -> R
    where
        R: Send,
        F: FnOnce() -> R + Send,
    {
        let decision = self.decision_for_stage(stage, job_count, total_ir);
        if !decision.parallelize {
            f()
        } else if let Some(pool) = self.pool.as_deref() {
            pool.install(f)
        } else {
            f()
        }
    }

    pub fn map<T, R, F>(&self, jobs: Vec<T>, total_ir: usize, f: F) -> Vec<R>
    where
        T: Send,
        R: Send,
        F: Fn(T) -> R + Sync + Send,
    {
        let job_count = jobs.len();
        if !self.should_parallelize(job_count, total_ir) {
            return jobs.into_iter().map(f).collect();
        }
        if let Some(pool) = self.pool.as_deref() {
            pool.install(|| jobs.into_par_iter().map(f).collect())
        } else {
            jobs.into_iter().map(f).collect()
        }
    }

    pub fn map_stage<T, R, F>(
        &self,
        stage: CompilerParallelStage,
        jobs: Vec<T>,
        total_ir: usize,
        f: F,
    ) -> Vec<R>
    where
        T: Send,
        R: Send,
        F: Fn(T) -> R + Sync + Send,
    {
        let job_count = jobs.len();
        let decision = self.decision_for_stage(stage, job_count, total_ir);
        if !decision.parallelize {
            return jobs.into_iter().map(f).collect();
        }
        if let Some(pool) = self.pool.as_deref() {
            pool.install(|| jobs.into_par_iter().map(f).collect())
        } else {
            jobs.into_iter().map(f).collect()
        }
    }

    pub fn map_try<T, R, E, F>(&self, jobs: Vec<T>, total_ir: usize, f: F) -> Result<Vec<R>, E>
    where
        T: Send,
        R: Send,
        E: Send,
        F: Fn(T) -> Result<R, E> + Sync + Send,
    {
        let job_count = jobs.len();
        if !self.should_parallelize(job_count, total_ir) {
            return jobs.into_iter().map(f).collect();
        }
        let Some(pool) = self.pool.as_deref() else {
            return jobs.into_iter().map(f).collect();
        };

        let cancelled = Arc::new(AtomicBool::new(false));
        let len = jobs.len();
        let queue = Arc::new(Mutex::new(
            jobs.into_iter()
                .enumerate()
                .collect::<VecDeque<(usize, T)>>(),
        ));
        let results = Arc::new(Mutex::new({
            let mut out = Vec::with_capacity(len);
            out.resize_with(len, || None);
            out
        }));
        let first_err: Arc<Mutex<Option<(usize, E)>>> = Arc::new(Mutex::new(None));
        let worker_count = self.worker_count().min(job_count.max(1));
        let f = &f;
        pool.scope(|scope| {
            for _ in 0..worker_count {
                let queue = Arc::clone(&queue);
                let results = Arc::clone(&results);
                let first_err = Arc::clone(&first_err);
                let cancelled = Arc::clone(&cancelled);
                scope.spawn(move |_| {
                    loop {
                        if cancelled.load(Ordering::Relaxed) {
                            break;
                        }
                        let next_job = {
                            let mut guard = lock_or_recover(&queue);
                            guard.pop_front()
                        };
                        let Some((idx, job)) = next_job else {
                            break;
                        };
                        match f(job) {
                            Ok(value) => {
                                let mut guard = lock_or_recover(&results);
                                guard[idx] = Some(value);
                            }
                            Err(err) => {
                                cancelled.store(true, Ordering::Relaxed);
                                let mut guard = lock_or_recover(&first_err);
                                match guard.as_ref() {
                                    None => *guard = Some((idx, err)),
                                    Some((best_idx, _)) if idx < *best_idx => {
                                        *guard = Some((idx, err))
                                    }
                                    Some(_) => {}
                                }
                                break;
                            }
                        }
                    }
                });
            }
        });

        if let Some((_idx, err)) = lock_or_recover(&first_err).take() {
            return Err(err);
        }

        let ordered = {
            let mut guard = lock_or_recover(&results);
            std::mem::take(&mut *guard)
        };
        Ok(ordered
            .into_iter()
            .map(|value| {
                value.unwrap_or_else(|| unreachable!("parallel map produced every result"))
            })
            .collect())
    }

    pub fn map_try_stage<T, R, E, F>(
        &self,
        stage: CompilerParallelStage,
        jobs: Vec<T>,
        total_ir: usize,
        f: F,
    ) -> Result<Vec<R>, E>
    where
        T: Send,
        R: Send,
        E: Send,
        F: Fn(T) -> Result<R, E> + Sync + Send,
    {
        let job_count = jobs.len();
        let decision = self.decision_for_stage(stage, job_count, total_ir);
        if !decision.parallelize {
            return jobs.into_iter().map(f).collect();
        }
        let Some(pool) = self.pool.as_deref() else {
            return jobs.into_iter().map(f).collect();
        };

        let cancelled = Arc::new(AtomicBool::new(false));
        let len = jobs.len();
        let queue = Arc::new(Mutex::new(
            jobs.into_iter()
                .enumerate()
                .collect::<VecDeque<(usize, T)>>(),
        ));
        let results = Arc::new(Mutex::new({
            let mut out = Vec::with_capacity(len);
            out.resize_with(len, || None);
            out
        }));
        let first_err: Arc<Mutex<Option<(usize, E)>>> = Arc::new(Mutex::new(None));
        let worker_count = self.worker_count().min(job_count.max(1));
        let f = &f;
        pool.scope(|scope| {
            for _ in 0..worker_count {
                let queue = Arc::clone(&queue);
                let results = Arc::clone(&results);
                let first_err = Arc::clone(&first_err);
                let cancelled = Arc::clone(&cancelled);
                scope.spawn(move |_| {
                    loop {
                        if cancelled.load(Ordering::Relaxed) {
                            break;
                        }
                        let next_job = {
                            let mut guard = lock_or_recover(&queue);
                            guard.pop_front()
                        };
                        let Some((idx, job)) = next_job else {
                            break;
                        };
                        match f(job) {
                            Ok(value) => {
                                let mut guard = lock_or_recover(&results);
                                guard[idx] = Some(value);
                            }
                            Err(err) => {
                                cancelled.store(true, Ordering::Relaxed);
                                let mut guard = lock_or_recover(&first_err);
                                match guard.as_ref() {
                                    None => *guard = Some((idx, err)),
                                    Some((best_idx, _)) if idx < *best_idx => {
                                        *guard = Some((idx, err))
                                    }
                                    Some(_) => {}
                                }
                                break;
                            }
                        }
                    }
                });
            }
        });

        if let Some((_idx, err)) = lock_or_recover(&first_err).take() {
            return Err(err);
        }

        let ordered = {
            let mut guard = lock_or_recover(&results);
            std::mem::take(&mut *guard)
        };
        Ok(ordered
            .into_iter()
            .map(|value| {
                value.unwrap_or_else(|| unreachable!("parallel map produced every result"))
            })
            .collect())
    }
}

fn lock_or_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn shared_pool_cache() -> &'static Mutex<FxHashMap<usize, Arc<ThreadPool>>> {
    static CACHE: OnceLock<Mutex<FxHashMap<usize, Arc<ThreadPool>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(FxHashMap::default()))
}

fn shared_pool_for_threads(threads: usize) -> Option<Arc<ThreadPool>> {
    // This cache is performance-only. It is keyed solely by worker count and
    // does not contribute to compilation results, emitted order, or diagnostics.
    let cache = shared_pool_cache();
    if let Ok(guard) = cache.lock()
        && let Some(pool) = guard.get(&threads)
    {
        return Some(pool.clone());
    }
    let built = ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .ok()
        .map(Arc::new)?;
    if let Ok(mut guard) = cache.lock() {
        let pooled = guard.entry(threads).or_insert_with(|| built.clone());
        return Some(pooled.clone());
    }
    Some(built)
}
