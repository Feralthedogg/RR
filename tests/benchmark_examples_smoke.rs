mod common;

use common::{compile_rr, compile_rr_env, normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::{Path, PathBuf};

fn rr_bin_path(root: &Path) -> PathBuf {
    let env_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    if env_bin.exists() {
        return env_bin;
    }
    let release_bin = root.join("target").join("release").join("RR");
    if release_bin.exists() {
        return release_bin;
    }
    root.join("target").join("debug").join("RR")
}

fn collect_rr_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .expect("missing benchmark example directory")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    for path in entries {
        if path.extension().and_then(|s| s.to_str()) == Some("rr") {
            out.push(path);
        }
    }
}

#[test]
fn benchmark_examples_compile_at_o2() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = rr_bin_path(&root);
    let src_dir = root.join("example").join("benchmarks");
    let out_dir = root.join("target").join("benchmark_examples");
    fs::create_dir_all(&out_dir).expect("failed to create target/benchmark_examples");

    let mut files = Vec::new();
    collect_rr_files(&src_dir, &mut files);
    assert!(files.len() >= 5, "expected at least 5 benchmark workloads");

    for rr_path in files {
        let stem = rr_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("bench");
        let out_path = out_dir.join(format!("{}_o2.R", stem));
        compile_rr(&rr_bin, &rr_path, &out_path, "-O2");
        let code = fs::read_to_string(&out_path).expect("failed to read compiled benchmark R");
        assert!(
            code.contains("function(") || code.contains("print("),
            "compiled benchmark output for {} looked empty",
            rr_path.display()
        );
    }
}

#[test]
fn benchmark_examples_run_at_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping benchmark runtime smoke: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = rr_bin_path(&root);
    let src_dir = root.join("example").join("benchmarks");
    let out_dir = root.join("target").join("benchmark_examples_runtime");
    fs::create_dir_all(&out_dir).expect("failed to create target/benchmark_examples_runtime");

    let mut files = Vec::new();
    collect_rr_files(&src_dir, &mut files);
    assert!(files.len() >= 5, "expected at least 5 benchmark workloads");

    for rr_path in files {
        let stem = rr_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("bench");
        let out_path = out_dir.join(format!("{}_o2.R", stem));
        compile_rr(&rr_bin, &rr_path, &out_path, "-O2");
        let run = run_rscript(&rscript, &out_path);
        assert!(
            run.status == 0,
            "benchmark runtime failed for {}:\nstdout={}\nstderr={}",
            rr_path.display(),
            run.stdout,
            run.stderr
        );
        assert!(
            !normalize(&run.stdout).trim().is_empty(),
            "benchmark runtime produced empty stdout for {}",
            rr_path.display()
        );
    }
}

#[test]
fn signal_pipeline_emits_only_reachable_helpers() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = rr_bin_path(&root);
    let rr_path = root
        .join("example")
        .join("benchmarks")
        .join("signal_pipeline_bench.rr");
    let out_dir = root.join("target").join("benchmark_examples_reachable");
    fs::create_dir_all(&out_dir).expect("failed to create target/benchmark_examples_reachable");
    let out_path = out_dir.join("signal_pipeline_bench_o2.R");

    compile_rr(&rr_bin, &rr_path, &out_path, "-O2");
    let code = fs::read_to_string(&out_path).expect("failed to read compiled signal pipeline R");

    let fn_count = code
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("Sym_") && trimmed.contains("<- function")
        })
        .count();
    assert!(
        fn_count <= 6,
        "expected signal_pipeline to emit only reachable helpers, saw {fn_count} functions"
    );
    assert!(
        !code.contains("1103515245") && !code.contains("2147483648"),
        "unexpected unused RNG helper body remained in signal pipeline output"
    );
}

