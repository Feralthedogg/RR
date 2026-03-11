use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn dotted_builtin_names_compile_in_call_position() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = root
        .join("target")
        .join("tests")
        .join("dotted_builtin_calls");
    fs::create_dir_all(&out_dir).expect("failed to create target/tests/dotted_builtin_calls");

    let rr_src = r#"
let main <- function() {
  let x <- c(1L, NA, 3L)
  print(is.na(x[2L]))
  print(is.finite(3.14))
  0L
}

print(main())
"#;

    let rr_path = out_dir.join("dotted_builtin_calls.rr");
    fs::write(&rr_path, rr_src).expect("failed to write dotted_builtin_calls.rr");
    let out_path = out_dir.join("dotted_builtin_calls.R");
    let rr_bin = PathBuf::from(env!("CARGO_BIN_EXE_RR"));

    let status = Command::new(rr_bin)
        .arg(&rr_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--no-runtime")
        .arg("-O1")
        .status()
        .expect("failed to run RR compiler");

    assert!(status.success(), "RR compile failed for dotted builtins");

    let emitted = fs::read_to_string(&out_path).expect("failed to read emitted R file");
    assert!(
        emitted.contains("is.na"),
        "expected emitted code to contain dotted builtin call `is.na`"
    );
    assert!(
        emitted.contains("is.finite"),
        "expected emitted code to contain dotted builtin call `is.finite`"
    );
}
