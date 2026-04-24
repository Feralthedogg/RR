use RR::compiler::OptLevel;

pub(super) fn apply_opt_flag(arg: &str, level: &mut OptLevel) -> bool {
    if arg == "-O0" || arg == "-o0" {
        *level = OptLevel::O0;
        true
    } else if arg == "-O1" || arg == "-o1" {
        *level = OptLevel::O1;
        true
    } else if arg == "-O2" || arg == "-O" || arg == "-o2" {
        *level = OptLevel::O2;
        true
    } else {
        false
    }
}

pub(super) fn parse_nonnegative_usize(raw: &str) -> Option<usize> {
    raw.trim().parse::<usize>().ok()
}

pub(super) fn parse_bool_flag(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CommonCompileFlag {
    TypeMode,
    NativeBackend,
    ParallelMode,
    ParallelBackend,
    ParallelThreads,
    ParallelMinTrip,
    CompilerParallelMode,
    CompilerParallelThreads,
    CompilerParallelMinFunctions,
    CompilerParallelMinFnIr,
    CompilerParallelMaxJobs,
    StrictLet,
    WarnImplicitDecl,
}

impl CommonCompileFlag {
    pub(super) fn from_arg(arg: &str) -> Option<Self> {
        match arg {
            "--type-mode" => Some(Self::TypeMode),
            "--native-backend" => Some(Self::NativeBackend),
            "--parallel-mode" => Some(Self::ParallelMode),
            "--parallel-backend" => Some(Self::ParallelBackend),
            "--parallel-threads" => Some(Self::ParallelThreads),
            "--parallel-min-trip" => Some(Self::ParallelMinTrip),
            "--compiler-parallel-mode" => Some(Self::CompilerParallelMode),
            "--compiler-parallel-threads" => Some(Self::CompilerParallelThreads),
            "--compiler-parallel-min-functions" => Some(Self::CompilerParallelMinFunctions),
            "--compiler-parallel-min-fn-ir" => Some(Self::CompilerParallelMinFnIr),
            "--compiler-parallel-max-jobs" => Some(Self::CompilerParallelMaxJobs),
            "--strict-let" => Some(Self::StrictLet),
            "--warn-implicit-decl" => Some(Self::WarnImplicitDecl),
            _ => None,
        }
    }

    pub(super) fn missing_value_error(self) -> &'static str {
        match self {
            Self::TypeMode => "Missing value after --type-mode (strict|gradual)",
            Self::NativeBackend => "Missing value after --native-backend (off|optional|required)",
            Self::ParallelMode => "Missing value after --parallel-mode (off|optional|required)",
            Self::ParallelBackend => "Missing value after --parallel-backend (auto|r|openmp)",
            Self::ParallelThreads => "Missing value after --parallel-threads",
            Self::ParallelMinTrip => "Missing value after --parallel-min-trip",
            Self::CompilerParallelMode => {
                "Missing value after --compiler-parallel-mode (off|auto|on)"
            }
            Self::CompilerParallelThreads => "Missing value after --compiler-parallel-threads",
            Self::CompilerParallelMinFunctions => {
                "Missing value after --compiler-parallel-min-functions"
            }
            Self::CompilerParallelMinFnIr => "Missing value after --compiler-parallel-min-fn-ir",
            Self::CompilerParallelMaxJobs => "Missing value after --compiler-parallel-max-jobs",
            Self::StrictLet => "Missing value after --strict-let (on|off)",
            Self::WarnImplicitDecl => "Missing value after --warn-implicit-decl (on|off)",
        }
    }
}

pub(super) fn next_flag_value<'a>(args: &'a [String], i: &mut usize) -> Result<&'a str, i32> {
    if *i + 1 >= args.len() {
        return Err(1);
    }
    *i += 1;
    Ok(&args[*i])
}
