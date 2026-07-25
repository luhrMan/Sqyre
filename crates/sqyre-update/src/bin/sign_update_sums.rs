//! Sign `SHA256SUMS` for a GitHub release (`SHA256SUMS.sig`, raw 64-byte Ed25519).
//!
//! Reads private key hex from `SQYRE_UPDATE_SIGNING_KEY` (never from a repo file).

use ed25519_dalek::{Signer, SigningKey};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(sums_path) = args.next().map(PathBuf::from) else {
        eprintln!("usage: sign-update-sums <SHA256SUMS>");
        return ExitCode::from(2);
    };
    let Ok(key_hex) = env::var("SQYRE_UPDATE_SIGNING_KEY") else {
        eprintln!("SQYRE_UPDATE_SIGNING_KEY is not set");
        return ExitCode::from(1);
    };
    let seed = match sqyre_update::sign::signing_key_bytes_from_hex(&key_hex) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("invalid signing key: {e}");
            return ExitCode::from(1);
        }
    };
    let sk = SigningKey::from_bytes(&seed);
    let msg = match fs::read(&sums_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("read {}: {e}", sums_path.display());
            return ExitCode::from(1);
        }
    };
    let sig = sk.sign(&msg);
    let mut out_path = sums_path.clone();
    if let Some(name) = sums_path.file_name().and_then(|s| s.to_str()) {
        out_path.set_file_name(format!("{name}.sig"));
    } else {
        out_path = PathBuf::from("SHA256SUMS.sig");
    }
    if let Err(e) = fs::write(&out_path, sig.to_bytes()) {
        eprintln!("write {}: {e}", out_path.display());
        return ExitCode::from(1);
    }
    eprintln!("wrote {}", out_path.display());
    ExitCode::SUCCESS
}
