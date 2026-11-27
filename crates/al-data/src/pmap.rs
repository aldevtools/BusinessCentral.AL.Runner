//! A persistent ordered map: cloning is O(1) and shares everything; a write copies only the pieces it changes.
//!
//! Layout: a sorted spine of `Arc`-shared chunks, each a sorted vector of at most [`MAX`] entries. A clone shares the
//! spine; the first write on either side copies the spine (one pointer and one key per chunk) and the one chunk it
//! touches. Written for the shared test-data snapshot in `memory.rs` (tests fork from a baseline and change a few rows).
//!
//! The spine keeps a copy of each chunk's first key in one vector of its own (the fence keys), so the search for a
//! key's chunk compares against keys side by side and reads no chunk; only the one chunk found is opened. Every write
//! that changes a chunk's first key (a new smallest key, a split, a removed first entry, a merge, an emptied chunk)
//! updates the copy in the same step.

use std::ops::Bound;
use std::ops::RangeBounds;
use std::sync::Arc;

/// Most entries in one chunk. A chunk is copied whole on the first write after a fork.
const MAX: usize = 64;
/// A chunk smaller than this is merged into a neighbor when the two fit in one chunk.
const MIN: usize = 16;
/// Entries per chunk when a map is built from a sorted run (room left for later inserts).
const FILL: usize = 48;

/// Entries are shared too, so copying a chunk on the first write after a fork bumps counters and copies no rows.
type Chunk<K, V> = Vec<Arc<(K, V)>>;

/// The chunks in key order. `firsts[i]` is a copy of the first key of `chunks[i]`; no chunk is empty.
struct Spine<K, V> {
    firsts: Vec<K>,
    chunks: Vec<Arc<Chunk<K, V>>>,
}

impl<K: Clone, V> Clone for Spine<K, V> {
    fn clone(&self) -> Self {
        Spine {
            firsts: self.firsts.clone(),
            chunks: self.chunks.clone(),
        }
    }
}

impl<K: Clone, V> Spine<K, V> {
    /// Copies the first key of chunk `ci` again, after a write that may have changed it.
    fn refresh_first(&mut self, ci: usize) {
        self.firsts[ci] = self.chunks[ci][0].0.clone();
    }

    fn push(&mut self, chunk: Chunk<K, V>) {
        self.firsts.push(chunk[0].0.clone());
        self.chunks.push(Arc::new(chunk));
    }
}

pub struct PMap<K, V> {
    spine: Arc<Spine<K, V>>,
    len: usize,
}

impl<K, V> Clone for PMap<K, V> {
    fn clone(&self) -> Self {
        PMap {
            spine: self.spine.clone(),
            len: self.len,
        }
    }
}

impl<K, V> Default for PMap<K, V> {
    fn default() -> Self {
        PMap {
            spine: Arc::new(Spine {
                firsts: Vec::new(),
                chunks: Vec::new(),
            }),
            len: 0,
        }
    }
}

impl<K, V> PMap<K, V> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Whether the two maps share their storage (a clone nobody has written to since).
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.spine, &other.spine)
    }

    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            spine: &self.spine.chunks,
            front: (0, 0),
            back: (self.spine.chunks.len(), 0),
        }
    }

    pub fn keys(&self) -> impl DoubleEndedIterator<Item = &K> {
        self.iter().map(|(k, _)| k)
    }

    pub fn values(&self) -> impl DoubleEndedIterator<Item = &V> {
        self.iter().map(|(_, v)| v)
    }
}

impl<K: Ord, V> PMap<K, V> {
    /// The index of the chunk that holds `k` or would hold it. Searches the fence keys only.
    fn chunk_of(&self, k: &K) -> usize {
        self.spine.firsts.partition_point(|f| f <= k).saturating_sub(1)
    }

    pub fn get(&self, k: &K) -> Option<&V> {
        let c = self.spine.chunks.get(self.chunk_of(k))?;
        c.binary_search_by(|e| e.0.cmp(k)).ok().map(|i| &c[i].1)
    }

    pub fn get_key_value(&self, k: &K) -> Option<(&K, &V)> {
        let c = self.spine.chunks.get(self.chunk_of(k))?;
        c.binary_search_by(|e| e.0.cmp(k)).ok().map(|i| (&c[i].0, &c[i].1))
    }

