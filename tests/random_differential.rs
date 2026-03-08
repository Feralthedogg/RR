mod common;

use common::random_rr::{generate_cases, suite_summary};
use common::{compile_rr_env, normalize, rscript_available, rscript_path, run_rscript, unique_dir};
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

fn env_case_count() -> usize {
    env::var("RR_RANDOM_DIFFERENTIAL_COUNT")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&v| v >= 8)
        .unwrap_or(18)
}

fn env_seed() -> u64 {
    env::var("RR_RANDOM_DIFFERENTIAL_SEED")
        .ok()
        .and_then(|v| {
            u64::from_str_radix(v.trim_start_matches("0x"), 16)
                .ok()
                .or_else(|| v.parse::<u64>().ok())
        })
        .unwrap_or(0x0DD1_FFEE_CAFE_BAAD)
}

struct FailureBundleInput<'a> {
    root: &'a std::path::Path,
    case_name: &'a str,
    opt_tag: &'a str,
    rr_src: &'a str,
    ref_src: &'a str,
    emitted_r: &'a str,
    reference: &'a common::RunResult,
    compiled: &'a common::RunResult,
}

fn write_failure_bundle(input: FailureBundleInput<'_>) -> PathBuf {
    let FailureBundleInput {
        root,
        case_name,
        opt_tag,
        rr_src,
        ref_src,
        emitted_r,
        reference,
        compiled,
    } = input;
    let failure_root = root
        .join("target")
        .join("tests")
        .join("random_differential_failures");
    fs::create_dir_all(&failure_root).expect("failed to create differential failure root");
    let bundle_dir = unique_dir(&failure_root, &format!("{case_name}_{opt_tag}"));
    fs::create_dir_all(&bundle_dir).expect("failed to create differential failure dir");
    fs::write(bundle_dir.join("case.rr"), rr_src).expect("failed to write RR failure case");
    fs::write(bundle_dir.join("reference.R"), ref_src)
        .expect("failed to write reference failure case");
    fs::write(bundle_dir.join("compiled.R"), emitted_r)
        .expect("failed to write compiled R artifact");
    fs::write(bundle_dir.join("reference.stdout"), &reference.stdout)
        .expect("failed to write reference stdout");
    fs::write(bundle_dir.join("reference.stderr"), &reference.stderr)
        .expect("failed to write reference stderr");
    fs::write(bundle_dir.join("compiled.stdout"), &compiled.stdout)
        .expect("failed to write compiled stdout");
    fs::write(bundle_dir.join("compiled.stderr"), &compiled.stderr)
        .expect("failed to write compiled stderr");
    let mut manifest = fs::File::create(bundle_dir.join("bundle.manifest"))
        .expect("failed to write machine manifest");
    writeln!(manifest, "schema: rr-triage-bundle").expect("failed to write manifest");
    writeln!(manifest, "version: 1").expect("failed to write manifest");
    writeln!(manifest, "kind: differential").expect("failed to write manifest");
    writeln!(manifest, "case: {case_name}").expect("failed to write manifest");
    writeln!(manifest, "opt: {opt_tag}").expect("failed to write manifest");
    writeln!(manifest, "reference_status: {}", reference.status).expect("failed to write manifest");
    writeln!(manifest, "compiled_status: {}", compiled.status).expect("failed to write manifest");

    let mut readme =
        fs::File::create(bundle_dir.join("README.txt")).expect("failed to write failure manifest");
    writeln!(readme, "case: {case_name}").expect("failed to write manifest");
    writeln!(readme, "opt: {opt_tag}").expect("failed to write manifest");
    writeln!(readme, "reference_status: {}", reference.status).expect("failed to write manifest");
    writeln!(readme, "compiled_status: {}", compiled.status).expect("failed to write manifest");
    writeln!(readme, "files:").expect("failed to write manifest");
    writeln!(readme, "  - case.rr").expect("failed to write manifest");
    writeln!(readme, "  - reference.R").expect("failed to write manifest");
    writeln!(readme, "  - compiled.R").expect("failed to write manifest");
    writeln!(readme, "  - reference.stdout / reference.stderr").expect("failed to write manifest");
    writeln!(readme, "  - compiled.stdout / compiled.stderr").expect("failed to write manifest");
    bundle_dir
}

