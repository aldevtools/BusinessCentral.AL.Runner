//! The entry file: header, index, dictionary, blocks.
//!
//! ```text
//! header  (140 bytes)  magic, format, key digest, where the index is, its length, raw length and
//!                      hash, file length, hash of the header so far
//! dict    the zstd dictionary the blocks share
//! blocks  zstd(postcard(DepBlock)) per object, with the dictionary and a content checksum
//! heads   zstd(postcard(Vec<ObjectHead>)) per 256 objects, with a content checksum, read on the first ask
//! index   zstd(postcard(Index)): the facts of every object, where each block and head chunk sits
//! ```
//!
//! Open reads the header and the index. A damaged header, index or dictionary is refused at open
//! (a hash each). A damaged block is refused when it is read (zstd's content checksum, then the
//! decoder). A file of the wrong length is refused at open. Nothing in a refused file is used.

use crate::Key;
use al_ir::ObjectHead;
use al_sema::provided::{DepBlock, DepFacts, DepLayer, DepSet, DepSource, LayerSource};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAGIC: &[u8; 8] = b"ALRCDEP\0";
const HEADER: usize = 8 + 4 + 32 + 8 + 8 + 8 + 32 + 8 + 32;
const LEVEL: i32 = 3;
const DICT_SIZE: usize = 512 * 1024;
/// How many heads one chunk of the head region holds (the chunks decode on several threads).
const HEAD_CHUNK: usize = 256;
/// The training samples: about this many bytes of blocks, taken evenly.
const SAMPLE_BYTES: usize = 24 << 20;

/// Why a cache file cannot be used. The caller ignores the file and builds a new one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheError {
    /// Not a cache file, or cut short.
    Truncated(String),
    /// Another layout of the file.
    Format { found: u32, expected: u32 },
    /// Made for other inputs (apps, versions, sources, target or compiler).
    KeyMismatch,
    /// A hash or checksum does not match, or a decoder gave up.
    Damaged(String),
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheError::Truncated(w) => write!(f, "truncated or not a cache file ({w})"),
            CacheError::Format { found, expected } => write!(f, "format {found}, this build reads {expected}"),
            CacheError::KeyMismatch => write!(f, "made for other inputs"),
            CacheError::Damaged(w) => write!(f, "damaged ({w})"),
        }
    }
}

impl std::error::Error for CacheError {}

#[derive(Serialize, Deserialize)]
struct BlockRef {
    offset: u64,
    len: u32,
    raw_len: u32,
}

#[derive(Serialize, Deserialize)]
struct Index {
    /// How many objects the layers below hold (0 for the first layer).
    base_len: u64,
    /// The program index of each slot (`DepLayer::objects`).
    objects: Vec<u32>,
    facts: Vec<DepFacts>,
    /// The heads of the objects (ADR-054 decision 2), `HEAD_CHUNK` slots per chunk, each chunk
    /// `zstd(postcard(Vec<ObjectHead>))` with a content checksum. Read on the first ask, not at open.
    head_chunks: Vec<BlockRef>,
    parse_problems: Vec<(String, String)>,
    blocks: Vec<BlockRef>,
    dict_offset: u64,
    dict_len: u64,
    dict_sha: [u8; 32],
}

