//! The SHA-256 helpers of the cache keys: a value is hashed with its length first, so two lists of values never hash
//! alike by moving a byte from one value to the next.

use sha2::{Digest, Sha256};

/// Adds `bytes` to `h`, its length first.
pub fn put(h: &mut Sha256, bytes: &[u8]) {
    h.update((bytes.len() as u64).to_le_bytes());
    h.update(bytes);
}

/// The SHA-256 of `bytes`.
pub fn sha(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// `bytes` as lower-case hex.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
