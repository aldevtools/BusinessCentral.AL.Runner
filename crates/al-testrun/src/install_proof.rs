//! The influence graph of a partner group's install sessions (ADR-079 decision 2): which app's install reads from which,
//! and which two collide, built from the footprints the sessions recorded (al-vm `install_footprint.rs`, the one owner
//! of the conflict rules). Its strongly connected components are the units the subset runs of `install_orders.rs` prove
//! independent of the install order.
//!
//! Only pairs of apps with no dependency path between them count: the order of the others is the same in every install.

use al_vm::install_footprint::InstallFootprint;
use std::collections::{BTreeMap, BTreeSet};

/// The footprint of each partner app: the union over every install session of the app in every run so far.
pub(crate) type Footprints = BTreeMap<u32, InstallFootprint>;

/// The edges between the unordered apps of a group.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Graph {
    /// (x, y): x's install reads from y's (it met y, read what y wrote, or one of them touches everything).
    pub reads: BTreeSet<(u32, u32)>,
    /// (x, y) and (y, x): both installs write the same row, counter, sequence or storage key.
    pub collide: BTreeSet<(u32, u32)>,
}

impl Graph {
    /// The graph of `apps` from their footprints; `unordered(x, y)` says that no dependency path joins x and y. Only an
    /// app with install code has a footprint (`PartnerGroup::recorded_run`). An app without one is reached only by a
    /// meet: x reads from it when x's session met it or read the app tables; it reads from none and collides with none,
    /// a session that touches everything included (ADR-079 decision 1, clarification of 2026-11-23: such a session
    /// conflicts with every other install *session*).
    pub(crate) fn of(apps: &BTreeSet<u32>, fp: &Footprints, unordered: impl Fn(u32, u32) -> bool) -> Graph {
        let mut g = Graph::default();
        for &x in apps {
            let Some(a) = fp.get(&x) else { continue };
            for &y in apps {
                if !unordered(x, y) {
                    continue;
                }
                let reads = match fp.get(&y) {
                    Some(b) => a.reads_from(b, y),
                    None => a.meets(y),
                };
                if reads {
                    g.reads.insert((x, y));
                }
                if x < y && fp.get(&y).is_some_and(|b| a.collides(b)) {
                    g.collide.insert((x, y));
                    g.collide.insert((y, x));
                }
            }
        }
        g
    }

    fn linked(&self, x: u32, y: u32) -> bool {
        self.reads.contains(&(x, y)) || self.collide.contains(&(x, y))
    }

    /// The units of `apps`: the strongly connected components of "reads from" plus "collide", each sorted, the units
    /// listed so that a unit comes after every unit it reads from and after every unit with an app one of its apps
    /// depends on (`needs(a, b)`: a depends on b). Every order the proof builds from this list then keeps the
    /// dependencies, unless the reads run against them or the dependencies of two units cross (ADR-079 decision 3,
    /// clarification of 2026-11-23, review round 4; neighbor app `install-order-footprint-final-order`). Then the final
    /// install takes the plan order (`PartnerGroup::final_order`).
    pub(crate) fn units(&self, apps: &BTreeSet<u32>, needs: impl Fn(u32, u32) -> bool) -> Vec<Vec<u32>> {
        let reach = self.reach(apps);
        let reaches = |x: u32, y: u32| x == y || reach.contains(&(x, y));
        let mut units: Vec<Vec<u32>> = Vec::new();
        for &a in apps {
            if units.iter().any(|u| u.contains(&a)) {
                continue;
            }
            units.push(
                apps.iter()
                    .copied()
                    .filter(|&b| reaches(a, b) && reaches(b, a))
                    .collect(),
            );
        }
        // Inputs and dependencies first: a unit that reaches another reads from it (directly or through others; two
        // units never reach each other, else they would be one). When the reads run against the dependencies, or the
        // dependencies of two units cross, no unit is free and the list order stays.
        let after =
            |u: &[u32], v: &[u32]| reach.contains(&(u[0], v[0])) || u.iter().any(|&a| v.iter().any(|&b| needs(a, b)));
        let mut ordered = Vec::new();
        while !units.is_empty() {
            let k = (0..units.len())
                .find(|&i| !units.iter().enumerate().any(|(j, v)| j != i && after(&units[i], v)))
                .unwrap_or(0);
            ordered.push(units.remove(k));
        }
        ordered
    }

    /// Every pair (x, y) of `apps` with a path from x to y.
    fn reach(&self, apps: &BTreeSet<u32>) -> BTreeSet<(u32, u32)> {
        let mut r: BTreeSet<(u32, u32)> = (apps.iter())
            .flat_map(|&x| apps.iter().map(move |&y| (x, y)))
            .filter(|&(x, y)| self.linked(x, y))
            .collect();
        for &k in apps {
            for &x in apps {
                if !r.contains(&(x, k)) {
                    continue;
                }
                for &y in apps {
                    if r.contains(&(k, y)) {
                        r.insert((x, y));
                    }
                }
            }
        }
        r
    }

    /// The units `unit` reads from directly (its inputs), in the order of `units`.
    pub(crate) fn inputs<'u>(&self, units: &'u [Vec<u32>], unit: &[u32]) -> Vec<&'u Vec<u32>> {
        (units.iter())
            .filter(|v| v.as_slice() != unit && unit.iter().any(|&x| v.iter().any(|&y| self.reads.contains(&(x, y)))))
            .collect()
    }

    /// The units `unit` reads from directly or through others, in the order of `units`.
    pub(crate) fn ancestors<'u>(&self, units: &'u [Vec<u32>], unit: &[u32]) -> Vec<&'u Vec<u32>> {
        let mut found: Vec<&Vec<u32>> = self.inputs(units, unit);
        let mut k = 0;
        while k < found.len() {
            for v in self.inputs(units, found[k]) {
                if v.as_slice() != unit && !found.contains(&v) {
                    found.push(v);
                }
            }
            k += 1;
        }
        units.iter().filter(|u| found.contains(u)).collect()
    }
}

/// What ties apps `x` and `y` in the graph, for the note of a refusal: the reason one touches everything, a meet, or a
/// resource one reads or both write.
pub(crate) fn tie(
    fp: &Footprints,
    x: u32,
    y: u32,
    name: impl Fn(u32) -> String,
    table: impl Fn(u32) -> String,
) -> String {
    let none = InstallFootprint::default();
    let (a, b) = (fp.get(&x).unwrap_or(&none), fp.get(&y).unwrap_or(&none));
    if let Some(why) = a.everything.first().or(b.everything.first()) {
        return format!("an install that touches everything ({why})");
    }
    if a.meets(y) {
        return format!("the install of {} meets {}", name(x), name(y));
    }
    let read = (a.reads.iter()).find_map(|r| b.writes.iter().find(|w| r.read_sees_write(w)).map(|w| (r, w)));
    if let Some((r, w)) = read {
        let (r, w) = (r.describe(&table), w.describe(&table));
        return format!("{} reads {r}, which {} writes ({w})", name(x), name(y));
    }
    let both = (a.writes.iter()).find(|w| b.writes.iter().any(|v| w.writes_collide(v)));
    match both {
        Some(w) => format!("{} and {} both write {}", name(x), name(y), w.describe(&table)),
        None => format!("{} reads from {}", name(x), name(y)),
    }
}

#[cfg(test)]
#[path = "install_proof_tests.rs"]
mod tests;