fn sha(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn zstd_err(e: std::io::Error) -> CacheError {
    CacheError::Damaged(format!("zstd: {e}"))
}

/// The bytes of the entry file for `set`, the first layer. The same set and key give the same bytes.
pub fn build(set: &DepSet, key: &Key) -> Vec<u8> {
    let objects: Vec<u32> = (0..set.facts.len() as u32).collect();
    build_parts(0, &objects, set, key, None)
}

/// The bytes of the entry file for a layer (`build` is the first layer).
pub fn build_layer(layer: &DepLayer, key: &Key) -> Vec<u8> {
    build_parts(layer.base_len, &layer.objects, &layer.set, key, None)
}

/// [`build_layer`] with the dictionary of another entry (the first layer's) instead of one trained
/// on this layer: training takes most of the time of compressing a small layer. The result is a
/// function of the layer, the key and that dictionary.
pub fn build_layer_with_dictionary(layer: &DepLayer, key: &Key, dictionary: &[u8]) -> Vec<u8> {
    build_parts(layer.base_len, &layer.objects, &layer.set, key, Some(dictionary))
}

fn build_parts(base_len: usize, objects: &[u32], set: &DepSet, key: &Key, shared_dict: Option<&[u8]>) -> Vec<u8> {
    assert_eq!(set.facts.len(), set.blocks.len(), "one block per object");
    assert_eq!(set.facts.len(), set.heads.len(), "one head per object");
    assert_eq!(set.facts.len(), objects.len(), "one object index per block");
    let raw: Vec<Vec<u8>> = set
        .blocks
        .par_iter()
        .map(|b| postcard::to_stdvec(b).expect("a block serializes"))
        .collect();
    // Train the dictionary on blocks taken evenly across the set.
    let total: usize = raw.iter().map(Vec::len).sum();
    let stride = (total / SAMPLE_BYTES).max(1);
    let samples: Vec<&[u8]> = raw.iter().step_by(stride).map(Vec::as_slice).collect();
    let dict = if let Some(d) = shared_dict {
        d.to_vec()
    } else if samples.len() >= 8 {
        zstd::dict::from_samples(&samples, DICT_SIZE).unwrap_or_default()
    } else {
        Vec::new()
    };
    let compressed: Vec<Vec<u8>> = raw
        .par_iter()
        .map_init(
            || {
                let mut c = zstd::bulk::Compressor::with_dictionary(LEVEL, &dict).expect("a zstd compressor");
                c.include_checksum(true).expect("checksum flag");
                c
            },
            |c, bytes| c.compress(bytes).expect("a block compresses"),
        )
        .collect();
    // The layout: header, dictionary, blocks, index. The index holds the offsets and comes last,
    // so nothing before it depends on its size.
    let mut at = HEADER as u64;
    let dict_offset = at;
    at += dict.len() as u64;
    let mut block_refs = Vec::with_capacity(compressed.len());
    for (r, c) in raw.iter().zip(&compressed) {
        block_refs.push(BlockRef {
            offset: at,
            len: c.len() as u32,
            raw_len: r.len() as u32,
        });
        at += c.len() as u64;
    }
    // The heads, after the blocks.
    let heads: Vec<(usize, Vec<u8>)> = set
        .heads
        .par_chunks(HEAD_CHUNK)
        .map(|chunk| {
            let raw = postcard::to_stdvec(chunk).expect("heads serialize");
            let mut c = zstd::bulk::Compressor::new(LEVEL).expect("a zstd compressor");
            c.include_checksum(true).expect("checksum flag");
            (raw.len(), c.compress(&raw).expect("heads compress"))
        })
        .collect();
    let mut head_chunks = Vec::with_capacity(heads.len());
    for (raw_len, c) in &heads {
        head_chunks.push(BlockRef {
            offset: at,
            len: c.len() as u32,
            raw_len: *raw_len as u32,
        });
        at += c.len() as u64;
    }
    let index = Index {
        base_len: base_len as u64,
        objects: objects.to_vec(),
        facts: set.facts.clone(),
        head_chunks,
        parse_problems: set.parse_problems.clone(),
        blocks: block_refs,
        dict_offset,
        dict_len: dict.len() as u64,
        dict_sha: sha(&dict),
    };
    let index_raw = postcard::to_stdvec(&index).expect("the index serializes");
    let index_bytes = zstd::bulk::compress(&index_raw, LEVEL).expect("the index compresses");
    let index_offset = at;
    let total_len = index_offset + index_bytes.len() as u64;
    let mut out = Vec::with_capacity(total_len as usize);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&key.format.to_le_bytes());
    out.extend_from_slice(&key.digest());
    out.extend_from_slice(&index_offset.to_le_bytes());
    out.extend_from_slice(&(index_bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(&(index_raw.len() as u64).to_le_bytes());
    out.extend_from_slice(&sha(&index_bytes));
    out.extend_from_slice(&total_len.to_le_bytes());
    let header_hash = sha(&out);
    out.extend_from_slice(&header_hash);
    debug_assert_eq!(out.len(), HEADER);
    out.extend_from_slice(&dict);
    for c in &compressed {
        out.extend_from_slice(c);
    }
    for (_, c) in &heads {
        out.extend_from_slice(c);
    }
    out.extend_from_slice(&index_bytes);
    assert_eq!(out.len() as u64, total_len, "the layout is as planned");
    out
}

/// An opened entry file. The blocks are read and checked when asked for.
pub struct Entry {
    bytes: Vec<u8>,
    index: Index,
    dict: zstd::dict::DecoderDictionary<'static>,
    /// The heads, decoded on the first ask.
    heads: std::sync::OnceLock<Vec<ObjectHead>>,
}

fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().expect("8 bytes"))
}

