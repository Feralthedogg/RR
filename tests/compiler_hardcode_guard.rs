use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

#[test]
fn compiler_core_does_not_reintroduce_benchmark_named_rewrites() {
    let root = repo_root();
    let files = [
        root.join("src").join("compiler").join("pipeline.rs"),
        root.join("src").join("compiler").join("r_peephole.rs"),
        root.join("src").join("mir").join("opt.rs"),
        root.join("src").join("mir").join("opt").join("helpers.rs"),
        root.join("src")
            .join("mir")
            .join("opt")
            .join("v_opt")
            .join("analysis.rs"),
        root.join("src")
            .join("mir")
            .join("opt")
            .join("v_opt")
            .join("api.rs"),
        root.join("src")
            .join("mir")
            .join("opt")
            .join("v_opt")
            .join("planning.rs"),
        root.join("src")
            .join("mir")
            .join("opt")
            .join("v_opt")
            .join("reconstruct.rs"),
        root.join("src")
            .join("mir")
            .join("opt")
            .join("v_opt")
            .join("transform.rs"),
    ];
    let forbidden = [
        "RR_ENABLE_BENCH_REWRITES",
        "signal_pipeline",
        "heat_diffusion",
        "reaction_diffusion",
        "vector_fusion",
        "orbital_sweep",
        "bootstrap_resample",
        "morphogenesis",
        "tesseract",
    ];

    let mut hits = Vec::new();
    for path in files {
        let src = read(&path);
        for needle in forbidden {
            if src.contains(needle) {
                hits.push(format!("{} => {}", path.display(), needle));
            }
        }
    }

    assert!(
        hits.is_empty(),
        "compiler/optimizer core unexpectedly contains benchmark-named logic again: {:?}",
        hits
    );
}
