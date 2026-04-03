use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn stable_hash_bytes(bytes: &[u8]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET_BASIS;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn collect_rs_files(root: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    Ok(())
}

fn main() {
    println!("cargo:rustc-check-cfg=cfg(rr_has_isl)");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-env-changed=RR_ISL_LIB_DIR");

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let mut files = Vec::new();
    collect_rs_files(&manifest_dir.join("src"), &mut files).expect("failed to scan src/");
    files.push(manifest_dir.join("build.rs"));
    files.push(manifest_dir.join("Cargo.toml"));
    files.sort();

    let mut hash = 0xcbf29ce484222325u64;
    for path in files {
        let rel = path
            .strip_prefix(&manifest_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();
        hash ^= stable_hash_bytes(rel.as_bytes());
        hash = hash.wrapping_mul(0x100000001b3);
        let bytes = fs::read(&path).unwrap_or_else(|err| {
            panic!(
                "failed to read {} for compiler build hash: {}",
                path.display(),
                err
            )
        });
        hash ^= stable_hash_bytes(&bytes);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    println!("cargo:rustc-env=RR_COMPILER_BUILD_HASH={hash:016x}");

    let isl_candidates = std::env::var_os("RR_ISL_LIB_DIR")
        .map(PathBuf::from)
        .into_iter()
        .chain([
            PathBuf::from("/opt/homebrew/lib"),
            PathBuf::from("/usr/local/lib"),
            PathBuf::from("/usr/lib"),
        ]);
    for dir in isl_candidates {
        let dylib = dir.join("libisl.dylib");
        let so = dir.join("libisl.so");
        let archive = dir.join("libisl.a");
        if dylib.exists() || so.exists() || archive.exists() {
            println!("cargo:rustc-link-search=native={}", dir.display());
            println!("cargo:rustc-link-lib=dylib=isl");
            println!("cargo:rustc-cfg=rr_has_isl");
            println!("cargo:rustc-env=RR_HAS_ISL=1");
            return;
        }
    }
    println!("cargo:rustc-env=RR_HAS_ISL=0");
}
