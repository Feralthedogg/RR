mod common;

use common::{normalize, rscript_available, rscript_path, run_rscript, unique_dir};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn generated_rr_program(expressions: &[String]) -> String {
    let mut src = String::from("fn main() {\n");
    for expr in expressions {
        src.push_str("  print(");
        src.push_str(expr);
        src.push_str(")\n");
    }
    src.push_str("  return 0L\n}\n\nprint(main())\n");
    src
}

fn generated_reference_program(expressions: &[String]) -> String {
    let mut src = String::from("main <- function() {\n");
    for expr in expressions {
        src.push_str("  print(");
        src.push_str(expr);
        src.push_str(")\n");
    }
    src.push_str("  0L\n}\n\nprint(main())\n");
    src
}

#[test]
fn generated_numeric_edges_match_reference_across_opt_levels() {
    let Some(rscript) = rscript_path() else {
        return;
    };
    if !rscript_available(&rscript) {
        return;
    }

    let int_edges = [
        "2147483646L + 1L",
        "-2147483647L + 1L",
        "46340L * 46340L",
        "-46340L * 46340L",
        "2147483647L / 3L",
        "-2147483647L / 3L",
        "abs(-2147483647L)",
        "trunc(-2.9)",
        "ceiling(2.1)",
    ];
    let float_edges = [
        "(1.5 + 2.25) * -3.0",
        "(1.0 / 3.0) + (2.0 / 3.0)",
        "floor(-2.75)",
        "round(2.4)",
        "round(-2.4)",
        "sqrt(2.0) * sqrt(2.0)",
    ];
    let expressions: Vec<String> = int_edges
        .into_iter()
        .chain(float_edges)
        .map(str::to_string)
        .collect();

    let rr_src = generated_rr_program(&expressions);
    let ref_src = generated_reference_program(&expressions);

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sandbox_root = root
        .join("target")
        .join("tests")
        .join("numeric_property_differential");
    fs::create_dir_all(&sandbox_root).expect("failed to create sandbox root");
    let proj = unique_dir(&sandbox_root, "numeric");
    fs::create_dir_all(&proj).expect("failed to create project dir");

    let rr_path = proj.join("numeric.rr");
    let ref_path = proj.join("reference.R");
    fs::write(&rr_path, &rr_src).expect("failed to write generated RR source");
    fs::write(&ref_path, &ref_src).expect("failed to write generated reference source");

    let reference = run_rscript(&rscript, &ref_path);
    assert_eq!(
        reference.status, 0,
        "reference numeric script failed:\n{}",
        reference.stderr
    );

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    for opt in ["-O0", "-O1", "-O2"] {
        let out_path = proj.join(format!("compiled_{}.R", opt.trim_start_matches('-')));
        let compile = Command::new(&rr_bin)
            .arg(&rr_path)
            .arg("-o")
            .arg(&out_path)
            .arg(opt)
            .output()
            .expect("failed to compile generated RR numeric case");
        assert!(
            compile.status.success(),
            "numeric differential compile failed for {opt}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        );
        let compiled = run_rscript(&rscript, &out_path);
        assert_eq!(
            compiled.status, 0,
            "compiled numeric script failed for {opt}:\n{}",
            compiled.stderr
        );
        assert_eq!(
            normalize(&reference.stdout),
            normalize(&compiled.stdout),
            "stdout mismatch for {opt}"
        );
        assert_eq!(
            normalize(&reference.stderr),
            normalize(&compiled.stderr),
            "stderr mismatch for {opt}"
        );
    }
}
