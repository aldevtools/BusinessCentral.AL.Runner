//! The carried starts of the serial sessions on worker threads (ADR-066 amendment 5, decision 5; al-runner issue 6924).
//!
//! A codeunit of a serial session that comes after a carry-chain member (RequiredTestIsolation = Disabled, ADR-055)
//! starts from the baseline that member left. Such a baseline holds `Rc`s and stays on the main thread, so a worker
//! thread asks the main thread for it.
//!
//! As first built, the main thread wrote the whole baseline as bytes for every step that asked, and the worker read it
//! back and dropped it. A partner-shaped suite of 701 carry members over about 65,000 rows of test data asked 6,737
//! times; at about 78 ms each, one after another on the main thread, the lane took 523 s against 18 s for one serial
//! lane. Now:
//!
//! - The main thread makes each carry's start once, as a [`RowDelta`]: the rows it holds apart from the carry before it
//!   (or the install baseline for the first). Next to each other in the chain, two baselines differ in the rows one
//!   codeunit wrote, and their persistent maps share every other entry, so the delta costs pointer compares and a few
//!   rows ([`Deltas`]). They are kept for every session that asks, within [`DELTA_BUDGET`] bytes; past it the cache is
//!   cleared and a delta is made again when asked (the same bytes: the baselines stay).
//! - A worker thread keeps the one start it built last ([`Held`]) and goes forward from it with the deltas between, or
//!   from its own copy of the install baseline. It holds one carried start at a time (the memory rule of issue 6924).
//! - A start whose delta cannot be made (a row that is not a plain value, other company copies) goes as whole bytes,
//!   as before ([`Starts`]).

use super::chain_walk::Carry as ChainCarry;
use super::*;
use al_vm::RowDelta;
use std::collections::BTreeMap;

/// The most bytes of deltas the main thread keeps at once; past it the kept deltas are dropped and made again when a
/// worker asks (al-runner issue 6924: the memory a lane holds must not grow with the sessions that ask).
pub(super) const DELTA_BUDGET: usize = 256 << 20;

/// A carried baseline as bytes, for a worker thread.
pub(super) struct Start {
    bytes: Vec<u8>,
    install_rows: Option<std::collections::BTreeSet<u32>>,
    origin: u64,
}

impl Start {
    /// The worker's own copy (the same data, so the same origin, over the engine's test data).
    pub(super) fn decode(&self, engine: &Arc<Engine>) -> Option<al_vm::Baseline> {
        al_vm::Baseline::from_bytes(&self.bytes).map(|b| {
            let mut b = b.backed_by(engine).with_origin(self.origin);
            b.install_rows = self.install_rows.clone();
            b
        })
    }
}

/// The carried baselines a worker asks for whole, written as bytes when one is asked for, not before. This side keeps
/// only a weak reference and a worker drops the bytes once it has read them back, so a start lives only while a worker
/// reads it: at most one per worker thread at once. Two workers that ask for the same start at once share it.
#[derive(Default)]
pub(super) struct Starts {
    /// By carry index: the start while a worker holds it; `None`: it cannot be written as bytes.
    known: BTreeMap<usize, Option<std::sync::Weak<Start>>>,
    /// How many starts were written.
    pub written: usize,
    /// The most starts alive at once.
    pub peak: usize,
}

impl Starts {
    /// The start of carry `carry`, whose baseline is `b`; `None` when it cannot be written as bytes.
    pub(super) fn get(&mut self, carry: usize, b: &al_vm::Baseline) -> Option<Arc<Start>> {
        match self.known.get(&carry) {
            Some(None) => return None,
            Some(Some(alive)) => {
                if let Some(s) = alive.upgrade() {
                    return Some(s);
                }
            }
            None => {}
        }
        let Some(bytes) = b.to_bytes() else {
            self.known.insert(carry, None);
            return None;
        };
        let s = Arc::new(Start {
            bytes,
            install_rows: b.install_rows.clone(),
            origin: b.origin(),
        });
        self.known.insert(carry, Some(Arc::downgrade(&s)));
        self.written += 1;
        let alive = self.known.values().flatten().filter(|w| w.strong_count() > 0).count();
        self.peak = self.peak.max(alive);
        Some(s)
    }
}

/// Where a carry's delta stands ([`Deltas`]).
enum Made {
    /// The delta from the start of carry `base` (`None`: the install baseline), while it is kept.
    Ok {
        base: Option<usize>,
        delta: Option<Arc<RowDelta>>,
    },
    /// No delta: no baseline, or one that cannot be written as rows.
    No,
}

/// The deltas of the carried starts, each made once on the main thread ([`RowDelta`]). Carry `i`'s delta is taken from
/// the last earlier carry that has one (the install baseline for the first), so the deltas from the install baseline to
/// any carry form one line that a worker walks forward on.
pub(super) struct Deltas<'a> {
    carried: &'a [ChainCarry],
    install: &'a al_vm::Baseline,
    made: Vec<Made>,
    /// The last carry with a delta among those made.
    last: Option<usize>,
    kept: usize,
    /// How many deltas were made (again after the budget dropped them, too).
    pub count: usize,
    /// The most bytes of deltas kept at once.
    pub peak: usize,
}

