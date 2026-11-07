//! The globals written and the session state changed by everything a routine reaches, memoized per routine.
//!
//! The SingleInstance walk asks, for the refused tests of each codeunit, which SingleInstance globals their reach can
//! write. That reach is a pure union: the facts of the routines in the set closed under what each routine pushes. So the
//! answer for a routine is its own facts joined with the answers of the routines it pushes. Routines that reach each
//! other share one answer (a strongly connected component of the call graph, found with Tarjan's algorithm), and every
//! later question reuses the answers already found, where a walk per codeunit visited the whole reach again.
//!
//! What a routine pushes is taken from one application of its summary to a walk that has entered nothing, which pushes
//! every routine that the body, the triggers it runs and the subscribers it raises can reach. The order of a walk does not
//! matter here. The first reason a reach is not known does depend on it, so the caller takes that from a walk.

use super::*;
use std::collections::HashMap;

/// A closure: the state bits written and the globals written, of the objects the filter keeps. Sorted, no duplicates.
#[derive(Default, Clone)]
pub(super) struct Facts {
    state_writes: u16,
    globals: Vec<(usize, usize)>,
}

impl Facts {
    fn join(&mut self, other: &Facts) {
        self.state_writes |= other.state_writes;
        // Most joins add nothing: look each global up before touching the list.
        for g in &other.globals {
            if let Err(at) = self.globals.binary_search(g) {
                self.globals.insert(at, *g);
            }
        }
    }
}

/// The answers found so far, kept in the [`ReachIndex`]. They depend on the program and on the filter (which objects'
/// globals count), which every caller on one index passes the same.
#[derive(Default)]
pub(super) struct Memo {
    /// Per routine (dense number, [`Memo::id`]): its component's answer, or [`NONE`].
    component_of: Vec<u32>,
    /// Per routine: its entry number while a search is in it, else 0.
    open: Vec<u32>,
    components: Vec<Facts>,
    /// Numbers of routines the index does not know of (none: the program does not change under an engine).
    beyond: HashMap<(usize, usize), usize>,
}

const NONE: u32 = u32::MAX;

/// A routine the search is in.
struct Frame {
    node: (usize, usize),
    id: usize,
    index: u32,
    low: u32,
    edges: Vec<(usize, usize)>,
    next: usize,
    /// Its own facts joined with the answers of the complete components it reaches.
    acc: Facts,
}

/// The state bits and the globals of kept objects that the joined reach of `roots` writes.
pub(super) fn written(
    events: &al_vm::events::Registry,
    lowered: &Lowered,
    index: &ReachIndex,
    roots: &[(usize, usize)],
    keep: &dyn Fn(usize) -> bool,
) -> (u16, Vec<(usize, usize)>) {
    let mut memo = index.closures.lock().unwrap_or_else(|e| e.into_inner());
    let mut scratch = Walk::new(events, lowered, index);
    scratch.facts = false;
    scratch.log_entered = true;
    let mut total = Facts::default();
    for &root in roots {
        let id = memo.id(index, root);
        if memo.component_of[id] == NONE {
            memo.search(&mut scratch, keep, root);
        }
        total.join(&memo.components[memo.component_of[id] as usize]);
    }
    (total.state_writes, total.globals)
}

impl Memo {
    /// The dense number of a routine: its bit in a walk's `seen`, or one past those for a routine the index does not
    /// know of.
    fn id(&mut self, index: &ReachIndex, (o, r): (usize, usize)) -> usize {
        let first = &index.first_routine;
        let id = match first.get(o) {
            Some(&base) if o + 1 < first.len() && base + r < first[o + 1] => base + r,
            _ => {
                let next = first.last().copied().unwrap_or(0) + self.beyond.len();
                *self.beyond.entry((o, r)).or_insert(next)
            }
        };
        if id >= self.component_of.len() {
            self.component_of.resize(id + 1, NONE);
            self.open.resize(id + 1, 0);
        }
        id
    }

    /// Finds the answer of `root` and of every routine it reaches that has none yet.
    fn search(&mut self, scratch: &mut Walk<'_>, keep: &dyn Fn(usize) -> bool, root: (usize, usize)) {
        let index = scratch.index;
        let mut counter = 0u32;
        let mut stack: Vec<Frame> = Vec::new();
        // Finished routines whose component is not complete: (routine, its number, entry number, facts).
        let mut finished: Vec<((usize, usize), usize, u32, Facts)> = Vec::new();
        let mut enter = |memo: &mut Memo, node: (usize, usize), stack: &mut Vec<Frame>| {
            let (own, edges) = Self::step(scratch, keep, node);
            counter += 1;
            let id = memo.id(index, node);
            memo.open[id] = counter;
            stack.push(Frame {
                node,
                id,
                index: counter,
                low: counter,
                edges,
                next: 0,
                acc: own,
            });
        };
        enter(self, root, &mut stack);
        while let Some(top) = stack.last_mut() {
            if top.next < top.edges.len() {
                let to = top.edges[top.next];
                top.next += 1;
                let to_id = self.id(index, to);
                let top = stack.last_mut().expect("a frame");
                let c = self.component_of[to_id];
                let n = self.open[to_id];
                if c != NONE {
                    top.acc.join(&self.components[c as usize]);
                } else if n != 0 {
                    top.low = top.low.min(n);
                } else {
                    enter(self, to, &mut stack);
                }
                continue;
            }
            let done = stack.pop().expect("a frame");
            finished.push((done.node, done.id, done.index, done.acc));
            if done.low == done.index {
                // The routines entered at or after this one that are still open are its component.
                let id = self.components.len() as u32;
                let mut joined = Facts::default();
                while finished.last().is_some_and(|(_, _, i, _)| *i >= done.index) {
                    let (_, m, _, acc) = finished.pop().expect("a member");
                    joined.join(&acc);
                    self.open[m] = 0;
                    self.component_of[m] = id;
                }
                if let Some(parent) = stack.last_mut() {
                    parent.acc.join(&joined);
                }
                self.components.push(joined);
            } else if let Some(parent) = stack.last_mut() {
                parent.low = parent.low.min(done.low);
            }
        }
    }

    /// The routine's own facts and the routines one application of its summary pushes.
    fn step(
        scratch: &mut Walk<'_>,
        keep: &dyn Fn(usize) -> bool,
        (o, r): (usize, usize),
    ) -> (Facts, Vec<(usize, usize)>) {
        scratch.stack.clear();
        for o in scratch.entered_log.drain(..) {
            scratch.entered[o] = 0;
        }
        scratch.entered_beyond.clear();
        scratch.global_triggers = false;
        let (state_writes, globals) = scratch.routine_own(o, r, keep);
        let own = Facts { state_writes, globals };
        (own, std::mem::take(&mut scratch.stack))
    }
}
