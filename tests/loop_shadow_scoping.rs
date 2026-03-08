mod common;

use common::{compile_rr, normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::PathBuf;

#[test]
fn shadowed_loop_local_does_not_leak_outside_while_scope() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping loop_shadow_scoping test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("loop_shadow_scoping");
    fs::create_dir_all(&out_dir).expect("failed to create loop_shadow_scoping dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
fn update_box(box, step, gain, shift) {
  box.total = box.total + ((step * gain) + shift);
  if ((step % 2L) == 0L) {
    box.evens = box.evens + step;
  } else {
    box.odds = box.odds + step;
  }
  return box;
}

fn main() {
  let box = {total: 0L, evens: 0L, odds: 0L};
  let i = 1L;
  while (i <= 6L) {
    let box = update_box(box, i, 3L, 1L);
    i = i + 1L;
  }
  print(box.total);
  print(box.evens);
  print(box.odds);
  return box.total + box.evens + box.odds;
}

print(main());
"#;

    let rr_path = out_dir.join("loop_shadow_scoping.rr");
    fs::write(&rr_path, rr_src).expect("failed to write RR source");

    let o0 = out_dir.join("loop_shadow_scoping_o0.R");
    let o2 = out_dir.join("loop_shadow_scoping_o2.R");

    compile_rr(&rr_bin, &rr_path, &o0, "-O0");
    compile_rr(&rr_bin, &rr_path, &o2, "-O2");

    let run_o0 = run_rscript(&rscript, &o0);
    let run_o2 = run_rscript(&rscript, &o2);

    assert_eq!(run_o0.status, 0, "O0 failed:\n{}", run_o0.stderr);
    assert_eq!(run_o2.status, 0, "O2 failed:\n{}", run_o2.stderr);
    assert_eq!(normalize(&run_o0.stdout), normalize(&run_o2.stdout));
    assert_eq!(normalize(&run_o0.stderr), normalize(&run_o2.stderr));
    assert_eq!(normalize(&run_o0.stdout), "[1] 0\n[1] 0\n[1] 0\n[1] 0\n");
}
