mod common;

use common::{normalize, rscript_available, rscript_path};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn ggplot2_available(rscript: &str) -> bool {
    Command::new(rscript)
        .arg("--vanilla")
        .arg("-e")
        .arg("quit(status = if (requireNamespace('ggplot2', quietly = TRUE)) 0 else 1)")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[test]
fn ggplot2_direct_surface_matches_between_o0_and_o2() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) && ggplot2_available(&p) => p,
        _ => {
            eprintln!("Skipping ggplot2 direct interop runtime test: ggplot2 unavailable.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("ggplot2_direct_interop");
    fs::create_dir_all(&out_dir).expect("failed to create output dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let src = r#"
import r default from "ggplot2"
import r * as base from "base"

fn save_plot() -> char {
  let outfile = "ggplot2_direct_interop.png"
  let df = base.data.frame(
    x = c(1.0, 2.0, 3.0),
    y = c(1.0, 4.0, 9.0)
  )
  let p = ggplot2.ggplot(df, ggplot2.aes(x = x, y = y)) +
    ggplot2.geom_line(color = "steelblue") +
    ggplot2.ggtitle("direct interop") +
    ggplot2.theme_minimal()
  let saved = ggplot2.ggsave(filename = outfile, plot = p, width = 4, height = 3, dpi = 72)
  print(saved)
  return saved
}

print(save_plot())
"#;

    let rr_path = out_dir.join("ggplot2_direct_interop.rr");
    let o0 = out_dir.join("ggplot2_direct_interop_o0.R");
    let o2 = out_dir.join("ggplot2_direct_interop_o2.R");

    fs::write(&rr_path, src).expect("failed to write source");

    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&o0)
        .arg("-O0")
        .status()
        .expect("failed to compile O0");
    assert!(status.success(), "O0 compile failed");

    let status = Command::new(&rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&o2)
        .arg("-O2")
        .status()
        .expect("failed to compile O2");
    assert!(status.success(), "O2 compile failed");

    let run_o0 = Command::new(&rscript)
        .current_dir(&out_dir)
        .arg("--vanilla")
        .arg(&o0)
        .output()
        .expect("failed to execute O0 Rscript");
    let run_o2 = Command::new(&rscript)
        .current_dir(&out_dir)
        .arg("--vanilla")
        .arg(&o2)
        .output()
        .expect("failed to execute O2 Rscript");

    assert_eq!(
        run_o0.status.code().unwrap_or(-1),
        0,
        "O0 runtime failed:\n{}",
        String::from_utf8_lossy(&run_o0.stderr)
    );
    assert_eq!(
        run_o2.status.code().unwrap_or(-1),
        0,
        "O2 runtime failed:\n{}",
        String::from_utf8_lossy(&run_o2.stderr)
    );

    let stdout_o0 = normalize(&String::from_utf8_lossy(&run_o0.stdout));
    let stdout_o2 = normalize(&String::from_utf8_lossy(&run_o2.stdout));
    let stderr_o0 = normalize(&String::from_utf8_lossy(&run_o0.stderr));
    let stderr_o2 = normalize(&String::from_utf8_lossy(&run_o2.stderr));

    assert_eq!(stdout_o0, stdout_o2, "stdout mismatch O0 vs O2");
    assert_eq!(stderr_o0, stderr_o2, "stderr mismatch O0 vs O2");

    let png_path = out_dir.join("ggplot2_direct_interop.png");
    let meta = fs::metadata(&png_path).expect("expected ggplot2 PNG output");
    assert!(meta.len() > 0, "expected non-empty ggplot2 PNG output");
}