impl<'a> Deltas<'a> {
    pub(super) fn new(carried: &'a [ChainCarry], install: &'a al_vm::Baseline) -> Self {
        Deltas {
            carried,
            install,
            made: Vec::with_capacity(carried.len()),
            last: None,
            kept: 0,
            count: 0,
            peak: 0,
        }
    }

    fn baseline(&self, carry: Option<usize>) -> &'a al_vm::Baseline {
        match carry {
            Some(c) => self.carried[c]
                .3
                .baseline
                .as_ref()
                .expect("a carry with a delta has a baseline"),
            None => self.install,
        }
    }

    fn make(&mut self, carry: usize, base: Option<usize>) -> Option<Arc<RowDelta>> {
        let b = self.carried[carry].3.baseline.as_ref()?;
        let d = Arc::new(b.row_delta_from(self.baseline(base))?);
        self.count += 1;
        if self.kept + d.size() > DELTA_BUDGET {
            self.forget();
        }
        self.kept += d.size();
        self.peak = self.peak.max(self.kept);
        Some(d)
    }

    /// Drops every delta kept for a later session (the memory budget asks, `memory.rs`); a later ask makes it again.
    /// Whether any was kept.
    pub(super) fn forget(&mut self) -> bool {
        for m in &mut self.made {
            if let Made::Ok { delta, .. } = m {
                *delta = None;
            }
        }
        std::mem::replace(&mut self.kept, 0) > 0
    }

    /// The delta of carry `carry` and the carry it is taken from (`None`: the install baseline); `None` when it has none.
    pub(super) fn get(&mut self, carry: usize) -> Option<(Arc<RowDelta>, Option<usize>)> {
        while self.made.len() <= carry {
            let i = self.made.len();
            let base = self.last;
            let m = match self.make(i, base) {
                Some(d) => {
                    self.last = Some(i);
                    Made::Ok { base, delta: Some(d) }
                }
                None => Made::No,
            };
            self.made.push(m);
        }
        let (base, kept) = match &self.made[carry] {
            Made::No => return None,
            Made::Ok { base, delta } => (*base, delta.clone()),
        };
        let d = match kept {
            Some(d) => d,
            None => {
                let d = self.make(carry, base)?;
                if let Made::Ok { delta, .. } = &mut self.made[carry] {
                    *delta = Some(d.clone());
                }
                d
            }
        };
        Some((d, base))
    }
}

/// What the main thread hands a worker for a carried start.
pub(super) enum Reply {
    /// The delta of the start and the carry it is taken from (`None`: the install baseline).
    Delta(Arc<RowDelta>, Option<usize>),
    /// The start as bytes (`None`: it cannot be written).
    Whole(Option<Arc<Start>>),
}

/// A worker's question: the carry, whether it wants the start whole, and where to answer.
pub(super) type Ask = (usize, bool, std::sync::mpsc::Sender<Reply>);

/// The carried start a worker thread built last: it goes forward from it.
pub(super) type Held = Option<(usize, al_vm::Baseline)>;

/// The start of carry `carry` on a worker thread: from the start it holds or its own copy of the install baseline
/// `install`, with the deltas `ask` gets from the main thread up to it; whole bytes when the carry has no delta.
/// `Err(())`: the start cannot be had on this thread. `Ok(None)`: it does not read back (the codeunit keeps its first
/// run, as before).
pub(super) fn carried_start(
    carry: usize,
    held: &mut Held,
    install: Option<&al_vm::Baseline>,
    engine: &Arc<Engine>,
    ask: &mut dyn FnMut(usize, bool) -> Option<Reply>,
) -> Result<Option<al_vm::Baseline>, ()> {
    if let Some((c, b)) = held.as_ref()
        && *c == carry
    {
        return Ok(Some(b.clone()));
    }
    let mut path: Vec<Arc<RowDelta>> = Vec::new();
    let mut at = carry;
    let from = loop {
        if let Some((c, b)) = held.as_ref()
            && *c == at
        {
            break Some(b.clone());
        }
        match ask(at, false) {
            Some(Reply::Delta(d, base)) => {
                path.push(d);
                match base {
                    Some(b) => at = b,
                    // Without an install baseline the first delta was taken from one that holds nothing.
                    None => {
                        break Some(
                            install
                                .cloned()
                                .unwrap_or_else(|| al_vm::Baseline::empty().backed_by(engine)),
                        );
                    }
                }
            }
            Some(Reply::Whole(Some(s))) if path.is_empty() => {
                *held = None;
                return Ok(s.decode(engine));
            }
            _ => return whole(carry, held, engine, ask),
        }
    };
    let Some(mut b) = from else {
        return whole(carry, held, engine, ask);
    };
    for d in path.iter().rev() {
        match b.with_row_delta(d) {
            Some(next) => b = next,
            None => return whole(carry, held, engine, ask),
        }
    }
    *held = Some((carry, b.clone()));
    Ok(Some(b))
}

/// The start of carry `carry` as whole bytes from the main thread; the worker then holds no start.
fn whole(
    carry: usize,
    held: &mut Held,
    engine: &Arc<Engine>,
    ask: &mut dyn FnMut(usize, bool) -> Option<Reply>,
) -> Result<Option<al_vm::Baseline>, ()> {
    *held = None;
    match ask(carry, true) {
        Some(Reply::Whole(Some(s))) => Ok(s.decode(engine)),
        _ => Err(()),
    }
}
