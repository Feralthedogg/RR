pub(crate) fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  RR --version");
    eprintln!("  RR version");
    eprintln!("  RR <input.rr> [options]");
    eprintln!("  RR new [--bin|--lib] <module-path|.> [dir|.]");
    eprintln!("  RR init [--bin|--lib] [module-path]");
    eprintln!("  RR install <github-url|module-path>[@version]");
    eprintln!("  RR remove <module-path>");
    eprintln!("  RR outdated");
    eprintln!("  RR update [module-path]");
    eprintln!(
        "  RR publish <version> [--dry-run] [--allow-dirty] [--push-tag] [--remote <name>] [--registry <dir>]"
    );
    eprintln!("  RR search <query> [--registry <dir>]");
    eprintln!("  RR registry keygen [identity] [--out-dir <dir>]");
    eprintln!(
        "  RR registry onboard [identity] [--out-dir <dir>] [--require-signed] [--require-approval] [--auto-approve] [--registry <dir>]"
    );
    eprintln!("  RR registry list [--registry <dir>]");
    eprintln!("  RR registry report [module-path] [--registry <dir>]");
    eprintln!("  RR registry diff <module-path> <from-version> <to-version> [--registry <dir>]");
    eprintln!(
        "  RR registry risk <module-path> <version> [--against <version>] [--registry <dir>]"
    );
    eprintln!("  RR registry channel show <module-path> [--registry <dir>]");
    eprintln!("  RR registry channel set <module-path> <channel> <version> [--registry <dir>]");
    eprintln!("  RR registry channel clear <module-path> <channel> [--registry <dir>]");
    eprintln!("  RR registry queue [--registry <dir>]");
    eprintln!(
        "  RR registry audit [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>]"
    );
    eprintln!(
        "  RR registry audit export <file> [--format <tsv|jsonl>] [--limit <n>] [--action <kind>] [--module <path>] [--contains <text>] [--registry <dir>]"
    );
    eprintln!(
        "  RR registry policy bootstrap <trusted-public-key> [--signer <identity>] [--auto-approve-signer <identity>] [--require-signed] [--require-approval] [--registry <dir>]"
    );
    eprintln!("  RR registry policy show [--registry <dir>]");
    eprintln!("  RR registry policy lint [--registry <dir>]");
    eprintln!(
        "  RR registry policy rotate-key <old-public-key> <new-public-key> [--registry <dir>]"
    );
    eprintln!("  RR registry policy apply <file> [--registry <dir>]");
    eprintln!("  RR registry info <module-path> [--registry <dir>]");
    eprintln!("  RR registry approve <module-path> <version> [--registry <dir>]");
    eprintln!("  RR registry unapprove <module-path> <version> [--registry <dir>]");
    eprintln!("  RR registry promote <module-path> <version> [--registry <dir>]");
    eprintln!("  RR registry yank <module-path> <version> [--registry <dir>]");
    eprintln!("  RR registry unyank <module-path> <version> [--registry <dir>]");
    eprintln!("  RR registry deprecate <module-path> <message> [--registry <dir>]");
    eprintln!("  RR registry undeprecate <module-path> [--registry <dir>]");
    eprintln!("  RR registry verify [module-path] [--registry <dir>]");
    eprintln!("  RR mod graph");
    eprintln!("  RR mod why <module-path>");
    eprintln!("  RR mod verify");
    eprintln!("  RR mod tidy");
    eprintln!("  RR mod vendor");
    eprintln!("  RR run [entry.rr|dir|.] [options]");
    eprintln!("  RR build [dir|file.rr] [options]");
    eprintln!("  RR watch [entry.rr|dir|.] [options]");
    eprintln!("Options:");
    eprintln!("  -o <file> / --out-dir <dir>   Output file (legacy) or build output dir");
    eprintln!("  -O0, -O1, -O2                 Optimization level (default O1)");
    eprintln!("  -o0, -o1, -o2                 (Also accepted) Optimization level");
    eprintln!("  --bin                         Scaffold a binary project for RR new/init");
    eprintln!("  --lib                         Scaffold a library project for RR new/init");
    eprintln!("  --signer <identity>           Registry policy bootstrap signer allowlist entry");
    eprintln!("  --auto-approve-signer <identity>  Registry policy bootstrap auto-approval signer");
    eprintln!("  --auto-approve               Registry onboard: auto-approve the generated signer");
    eprintln!("  --action <kind>             Registry audit action filter");
    eprintln!("  --module <path>             Registry audit module filter");
    eprintln!("  --contains <text>           Registry audit substring filter");
    eprintln!("  --format <tsv|jsonl>        Registry audit export output format");
    eprintln!("  --type-mode <strict|gradual>  Static typing mode (default strict)");
    eprintln!("  --native-backend <off|optional|required>  Native intrinsic backend mode");
    eprintln!("  --parallel-mode <off|optional|required>   Parallel execution mode");
    eprintln!("  --parallel-backend <auto|r|openmp>        Parallel backend selection");
    eprintln!("  --parallel-threads <N>                    Parallel worker threads (0=auto)");
    eprintln!("  --parallel-min-trip <N>                   Minimum trip-count for parallel path");
    eprintln!(
        "  --compiler-parallel-mode <off|auto|on>    Compiler scheduling mode (default auto)"
    );
    eprintln!(
        "  --compiler-parallel-threads <N>           Compiler worker threads (0=auto, default)"
    );
    eprintln!(
        "  --compiler-parallel-min-functions <N>     Minimum functions before compiler parallelism (default 2)"
    );
    eprintln!(
        "  --compiler-parallel-min-fn-ir <N>         Minimum aggregate IR before compiler parallelism (default 128)"
    );
    eprintln!(
        "  --compiler-parallel-max-jobs <N>          Maximum concurrent compiler jobs (0=threads)"
    );
    eprintln!("  --strict-let <on|off>                     Require explicit let before assignment");
    eprintln!("  --warn-implicit-decl <on|off>             Warn on legacy implicit declaration");
    eprintln!("  --incremental[=auto|off|1|1,2|1,2,3|all] Enable incremental compile phases");
    eprintln!("  --incremental-phases <...>                Same as above (separate arg form)");
    eprintln!("  --no-incremental                          Disable automatic incremental compile");
    eprintln!(
        "  --cold                                   Bypass warm compile caches for this compile"
    );
    eprintln!(
        "  --strict-incremental-verify               Extra validation gate for incremental mode"
    );
    eprintln!(
        "  --profile-compile                         Emit compile profile JSON for this compile"
    );
    eprintln!("  --profile-compile-out <file>              Write compile profile JSON to a file");
    eprintln!(
        "  --compile-mode <standard|fast-dev>        Compiler pass profile selection (build/run/watch default fast-dev)"
    );
    eprintln!("  --poll-ms <N>                             Watch polling interval in milliseconds");
    eprintln!("  --once                                    Run a single watch tick and exit");
    eprintln!("  --keep-r                      Keep generated .gen.R when running");
    eprintln!("  --no-runtime                  Emit helper-only R without source/native bootstrap");
    eprintln!("  --preserve-all-defs          Keep all top-level Sym_* definitions in emitted R");
    eprintln!("  --preserve-all-def           Alias for --preserve-all-defs");
}

pub(crate) fn print_version() {
    println!("RR Tachyon v{}", env!("CARGO_PKG_VERSION"));
}
