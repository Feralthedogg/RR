use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn poly_backend_isl_emits_validity_relation_for_true_loop_carried_dependence() {
    if std::env::var("RR_HAS_ISL").ok().as_deref() != Some("1") {
        return;
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("poly_isl_dependence");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/poly_isl_dependence");

    let rr_src = r#"
fn poly_isl_dependence_stencil(n) {
  let x = seq_len(n)
  for (i in 2..length(x)) {
    x[i] = x[i - 1] + x[i]
  }
  return x
}

print(poly_isl_dependence_stencil(6))
"#;

    let rr_path = out_dir.join("poly_isl_dependence.rr");
    let out_path = out_dir.join("poly_isl_dependence.R");
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
        .env("RR_POLY_BACKEND", "isl")
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
    let cert_name = dumps
        .iter()
        .find(|name| name.ends_with(".poly.txt"))
        .expect("expected poly certificate file");
    let cert_body =
        fs::read_to_string(dump_dir.join(cert_name)).expect("failed to read poly certificate dump");
    assert!(
        cert_body.contains("DependenceSummary { state: Unknown")
            && cert_body.contains("validity_relation: Some(")
            && cert_body.contains("raw_relation: Some(")
            && cert_body.contains("proximity_relation: Some(")
            && cert_body.contains("proximity="),
        "expected isl dependence relation in certificate dump, got:\n{}",
        cert_body
    );
}
