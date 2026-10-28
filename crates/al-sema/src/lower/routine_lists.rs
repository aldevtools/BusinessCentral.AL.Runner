//! What `Lowered` holds per object and routine: why a routine is refused and which routines it calls. For an object a
//! compile cache supplies, both are read from the facts the cache stored (ADR-054), not copied into the compile: a run
//! on Base Application takes about ten thousand objects from the cache and reads the lists of a few.

use crate::provided::DepSource;
use std::sync::Arc;

/// The facts of the objects a compile cache supplies.
#[derive(Clone)]
pub(crate) struct CachedFacts {
    pub(crate) src: Arc<dyn DepSource + Send>,
    /// Per object: the cache supplies it.
    pub(crate) provided: Arc<[bool]>,
}

impl CachedFacts {
    fn has(&self, o: usize) -> bool {
        self.provided.get(o).copied().unwrap_or(false)
    }
}

/// Per object, per routine: why it cannot run (empty = it can). Indexes and iterates as a `Vec<Vec<Vec<String>>>`.
#[derive(Clone, Default)]
pub struct Refusals {
    /// The lists of the objects lowered here; empty for an object read from `cached`.
    own: Vec<Vec<Vec<String>>>,
    cached: Option<CachedFacts>,
}

impl Refusals {
    pub(crate) fn new(own: Vec<Vec<Vec<String>>>, cached: Option<CachedFacts>) -> Refusals {
        Refusals { own, cached }
    }

    pub fn len(&self) -> usize {
        self.own.len()
    }

    pub fn is_empty(&self) -> bool {
        self.own.is_empty()
    }

    pub fn get(&self, o: usize) -> Option<&Vec<Vec<String>>> {
        (o < self.own.len()).then(|| &self[o])
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &Vec<Vec<String>>> + '_ {
        (0..self.own.len()).map(move |o| &self[o])
    }
}

impl std::ops::Index<usize> for Refusals {
    type Output = Vec<Vec<String>>;
    fn index(&self, o: usize) -> &Vec<Vec<String>> {
        match &self.cached {
            Some(c) if c.has(o) => &c.src.facts(o).unsupported,
            _ => &self.own[o],
        }
    }
}

impl std::ops::IndexMut<usize> for Refusals {
    /// Only an object lowered here changes (the incremental compile renews workspace objects).
    fn index_mut(&mut self, o: usize) -> &mut Vec<Vec<String>> {
        assert!(
            !self.cached.as_ref().is_some_and(|c| c.has(o)),
            "the refusals of an object the cache supplies do not change"
        );
        &mut self.own[o]
    }
}

impl<'a> IntoIterator for &'a Refusals {
    type Item = &'a Vec<Vec<String>>;
    type IntoIter = Box<dyn ExactSizeIterator<Item = &'a Vec<Vec<String>>> + 'a>;
    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.iter())
    }
}

