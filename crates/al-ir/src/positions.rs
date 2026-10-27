//! The source positions of a program (`Program::positions`), some of them read on first use (ADR-054).
//!
//! A position id indexes one table. In a run whose dependency objects come from a compile cache, the positions of an
//! object that the cache supplies are not read at start: the object's range of ids is a hole, and the first read of an
//! id in it asks the source for the object's positions. The ids are the ones a complete compile gives, so a hole
//! changes when positions are read, never which id names which position.

use crate::SrcPos;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::Arc;

/// Gives the positions of an object whose range is a hole.
pub trait PositionSource: Send + Sync {
    /// The positions of `object`, in id order. A source that cannot read them panics (a damaged cache block whose
    /// start check passed: an internal error of the test, ADR-054 owner decision 2).
    fn positions(&self, object: usize) -> &[SrcPos];
}

/// A range of ids whose positions the source gives.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Hole {
    start: u32,
    len: u32,
    object: u32,
    /// How many ids of holes come before this one (so the dense index of an id after it is `id - before - len`).
    before: u32,
}

/// The position table. Reads like the `Vec<SrcPos>` it was.
#[derive(Clone, Default)]
pub struct Positions {
    /// The positions that are not in a hole, in id order.
    dense: Vec<SrcPos>,
    holes: Vec<Hole>,
    source: Option<Arc<dyn PositionSource>>,
}

impl Positions {
    pub fn new() -> Positions {
        Positions::default()
    }

    pub fn with_capacity(n: usize) -> Positions {
        Positions {
            dense: Vec::with_capacity(n),
            ..Default::default()
        }
    }

    /// How many ids the table has, holes included.
    pub fn len(&self) -> usize {
        self.dense.len() + self.holes.last().map_or(0, |h| (h.before + h.len) as usize)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether some ids are read on first use.
    pub fn has_holes(&self) -> bool {
        !self.holes.is_empty()
    }

    /// The position `id` names; `None` outside the table.
    pub fn get(&self, id: usize) -> Option<&SrcPos> {
        if self.holes.is_empty() {
            return self.dense.get(id);
        }
        let id32 = u32::try_from(id).ok()?;
        let at = self.holes.partition_point(|h| h.start <= id32);
        if at == 0 {
            return self.dense.get(id);
        }
        let h = &self.holes[at - 1];
        if id32 < h.start + h.len {
            // A hole without a source (a layer build: the objects of the layers below are not read) names nothing.
            return self
                .source
                .as_ref()?
                .positions(h.object as usize)
                .get((id32 - h.start) as usize);
        }
        self.dense.get(id - (h.before + h.len) as usize)
    }

    pub fn push(&mut self, p: SrcPos) {
        self.dense.push(p);
    }

    pub fn append(&mut self, other: &mut Vec<SrcPos>) {
        self.dense.append(other);
    }

    pub fn extend_from_slice(&mut self, other: &[SrcPos]) {
        self.dense.extend_from_slice(other);
    }

    pub fn reserve(&mut self, n: usize) {
        self.dense.reserve(n);
    }

    /// Adds `len` ids whose positions `source` gives as those of `object` (set the source with
    /// [`Positions::set_source`]). An empty range adds nothing.
    pub fn push_hole(&mut self, len: u32, object: usize) {
        if len == 0 {
            return;
        }
        let before = self.holes.last().map_or(0, |h| h.before + h.len);
        self.holes.push(Hole {
            start: self.len() as u32,
            len,
            object: object as u32,
            before,
        });
    }

    pub fn set_source(&mut self, source: Arc<dyn PositionSource>) {
        self.source = Some(source);
    }

    /// Every position, in id order; reads the holes (those without a source are left out).
    pub fn iter(&self) -> impl Iterator<Item = &SrcPos> + '_ {
        (0..self.len()).filter_map(|id| self.get(id))
    }