impl Entry {
    /// Opens `bytes` as the entry for `key`. Reads the header, the index and the dictionary;
    /// nothing else.
    pub fn open(bytes: Vec<u8>, key: &Key) -> Result<Entry, CacheError> {
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
        if format != key.format {
            return Err(CacheError::Format {
                found: format,
                expected: key.format,
            });
        }
        if bytes[12..44] != key.digest() {
            return Err(CacheError::KeyMismatch);
        }
        let index_offset = u64_at(&bytes, 44) as usize;
        let index_len = u64_at(&bytes, 52) as usize;
        let index_raw_len = u64_at(&bytes, 60) as usize;
        let total = u64_at(&bytes, 100);
        if total != bytes.len() as u64 {
            return Err(CacheError::Truncated(format!(
                "{} bytes, the header says {total}",
                bytes.len()
            )));
        }
        let index_end = index_offset
            .checked_add(index_len)
            .filter(|&e| e == bytes.len() && index_offset >= HEADER)
            .ok_or_else(|| CacheError::Damaged("index span".into()))?;
        let index_bytes = &bytes[index_offset..index_end];
        if sha(index_bytes) != bytes[68..100] {
            return Err(CacheError::Damaged("index".into()));
        }
        let raw = zstd::bulk::decompress(index_bytes, index_raw_len).map_err(zstd_err)?;
        let index: Index = postcard::from_bytes(&raw).map_err(|e| CacheError::Damaged(format!("index decode: {e}")))?;
        let dict_end = (index.dict_offset as usize)
            .checked_add(index.dict_len as usize)
            .filter(|&e| e <= index_offset && index.dict_offset as usize >= HEADER)
            .ok_or_else(|| CacheError::Damaged("dictionary span".into()))?;
        let dict_bytes = &bytes[index.dict_offset as usize..dict_end];
        if sha(dict_bytes) != index.dict_sha {
            return Err(CacheError::Damaged("dictionary".into()));
        }
        if index.facts.len() != index.blocks.len()
            || index.head_chunks.len() != index.blocks.len().div_ceil(HEAD_CHUNK)
            || index.objects.len() != index.blocks.len()
        {
            return Err(CacheError::Damaged("object count".into()));
        }
        // The objects ascend; those from `base_len` on are the layer's own, with no gap.
        let base = index.base_len;
        let mut own = 0u64;
        let mut last: Option<u32> = None;
        for &o in &index.objects {
            let ordered = last.is_none_or(|p| p < o);
            last = Some(o);
            let o = u64::from(o);
            let fits = if o < base {
                true
            } else {
                let ok = o == base + own;
                own += 1;
                ok
            };
            if !ordered || !fits {
                return Err(CacheError::Damaged("object indexes".into()));
            }
        }
        for b in index.blocks.iter().chain(&index.head_chunks) {
            let end = (b.offset as usize).checked_add(b.len as usize);
            if end.is_none_or(|e| e > index_offset) || (b.offset as usize) < dict_end {
                return Err(CacheError::Damaged("block span".into()));
            }
        }
        let dict = zstd::dict::DecoderDictionary::copy(dict_bytes);
        Ok(Entry {
            bytes,
            index,
            dict,
            heads: std::sync::OnceLock::new(),
        })
    }

    /// Reads every block. For CI and `cache verify`: a damaged block anywhere is found.
    pub fn verify(&self) -> Result<(), CacheError> {
        (0..self.index.blocks.len())
            .into_par_iter()
            .try_for_each(|i| self.read_block(i).map(|_| ()))?;
        (0..self.index.head_chunks.len())
            .into_par_iter()
            .try_for_each(|i| self.read_head_chunk(i).map(|_| ()))
    }

