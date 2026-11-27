use super::PMap;
use std::collections::BTreeMap;
use std::ops::Bound::{self, Excluded, Included, Unbounded};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn bound(r: &mut Rng, space: u64) -> Bound<u64> {
    match r.below(3) {
        0 => Unbounded,
        1 => Included(r.below(space + 4)),
        _ => Excluded(r.below(space + 4)),
    }
}

fn same(p: &PMap<u64, u64>, m: &BTreeMap<u64, u64>) {
    // The fence keys are the chunks' first keys, one per chunk, and no chunk is empty.
    assert!(p.spine.chunks.iter().all(|c| !c.is_empty()));
    assert_eq!(p.spine.firsts, chunk_firsts(p));
    assert_eq!(p.len(), m.len());
    assert_eq!(p.is_empty(), m.is_empty());
    assert!(p.iter().eq(m.iter()));
    assert!(p.iter().rev().eq(m.iter().rev()));
    assert!(p.keys().eq(m.keys()));
    assert!(p.values().eq(m.values()));
}

fn check_ranges(p: &PMap<u64, u64>, m: &BTreeMap<u64, u64>, r: &mut Rng, space: u64) {
    for _ in 0..6 {
        let (lo, hi) = (bound(r, space), bound(r, space));
        // BTreeMap panics on a start after the end, or on equal excluded bounds; the map must stay quiet.
        let bad = match (lo, hi) {
            (Included(a) | Excluded(a), Included(b) | Excluded(b)) => {
                a > b || (a == b && matches!((lo, hi), (Excluded(_), Excluded(_))))
            }
            _ => false,
        };
        if bad {
            assert_eq!(p.range((lo, hi)).count(), 0);
            continue;
        }
        assert!(p.range((lo, hi)).eq(m.range((lo, hi))), "{lo:?} {hi:?}");
        assert!(p.range((lo, hi)).rev().eq(m.range((lo, hi)).rev()), "rev {lo:?} {hi:?}");
        // Both ends at once.
        let mut a = p.range((lo, hi));
        let mut b = m.range((lo, hi));
        loop {
            let (x, y) = if r.below(2) == 0 {
                (a.next(), b.next())
            } else {
                (a.next_back(), b.next_back())
            };
            assert_eq!(x, y);
            if x.is_none() {
                break;
            }
        }
    }
}

#[test]
fn random_operations_match_btreemap() {
    for seed in 1..=40u64 {
        let mut r = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
        // Small key spaces force splits, merges and empty chunks; large ones give many chunks.
        let space = [8, 70, 400, 5000][(seed % 4) as usize];
        let mut p: PMap<u64, u64> = PMap::new();
        let mut m = BTreeMap::new();
        let mut forks: Vec<(PMap<u64, u64>, BTreeMap<u64, u64>)> = vec![];
        for step in 0..3000u64 {
            let k = r.below(space);
            match r.below(10) {
                0..=4 => assert_eq!(p.insert(k, step), m.insert(k, step)),
                5..=7 => assert_eq!(p.remove(&k), m.remove(&k)),
                8 => {
                    if let (Some(a), Some(b)) = (p.get_mut(&k), m.get_mut(&k)) {
                        *a += 1;
                        *b += 1;
                    }
                    assert_eq!(p.get(&k), m.get(&k));
                    assert_eq!(p.contains_key(&k), m.contains_key(&k));
                }
                _ => {
                    // Fork, then change the fork or the original; the other side must not move.
                    let f = p.clone();
                    assert!(p.ptr_eq(&f));
                    forks.push((f, m.clone()));
                    if forks.len() > 6 {
                        forks.remove(0);
                    }
                }
            }
            if step % 97 == 0 {
                same(&p, &m);
                check_ranges(&p, &m, &mut r, space);
                for (fp, fm) in &mut forks {
                    same(fp, fm);
                    // Mutate the fork.
                    let k = r.below(space);
                    assert_eq!(fp.insert(k, 7), fm.insert(k, 7));
                    let k = r.below(space);
                    assert_eq!(fp.remove(&k), fm.remove(&k));
                    same(fp, fm);
                }
                same(&p, &m);
            }
        }
        same(&p, &m);
    }
}

#[test]
fn collect_keeps_the_last_of_equal_keys() {
    let p: PMap<u64, u64> = [(3, 1), (1, 2), (3, 3), (2, 4), (1, 5)].into_iter().collect();
    assert_eq!(p.len(), 3);
    assert!(p.iter().map(|(k, v)| (*k, *v)).eq([(1, 5), (2, 4), (3, 3)]));
    let big: PMap<u64, u64> = (0..1000u64).rev().map(|k| (k, k)).collect();
    assert!(big.keys().copied().eq(0..1000));
    assert!(big.iter().rev().map(|(k, _)| *k).eq((0..1000).rev()));
}

#[test]
fn boundary_keys_and_empty_map() {
    let mut p: PMap<u64, u64> = PMap::new();
    assert_eq!(p.get(&0), None);
    assert_eq!(p.remove(&0), None);
    assert_eq!(p.range(..).count(), 0);
    assert_eq!(p.iter().next_back(), None);
    for k in [0, u64::MAX, 1, u64::MAX - 1] {
        p.insert(k, k);
    }
    assert_eq!(p.range(..=0).count(), 1);
    assert_eq!(p.range(u64::MAX..).count(), 1);
    assert_eq!(p.range((Excluded(0), Excluded(u64::MAX))).count(), 2);
    assert_eq!(p.iter().next_back(), Some((&u64::MAX, &u64::MAX)));
}

