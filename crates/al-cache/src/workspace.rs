//! The entry file of the workspace layer (ADR-022 amendment 2026-11-15, decisions 3, 8 and 10): an al-cache layer file
//! of the objects the compile of the workspace lowered (`build_layer`: one zstd block per object, with its checksum,
//! compressed with the dictionary of the base layer when there is one), and one more section, the caller's extra
//! (`WorkspaceExtra` of al-testrun), postcard in zstd with a SHA-256.
//!
//! ```text
//! header  (128 bytes)  magic, format, key digest, layer length, extra length, extra raw length, extra hash,
//!                      header hash
//! layer   an al-cache layer file for the carrier key of the digest (`carrier_key`)
//! extra   zstd(postcard(extra))
//! ```
//!
//! Open checks the header, the length, the extra's hash and decodes it, and opens the layer (which checks its own header,
//! index and every byte of its blocks, `Entry::open`). Any failure is an error, never a panic: the caller treats it as a
//! miss.

use crate::{CacheError, Entry, Key};
use al_sema::provided::DepLayer;
use sha2::{Digest, Sha256};

const MAGIC: &[u8; 8] = b"ALRCWSP\0";
const HEADER: usize = 8 + 4 + 32 + 8 + 8 + 8 + 32 + 32;
const LEVEL: i32 = 3;

/// The format of the workspace layer file. A change of the layout or of what the extra holds raises it.
pub const WORKSPACE_FORMAT: u32 = 1;

/// The al-cache key a workspace layer's blocks are filed under: the layer format checks a key's digest, and the
/// workspace key is the caller's (`WorkspaceKey` of al-testrun), so its digest is carried in `below`.
pub fn carrier_key(digest: [u8; 32]) -> Key {
    Key::layered([0; 32], "workspace layer".into(), Vec::new(), digest)
}

fn sha(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().expect("8 bytes"))
}

