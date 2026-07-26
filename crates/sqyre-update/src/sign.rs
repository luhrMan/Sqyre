//! Ed25519 verification for release `SHA256SUMS` manifests.

use ed25519_dalek::{Signature, VerifyingKey, SIGNATURE_LENGTH};
use thiserror::Error;

/// Embedded verifying key hex (64 chars), or `UNCONFIGURED` until the maintainer
/// generates a keypair and commits the public key (see docs/DEVELOPING.md).
const PUBKEY_HEX: &str = include_str!("../update_pubkey.hex");

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SignError {
    #[error(
        "update signing key is not configured (set crates/sqyre-update/update_pubkey.hex; see docs/DEVELOPING.md)"
    )]
    NotConfigured,
    #[error("invalid update public key hex")]
    BadPublicKey,
    #[error("invalid update signature (expected {SIGNATURE_LENGTH} raw bytes or 128 hex chars)")]
    BadSignatureFormat,
    #[error("SHA256SUMS signature verification failed")]
    VerifyFailed,
}

/// Parse a verifying key from 64 hex characters.
pub fn verifying_key_from_hex(hex: &str) -> Result<VerifyingKey, SignError> {
    let hex = hex.trim();
    if hex.is_empty() || hex.eq_ignore_ascii_case("UNCONFIGURED") {
        return Err(SignError::NotConfigured);
    }
    let bytes = parse_hex32(hex).ok_or(SignError::BadPublicKey)?;
    VerifyingKey::from_bytes(&bytes).map_err(|_| SignError::BadPublicKey)
}

/// Verifying key baked into this build.
pub fn embedded_verifying_key() -> Result<VerifyingKey, SignError> {
    verifying_key_from_hex(PUBKEY_HEX)
}

/// Verify that `signature` is a valid Ed25519 signature over `message`.
pub fn verify(message: &[u8], signature: &[u8], key: &VerifyingKey) -> Result<(), SignError> {
    let sig = parse_signature(signature)?;
    key.verify_strict(message, &sig)
        .map_err(|_| SignError::VerifyFailed)
}

fn parse_signature(sig: &[u8]) -> Result<Signature, SignError> {
    if sig.len() == SIGNATURE_LENGTH {
        return Signature::from_slice(sig).map_err(|_| SignError::BadSignatureFormat);
    }
    let text = std::str::from_utf8(sig).map_err(|_| SignError::BadSignatureFormat)?;
    let hex: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    if hex.len() != SIGNATURE_LENGTH * 2 {
        return Err(SignError::BadSignatureFormat);
    }
    let mut bytes = [0u8; SIGNATURE_LENGTH];
    for i in 0..SIGNATURE_LENGTH {
        bytes[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|_| SignError::BadSignatureFormat)?;
    }
    Ok(Signature::from_bytes(&bytes))
}

fn parse_hex32(hex: &str) -> Option<[u8; 32]> {
    if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// Parse a 32-byte signing seed from hex (CI / maintainer tooling).
pub fn signing_key_bytes_from_hex(hex: &str) -> Result<[u8; 32], SignError> {
    parse_hex32(hex.trim()).ok_or(SignError::BadPublicKey)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn ephemeral() -> (SigningKey, VerifyingKey) {
        let mut seed = [0u8; 32];
        for (i, b) in seed.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(17).wrapping_add(3);
        }
        let sk = SigningKey::from_bytes(&seed);
        let pk = sk.verifying_key();
        (sk, pk)
    }

    #[test]
    fn roundtrip_raw_and_hex_signatures() {
        let (sk, pk) = ephemeral();
        let msg = b"abc  SHA256SUMS\n";
        let sig = sk.sign(msg);
        verify(msg, sig.to_bytes().as_ref(), &pk).unwrap();
        let hex = sig
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        verify(msg, hex.as_bytes(), &pk).unwrap();
        assert!(verify(b"tampered", sig.to_bytes().as_ref(), &pk).is_err());
    }

    #[test]
    fn unconfigured_pubkey_errors() {
        assert_eq!(
            verifying_key_from_hex("UNCONFIGURED").unwrap_err(),
            SignError::NotConfigured
        );
    }
}