fn assert_run_matches_reference(
    root: &std::path::Path,
    case: &common::random_rr::GeneratedCase,
    opt_flag: &str,
    opt_tag: &str,
    emitted_r: &str,
    reference: &common::RunResult,
    compiled: &common::RunResult,
) {
    let same_status = reference.status == compiled.status;
    let same_stdout = normalize(&reference.stdout) == normalize(&compiled.stdout);
    let same_stderr = normalize(&reference.stderr) == normalize(&compiled.stderr);
    if same_status && same_stdout && same_stderr {
        return;
    }

    let bundle_dir = write_failure_bundle(FailureBundleInput {
        root,
        case_name: &case.name,
        opt_tag,
        rr_src: &case.rr_src,
        ref_src: &case.ref_r_src,
        emitted_r,
        reference,
        compiled,
    });

    assert!(
        same_status,
        "status mismatch for {} ({opt_flag})\nreference stdout:\n{}\ncompiled stdout:\n{}\nreference stderr:\n{}\ncompiled stderr:\n{}\nrr source:\n{}\nfailure bundle: {}",
        case.name,
        reference.stdout,
        compiled.stdout,
        reference.stderr,
        compiled.stderr,
        case.rr_src,
        bundle_dir.display()
    );
    assert!(
        same_stdout,
        "stdout mismatch for {} ({opt_flag})\nreference:\n{}\ncompiled:\n{}\nrr source:\n{}\nfailure bundle: {}",
        case.name,
        reference.stdout,
        compiled.stdout,
        case.rr_src,
        bundle_dir.display()
    );
    assert!(
        same_stderr,
        "stderr mismatch for {} ({opt_flag})\nreference:\n{}\ncompiled:\n{}\nrr source:\n{}\nfailure bundle: {}",
        case.name,
        reference.stderr,
        compiled.stderr,
        case.rr_src,
        bundle_dir.display()
    );
}

#[test]
fn generated_rr_programs_match_reference_r_across_opt_levels() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping random differential test: Rscript not available.");
            return;
        }
    };

    let case_count = env_case_count();
    let cases = generate_cases(env_seed(), case_count);
    assert_eq!(
        cases.len(),
        case_count,
        "generator returned unexpected case count"
    );

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("random_differential");
    fs::create_dir_all(&sandbox_root).expect("failed to create random differential sandbox");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create project dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    for case in &cases {
        let rr_path = proj_dir.join(format!("{}.rr", case.name));
        let ref_path = proj_dir.join(format!("{}_ref.R", case.name));
        fs::write(&rr_path, &case.rr_src).expect("failed to write RR case");
        fs::write(&ref_path, &case.ref_r_src).expect("failed to write reference R case");

        let reference = run_rscript(&rscript, &ref_path);
        assert_eq!(
            reference.status, 0,
            "reference R failed for {}\nstdout:\n{}\nstderr:\n{}\nsource:\n{}",
            case.name, reference.stdout, reference.stderr, case.ref_r_src
        );

        for (flag, tag) in [("-O0", "o0"), ("-O1", "o1"), ("-O2", "o2")] {
            let out_path = proj_dir.join(format!("{}_{}.R", case.name, tag));
            compile_rr_env(
                &rr_bin,
                &rr_path,
                &out_path,
                flag,
                &[("RR_VERIFY_EACH_PASS", "1")],
            );
            let compiled = run_rscript(&rscript, &out_path);
            let emitted_r =
                fs::read_to_string(&out_path).expect("failed to read emitted differential R");
            assert_run_matches_reference(&root, case, flag, tag, &emitted_r, &reference, &compiled);
        }
    }
}

#[test]
fn generated_rr_suite_covers_multiple_program_shapes() {
    let cases = generate_cases(0x5EED_1234_5678_9ABC, 24);
    let summary = suite_summary(&cases);
    assert!(summary.contains("branch_vec_fold"));
    assert!(summary.contains("recurrence"));
    assert!(summary.contains("matrix_fold"));
    assert!(summary.contains("nested_loop"));
    assert!(summary.contains("call_chain"));
    assert!(summary.contains("tail_recur"));
    assert!(summary.contains("record_state"));
    assert!(summary.contains("stats_namespace"));
}