/// The bytes of a workspace layer file for `layer` (the objects lowered by the compile of the workspace), filed under
/// `digest`, with `extra`. `dictionary`: the base layer's, when the run has dependency layers. The same inputs give the
/// same bytes.
pub fn build_workspace<T: serde::Serialize>(
    layer: &DepLayer,
    digest: [u8; 32],
    dictionary: Option<&[u8]>,
    extra: &T,
) -> Vec<u8> {
    let key = carrier_key(digest);
    let layer_bytes = match dictionary {
        Some(d) => crate::build_layer_with_dictionary(layer, &key, d),
        None => crate::build_layer(layer, &key),
    };
    let raw = postcard::to_stdvec(extra).expect("the extra serializes");
    let mut c = zstd::bulk::Compressor::new(LEVEL).expect("a zstd compressor");
    c.include_checksum(true).expect("checksum flag");
    let packed = c.compress(&raw).expect("the extra compresses");
    let mut out = Vec::with_capacity(HEADER + layer_bytes.len() + packed.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&WORKSPACE_FORMAT.to_le_bytes());
    out.extend_from_slice(&digest);
    out.extend_from_slice(&(layer_bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(&(packed.len() as u64).to_le_bytes());
    out.extend_from_slice(&(raw.len() as u64).to_le_bytes());
    out.extend_from_slice(&sha(&packed));
    let header_hash = sha(&out);
    out.extend_from_slice(&header_hash);
    debug_assert_eq!(out.len(), HEADER);
    out.extend_from_slice(&layer_bytes);
    out.extend_from_slice(&packed);
    out
}

/// Opens `bytes` as the workspace layer file for `digest`: the layer and the extra. Every check of the module.
pub fn open_workspace<T: serde::de::DeserializeOwned>(
    mut bytes: Vec<u8>,
    digest: [u8; 32],
) -> Result<(Entry, T), CacheError> {
    if bytes.len() < HEADER {
        return Err(CacheError::Truncated("shorter than the header".into()));
    }
    if &bytes[..8] != MAGIC {
        return Err(CacheError::Truncated("no magic".into()));
    }
    if sha(&bytes[..HEADER - 32]) != bytes[HEADER - 32..HEADER] {
        return Err(CacheError::Damaged("header".into()));
    }
    let format = u32::from_le_bytes(bytes[8..12].try_into().expect("4 bytes"));
    if format != WORKSPACE_FORMAT {
        return Err(CacheError::Format {
            found: format,
            expected: WORKSPACE_FORMAT,
        });
    }
    if bytes[12..44] != digest {
        return Err(CacheError::KeyMismatch);
    }
    let layer_len = u64_at(&bytes, 44);
    let extra_len = u64_at(&bytes, 52);
    let extra_raw = u64_at(&bytes, 60);
    let want = (HEADER as u64)
        .checked_add(layer_len)
        .and_then(|l| l.checked_add(extra_len));
    if want != Some(bytes.len() as u64) {
        return Err(CacheError::Truncated(format!(
            "{} bytes, the header says {want:?}",
            bytes.len()
        )));
    }
    let extra_at = HEADER + layer_len as usize;
    let packed = &bytes[extra_at..];
    if sha(packed) != bytes[68..100] {
        return Err(CacheError::Damaged("the extra section".into()));
    }
    // The raw length bounds the allocation; a damaged one was caught by the hash.
    let raw =
        zstd::bulk::decompress(packed, extra_raw as usize).map_err(|e| CacheError::Damaged(format!("zstd: {e}")))?;
    let extra: T = postcard::from_bytes(&raw).map_err(|e| CacheError::Damaged(format!("extra decode: {e}")))?;
    bytes.truncate(extra_at);
    let layer = bytes.split_off(HEADER);
    let entry = Entry::open(layer, &carrier_key(digest))?;
    Ok((entry, extra))
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_sema::provided::{DepBlock, DepFacts, DepSet};

    fn layer() -> DepLayer {
        let objects: Vec<al_ir::Object> = (0..3)
            .map(|i| al_ir::Object {
                name: format!("Object {i}"),
                ..Default::default()
            })
            .collect();
        DepLayer {
            base_len: 2,
            objects: vec![1, 2, 3, 4],
            set: DepSet {
                facts: vec![DepFacts::default(); 4],
                blocks: (0..4)
                    .map(|i| DepBlock {
                        object: Some(std::sync::Arc::new(objects[i % 3].clone())),
                        pos_start: i as u32,
                        positions: Vec::new(),
                    })
                    .collect(),
                heads: (0..4).map(|i| al_ir::ObjectHead::of(&objects[i % 3])).collect(),
                parse_problems: Vec::new(),
            },
        }
    }

    #[test]
    fn a_built_file_opens_and_the_same_inputs_give_the_same_bytes() {
        let extra = (vec![1u32, 2, 3], "extra".to_string());
        let a = build_workspace(&layer(), [9; 32], None, &extra);
        assert_eq!(a, build_workspace(&layer(), [9; 32], None, &extra));
        let (entry, back): (Entry, (Vec<u32>, String)) = open_workspace(a, [9; 32]).unwrap();
        assert_eq!(back, extra);
        entry.verify().unwrap();
    }

    #[test]
    fn a_damaged_truncated_or_foreign_file_is_an_error_never_a_panic() {
        let extra = (vec![1u32, 2, 3], "extra".to_string());
        let good = build_workspace(&layer(), [9; 32], None, &extra);
        let open = |b: Vec<u8>, d: [u8; 32]| open_workspace::<(Vec<u32>, String)>(b, d).map(|_| ());
        assert!(matches!(open(good.clone(), [8; 32]), Err(CacheError::KeyMismatch)));
        for cut in [0, 7, HEADER - 1, HEADER + 5, good.len() / 2, good.len() - 1] {
            assert!(open(good[..cut].to_vec(), [9; 32]).is_err(), "cut at {cut}");
        }
        // Every byte flipped, one at a time: an error, never a panic.
        for at in 0..good.len() {
            let mut b = good.clone();
            b[at] ^= 0x41;
            assert!(open(b, [9; 32]).is_err(), "byte {at} flipped");
        }
        let mut longer = good.clone();
        longer.push(0);
        assert!(open(longer, [9; 32]).is_err());
    }
}
