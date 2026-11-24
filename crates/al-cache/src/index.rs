//! The index entry: what the scan of the dependency sources found (ADR-022, "pure tests"). A run
//! that has it does not read or scan the sources; it reads this small file instead (a few MB for
//! Base Application). It is much cheaper to build than the full entry (a scan, no compile), so a run
//! with a cold cache can make it.
//!
//! ```text
//! magic (8)  format (4)  digest (32)  payload length (8)  payload hash (32)  payload
//! ```
//! `digest` is made from the key, so an index made for other inputs is refused; the payload is
//! `zstd(postcard(Vec<Vec<FileIndex>>))`, one list of files per dependency app.
//!
//! The index entry (ADR-022, pure tests): the scan of the dependency sources, keyed, for a run that
//! reads none of them. Entry points: `build_index`, `open_index`, `index_file_name`.

use crate::{CacheError, FORMAT, Key};
use al_sema::FileIndex;
use sha2::{Digest, Sha256};

const MAGIC: &[u8; 8] = b"ALRCIDX\0";
const HEADER: usize = 8 + 4 + 32 + 8 + 32;

/// The file name of the index for `key`.
pub fn index_file_name(key: &Key) -> String {
    format!(
        "{}-index-{}.alci",
        crate::build_tag(&key.runtime_build),
        &key.hex()[..32]
    )
}

fn digest(key: &Key) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"al-runtime source index");
    h.update(key.digest());
    h.finalize().into()
}

/// The bytes of the index file for `index`, one list of files per dependency app.
pub fn build_index(key: &Key, index: &[Vec<FileIndex>]) -> Vec<u8> {
    let raw = postcard::to_stdvec(index).expect("the index serializes");
    let payload = zstd::encode_all(&raw[..], 3).expect("zstd encodes in memory");
    let mut out = Vec::with_capacity(HEADER + payload.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&FORMAT.to_le_bytes());
    out.extend_from_slice(&digest(key));
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    out.extend_from_slice(&Sha256::digest(&payload));
    out.extend_from_slice(&payload);
    out
}

/// Reads an index file made for `key`.
pub fn open_index(bytes: &[u8], key: &Key) -> Result<Vec<Vec<FileIndex>>, CacheError> {
    let short = |w: &str| CacheError::Truncated(w.to_string());
    if bytes.len() < HEADER || &bytes[..8] != MAGIC {
        return Err(short("not an index file"));
    }
    let format = u32::from_le_bytes(bytes[8..12].try_into().expect("4 bytes"));
    if format != FORMAT {
        return Err(CacheError::Format {
            found: format,
            expected: FORMAT,
        });
    }
    if bytes[12..44] != digest(key) {
        return Err(CacheError::KeyMismatch);
    }
    let len = u64::from_le_bytes(bytes[44..52].try_into().expect("8 bytes")) as usize;
    let payload = bytes.get(HEADER..HEADER + len).ok_or_else(|| short("payload"))?;
    if bytes.len() != HEADER + len {
        return Err(short("length"));
    }
    if bytes[52..84] != Sha256::digest(payload)[..] {
        return Err(CacheError::Damaged("payload hash".into()));
    }
    let raw = zstd::decode_all(payload).map_err(|e| CacheError::Damaged(format!("zstd: {e}")))?;
    postcard::from_bytes(&raw).map_err(|e| CacheError::Damaged(format!("postcard: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::AppKey;

    fn key(source: u8) -> Key {
        Key::new(
            [1; 32],
            String::new(),
            vec![AppKey {
                id: "a".into(),
                name: "A".into(),
                dependencies: vec![],
                publisher: "P".into(),
                version: "1".into(),
                symbols: vec![],
                source: [source; 32],
            }],
        )
    }

    fn sample() -> Vec<Vec<FileIndex>> {
        vec![vec![FileIndex {
            namespace: Some("N".into()),
            usings: vec!["U".into()],
            objects: vec![],
            dotnet: vec![],
        }]]
    }

    #[test]
    fn an_index_reads_back_as_it_was_written() {
        let bytes = build_index(&key(1), &sample());
        assert_eq!(open_index(&bytes, &key(1)).unwrap(), sample());
        assert_eq!(build_index(&key(1), &sample()), bytes, "the bytes are deterministic");
    }

    #[test]
    fn an_index_for_other_inputs_a_cut_or_a_damaged_file_is_refused() {
        let bytes = build_index(&key(1), &sample());
        assert_eq!(open_index(&bytes, &key(2)), Err(CacheError::KeyMismatch));
        assert!(open_index(&bytes[..bytes.len() - 1], &key(1)).is_err());
        let mut bad = bytes.clone();
        let last = bad.len() - 1;
        bad[last] ^= 1;
        assert!(matches!(open_index(&bad, &key(1)), Err(CacheError::Damaged(_))));
        assert!(open_index(b"nothing", &key(1)).is_err());
    }
}
