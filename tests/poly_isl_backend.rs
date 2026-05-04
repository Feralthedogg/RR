use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn poly_backend_isl_emits_real_isl_artifact_in_certificate_dump() {
    if std::env::var("RR_HAS_ISL").ok().as_deref() != Some("1") {
        return;
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("poly_isl_backend");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/poly_isl_backend");

    let rr_src = r#"
fn poly_isl_backend_map(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let b = matrix(seq_len((n * m)), n, m)
  let out = matrix(seq_len((n * m)), n, m)
  for (r in 1..n) {
    for (c in 1..m) {
      out[r, c] = a[r, c] + b[r, c]
    }
  }
  return out
}

print(poly_isl_backend_map(4, 4))
"#;

    let rr_path = out_dir.join("poly_isl_backend.rr");
    let out_path = out_dir.join("poly_isl_backend.R");
    let dump_dir = out_dir.join("poly_dump");
    let stats_path = out_dir.join("poly_isl_backend_stats.json");
    let _ = fs::remove_dir_all(&dump_dir);
    fs::create_dir_all(&dump_dir).expect("failed to create poly dump dir");
    fs::write(&rr_path, rr_src).expect("failed to write rr source");

    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));
    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .env("RR_POLY_ENABLE", "1")
        .env_remove("RR_POLY_BACKEND")
        .env("RR_POLY_GENERIC_MIR", "1")
        .env("RR_POLY_DUMP_DIR", &dump_dir)
        .env("RR_PULSE_JSON_PATH", &stats_path)
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let stats = fs::read_to_string(&stats_path).ok();

    let dumps = fs::read_dir(&dump_dir)
        .expect("failed to read dump dir")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    if let Some(cert_name) = dumps.iter().find(|name| name.ends_with(".poly.txt")) {
        let cert_body = fs::read_to_string(dump_dir.join(cert_name))
            .expect("failed to read poly certificate dump");
        assert!(
            cert_body.contains("schedule_tree_primary: SchedulePlan { kind:")
                && cert_body.contains("first_band_partial_schedule=")
                && cert_body.contains("S0[")
                && cert_body.contains("schedule_tree_backend_artifact: Some(")
                && cert_body.contains("domain=[")
                && cert_body.contains("S0[")
                && cert_body.contains("validity=")
                && cert_body.contains("proximity=")
                && cert_body.contains("coincidence=")
                && cert_body.contains("conditional_validity=")
                && cert_body.contains("conditional_validity_candidate=")
                && cert_body.contains("hint_selected=")
                && cert_body.contains("hint_reason=")
                && cert_body.contains("computed_schedule=")
                && cert_body.contains("candidate_roundtrip=")
                && cert_body.contains("schedule_decision_reason:"),
            "expected real isl backend artifact in certificate dump, got:\n{}",
            cert_body
        );
    } else if let Some(stats) = stats {
        fn extract_stat(stats: &str, key: &str) -> Option<u64> {
            let needle = format!("\"{key}\":");
            let start = stats.find(&needle)? + needle.len();
            let digits = stats[start..]
                .chars()
                .skip_while(|ch| ch.is_ascii_whitespace())
                .take_while(|ch| ch.is_ascii_digit())
                .collect::<String>();
            digits.parse::<u64>().ok()
        }
        let attempted = extract_stat(&stats, "poly_schedule_attempted").unwrap_or(0);
        let hinted = extract_stat(&stats, "poly_schedule_backend_hint_selected").unwrap_or(0);
        assert!(
            attempted >= 1 && hinted >= 1,
            "expected isl backend to at least surface backend-hint stats, got:\n{}",
            stats
        );
    }
}

#[test]
fn poly_backend_isl_surfaces_tile_choice_for_dense_2d_map() {
    if std::env::var("RR_HAS_ISL").ok().as_deref() != Some("1") {
        return;
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("poly_isl_backend_tile");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/poly_isl_backend_tile");

    let rr_src = r#"
fn poly_isl_backend_tile_map(n, m) {
  let a = matrix(seq_len((n * m)), n, m)
  let b = matrix(seq_len((n * m)), n, m)
  let out = matrix(seq_len((n * m)), n, m)
  let r = 1
  while (r <= n) {
    let c = 1
    while (c <= m) {
      out[r, c] = a[r, c] + b[r, c]
      c += 1
    }
    r += 1
  }
  return out
}

print(poly_isl_backend_tile_map(16, 16))
"#;

    let rr_path = out_dir.join("poly_isl_backend_tile.rr");
    let out_path = out_dir.join("poly_isl_backend_tile.R");
    let dump_dir = out_dir.join("poly_dump");
    let stats_path = out_dir.join("poly_isl_backend_tile_stats.json");
    let _ = fs::remove_dir_all(&dump_dir);
    fs::create_dir_all(&dump_dir).expect("failed to create poly dump dir");
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
        .env("RR_POLY_GENERIC_MIR", "1")
        .env("RR_POLY_TILE_2D", "1")
        .env("RR_POLY_DUMP_DIR", &dump_dir)
        .env("RR_PULSE_JSON_PATH", &stats_path)
        .status()
        .expect("failed to run RR compiler");
    assert!(
        status.success(),
        "RR compile failed for {}",
        rr_path.display()
    );

    let cert_bodies = fs::read_dir(&dump_dir)
        .expect("failed to read dump dir")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("txt"))
        .filter_map(|path| fs::read_to_string(path).ok())
        .collect::<Vec<_>>();
    if !cert_bodies.is_empty() {
        assert!(
            cert_bodies.iter().any(|body| {
                body.contains("chosen_kind=Tile2D")
                    && body.contains("hint_candidate_tile=1")
                    && body.contains("schedule_tree_backend_artifact: Some(")
            }),
            "expected isl backend tile choice surface in certificate dump, got:\n{}",
            cert_bodies.join("\n---\n")
        );
    } else {
        let stats = fs::read_to_string(&stats_path).ok();
        let emitted = fs::read_to_string(&out_path).expect("failed to read emitted R");
        assert!(
            stats
                .as_deref()
                .is_none_or(|stats| stats.contains("\"poly_schedule_applied_tile2d\": 1"))
                && emitted.contains(".__poly_gen_iv_tile_2_r <- (.__poly_gen_iv_tile_2_r +")
                && emitted.contains(".__poly_gen_iv_tile_2_c <- (.__poly_gen_iv_tile_2_c +"),
            "expected isl backend tile schedule to surface through stats/emitted code when certificate dump is absent, stats:\n{}\n\nemitted:\n{}",
            stats.as_deref().unwrap_or("<missing>"),
            emitted
        );
    }
}
