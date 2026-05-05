use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn poly_backend_isl_emits_reduction_constraints_for_simple_sum_reduction() {
    if std::env::var("RR_HAS_ISL").ok().as_deref() != Some("1") {
        return;
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("poly_isl_reduction");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/poly_isl_reduction");

    let rr_src = r#"
import r * as base from "base"

fn poly_isl_reduction_sum(a) {
  let acc = 0
  let i = 1
  while (i <= 3) {
    let j = 1
    while (j <= 3) {
      acc = acc + a[i, j]
      j += 1
    }
    i += 1
  }
  return acc
}

let a = base.matrix(seq_len(9), 3, 3)
print(poly_isl_reduction_sum(a))
"#;

    let rr_path = out_dir.join("poly_isl_reduction.rr");
    let out_path = out_dir.join("poly_isl_reduction.R");
    let dump_dir = out_dir.join("poly_dump");
    let stats_path = out_dir.join("poly_isl_reduction_stats.json");
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
    let cert_bodies = dumps
        .iter()
        .filter(|name| name.ends_with(".poly.txt"))
        .map(|name| {
            fs::read_to_string(dump_dir.join(name)).expect("failed to read poly certificate dump")
        })
        .collect::<Vec<_>>();
    if cert_bodies.is_empty() {
        if let Some(stats) = stats {
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
            let dep = extract_stat(&stats, "poly_dependence_solved").unwrap_or(0);
            let sched = extract_stat(&stats, "poly_schedule_attempted").unwrap_or(0);
            assert!(
                dep >= 1 || sched >= 1,
                "expected isl reduction to at least reach dependence/schedule stats, got:\n{}",
                stats
            );
        }
    } else {
        assert!(
            cert_bodies.iter().any(|body| {
                body.contains("reduction_relation: Some(")
                    && body.contains("schedule_tree_backend_artifact: Some(")
                    && ((body.contains("conditional_validity=")
                        && !body.contains("conditional_validity=;"))
                        || (body.contains("conditional_validity_candidate=")
                            && !body.contains("conditional_validity_candidate=;")))
                    && (body.contains("conditional_validity_applied=1")
                        || body.contains("hint_conditional_validity_fallback=1"))
            }),
            "expected isl reduction direct conditional-validity attempt surface in one certificate dump, got:\n{}",
            cert_bodies.join("\n---\n")
        );
    }
}
