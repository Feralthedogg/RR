pub(crate) fn stable_hash_bytes(bytes: &[u8]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET_BASIS;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

pub(crate) fn fn_emit_cache_salt() -> u64 {
    let build_hash = option_env!("RR_COMPILER_BUILD_HASH").unwrap_or("no-build-script");
    stable_hash_bytes(format!("rr-fn-emit-cache-salt-v3|{build_hash}").as_bytes())
}