#[test]
fn signal_pipeline_rewrites_back_to_vector_whole_slice_shape() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = rr_bin_path(&root);
    let rr_path = root
        .join("example")
        .join("benchmarks")
        .join("signal_pipeline_bench.rr");
    let out_dir = root
        .join("target")
        .join("benchmark_examples_signal_pipeline_loop");
    fs::create_dir_all(&out_dir)
        .expect("failed to create target/benchmark_examples_signal_pipeline_loop");
    let out_path = out_dir.join("signal_pipeline_bench_o2.R");

    compile_rr(&rr_bin, &rr_path, &out_path, "-O2");
    let code = fs::read_to_string(&out_path).expect("failed to read compiled signal pipeline R");

    assert!(
        code.contains("score <- pmax("),
        "signal_pipeline score stage failed to lower back to whole-vector pmax"
    );
    assert!(
        (code.contains(".tachyon_callmap_arg0_0 <- abs((((x * 0.65) + (y * 0.35)) - 0.08))")
            && code.contains("score <- pmax(.tachyon_callmap_arg0_0, 0.05)"))
            || code.contains("score <- pmax(abs((((x * 0.65) + (y * 0.35)) - 0.08)), 0.05)"),
        "signal_pipeline score stage should recover direct x/y whole-vector math before helper cleanup"
    );
    assert!(
        !code.contains("score <- rr_call_map_whole_auto(")
            && !code.contains("score <- rr_call_map_slice_auto("),
        "signal_pipeline score stage unexpectedly still depends on runtime call-map helpers"
    );
    assert!(
        code.contains("clean <- ifelse(") || code.contains("clean <- rr_ifelse_strict("),
        "signal_pipeline clean stage failed to lower back to vector ifelse form"
    );
    assert!(
        code.contains("print(clean[n])") && !code.contains("rr_index1_read(clean, n, \"index\")"),
        "signal_pipeline tail read should lower to direct base indexing"
    );
    assert!(
        code.contains("x <- (clean + (y * 0.15))"),
        "signal_pipeline x stage should recover direct whole-range y reuse rather than rr_index1_read_vec"
    );
    assert!(
        !code.contains("rr_index1_read_vec(y, rr_index_vec_floor(1L:n))"),
        "signal_pipeline x stage unexpectedly still contains a full-range rr_index1_read_vec wrapper"
    );
    assert!(
        !code.contains("i_9 <- 1L"),
        "signal_pipeline clean stage unexpectedly remained as a scalar inner loop"
    );
    assert!(
        code.contains(".__rr_body_Sym_1 <- quote({")
            && code.contains("eval(.__rr_body_Sym_1, envir = environment())"),
        "signal_pipeline entry kernel should use a quoted-body wrapper to reduce cold-start compile cost"
    );
}

#[test]
fn reaction_diffusion_seed_fill_lowers_to_direct_slice_write() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = rr_bin_path(&root);
    let rr_path = root
        .join("example")
        .join("benchmarks")
        .join("reaction_diffusion_bench.rr");
    let out_dir = root
        .join("target")
        .join("benchmark_examples_reaction_diffusion");
    fs::create_dir_all(&out_dir)
        .expect("failed to create target/benchmark_examples_reaction_diffusion");
    let out_path = out_dir.join("reaction_diffusion_bench_o2.R");

    compile_rr(&rr_bin, &rr_path, &out_path, "-O2");
    let code = fs::read_to_string(&out_path).expect("failed to read compiled reaction diffusion R");

    assert!(
        code.contains("b[88:104] <- rep.int(1,") || code.contains("b[88:104] <- rep.int(1.0,"),
        "reaction_diffusion benchmark should lower the seed fill to a direct slice write"
    );
    assert!(
        !code.contains("b <- rr_assign_slice(b, i, 104,"),
        "reaction_diffusion benchmark unexpectedly still relies on rr_assign_slice for the seed fill"
    );
}

