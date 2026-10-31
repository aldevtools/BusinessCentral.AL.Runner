//! The position-keyed maps the checker fills: FxHash and the per-file `PosMap`.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// FxHash (as in rustc): the per-expression maps below are keyed by three
/// small integers and written for every node, where SipHash dominated their
/// cost. Not DoS-resistant, which does not matter for compiler-internal keys;
/// it is also deterministic.
#[derive(Default, Clone, Copy)]
pub struct FxHasher(u64);

impl Hasher for FxHasher {
    fn write(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(8);
        for c in &mut chunks {
            self.write_u64(u64::from_le_bytes(c.try_into().expect("8 bytes")));
        }
        let mut tail = 0u64;
        for &b in chunks.remainder() {
            tail = tail << 8 | b as u64;
        }
        self.write_u64(tail);
    }
    fn write_u64(&mut self, i: u64) {
        self.0 = (self.0.rotate_left(5) ^ i).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
    fn write_usize(&mut self, i: usize) {
        self.write_u64(i as u64);
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

pub(crate) type PosKey = (usize, usize, usize);

/// A `HashMap` hashed with [`FxHasher`]: for the small string-keyed maps of the check and the lowering.
pub(crate) type FxMap<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;

/// A `HashSet` hashed with [`FxHasher`].
pub(crate) type FxSet<K> = std::collections::HashSet<K, BuildHasherDefault<FxHasher>>;

/// One file's entries of a [`PosMap`]: (start, end) packed into one `u64`, so a probe hashes a
/// single word.
pub(crate) type FileMap<V> = HashMap<u64, V, BuildHasherDefault<FxHasher>>;

/// How many threads [`PosMap::union`] merges on (file `f` goes to thread `f % POS_SHARDS`).
pub(crate) const POS_SHARDS: usize = 16;

/// Node position (file index, start offset, end offset) to a value. One small hash map per
/// file, indexed by the file: the lowering of an object only probes the map of its own file
/// (which stays in cache), and a growing map rehashes only that file's entries. The union of the
/// parallel check's maps moves a file's map whole when one worker filled it.
pub struct PosMap<V> {
    files: Vec<FileMap<V>>,
}

impl<V> Default for PosMap<V> {
    fn default() -> Self {
        PosMap { files: Vec::new() }
    }
}

fn pack(start: usize, end: usize) -> u64 {
    debug_assert!(
        start <= u32::MAX as usize && end <= u32::MAX as usize,
        "offset over 4 GiB"
    );
    (start as u64) << 32 | end as u64
}

fn unpack(k: u64) -> (usize, usize) {
    ((k >> 32) as usize, (k & 0xffff_ffff) as usize)
}

impl<V> PosMap<V> {
    pub fn get(&self, key: &PosKey) -> Option<&V> {
        self.files.get(key.0)?.get(&pack(key.1, key.2))
    }

    pub fn insert(&mut self, key: PosKey, value: V) -> Option<V> {
        if key.0 >= self.files.len() {
            self.files.resize_with(key.0 + 1, FileMap::default);
        }
        self.files[key.0].insert(pack(key.1, key.2), value)
    }

    pub fn len(&self) -> usize {
        self.files.iter().map(FileMap::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.files.iter().all(FileMap::is_empty)
    }

    pub fn keys(&self) -> impl Iterator<Item = PosKey> + '_ {
        self.iter().map(|(k, _)| k)
    }

    /// Drops the entries of the files from `files` on (ADR-063: the apps they belong to left the workspace).
    pub fn truncate_files(&mut self, files: usize) {
        self.files.truncate(files);
    }

    /// Every entry, file by file; within a file in no particular order.
    pub fn iter(&self) -> impl Iterator<Item = (PosKey, &V)> {
        self.files.iter().enumerate().flat_map(|(file, m)| {
            m.iter().map(move |(&k, v)| {
                let (start, end) = unpack(k);
                ((file, start, end), v)
            })
        })
    }

    /// The union of `parts`, merged on [`POS_SHARDS`] threads, a file at a time. A key does not
    /// repeat across parts (a routine is checked once); if one did, one of its values would be kept.
    pub(crate) fn union(mut parts: Vec<PosMap<V>>) -> PosMap<V>
    where
        V: Send,
    {
        if parts.len() <= 1 {
            return parts.pop().unwrap_or_default();
        }
        let n_files = parts.iter().map(|p| p.files.len()).max().unwrap_or(0);
        // per_file[f]: the non-empty maps of file f, one per part that has any.
        let mut per_file: Vec<Vec<FileMap<V>>> = (0..n_files).map(|_| Vec::new()).collect();
        for part in parts {
            for (f, map) in part.files.into_iter().enumerate() {
                if !map.is_empty() {
                    per_file[f].push(map);
                }
            }
        }
        let mut per_shard: Vec<Vec<(usize, Vec<FileMap<V>>)>> = (0..POS_SHARDS).map(|_| Vec::new()).collect();
        for (f, maps) in per_file.into_iter().enumerate() {
            if !maps.is_empty() {
                per_shard[f % POS_SHARDS].push((f, maps));
            }
        }
        let merged: Vec<Vec<(usize, FileMap<V>)>> = std::thread::scope(|scope| {
            let handles: Vec<_> = per_shard
                .into_iter()
                .map(|files| {
                    scope.spawn(move || {
                        files
                            .into_iter()
                            .map(|(f, mut maps)| {
                                // Start from the biggest map so the fewest entries move.
                                let biggest = (0..maps.len()).max_by_key(|&i| maps[i].len()).unwrap_or(0);
                                let mut union = maps.swap_remove(biggest);
                                for map in maps {
                                    union.extend(map);
                                }
                                (f, union)
                            })
                            .collect()
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("merging a shard"))
                .collect()
        });
        let mut files: Vec<FileMap<V>> = (0..n_files).map(|_| FileMap::default()).collect();
        for (f, map) in merged.into_iter().flatten() {
            files[f] = map;
        }
        PosMap { files }
    }
}

#[cfg(test)]
mod tests {
    use super::PosMap;

    #[test]
    fn get_finds_what_insert_stored_per_file_and_range() {
        let mut m: PosMap<u32> = PosMap::default();
        assert_eq!(m.insert((3, 10, 20), 1), None);
        assert_eq!(m.insert((3, 10, 21), 2), None);
        assert_eq!(m.insert((0, 10, 20), 3), None);
        assert_eq!(m.insert((3, 10, 20), 4), Some(1));
        assert_eq!(m.get(&(3, 10, 20)), Some(&4));
        assert_eq!(m.get(&(3, 10, 21)), Some(&2));
        assert_eq!(m.get(&(0, 10, 20)), Some(&3));
        assert_eq!(m.get(&(1, 10, 20)), None);
        assert_eq!(m.get(&(9, 10, 20)), None);
        assert_eq!(m.len(), 3);
        let big = (5, u32::MAX as usize - 1, u32::MAX as usize);
        m.insert(big, 7);
        let mut all: Vec<_> = m.iter().map(|(k, v)| (k, *v)).collect();
        all.sort();
        assert_eq!(
            all,
            vec![((0, 10, 20), 3), ((3, 10, 20), 4), ((3, 10, 21), 2), (big, 7)]
        );
    }

    #[test]
    fn union_keeps_every_entry_of_every_part() {
        let mut parts = Vec::new();
        for p in 0..5usize {
            let mut m: PosMap<usize> = PosMap::default();
            for i in 0..200usize {
                // Parts share files but never a key.
                m.insert((i % 37, i * 10 + p, i * 10 + p + 1), p * 1000 + i);
            }
            parts.push(m);
        }
        parts.push(PosMap::default());
        let u = PosMap::union(parts);
        assert_eq!(u.len(), 1000);
        for p in 0..5usize {
            for i in 0..200usize {
                assert_eq!(u.get(&(i % 37, i * 10 + p, i * 10 + p + 1)), Some(&(p * 1000 + i)));
            }
        }
    }
}
