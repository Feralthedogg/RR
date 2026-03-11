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
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=src");

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
}