#[test]
fn bootstrap_resample_unit_index_and_metric_helpers_inline() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = rr_bin_path(&root);
    let rr_path = root
        .join("example")
        .join("benchmarks")
        .join("bootstrap_resample_bench.rr");
    let out_dir = root
        .join("target")
        .join("benchmark_examples_bootstrap_resample");
    fs::create_dir_all(&out_dir)
        .expect("failed to create target/benchmark_examples_bootstrap_resample");
    let out_path = out_dir.join("bootstrap_resample_bench_o2.R");

    compile_rr(&rr_bin, &rr_path, &out_path, "-O2");
    let code = fs::read_to_string(&out_path).expect("failed to read compiled bootstrap R");
    let has_clamp_head_int = code.contains("pmin(pmax((1 + floor(");
    let has_clamp_head_float = code.contains("pmin(pmax((1.0 + floor(");
    let has_clamp_head = has_clamp_head_int || has_clamp_head_float;
    let has_draw_source = code.contains("draws[")
        || code.contains("rr_index1_read(draws,")
        || code.contains("rr_gather(draws,")
        || code.contains("licm_50 + inner");
    let has_clamp_tail = code.contains(", 1), n))") || code.contains(", 1.0), n))");
    let inlined_unit_index = has_clamp_head && has_draw_source && has_clamp_tail;

    assert!(
        inlined_unit_index,
        "bootstrap_resample should inline unit_index into a clamp expression, either as a direct scalar idx or an equivalent vectorized gather"
    );
    let has_direct_sample_index = code.contains("s <- (s + samples[idx])")
        && !code.contains("rr_index1_read(samples, idx, \"index\")");
    let has_vectorized_sample_gather = code.contains("s <- sum(rr_gather(samples, ");
    assert!(
        has_direct_sample_index || has_vectorized_sample_gather,
        "bootstrap_resample should lower the bounded samples lookup to direct base indexing or an equivalent vectorized gather"
    );
    assert!(
        !code.contains("idx <- Sym_14(") && !code.contains("Sym_14 <- function"),
        "bootstrap_resample unexpectedly still retains the unit_index helper"
    );
    assert!(
        code.contains("print(\"bootstrap_bench_acc\")")
            && code.contains(".__rr_inline_metric_0 <- acc")
            && code.contains("return(.__rr_inline_metric_0)"),
        "bootstrap_resample should inline the final print_metric helper at the return site"
    );
    assert!(
        !code.contains("return(Sym_15(\"bootstrap_bench_acc\", acc))")
            && !code.contains("Sym_15 <- function"),
        "bootstrap_resample unexpectedly still retains the print_metric helper"
    );
}

#[test]
fn heat_diffusion_copy_vec_collapse_survives_alias_cleanup() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = rr_bin_path(&root);
    let rr_path = root
        .join("example")
        .join("benchmarks")
        .join("heat_diffusion_bench.rr");
    let out_dir = root
        .join("target")
        .join("benchmark_examples_heat_diffusion");
    fs::create_dir_all(&out_dir)
        .expect("failed to create target/benchmark_examples_heat_diffusion");
    let out_path = out_dir.join("heat_diffusion_bench_o2.R");

    compile_rr(&rr_bin, &rr_path, &out_path, "-O2");
    let code = fs::read_to_string(&out_path).expect("failed to read compiled heat diffusion R");

    assert!(
        code.contains("next_temp <- temp")
            || code.contains("next_temp <- (temp)")
            || code.contains("next_temp <- ((temp))"),
        "heat_diffusion benchmark should keep the copy_vec alias rewrite"
    );
    assert!(
        code.contains("temp <- next_temp"),
        "heat_diffusion benchmark should collapse the final full-range copy back into a direct swap"
    );
    assert!(
        !code.contains("temp <- rr_assign_slice(inlined_9_out, inlined_9_i, inlined_9_n, temp)"),
        "heat_diffusion benchmark unexpectedly still relies on the helper-heavy copy_vec replay"
    );
}

#[test]
fn heat_diffusion_copy_vec_swap_happens_before_peephole() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = rr_bin_path(&root);
    let rr_path = root
        .join("example")
        .join("benchmarks")
        .join("heat_diffusion_bench.rr");
    let out_dir = root
        .join("target")
        .join("benchmark_examples_heat_diffusion_raw");
    fs::create_dir_all(&out_dir)
        .expect("failed to create target/benchmark_examples_heat_diffusion_raw");
    let out_path = out_dir.join("heat_diffusion_bench_o2.R");
    let raw_path = out_dir.join("heat_diffusion_bench_raw.R");

    let raw_env = raw_path
        .to_str()
        .expect("raw benchmark output path should be valid unicode")
        .to_string();
    compile_rr_env(
        &rr_bin,
        &rr_path,
        &out_path,
        "-O2",
        &[("RR_DEBUG_RAW_R_PATH", raw_env.as_str())],
    );

    let raw = fs::read_to_string(&raw_path).expect("failed to read raw emitted heat diffusion R");
    assert!(
        raw.contains("temp <- next_temp"),
        "heat_diffusion raw emitted R should recover the final copy_vec swap before peephole"
    );
    assert!(
        ((raw.contains("next_temp[i] <- (temp[i] + (alpha * ((temp[(i - 1)] - (2 * temp[i])) + temp[(i + 1)])))")
            && !raw.contains("rr_index1_read(temp, i, \"index\")")
            && !raw.contains("rr_index1_write(i, \"index\")"))
            || (raw.contains("next_temp <- rr_assign_slice(next_temp, i,")
                && (raw.contains("rr_index1_read_vec(temp, rr_index_vec_floor(")
                    || raw.contains("rr_index1_read_vec(temp, .__rr_cse_")
                    || raw.contains(".__rr_cse_0 <- rr_index1_read_vec(temp, rr_index_vec_floor("))
                && raw.matches("rr_gather(temp,").count() >= 2
                && !raw.contains("next_temp[i] <-"))),
        "heat_diffusion raw emitted R should either keep direct scalar indexing or already expose the vectorized stencil slice"
    );
    assert!(
        !raw.contains("temp <- rr_assign_slice(inlined_9_out, inlined_9_i, inlined_9_n, temp)"),
        "heat_diffusion raw emitted R unexpectedly still depends on the final copy_vec helper replay"
    );
}