    pub fn contains_key(&self, k: &K) -> bool {
        self.get(k).is_some()
    }

    /// The position of the first entry at or after the bound, normalized (see [`Iter`]).
    fn lower(&self, b: Bound<&K>) -> (usize, usize) {
        match b {
            Bound::Unbounded => (0, 0),
            Bound::Included(k) => self.position(k, true),
            Bound::Excluded(k) => self.position(k, false),
        }
    }

    /// The position just past the last entry the bound allows.
    fn upper(&self, b: Bound<&K>) -> (usize, usize) {
        match b {
            Bound::Unbounded => (self.spine.chunks.len(), 0),
            Bound::Included(k) => self.position(k, false),
            Bound::Excluded(k) => self.position(k, true),
        }
    }

    /// The first entry with key >= `k` (`ge`) or key > `k`.
    fn position(&self, k: &K, ge: bool) -> (usize, usize) {
        if self.spine.chunks.is_empty() {
            return (0, 0);
        }
        let ci = self.chunk_of(k);
        let c = &self.spine.chunks[ci];
        let i = if ge {
            c.partition_point(|e| e.0 < *k)
        } else {
            c.partition_point(|e| e.0 <= *k)
        };
        if i == c.len() { (ci + 1, 0) } else { (ci, i) }
    }

    pub fn range<R: RangeBounds<K>>(&self, r: R) -> Iter<'_, K, V> {
        let front = self.lower(r.start_bound());
        let back = self.upper(r.end_bound());
        // An empty or reversed range yields nothing (std's BTreeMap panics here).
        if front >= back {
            return Iter {
                spine: &self.spine.chunks,
                front: (0, 0),
                back: (0, 0),
            };
        }
        Iter {
            spine: &self.spine.chunks,
            front,
            back,
        }
    }
}

impl<K: Ord + Clone, V: Clone> PMap<K, V> {
    pub fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        let ci = self.chunk_of(k);
        let i = self.spine.chunks.get(ci)?.binary_search_by(|e| e.0.cmp(k)).ok()?;
        let spine = Arc::make_mut(&mut self.spine);
        Some(&mut Arc::make_mut(&mut Arc::make_mut(&mut spine.chunks[ci])[i]).1)
    }

    /// Sets `k` to `v`; returns the value it replaced.
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        if self.spine.chunks.is_empty() {
            Arc::make_mut(&mut self.spine).push(vec![Arc::new((k, v))]);
            self.len = 1;
            return None;
        }
        let ci = self.chunk_of(&k);
        let found = self.spine.chunks[ci].binary_search_by(|e| e.0.cmp(&k));
        let spine = Arc::make_mut(&mut self.spine);
        let chunk = Arc::make_mut(&mut spine.chunks[ci]);
        match found {
            Ok(i) => Some(std::mem::replace(&mut Arc::make_mut(&mut chunk[i]).1, v)),
            Err(i) => {
                chunk.insert(i, Arc::new((k, v)));
                self.len += 1;
                if chunk.len() > MAX {
                    let right = chunk.split_off(chunk.len() / 2);
                    spine.firsts.insert(ci + 1, right[0].0.clone());
                    spine.chunks.insert(ci + 1, Arc::new(right));
                }
                if i == 0 {
                    // A new smallest key: only the first chunk takes a key below its first.
                    spine.refresh_first(ci);
                }
                None
            }
        }
    }

    pub fn remove(&mut self, k: &K) -> Option<V> {
        let ci = self.chunk_of(k);
        let i = self.spine.chunks.get(ci)?.binary_search_by(|e| e.0.cmp(k)).ok()?;
        let spine = Arc::make_mut(&mut self.spine);
        let chunk = Arc::make_mut(&mut spine.chunks[ci]);
        let (_, v) = Arc::try_unwrap(chunk.remove(i)).unwrap_or_else(|a| (*a).clone());
        self.len -= 1;
        let n = chunk.len();
        if n == 0 {
            spine.firsts.remove(ci);
            spine.chunks.remove(ci);
            return Some(v);
        }
        if i == 0 {
            spine.refresh_first(ci);
        }
        if n < MIN {
            // Merge into the smaller neighbor when the two fit in one chunk.
            let left = (ci > 0).then(|| spine.chunks[ci - 1].len());
            let right = spine.chunks.get(ci + 1).map(|c| c.len());
            let into_left = match (left, right) {
                (Some(l), Some(r)) => (l <= r && l + n <= MAX)
                    .then_some(true)
                    .or((r + n <= MAX).then_some(false)),
                (Some(l), None) => (l + n <= MAX).then_some(true),
                (None, Some(r)) => (r + n <= MAX).then_some(false),
                (None, None) => None,
            };
            match into_left {
                Some(true) => {
                    // The left chunk keeps its first key.
                    spine.firsts.remove(ci);
                    let c = spine.chunks.remove(ci);
                    let c = Arc::try_unwrap(c).unwrap_or_else(|a| (*a).clone());
                    Arc::make_mut(&mut spine.chunks[ci - 1]).extend(c);
                }
                Some(false) => {
                    // The merged chunk starts with this chunk's first key, so the right chunk's copy goes.
                    spine.firsts.remove(ci + 1);
                    let c = spine.chunks.remove(ci);
                    let c = Arc::try_unwrap(c).unwrap_or_else(|a| (*a).clone());
                    let next = Arc::make_mut(&mut spine.chunks[ci]);
                    next.splice(0..0, c);
                }
                None => {}
            }
        }
        Some(v)
    }
}

