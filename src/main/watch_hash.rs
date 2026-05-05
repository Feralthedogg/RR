use std::fs;
use std::path::Path;

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

pub(crate) fn watch_output_hash(content: &str) -> u64 {
    stable_hash_bytes(content.as_bytes())
}

pub(crate) fn watch_output_matches_hash(path: &Path, expected_hash: Option<u64>) -> bool {
    let Some(expected_hash) = expected_hash else {
        return false;
    };
    fs::read_to_string(path)
        .map(|content| watch_output_hash(&content) == expected_hash)
        .unwrap_or(false)
}