#[test]
fn copy_vec_helper_whole_assign_lowers_to_direct_alias_in_raw_output() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = rr_bin_path(&root);
    let rr_path = root
        .join("example")
        .join("benchmarks")
        .join("heat_diffusion_bench.rr");
    let out_dir = root.join("target").join("benchmark_examples_copy_vec_raw");
    fs::create_dir_all(&out_dir).expect("failed to create target/benchmark_examples_copy_vec_raw");
    let out_path = out_dir.join("heat_diffusion_bench_o2.R");
    let raw_path = out_dir.join("heat_diffusion_bench_raw.R");

    let raw_env = raw_path
        .to_str()
        .expect("raw benchmark output path should be valid unicode")
        .to_string();
    compile_rr_env(
        &rr_bin,
        &rr_path,
        &out_path,
        "-O2",
        &[("RR_DEBUG_RAW_R_PATH", raw_env.as_str())],
    );

    let raw = fs::read_to_string(&raw_path).expect("failed to read raw emitted heat diffusion R");
    assert!(
        raw.contains("out <- xs")
            || raw.contains("out <- .arg_xs")
            || raw.contains("temp <- next_temp"),
        "copy_vec helper should either lower the whole-range replay to a direct alias assignment or disappear after callsite collapse"
    );
    assert!(
        !raw.contains("out <- rr_assign_slice(out, i, length(.arg_xs), .arg_xs)"),
        "copy_vec helper unexpectedly still relies on rr_assign_slice in raw emitted R"
    );
}

#[test]
fn vector_fusion_copy_vec_callsite_collapses_to_direct_alias() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = rr_bin_path(&root);
    let rr_path = root
        .join("example")
        .join("benchmarks")
        .join("vector_fusion_bench.rr");
    let out_dir = root.join("target").join("benchmark_examples_vector_fusion");
    fs::create_dir_all(&out_dir).expect("failed to create target/benchmark_examples_vector_fusion");
    let out_path = out_dir.join("vector_fusion_bench_o2.R");

    compile_rr(&rr_bin, &rr_path, &out_path, "-O2");
    let code = fs::read_to_string(&out_path).expect("failed to read compiled vector fusion R");

    assert!(
        code.contains("x <- z")
            || code.contains("x <- ((z))")
            || code.contains("next_x <- z")
            || code.contains("next_x <- ((z))"),
        "vector_fusion benchmark should collapse the trivial copy/copy_vec path to a direct alias"
    );
    assert!(
        !code.contains("x <- Sym_10(z)"),
        "vector_fusion benchmark unexpectedly still routes through the trivial copy_vec helper"
    );
    assert!(
        code.contains("vector_fusion_mean")
            && code.contains("sum(z)")
            && code.contains("length(z)"),
        "vector_fusion benchmark should preserve the simplified vector_mean metric path"
    );
    assert!(
        !code.contains("Sym_10 <- function")
            && !code.contains("Sym_39 <- function")
            && !code.contains("Sym_11 <- function"),
        "vector_fusion benchmark unexpectedly still retains trivial copy/mean helper definitions"
    );
}

