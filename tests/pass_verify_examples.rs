mod common;

use common::unique_dir;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn write_failure_bundle(
    root: &std::path::Path,
    rr_src: &std::path::Path,
    output: &std::process::Output,
    emitted_r: Option<&str>,
    verify_dump_dir: &std::path::Path,
) -> PathBuf {
    let failure_root = root
        .join("target")
        .join("tests")
        .join("pass_verify_failures");
    fs::create_dir_all(&failure_root).expect("failed to create pass verify failure root");
    let case_name = rr_src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("case");
    let bundle_dir = unique_dir(&failure_root, case_name);
    fs::create_dir_all(&bundle_dir).expect("failed to create pass verify failure dir");
    fs::copy(rr_src, bundle_dir.join("case.rr")).expect("failed to copy failing RR case");
    fs::write(
        bundle_dir.join("compiler.stdout"),
        String::from_utf8_lossy(&output.stdout).as_bytes(),
    )
    .expect("failed to write compiler stdout");
    fs::write(
        bundle_dir.join("compiler.stderr"),
        String::from_utf8_lossy(&output.stderr).as_bytes(),
    )
    .expect("failed to write compiler stderr");
    if let Some(code) = emitted_r {
        fs::write(bundle_dir.join("compiled.R"), code).expect("failed to write emitted R");
    }
    if verify_dump_dir.exists() {
        let dump_dest = bundle_dir.join("verify-dumps");
        fs::create_dir_all(&dump_dest).expect("failed to create verify dump bundle dir");
        for entry in fs::read_dir(verify_dump_dir).expect("failed to read verify dump dir") {
            let entry = entry.expect("failed to read verify dump entry");
            let src = entry.path();
            if src.is_file() {
                let name = entry.file_name();
                fs::copy(&src, dump_dest.join(name)).expect("failed to copy verify dump file");
            }
        }
    }
    fs::write(
        bundle_dir.join("bundle.manifest"),
        format!(
            "schema: rr-triage-bundle\nversion: 1\nkind: pass-verify\ncase: {}\nstatus: {}\n",
            rr_src.display(),
            output.status.code().unwrap_or(-1)
        ),
    )
    .expect("failed to write pass verify machine manifest");
    fs::write(
        bundle_dir.join("README.txt"),
        format!(
            "case: {}\nstatus: {}\nfiles:\n  - case.rr\n  - compiler.stdout\n  - compiler.stderr\n  - compiled.R (optional)\n  - verify-dumps/* (optional)\n",
            rr_src.display(),
            output.status.code().unwrap_or(-1)
        ),
    )
    .expect("failed to write pass verify manifest");
    bundle_dir
}

#[test]
fn representative_examples_compile_with_verify_each_pass_enabled() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("pass_verify_examples");
    fs::create_dir_all(&sandbox_root).expect("failed to create pass_verify_examples root");
    let proj_dir = unique_dir(&sandbox_root, "proj");
    fs::create_dir_all(&proj_dir).expect("failed to create pass_verify_examples dir");

    let cases = [
        root.join("example")
            .join("physics")
            .join("heat_diffusion_1d.rr"),
        root.join("example")
            .join("data_science")
            .join("lm_predict_quantile_band.rr"),
        root.join("example")
            .join("visualization")
            .join("readr_tidyr_ggplot2_pipeline_modern.rr"),
        root.join("example")
            .join("visualization")
            .join("dplyr_ggplot2_pipeline_modern.rr"),
        root.join("example").join("tesseract.rr"),
    ];

    for rr_src in cases {
        let stem = rr_src
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("case");
        let out_path = proj_dir.join(format!("{stem}_o2.R"));
        let verify_dump_dir = proj_dir.join(format!("{stem}_verify_dumps"));
        fs::create_dir_all(&verify_dump_dir).expect("failed to create verify dump dir");
        let output = Command::new(&rr_bin)
            .arg(&rr_src)
            .arg("-o")
            .arg(&out_path)
            .arg("-O2")
            .env("RR_VERIFY_EACH_PASS", "1")
            .env("RR_VERIFY_DUMP_DIR", &verify_dump_dir)
            .output()
            .expect("failed to run RR compiler");
        let emitted_r = fs::read_to_string(&out_path).ok();
        if !output.status.success() {
            let bundle_dir = write_failure_bundle(
                &root,
                &rr_src,
                &output,
                emitted_r.as_deref(),
                &verify_dump_dir,
            );
            panic!(
                "pass-verify compile failed for {}\nstdout:\n{}\nstderr:\n{}\nfailure bundle: {}",
                rr_src.display(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
                bundle_dir.display()
            );
        }
        let code = emitted_r.expect("failed to read emitted R");
        assert!(
            code.contains("<- function(") || code.contains("print("),
            "unexpected empty emitted R for {}",
            rr_src.display()
        );
    }
}