    /// The ids of every object's positions, by object index (`objects` long), without reading a hole: a hole holds the
    /// positions of the one object it names (al-link sets `object` of each position to the index of its object).
    pub fn ids_by_object(&self, objects: usize) -> Vec<Vec<u32>> {
        let mut v = vec![Vec::new(); objects];
        let mut dense = self.dense.iter();
        let mut id = 0u32;
        let mut take_dense = |v: &mut Vec<Vec<u32>>, until: u32, id: &mut u32| {
            while *id < until {
                let Some(p) = dense.next() else { break };
                if let Some(list) = v.get_mut(p.object as usize) {
                    list.push(*id);
                }
                *id += 1;
            }
        };
        for h in &self.holes {
            take_dense(&mut v, h.start, &mut id);
            if let Some(list) = v.get_mut(h.object as usize) {
                list.extend(h.start..h.start + h.len);
            }
            id = h.start + h.len;
        }
        take_dense(&mut v, u32::MAX, &mut id);
        v
    }

    /// Reads every hole into the table, so it has none.
    pub fn fill_holes(&mut self) {
        if self.holes.is_empty() {
            return;
        }
        let all: Vec<SrcPos> = self.iter().copied().collect();
        *self = Positions::from(all);
    }
}

impl From<Vec<SrcPos>> for Positions {
    fn from(dense: Vec<SrcPos>) -> Positions {
        Positions {
            dense,
            ..Default::default()
        }
    }
}

impl std::ops::Index<usize> for Positions {
    type Output = SrcPos;
    fn index(&self, id: usize) -> &SrcPos {
        match self.get(id) {
            Some(p) => p,
            None => panic!("position {id} out of range for a table of {}", self.len()),
        }
    }
}

impl PartialEq for Positions {
    fn eq(&self, other: &Positions) -> bool {
        // The same holes stand for the positions of the same objects.
        if self.holes == other.holes {
            return self.dense == other.dense;
        }
        self.len() == other.len() && self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl std::fmt::Debug for Positions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl Serialize for Positions {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(self.iter())
    }
}

impl<'de> Deserialize<'de> for Positions {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Positions, D::Error> {
        Vec::<SrcPos>::deserialize(d).map(Positions::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(line: u32, object: u32) -> SrcPos {
        SrcPos {
            line,
            object,
            ..Default::default()
        }
    }

    struct Src(Vec<Vec<SrcPos>>);
    impl PositionSource for Src {
        fn positions(&self, object: usize) -> &[SrcPos] {
            &self.0[object]
        }
    }

    #[test]
    fn holes_read_from_the_source_and_ids_stay() {
        let plain = Positions::from(vec![pos(1, 0), pos(2, 1), pos(3, 1), pos(4, 2), pos(5, 3), pos(6, 3)]);
        let mut t = Positions::new();
        t.push(pos(1, 0));
        t.push_hole(2, 1);
        t.push(pos(4, 2));
        t.push_hole(0, 9);
        t.push_hole(2, 3);
        t.set_source(Arc::new(Src(vec![
            vec![],
            vec![pos(2, 1), pos(3, 1)],
            vec![],
            vec![pos(5, 3), pos(6, 3)],
        ])));
        assert_eq!(t.len(), 6);
        assert_eq!(t.get(3).map(|p| p.line), Some(4));
        assert_eq!(t[4].line, 5);
        assert_eq!(t.get(6), None);
        assert_eq!(t, plain);
        let mut filled = t.clone();
        filled.fill_holes();
        assert!(!filled.has_holes());
        assert_eq!(filled, plain);
        assert_eq!(
            postcard::to_allocvec(&t).unwrap(),
            postcard::to_allocvec(&plain).unwrap()
        );
    }

    struct NoRead;
    impl PositionSource for NoRead {
        fn positions(&self, _: usize) -> &[SrcPos] {
            panic!("a hole was read")
        }
    }

    #[test]
    fn ids_by_object_read_no_hole() {
        let plain = Positions::from(vec![
            pos(1, 0),
            pos(2, 1),
            pos(3, 1),
            pos(4, 2),
            pos(5, 3),
            pos(6, 3),
            pos(7, 0),
        ]);
        let mut t = Positions::new();
        t.push(pos(1, 0));
        t.push_hole(2, 1);
        t.push(pos(4, 2));
        t.push_hole(2, 3);
        t.push(pos(7, 0));
        t.set_source(Arc::new(NoRead));
        let want = vec![vec![0, 6], vec![1, 2], vec![3], vec![4, 5]];
        assert_eq!(plain.ids_by_object(4), want);
        assert_eq!(t.ids_by_object(4), want);
    }
}