impl PartialEq for Refusals {
    fn eq(&self, other: &Refusals) -> bool {
        self.len() == other.len() && self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl std::fmt::Debug for Refusals {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl serde::Serialize for Refusals {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(self.iter())
    }
}

/// Per object, per routine: the routines it calls directly, as (object, routine). For an object a compile cache supplies,
/// `DepFacts::calls` and, for what its variables of page, report or xmlport types drive, `DepFacts::drivers`.
#[derive(Clone, Default)]
pub struct Calls {
    own: Vec<Vec<Vec<(usize, usize)>>>,
    /// Per object lowered here, per routine: how many of its calls come first and are calls its code makes; the rest is
    /// what a variable of a page, report or xmlport type drives. Missing: all are taken as calls.
    ndirect: Vec<Vec<u32>>,
    cached: Option<CachedFacts>,
}

impl Calls {
    pub(crate) fn new(
        own: Vec<Vec<Vec<(usize, usize)>>>,
        ndirect: Vec<Vec<u32>>,
        cached: Option<CachedFacts>,
    ) -> Calls {
        debug_assert_eq!(own.len(), ndirect.len(), "one entry per object in both");
        Calls { own, ndirect, cached }
    }

    /// Replaces the calls of object `o`, lowered again here.
    pub(crate) fn replace(&mut self, o: usize, calls: Vec<Vec<(usize, usize)>>, ndirect: Vec<u32>) {
        self.own[o] = calls;
        if let Some(n) = self.ndirect.get_mut(o) {
            *n = ndirect;
        }
    }

    /// Where the driven routines start among the calls of routine `r` of object `o` lowered here.
    fn split(&self, o: usize, r: usize) -> usize {
        let all = self.own[o][r].len();
        self.ndirect
            .get(o)
            .and_then(|n| n.get(r))
            .map_or(all, |&n| (n as usize).min(all))
    }

    /// The calls the code of routine `r` of object `o` makes, appended to `out`: [`extend_into`](Self::extend_into)
    /// without what a variable of a page, report or xmlport type drives.
    pub fn extend_direct_into(&self, o: usize, r: usize, out: &mut Vec<(usize, usize)>) {
        match self.cached(o) {
            Some(_) => self.extend_into(o, r, out),
            None => out.extend(self.own[o][r][..self.split(o, r)].iter().copied()),
        }
    }

    /// What a variable of a page, report or xmlport type in routine `r` of object `o` lowered here drives, appended to
    /// `out` (the rest of [`extend_into`](Self::extend_into)). Nothing for an object the cache supplies, whose drivers
    /// are kept apart (`Lowered::calls_into`).
    pub fn extend_driven_into(&self, o: usize, r: usize, out: &mut Vec<(usize, usize)>) {
        if self.cached(o).is_none() {
            out.extend(self.own[o][r][self.split(o, r)..].iter().copied());
        }
    }

    pub fn len(&self) -> usize {
        self.own.len()
    }

    pub fn is_empty(&self) -> bool {
        self.own.is_empty()
    }

    fn cached(&self, o: usize) -> Option<&crate::provided::DepFacts> {
        self.cached.as_ref().filter(|c| c.has(o)).map(|c| c.src.facts(o))
    }

    /// The direct calls of routine `r` of object `o`, appended to `out`.
    pub fn extend_into(&self, o: usize, r: usize, out: &mut Vec<(usize, usize)>) {
        match self.cached(o) {
            Some(f) => out.extend(f.calls[r].iter().map(|&(o, r)| (o as usize, r as usize))),
            None => out.extend(self.own[o][r].iter().copied()),
        }
    }

    /// The direct calls of every routine of object `o`.
    pub fn of(&self, o: usize) -> std::borrow::Cow<'_, [Vec<(usize, usize)>]> {
        match self.cached(o) {
            Some(f) => f
                .calls
                .iter()
                .map(|r| r.iter().map(|&(o, r)| (o as usize, r as usize)).collect())
                .collect::<Vec<_>>()
                .into(),
            None => self.own[o].as_slice().into(),
        }
    }

    /// For an object the cache supplies: the types of the variables each routine holds that drive other objects
    /// (`DepFacts::drivers`). `None` for an object lowered here.
    pub(crate) fn drivers(&self, o: usize, r: usize) -> Option<&[al_builtins::value::RtType]> {
        self.cached(o).and_then(|f| f.drivers.get(r)).map(Vec::as_slice)
    }
}

impl std::ops::Index<usize> for Calls {
    type Output = Vec<Vec<(usize, usize)>>;
    /// The calls of an object lowered here (an object the cache supplies has its own in [`Calls::of`]).
    fn index(&self, o: usize) -> &Vec<Vec<(usize, usize)>> {
        assert!(
            self.cached(o).is_none(),
            "the calls of a cached object are read with `of`"
        );
        &self.own[o]
    }
}

impl std::ops::IndexMut<usize> for Calls {
    fn index_mut(&mut self, o: usize) -> &mut Vec<Vec<(usize, usize)>> {
        assert!(
            self.cached(o).is_none(),
            "the calls of an object the cache supplies do not change"
        );
        &mut self.own[o]
    }
}

impl PartialEq for Calls {
    fn eq(&self, other: &Calls) -> bool {
        self.len() == other.len() && (0..self.len()).all(|o| self.of(o) == other.of(o))
    }
}

impl std::fmt::Debug for Calls {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries((0..self.len()).map(|o| self.of(o))).finish()
    }
}

impl serde::Serialize for Calls {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq((0..self.len()).map(|o| self.of(o)))
    }
}