impl<K: Ord + Clone, V: Clone> FromIterator<(K, V)> for PMap<K, V> {
    /// Later entries win over earlier ones with an equal key.
    fn from_iter<I: IntoIterator<Item = (K, V)>>(it: I) -> Self {
        let mut all: Vec<(K, V)> = it.into_iter().collect();
        all.sort_by(|a, b| a.0.cmp(&b.0));
        let mut sorted: Vec<(K, V)> = Vec::with_capacity(all.len());
        for e in all {
            match sorted.last_mut() {
                Some(last) if last.0 == e.0 => *last = e,
                _ => sorted.push(e),
            }
        }
        let len = sorted.len();
        let mut spine = Spine {
            firsts: Vec::with_capacity(len / FILL + 1),
            chunks: Vec::with_capacity(len / FILL + 1),
        };
        let mut rest = sorted.into_iter();
        loop {
            let chunk: Chunk<K, V> = rest.by_ref().take(FILL).map(Arc::new).collect();
            if chunk.is_empty() {
                break;
            }
            spine.push(chunk);
        }
        PMap {
            spine: Arc::new(spine),
            len,
        }
    }
}

/// Walks entries from both ends. A position is (chunk, index in chunk); `front` is the next entry going up and
/// `back` is just past the next entry going down. Positions are normalized: an index is always below its chunk's
/// length, and the end is (number of chunks, 0), so two positions are equal exactly when they are the same place.
pub struct Iter<'a, K, V> {
    spine: &'a [Arc<Chunk<K, V>>],
    front: (usize, usize),
    back: (usize, usize),
}

impl<K, V> Clone for Iter<'_, K, V> {
    fn clone(&self) -> Self {
        Iter {
            spine: self.spine,
            front: self.front,
            back: self.back,
        }
    }
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.front >= self.back {
            return None;
        }
        let (ci, i) = self.front;
        let chunk = &self.spine[ci];
        let e = &chunk[i];
        self.front = if i + 1 == chunk.len() { (ci + 1, 0) } else { (ci, i + 1) };
        Some((&e.0, &e.1))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.front >= self.back {
            return (0, Some(0));
        }
        (1, None)
    }
}

impl<'a, K, V> DoubleEndedIterator for Iter<'a, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.front >= self.back {
            return None;
        }
        let (mut ci, mut i) = self.back;
        if i == 0 {
            ci -= 1;
            i = self.spine[ci].len();
        }
        i -= 1;
        self.back = (ci, i);
        let e = &self.spine[ci][i];
        Some((&e.0, &e.1))
    }
}

#[cfg(test)]
#[path = "pmap_tests.rs"]
mod tests;
