use super::common::{compile_rr, normalize, rscript_available, rscript_path, run_rscript};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
pub(crate) fn utils_head_now_uses_direct_interop_not_opaque_or_hybrid_fallback() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping opaque interop R import test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_package_import_utils");
    fs::create_dir_all(&out_dir).expect("failed to create utils import dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
	import r * as utils from "utils"


	let main <- function() {
	  print(utils.head(c(1, 2, 3), 2))
	  return 0L
	}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import_utils.rr");
    fs::write(&rr_path, rr_src).expect("failed to write utils import source");
    let out = out_dir.join("r_package_import_utils_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("utils::head(")
            && !code.contains("# rr-opaque-interop:")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected utils::head to lower as direct interop"
    );

    let output = Command::new(&rscript)
        .current_dir(&out_dir)
        .arg("--vanilla")
        .arg(&out)
        .output()
        .expect("failed to execute Rscript");
    assert_eq!(
        output.status.code().unwrap_or(-1),
        0,
        "runtime failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = normalize(&String::from_utf8_lossy(&output.stdout));
    assert!(stdout.contains("[1] 1 2"), "unexpected stdout: {}", stdout);
}

#[test]
pub(crate) fn datasets_namespace_alias_data_object_lowers_and_runs() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping datasets namespace alias test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_package_import_datasets_ns");
    fs::create_dir_all(&out_dir).expect("failed to create datasets namespace dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r * as datasets from "datasets"

let main <- function() {
  let iris_df <- datasets.iris
  print(nrow(iris_df))
  print(ncol(iris_df))
  return 0L
}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import_datasets_ns.rr");
    fs::write(&rr_path, rr_src).expect("failed to write datasets namespace source");
    let out = out_dir.join("r_package_import_datasets_ns_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("datasets::iris")
            && !code.contains("# rr-opaque-interop:")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected datasets namespace data access to lower directly"
    );

    let run = run_rscript(&rscript, &out);
    assert_eq!(run.status, 0, "runtime failed: {}", run.stderr);
    let stdout = normalize(&run.stdout);
    assert!(
        stdout.contains("[1] 150") && stdout.contains("[1] 5"),
        "unexpected stdout: {}",
        stdout
    );
}

#[test]
pub(crate) fn datasets_named_import_data_object_lowers_and_runs() {
    let rscript = match rscript_path() {
        Some(p) if rscript_available(&p) => p,
        _ => {
            eprintln!("Skipping datasets named import test: Rscript not available.");
            return;
        }
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("r_package_import_datasets_named");
    fs::create_dir_all(&out_dir).expect("failed to create datasets named dir");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let rr_src = r#"
import r { iris as iris_df } from "datasets"

let main <- function() {
  print(nrow(iris_df))
  print(ncol(iris_df))
  return 0L
}

print(main())
"#;

    let rr_path = out_dir.join("r_package_import_datasets_named.rr");
    fs::write(&rr_path, rr_src).expect("failed to write datasets named source");
    let out = out_dir.join("r_package_import_datasets_named_o2.R");
    compile_rr(&rr_bin, &rr_path, &out, "-O2");

    let code = fs::read_to_string(&out).expect("failed to read emitted R");
    assert!(
        code.contains("datasets::iris")
            && !code.contains("# rr-opaque-interop:")
            && !code.contains("# rr-hybrid-fallback:"),
        "expected datasets named import data access to lower directly"
    );

    let run = run_rscript(&rscript, &out);
    assert_eq!(run.status, 0, "runtime failed: {}", run.stderr);
    let stdout = normalize(&run.stdout);
    assert!(
        stdout.contains("[1] 150") && stdout.contains("[1] 5"),
        "unexpected stdout: {}",
        stdout
    );
}
