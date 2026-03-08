mod common;

use common::{compile_rr, normalize, rscript_available, rscript_path};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn tidy_eval_bare_symbols_lower_to_raw_package_calls() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root.join("target").join("tests").join("tidy_eval_interop");
    fs::create_dir_all(&out_dir).expect("failed to create tidy_eval_interop dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r default from "ggplot2"
import r default from "dplyr"
import r * as base from "base"

fn main() {
    df = base.data.frame(x = c(0, 1, 2), signal = c(0.1, 0.5, 0.9))
    out = df |> dplyr.mutate(trend = x * 0.5 + 0.2, smooth = signal * 0.8 + 0.1)
    p = ggplot2.ggplot(out, ggplot2.aes(x = x, y = trend)) +
        ggplot2.geom_line(color = "steelblue") +
        ggplot2.geom_point(ggplot2.aes(y = smooth), color = "tomato")
    print(p)
}

main()
"#;

    let rr_path = out_dir.join("tidy_eval_interop.rr");
    fs::write(&rr_path, rr_src).expect("failed to write tidy eval source");
    let out = out_dir.join("tidy_eval_interop_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("dplyr::mutate(")
            && code.contains("trend = ((x * 0.5) + 0.2)")
            && code.contains("smooth = ((signal * 0.8) + 0.1)")
            && code.contains("ggplot2::aes(x = x, y = trend)")
            && code.contains("ggplot2::aes(y = smooth)")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected tidy-eval bare symbols to survive as direct interop without hybrid fallback:\n{code}"
    );
}

#[test]
fn tidy_eval_column_and_env_markers_compile_when_packages_are_available() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping tidy eval runtime test: Rscript not available.");
            return;
        }
    };

    let packages_ok = Command::new(&rscript)
        .arg("--vanilla")
        .arg("-e")
        .arg(
            "quit(status = if (requireNamespace('ggplot2', quietly = TRUE) && requireNamespace('dplyr', quietly = TRUE)) 0 else 1)",
        )
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !packages_ok {
        eprintln!("Skipping tidy eval runtime test: ggplot2/dplyr not available.");
        return;
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("tidy_eval_interop_runtime");
    fs::create_dir_all(&out_dir).expect("failed to create tidy_eval_interop_runtime dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r default from "ggplot2"
import r default from "dplyr"
import r * as base from "base"

fn main() {
    offset = 0.25
    df = base.data.frame(x = c(0, 1, 2), signal = c(0.1, 0.5, 0.9))
    out = df |> dplyr.mutate(trend = @x * 0.5 + ^offset)
    p = ggplot2.ggplot(out, ggplot2.aes(x = @x, y = @trend)) +
        ggplot2.geom_line(color = "steelblue")
    print("tidy_eval_ok")
    print(p)
}

main()
"#;

    let rr_path = out_dir.join("tidy_eval_markers.rr");
    fs::write(&rr_path, rr_src).expect("failed to write tidy eval markers source");
    let out = out_dir.join("tidy_eval_markers_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let output = Command::new(&rscript)
        .current_dir(&out_dir)
        .arg("--vanilla")
        .arg(&out)
        .output()
        .expect("failed to execute Rscript");
    let stdout = normalize(&String::from_utf8_lossy(&output.stdout));
    let stderr = normalize(&String::from_utf8_lossy(&output.stderr));
    assert!(
        output.status.success(),
        "tidy eval runtime failed:\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("tidy_eval_ok"),
        "missing tidy eval runtime marker:\nstdout={stdout}"
    );
}