#[test]
fn a_clone_shares_until_either_side_writes() {
    let mut a: PMap<u64, u64> = (0..500u64).map(|k| (k, k)).collect();
    let b = a.clone();
    assert!(a.ptr_eq(&b));
    a.insert(1000, 1);
    assert!(!a.ptr_eq(&b));
    assert_eq!(b.len(), 500);
    assert_eq!(a.len(), 501);
}

/// The first key of every chunk: the keys a spine search compares against.
fn chunk_firsts(p: &PMap<u64, u64>) -> Vec<u64> {
    p.spine.chunks.iter().map(|c| c[0].0).collect()
}

/// Every lookup and every range seek on each key from just below the smallest to just above the largest, and on both
/// sides of each chunk's first key, answers like the model.
fn every_key_and_seek(p: &PMap<u64, u64>, m: &BTreeMap<u64, u64>) {
    same(p, m);
    let hi = m.keys().next_back().map_or(2, |k| k + 2);
    for k in 0..hi {
        assert_eq!(p.get(&k), m.get(&k), "get {k}");
        assert_eq!(p.get_key_value(&k), m.get_key_value(&k), "get_key_value {k}");
        assert_eq!(p.contains_key(&k), m.contains_key(&k), "contains_key {k}");
    }
    for f in chunk_firsts(p) {
        for k in [f.saturating_sub(1), f, f + 1] {
            for (lo, hi) in [
                (Included(k), Unbounded),
                (Excluded(k), Unbounded),
                (Unbounded, Included(k)),
                (Unbounded, Excluded(k)),
            ] {
                assert!(p.range((lo, hi)).eq(m.range((lo, hi))), "{lo:?} {hi:?}");
                assert!(p.range((lo, hi)).rev().eq(m.range((lo, hi)).rev()), "rev {lo:?} {hi:?}");
            }
        }
    }
}

#[test]
fn lookups_on_chunk_boundaries_survive_splits_front_inserts_and_merges() {
    // Even keys from 1000 up, so every chunk's first key has an absent neighbor on both sides.
    let mut m: BTreeMap<u64, u64> = (500..1500u64).map(|k| (2 * k, k)).collect();
    let mut p: PMap<u64, u64> = m.iter().map(|(k, v)| (*k, *v)).collect();
    assert!(chunk_firsts(&p).len() > 10);
    every_key_and_seek(&p, &m);
    // New smallest keys: each one becomes the first key of the first chunk, which splits as it grows.
    for k in (0..500u64).rev() {
        assert_eq!(p.insert(2 * k + 1, k), m.insert(2 * k + 1, k));
    }
    every_key_and_seek(&p, &m);
    // Fill the gaps inside one chunk until it splits, then check the new chunk's first key.
    let before = chunk_firsts(&p).len();
    // A chunk of the even run built by collect (48 even keys), so the odd keys are all new.
    let start = chunk_firsts(&p)[before - 3];
    for k in start..start + 80 {
        assert_eq!(p.insert(k | 1, k), m.insert(k | 1, k));
    }
    assert!(chunk_firsts(&p).len() > before);
    every_key_and_seek(&p, &m);
    // Remove the first key of every chunk, again and again: the next key becomes the first, and small chunks merge.
    for _ in 0..40 {
        for f in chunk_firsts(&p) {
            assert_eq!(p.remove(&f), m.remove(&f));
        }
        every_key_and_seek(&p, &m);
    }
    // Remove what is left from the front, emptying chunks one by one.
    while let Some(k) = m.keys().next().copied() {
        assert_eq!(p.remove(&k), m.remove(&k));
        if k % 37 == 0 {
            every_key_and_seek(&p, &m);
        }
    }
    every_key_and_seek(&p, &m);
}

#[test]
fn a_fork_keeps_its_chunk_boundaries_when_the_other_side_moves_them() {
    let ma: BTreeMap<u64, u64> = (10..1010u64).map(|k| (2 * k, k)).collect();
    let mut a: PMap<u64, u64> = ma.iter().map(|(k, v)| (*k, *v)).collect();
    let mut ma = ma;
    let b = a.clone();
    let mb = ma.clone();
    // Move boundaries on one side: a new smallest key, removed chunk-first keys, a split.
    assert_eq!(a.insert(1, 1), ma.insert(1, 1));
    for f in chunk_firsts(&a).into_iter().skip(1).step_by(2) {
        assert_eq!(a.remove(&f), ma.remove(&f));
    }
    let f = chunk_firsts(&a)[3];
    for k in f..f + 70 {
        assert_eq!(a.insert(k | 1, 0), ma.insert(k | 1, 0));
    }
    every_key_and_seek(&a, &ma);
    every_key_and_seek(&b, &mb);
    // And the other way round: a fork of the changed side, changed again, leaves the first two alone.
    let mut c = a.clone();
    let mut mc = ma.clone();
    for f in chunk_firsts(&c) {
        assert_eq!(c.remove(&f), mc.remove(&f));
    }
    assert_eq!(c.insert(0, 9), mc.insert(0, 9));
    every_key_and_seek(&c, &mc);
    every_key_and_seek(&a, &ma);
    every_key_and_seek(&b, &mb);
}
