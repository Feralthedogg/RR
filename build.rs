use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IslLinkMode {
    Auto,
    Static,
    Dylib,
}

#[derive(Debug, Clone)]
struct NativeLibPaths {
    dir: PathBuf,
    static_candidates: Vec<PathBuf>,
    dylib_candidates: Vec<PathBuf>,
}

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

fn parse_isl_link_mode() -> IslLinkMode {
    match std::env::var("RR_ISL_LINK")
        .ok()
        .map(|raw| raw.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("static") => IslLinkMode::Static,
        Some("dylib" | "dynamic" | "shared") => IslLinkMode::Dylib,
        _ => IslLinkMode::Auto,
    }
}

fn lib_candidates(dir: &Path, stem: &str) -> NativeLibPaths {
    NativeLibPaths {
        dir: dir.to_path_buf(),
        static_candidates: vec![
            dir.join(format!("lib{stem}.a")),
            dir.join(format!("{stem}.lib")),
        ],
        dylib_candidates: vec![
            dir.join(format!("lib{stem}.dylib")),
            dir.join(format!("lib{stem}.so")),
            dir.join(format!("{stem}.dll")),
            dir.join(format!("lib{stem}.dll.a")),
            dir.join(format!("{stem}.dll.lib")),
        ],
    }
}

fn static_exists(paths: &NativeLibPaths) -> bool {
    paths.static_candidates.iter().any(|path| path.exists())
}

fn dylib_exists(paths: &NativeLibPaths) -> bool {
    paths.dylib_candidates.iter().any(|path| path.exists())
}

fn push_dir_unique(out: &mut Vec<PathBuf>, dir: PathBuf) {
    if !out.iter().any(|seen| seen == &dir) {
        out.push(dir);
    }
}

fn collect_library_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for env_key in ["RR_ISL_LIB_DIR", "RR_GMP_LIB_DIR"] {
        if let Some(dir) = std::env::var_os(env_key).map(PathBuf::from) {
            push_dir_unique(&mut out, dir);
        }
    }
    for dir in [
        PathBuf::from("/opt/homebrew/lib"),
        PathBuf::from("/opt/homebrew/opt/isl/lib"),
        PathBuf::from("/opt/homebrew/opt/gmp/lib"),
        PathBuf::from("/usr/local/lib"),
        PathBuf::from("/usr/local/opt/isl/lib"),
        PathBuf::from("/usr/local/opt/gmp/lib"),
        PathBuf::from("/usr/lib"),
        PathBuf::from("/usr/lib/x86_64-linux-gnu"),
        PathBuf::from("C:/msys64/mingw64/lib"),
        PathBuf::from("C:/msys64/ucrt64/lib"),
    ] {
        push_dir_unique(&mut out, dir);
    }
    out
}

fn find_static_lib(dirs: &[PathBuf], stem: &str) -> Option<NativeLibPaths> {
    dirs.iter()
        .map(|dir| lib_candidates(dir, stem))
        .find(static_exists)
}

fn find_dylib_lib(dirs: &[PathBuf], stem: &str) -> Option<NativeLibPaths> {
    dirs.iter()
        .map(|dir| lib_candidates(dir, stem))
        .find(dylib_exists)
}

fn emit_search_path(dir: &Path) {
    println!("cargo:rustc-link-search=native={}", dir.display());
}

fn enable_isl(link_mode: &str) {
    println!("cargo:rustc-cfg=rr_has_isl");
    println!("cargo:rustc-env=RR_HAS_ISL=1");
    println!("cargo:rustc-env=RR_ISL_LINK_MODE={link_mode}");
}

fn main() {
    println!("cargo:rustc-check-cfg=cfg(rr_has_isl)");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-env-changed=RR_ISL_LIB_DIR");
    println!("cargo:rerun-if-env-changed=RR_GMP_LIB_DIR");
    println!("cargo:rerun-if-env-changed=RR_ISL_LINK");

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

    let link_mode = parse_isl_link_mode();
    let library_dirs = collect_library_dirs();
    let isl_static = find_static_lib(&library_dirs, "isl");
    let gmp_static = find_static_lib(&library_dirs, "gmp");
    let isl_dylib = find_dylib_lib(&library_dirs, "isl");

    let can_link_fully_static = isl_static.is_some() && gmp_static.is_some();

    match link_mode {
        IslLinkMode::Static => {
            let isl = isl_static.unwrap_or_else(|| panic!("RR_ISL_LINK=static requires libisl.a"));
            let gmp = gmp_static.unwrap_or_else(|| panic!("RR_ISL_LINK=static requires libgmp.a"));
            emit_search_path(&isl.dir);
            if isl.dir != gmp.dir {
                emit_search_path(&gmp.dir);
            }
            println!("cargo:rustc-link-lib=static=isl");
            println!("cargo:rustc-link-lib=static=gmp");
            enable_isl("static");
            return;
        }
        IslLinkMode::Auto if can_link_fully_static => {
            let isl = isl_static.expect("checked above");
            let gmp = gmp_static.expect("checked above");
            emit_search_path(&isl.dir);
            if isl.dir != gmp.dir {
                emit_search_path(&gmp.dir);
            }
            println!("cargo:rustc-link-lib=static=isl");
            println!("cargo:rustc-link-lib=static=gmp");
            enable_isl("static");
            return;
        }
        IslLinkMode::Auto | IslLinkMode::Dylib => {
            if let Some(isl) = isl_dylib {
                emit_search_path(&isl.dir);
                println!("cargo:rustc-link-lib=dylib=isl");
                enable_isl("dylib");
                return;
            }
            if matches!(link_mode, IslLinkMode::Dylib) {
                panic!("RR_ISL_LINK=dylib requires libisl.dylib or libisl.so");
            }
        }
    }

    panic!(
        "RR now requires ISL support at build time. Set RR_ISL_LIB_DIR/RR_GMP_LIB_DIR to directories containing libisl/libgmp static or shared libraries, or install the platform ISL/GMP development packages."
    );
}
