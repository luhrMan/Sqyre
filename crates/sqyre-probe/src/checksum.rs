//! FNV-1a checksum for capture sanity checks.

/// Hex-encoded FNV-1a 64-bit hash of `data`.
pub fn fnv1a_hex(data: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in data {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_empty() {
        assert_eq!(fnv1a_hex(b""), "cbf29ce484222325");
    }
}
