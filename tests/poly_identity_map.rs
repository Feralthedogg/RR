use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn compile_with_poly_outputs(name: &str, rr_src: &str) -> (String, String) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join(name);
    fs::create_dir_all(&out_dir).expect("failed to create target/tests dir");

    let rr_path = out_dir.join(format!("{name}.rr"));
    let out_path = out_dir.join(format!("{name}.R"));
    let stats_path = out_dir.join(format!("{name}_stats.json"));
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_PULSE_JSON_PATH", &stats_path)
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let stats = fs::read_to_string(&stats_path).expect("failed to read pulse stats json");
    let emitted = fs::read_to_string(&out_path).expect("failed to read emitted R");
    (stats, emitted)
}

#[test]
fn poly_identity_schedule_applies_for_simple_affine_map() {
    let (stats, emitted) = compile_with_poly_outputs(
        "poly_identity_map",
        r#"
fn poly_map(n) {
  let x = seq_len(n)

  let y = seq_len(n)

  for (i in 1..length(y)) {
    y[i] = x[i] + 1

  }
  return y

}

print(poly_map(6))
"#,
    );
    assert!(
        stats.contains("\"poly_scops_detected\": 1")
            || stats.contains("\"poly_scops_detected\": 2"),
        "expected at least one detected poly SCoP, got:\n{}",
        stats
    );
    assert!(
        stats.contains("\"poly_schedule_applied\": 1"),
        "expected one applied poly schedule, got:\n{}",
        stats
    );
    assert!(
        !emitted.contains("rr_assign_slice(") && !emitted.contains("rr_can_same_or_scalar("),
        "expected single 1d poly map to rebuild generic loop without slice helper, got:\n{}",
        emitted
    );
}

#[test]
fn poly_dump_dir_emits_certificate_files() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("poly_identity_dump");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/poly_identity_dump");

    let rr_src = r#"
fn poly_map_dump(n) {
  let x = seq_len(n)
  let y = seq_len(n)
  for (i in 1..length(y)) {
    y[i] = x[i] + 1
  }
  return y
}

print(poly_map_dump(6))
"#;

    let rr_path = out_dir.join("poly_identity_dump.rr");
    let out_path = out_dir.join("poly_identity_dump.R");
    let dump_dir = out_dir.join("poly_dump");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env("RR_POLY_DUMP_DIR", &dump_dir)
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let dumps = fs::read_dir(&dump_dir)
        .expect("failed to read dump dir")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert!(
        dumps.iter().any(|name| name.ends_with(".poly.txt")),
        "expected at least one poly certificate dump, got {:?}",
        dumps
    );
    let cert_name = dumps
        .iter()
        .find(|name| name.ends_with(".poly.txt"))
        .expect("expected poly certificate file");
    let cert_body =
        fs::read_to_string(dump_dir.join(cert_name)).expect("failed to read poly certificate dump");
    assert!(
        cert_body.contains("schedule_tree:")
            && cert_body.contains("dependence_relation:")
            && cert_body.contains("dependence_edge_snapshot:")
            && cert_body.contains("schedule_tree_primary:")
            && cert_body.contains("schedule_tree_band_depth:")
            && cert_body.contains("estimated_cost")
            && cert_body.contains("Filter")
            && cert_body.contains("Transform"),
        "expected schedule tree in certificate dump, got:\n{}",
        cert_body
    );
}