    fn read_head_chunk(&self, i: usize) -> Result<Vec<ObjectHead>, CacheError> {
        let r = &self.index.head_chunks[i];
        let packed = &self.bytes[r.offset as usize..r.offset as usize + r.len as usize];
        let raw = zstd::bulk::decompress(packed, r.raw_len as usize).map_err(zstd_err)?;
        let heads: Vec<ObjectHead> =
            postcard::from_bytes(&raw).map_err(|e| CacheError::Damaged(format!("head chunk {i} decode: {e}")))?;
        let want = HEAD_CHUNK.min(self.index.blocks.len() - i * HEAD_CHUNK);
        if heads.len() != want {
            return Err(CacheError::Damaged(format!(
                "head chunk {i} holds {} heads",
                heads.len()
            )));
        }
        Ok(heads)
    }

    fn read_block(&self, i: usize) -> Result<DepBlock, CacheError> {
        let r = &self.index.blocks[i];
        let packed = &self.bytes[r.offset as usize..r.offset as usize + r.len as usize];
        let mut d = zstd::bulk::Decompressor::with_prepared_dictionary(&self.dict).map_err(zstd_err)?;
        let raw = d.decompress(packed, r.raw_len as usize).map_err(zstd_err)?;
        postcard::from_bytes(&raw).map_err(|e| CacheError::Damaged(format!("block {i} decode: {e}")))
    }

    /// The heads of all slots. A chunk that fails its check after the open passed is an internal error (ADR-054 owner
    /// decision 2), as a damaged block is for the object that reads it.
    fn heads(&self) -> &[ObjectHead] {
        self.heads.get_or_init(|| {
            let chunks: Vec<Vec<ObjectHead>> = (0..self.index.head_chunks.len())
                .into_par_iter()
                .map(|i| self.read_head_chunk(i).unwrap_or_else(|e| panic!("compile cache: {e}")))
                .collect();
            chunks.into_iter().flatten().collect()
        })
    }

    /// The size of the file.
    pub fn size(&self) -> usize {
        self.bytes.len()
    }
}

impl Entry {
    /// How many objects the layers below this one hold (0 for the first layer).
    pub fn base_len(&self) -> usize {
        self.index.base_len as usize
    }

    /// The zstd dictionary the blocks of this entry share.
    pub fn dictionary(&self) -> &[u8] {
        let from = self.index.dict_offset as usize;
        &self.bytes[from..from + self.index.dict_len as usize]
    }

    /// The dependency files that did not parse when the layer was built.
    pub fn parse_problems(&self) -> &[(String, String)] {
        &self.index.parse_problems
    }

    fn slot_of(&self, object: usize) -> usize {
        self.index
            .objects
            .binary_search(&(object as u32))
            .unwrap_or_else(|_| panic!("object {object} is not in this layer"))
    }
}

/// An entry that stands alone (the first layer) or is read through a [`Stack`](al_sema::provided::Stack).
impl DepSource for Entry {
    fn len(&self) -> usize {
        self.base_len()
            + self
                .index
                .objects
                .iter()
                .filter(|&&o| o as u64 >= self.index.base_len)
                .count()
    }
    fn facts(&self, object: usize) -> &DepFacts {
        &self.index.facts[self.slot_of(object)]
    }
    fn head(&self, object: usize) -> &ObjectHead {
        &self.heads()[self.slot_of(object)]
    }
    fn block(&self, object: usize) -> Result<DepBlock, String> {
        self.read_block(self.slot_of(object)).map_err(|e| e.to_string())
    }
    fn parse_problems(&self) -> &[(String, String)] {
        &self.index.parse_problems
    }
}

impl LayerSource for Entry {
    fn base_len(&self) -> usize {
        self.base_len()
    }
    fn objects(&self) -> &[u32] {
        &self.index.objects
    }
    fn facts_at(&self, slot: usize) -> &DepFacts {
        &self.index.facts[slot]
    }
    fn head_at(&self, slot: usize) -> &ObjectHead {
        &self.heads()[slot]
    }
    fn block_at(&self, slot: usize) -> Result<DepBlock, String> {
        self.read_block(slot).map_err(|e| e.to_string())
    }
    fn parse_problems(&self) -> &[(String, String)] {
        &self.index.parse_problems
    }
}
