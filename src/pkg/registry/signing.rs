use super::env::{
    registry_signing_ed25519_secret, registry_signing_identity, registry_signing_key,
    registry_trust_ed25519_keys,
};
use super::primitives::{hex_decode, hex_encode, hmac_sha256_hex};
use super::*;
use ed25519_dalek::{Signer, SigningKey};

pub(super) fn sign_registry_release(
    module_path: &str,
    version: &str,
    archive_sum: &str,
) -> Result<(Option<String>, Option<String>), String> {
    let payload = registry_signature_payload(module_path, version, archive_sum);
    if let Some(secret) = registry_signing_ed25519_secret() {
        let signing_key = load_ed25519_signing_key(&secret)?;
        let public_key = hex_encode(signing_key.verifying_key().as_bytes());
        let signature = signing_key.sign(payload.as_bytes());
        let sig = format!("ed25519:{public_key}:{}", hex_encode(&signature.to_bytes()));
        return Ok((Some(sig), registry_signing_identity()));
    }
    if let Some(key) = registry_signing_key() {
        let digest = hmac_sha256_hex(&key, payload.as_bytes())?;
        return Ok((
            Some(format!("hmac-sha256:{digest}")),
            registry_signing_identity(),
        ));
    }
    Ok((None, None))
}

pub(super) fn verify_registry_signer_policy(
    module_path: &str,
    version: &str,
    signer: Option<&str>,
    policy: &RegistryTrustPolicy,
) -> Result<(), String> {
    if policy.allowed_signers.is_empty() {
        return Ok(());
    }
    let Some(signer) = signer.filter(|value| !value.trim().is_empty()) else {
        return Err(format!(
            "registry policy requires an allowed signer for '{} {}'",
            module_path, version
        ));
    };
    if policy
        .allowed_signers
        .iter()
        .any(|allowed| allowed == signer)
    {
        Ok(())
    } else {
        Err(format!(
            "registry signer '{}' is not allowed for '{} {}'",
            signer, module_path, version
        ))
    }
}

pub(super) fn resolved_registry_trust_ed25519_keys(
    policy: &RegistryTrustPolicy,
) -> Result<Vec<String>, String> {
    let mut keys = registry_trust_ed25519_keys();
    for key in &policy.trusted_ed25519_keys {
        if !keys.iter().any(|entry| entry == key) {
            keys.push(key.clone());
        }
    }
    if let Some(secret) = registry_signing_ed25519_secret() {
        let signing_key = load_ed25519_signing_key(&secret)?;
        let public = hex_encode(signing_key.verifying_key().as_bytes());
        if !keys.iter().any(|entry| entry == &public) {
            keys.push(public);
        }
    }
    Ok(keys)
}

fn load_ed25519_signing_key(secret_hex: &str) -> Result<SigningKey, String> {
    let secret_bytes = hex_decode(secret_hex)?;
    let secret_arr: [u8; 32] = secret_bytes
        .try_into()
        .map_err(|_| "RR_REGISTRY_SIGNING_ED25519_SECRET must be 32 bytes of hex".to_string())?;
    Ok(SigningKey::from_bytes(&secret_arr))
}

pub(super) fn registry_signature_payload(
    module_path: &str,
    version: &str,
    archive_sum: &str,
) -> String {
    format!("module={module_path}\nversion={version}\nsum={archive_sum}\n")
}
