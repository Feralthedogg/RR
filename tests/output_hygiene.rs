use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn generated_artifact_has_single_section_markers_and_no_duplicate_sym_defs() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let src = root.join("example").join("tesseract.rr");
    let out_dir = root.join("target").join("tests").join("output_hygiene");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let out = out_dir.join("tesseract_o2.R");

    let status = Command::new(&rr_bin)
        .arg(&src)
        .arg("-o")
        .arg(&out)
        .arg("-O2")
        .arg("--no-incremental")
        .status()
        .expect("failed to run RR compiler");
    assert!(status.success(), "RR compile failed for {}", src.display());

    let generated = fs::read_to_string(&out).expect("failed to read generated artifact");
    assert_eq!(
        generated
            .matches("# --- RR runtime (auto-generated) ---")
            .count(),
        1,
        "runtime banner should appear exactly once"
    );
    assert_eq!(
        generated
            .matches("# --- RR generated code (from user RR source) ---")
            .count(),
        1,
        "generated-code banner should appear exactly once"
    );
    assert_eq!(
        generated
            .matches("# --- RR synthesized entrypoints (auto-generated) ---")
            .count(),
        1,
        "entrypoint banner should appear exactly once"
    );

    let mut counts: HashMap<String, usize> = HashMap::new();
    for line in generated.lines() {
        if let Some(name) = line.strip_suffix(" <- function() ") {
            *counts.entry(name.to_string()).or_insert(0) += 1;
            continue;
        }
        if let Some((name, _)) = line.split_once(" <- function(") {
            *counts.entry(name.to_string()).or_insert(0) += 1;
        }
    }

    let dups: Vec<_> = counts.into_iter().filter(|(_, count)| *count > 1).collect();
    assert!(
        dups.is_empty(),
        "duplicate generated function definitions found: {:?}",
        dups
    );
}