#[test]
fn heat_diffusion_metric_helper_inlines_at_return_site() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = rr_bin_path(&root);
    let rr_path = root
        .join("example")
        .join("benchmarks")
        .join("heat_diffusion_bench.rr");
    let out_dir = root
        .join("target")
        .join("benchmark_examples_heat_diffusion_metric");
    fs::create_dir_all(&out_dir)
        .expect("failed to create target/benchmark_examples_heat_diffusion_metric");
    let out_path = out_dir.join("heat_diffusion_bench_o2.R");

    compile_rr(&rr_bin, &rr_path, &out_path, "-O2");
    let code = fs::read_to_string(&out_path).expect("failed to read compiled heat diffusion R");

    assert!(code.contains("print(\"heat_bench_energy\")"));
    assert!(
        code.contains(".__rr_inline_metric_0 <- (sum(temp))")
            || code.contains(".__rr_inline_metric_0 <- ((sum(temp)))")
    );
    assert!(code.contains("return(.__rr_inline_metric_0)"));
    assert!(!code.contains("return(Sym_10(\"heat_bench_energy\""));
    assert!(!code.contains("Sym_10 <- function"));
    assert!(!code.contains("Sym_11 <- function"));
    assert!(
        code.contains("next_temp <- rr_assign_slice(next_temp, i,")
            && (code.contains("rr_index1_read_vec(temp, rr_index_vec_floor(")
                || code.contains(".__rr_cse_0 <- rr_index1_read_vec(temp, rr_index_vec_floor("))
            && code.matches("rr_gather(temp,").count() >= 2
            && !code.contains("next_temp[i] <-")
            && !code.contains("rr_index1_read(temp, i, \"index\")")
            && !code.contains("rr_index1_write(i, \"index\")"),
        "heat_diffusion benchmark should vectorize the interior stencil loop into a slice assignment"
    );
}

#[test]
fn reaction_diffusion_metric_helper_inlines_at_return_site() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = rr_bin_path(&root);
    let rr_path = root
        .join("example")
        .join("benchmarks")
        .join("reaction_diffusion_bench.rr");
    let out_dir = root
        .join("target")
        .join("benchmark_examples_reaction_metric");
    fs::create_dir_all(&out_dir)
        .expect("failed to create target/benchmark_examples_reaction_metric");
    let out_path = out_dir.join("reaction_diffusion_bench_o2.R");

    compile_rr(&rr_bin, &rr_path, &out_path, "-O2");
    let code = fs::read_to_string(&out_path).expect("failed to read compiled reaction diffusion R");

    assert!(code.contains("print(\"rd_bench_mass\")"));
    assert!(
        code.contains(".__rr_inline_metric_0 <- (sum(b))")
            || code.contains(".__rr_inline_metric_0 <- ((sum(b)))")
    );
    assert!(code.contains("return(.__rr_inline_metric_0)"));
    assert!(!code.contains("return(Sym_21(\"rd_bench_mass\""));
    assert!(!code.contains("Sym_21 <- function"));
    assert!(
        code.contains("pmin(pmax(")
            && code.contains("rr_gather(a, (")
            && code.contains("rr_gather(b, (")
            && (code.contains("rr_index1_read_vec(a, rr_index_vec_floor(")
                || code.contains(".tachyon_exprmap0_0 <- pmin(pmax("))
            && (code.contains("rr_index1_read_vec(b, rr_index_vec_floor(")
                || code.contains(".tachyon_exprmap1_0 <- pmin(pmax("))
            && code.contains("next_a <- rr_assign_slice(")
            && (code.contains("next_b <- rr_assign_slice(next_b")
                || code.contains("next_b <- rr_assign_slice(b,"))
            && !code.contains("next_a[i] <-")
            && !code.contains("next_b[i] <-"),
        "reaction_diffusion benchmark should vectorize the dual stencil update and inline the clamp helper into pmin/pmax"
    );
    assert!(
        !code.contains("Sym_20 <- function")
            && !code.contains("Sym_20(next_a_cell, 0, 1)")
            && !code.contains("Sym_20(next_b_cell, 0, 1)")
            && !code.contains("rr_index1_read(a, i, \"index\")")
            && !code.contains("rr_index1_read(b, i, \"index\")")
            && !code.contains("rr_index1_write(i, \"index\")"),
        "reaction_diffusion benchmark unexpectedly still retains the trivial clamp helper or scalar index path"
    );
}

#[test]
fn reaction_diffusion_reuses_current_cell_vector_reads() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = rr_bin_path(&root);
    let rr_path = root
        .join("example")
        .join("benchmarks")
        .join("reaction_diffusion_bench.rr");
    let out_dir = root
        .join("target")
        .join("benchmark_examples_reaction_reuse");
    fs::create_dir_all(&out_dir)
        .expect("failed to create target/benchmark_examples_reaction_reuse");
    let out_path = out_dir.join("reaction_diffusion_bench_o2.R");

    compile_rr(&rr_bin, &rr_path, &out_path, "-O2");
    let code = fs::read_to_string(&out_path).expect("failed to read compiled reaction diffusion R");

    assert_eq!(
        code.matches("rr_index1_read_vec(a, rr_index_vec_floor(")
            .count(),
        1,
        "reaction_diffusion should materialize the current-cell vector read for a only once\n{code}"
    );
    assert_eq!(
        code.matches("rr_index1_read_vec(b, rr_index_vec_floor(")
            .count(),
        1,
        "reaction_diffusion should materialize the current-cell vector read for b only once\n{code}"
    );
}

