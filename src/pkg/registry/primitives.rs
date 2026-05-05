use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::BTreeSet;

pub(super) fn validate_unique_hex_keys(
    values: &[String],
    label: &str,
    warnings: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    let mut seen = BTreeSet::new();
    for value in values {
        match normalize_ed25519_public_key(value) {
            Ok(normalized) => {
                if !seen.insert(normalized.clone()) {
                    warnings.push(format!("duplicate {} entry: {}", label, normalized));
                }
            }
            Err(message) => {
                errors.push(format!("invalid {} entry '{}': {}", label, value, message))
            }
        }
    }
}

pub(super) fn validate_unique_strings(values: &[String], label: &str, warnings: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    for value in values {
        let normalized = value.trim();
        if !seen.insert(normalized.to_string()) {
            warnings.push(format!("duplicate {} entry: {}", label, normalized));
        }
    }
}

pub(super) fn normalize_ed25519_public_key(raw: &str) -> Result<String, String> {
    let decoded = hex_decode(raw)?;
    let arr: [u8; 32] = decoded
        .try_into()
        .map_err(|_| "ed25519 public key must be 32 bytes of hex".to_string())?;
    Ok(hex_encode(&arr))
}

pub(super) fn normalize_registry_channel(raw: &str) -> Result<String, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err("channel name must be non-empty".to_string());
    }
    if value == "latest" {
        return Err("channel name 'latest' is reserved".to_string());
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err(format!(
            "invalid channel name '{}': use only letters, digits, '.', '_' or '-'",
            value
        ));
    }
    Ok(value.to_string())
}

pub(super) fn hmac_sha256_hex(key: &str, payload: &[u8]) -> Result<String, String> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes())
        .map_err(|e| format!("failed to initialize registry signing key: {}", e))?;
    mac.update(payload);
    let bytes = mac.finalize().into_bytes();
    Ok(hex_encode(bytes.as_slice()))
}

pub(super) fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

pub(super) fn hex_decode(raw: &str) -> Result<Vec<u8>, String> {
    let raw = raw.trim();
    if !raw.len().is_multiple_of(2) {
        return Err(format!("invalid hex string length: {}", raw.len()));
    }
    let mut out = Vec::with_capacity(raw.len() / 2);
    let bytes = raw.as_bytes();
    let mut idx = 0usize;
    while idx < bytes.len() {
        let hi = decode_hex_nibble(bytes[idx])?;
        let lo = decode_hex_nibble(bytes[idx + 1])?;
        out.push((hi << 4) | lo);
        idx += 2;
    }
    Ok(out)
}

fn decode_hex_nibble(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(format!("invalid hex digit '{}'", byte as char)),
    }
}

pub(super) fn signature_scheme_name(signature: &str) -> Option<String> {
    signature.split(':').next().map(ToOwned::to_owned)
}