#[test]
fn reaction_diffusion_direct_indexing_happens_before_peephole() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = rr_bin_path(&root);
    let rr_path = root
        .join("example")
        .join("benchmarks")
        .join("reaction_diffusion_bench.rr");
    let out_dir = root.join("target").join("benchmark_examples_reaction_raw");
    fs::create_dir_all(&out_dir).expect("failed to create target/benchmark_examples_reaction_raw");
    let out_path = out_dir.join("reaction_diffusion_bench_o2.R");
    let raw_path = out_dir.join("reaction_diffusion_bench_raw.R");

    let raw_env = raw_path
        .to_str()
        .expect("raw benchmark output path should be valid unicode")
        .to_string();
    compile_rr_env(
        &rr_bin,
        &rr_path,
        &out_path,
        "-O2",
        &[("RR_DEBUG_RAW_R_PATH", raw_env.as_str())],
    );

    let raw =
        fs::read_to_string(&raw_path).expect("failed to read raw emitted reaction diffusion R");
    assert!(
        raw.contains("pmin(pmax(")
            && raw.contains("rr_gather(a, (")
            && raw.contains("rr_gather(b, (")
            && (raw.contains(".__rr_cse_0 <- rr_index1_read_vec(a,")
                || raw.contains(".tachyon_exprmap0_0 <- pmin(pmax("))
            && (raw.contains(".__rr_cse_1 <- rr_index1_read_vec(b,")
                || raw.contains(".tachyon_exprmap1_0 <- pmin(pmax("))
            && raw.contains("next_a <- rr_assign_slice(")
            && raw.contains("next_b <- rr_assign_slice(next_b")
            && !raw.contains("next_a[i] <-")
            && !raw.contains("next_b[i] <-")
            && !raw.contains("rr_index1_read(a, i, \"index\")")
            && !raw.contains("rr_index1_read(b, i, \"index\")")
            && !raw.contains("rr_index1_write(i, \"index\")")
            && !raw.contains("Sym_20(next_a_cell, 0, 1)")
            && !raw.contains("Sym_20(next_b_cell, 0, 1)"),
        "reaction_diffusion raw emitted R should vectorize the interior stencil loop before peephole"
    );
}

#[test]
fn signal_pipeline_x_stage_direct_read_happens_before_peephole() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = rr_bin_path(&root);
    let rr_path = root
        .join("example")
        .join("benchmarks")
        .join("signal_pipeline_bench.rr");
    let out_dir = root
        .join("target")
        .join("benchmark_examples_signal_pipeline_raw");
    fs::create_dir_all(&out_dir)
        .expect("failed to create target/benchmark_examples_signal_pipeline_raw");
    let out_path = out_dir.join("signal_pipeline_bench_o2.R");
    let raw_path = out_dir.join("signal_pipeline_bench_raw.R");

    let raw_env = raw_path
        .to_str()
        .expect("raw benchmark output path should be valid unicode")
        .to_string();
    let status = std::process::Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("-O2")
        .arg("--no-incremental")
        .env("RR_DEBUG_RAW_R_PATH", raw_env.as_str())
        .status()
        .expect("failed to compile signal pipeline benchmark with raw output enabled");
    assert!(status.success(), "signal pipeline benchmark compile failed");

    let raw = fs::read_to_string(&raw_path).expect("failed to read raw emitted signal pipeline R");
    assert!(
        raw.contains("x <- (clean + (y * 0.15))"),
        "signal_pipeline raw emitted R should recover the direct whole-range y read before peephole"
    );
    assert!(
        raw.contains("print(clean[n])") && !raw.contains("rr_index1_read(clean, n, \"index\")"),
        "signal_pipeline raw emitted R should recover the direct tail scalar read before peephole"
    );
    assert!(
        !raw.contains("rr_index1_read_vec(y, rr_index_vec_floor(1L:n))"),
        "signal_pipeline raw emitted R unexpectedly still keeps the full-range rr_index1_read_vec wrapper"
    );
}
