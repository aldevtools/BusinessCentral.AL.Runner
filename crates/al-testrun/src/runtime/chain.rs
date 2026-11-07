//! Which test codeunits run in the serial carry chain, and why (`--timings`, al-runner issue 6924).
//!
//! Test codeunits run in parallel, one session each, except the carry chain (`corpus.rs`): codeunits whose run hands
//! state to the later codeunits run first, one after the other, on the main thread. A codeunit outside the chain that
//! reads what a chain codeunit left runs again afterwards, also on the main thread. [`ChainReport`] counts both, with
//! the reason of each, so a run that is slower than its thread count promises can be read from `--timings`.

use super::TestIsolation;
use al_sema::lower::Lowered;
use std::sync::Arc;

/// Why a test codeunit runs in the carry chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChainWhy {
    /// Its isolation is Disabled (its `RequiredTestIsolation` under the TestRunnerHub, or the runner's): everything it
    /// writes reaches the later codeunits.
    Disabled,
    /// Under runner 130450 its `OnRun` trigger can write a table, and that write reaches the later codeunits.
    OnRunWritesTable,
    /// Under runner 130450 its `OnRun` trigger can change isolated storage or a number sequence.
    OnRunWritesState,
    /// Under runner 130450 its tests can take numbers from a number sequence, which the rollback does not give back.
    TestsTakeNumbers,
    /// It has RequiredTestIsolation = Disabled and is not run: the chain notes what it may have left.
    NotRun,
}

impl ChainWhy {
    pub fn text(self) -> &'static str {
        match self {
            ChainWhy::Disabled => "isolation Disabled",
            ChainWhy::OnRunWritesTable => "OnRun can write a table",
            ChainWhy::OnRunWritesState => "OnRun can change isolated storage or a number sequence",
            ChainWhy::TestsTakeNumbers => "tests can take NumberSequence numbers",
            ChainWhy::NotRun => "isolation Disabled, not run",
        }
    }
}

/// Why a codeunit outside the chain runs a second time, on the main thread, from what a chain codeunit left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RerunWhy {
    /// It read a table an earlier chain codeunit left rows in.
    ReadCarriedTable,
    /// An earlier chain codeunit took numbers or changed isolated storage, and its reach can read them.
    ReachesCarriedStorage,
    /// It read first a SingleInstance global that held another value at its start than the serial run gives it
    /// (`single_walk.rs`).
    ReadSingleInstance,
}

impl RerunWhy {
    pub fn text(self) -> &'static str {
        match self {
            RerunWhy::ReadCarriedTable => "read a table a chain codeunit wrote",
            RerunWhy::ReachesCarriedStorage => "can read the numbers or storage a chain codeunit changed",
            RerunWhy::ReadSingleInstance => "read SingleInstance state an earlier codeunit left",
        }
    }
}

/// Why a member of the carry chain ran on the walk's thread, not ahead on a helper thread (`chain_walk.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SerialWhy {
    /// The run had no helper threads for the chain: one thread, `--explain`, fewer than two members that can run ahead, or
    /// no install baseline.
    NoHelpers,
    /// Its isolation is Disabled, or it runs without a baseline: such a member is never run ahead.
    NeverAhead,
    /// Its suite had no start the helpers knew (its state could not be written for them).
    NoStart,
    /// Its suite was not run ahead any more: too many wrong predictions or new starts there (`MISSES`, `RESTARTS`).
    GaveUp,
    /// The helpers did not run its suite when the walk reached it: it is the first member from the suite's start (the
    /// install baseline or a new start), and the helpers run a suite's members only once one member ran from there.
    Closed,
    /// No helper had run it yet: the helpers were behind the walk or past their tries (`TRIES`).
    NotRun,
    /// Its run ahead started from a prediction made before a member before it had run, and was not run again in time.
    Early,
    /// Its run ahead started from a prediction that rested on an earlier member's own run ahead, which turned out wrong
    /// (that member's change depends on its start); not counted against `MISSES`.
    Guessed,
    /// Its run ahead started from another state than the walk's (a wrong prediction).
    Missed,
    /// Its run ahead started from the walk's rows, but read a SingleInstance global that held another value there than
    /// in the walk's state (ADR-066 amendment 1, `single_walk.rs`).
    SingleState,
    /// Its run ahead started from the walk's state, but what it left cannot be rebuilt on the walk's thread.
    NotRebuilt,
}

impl SerialWhy {
    pub fn text(self) -> &'static str {
        match self {
            SerialWhy::NoHelpers => "no helper threads for the chain",
            SerialWhy::NeverAhead => "isolation Disabled or no baseline, never run ahead",
            SerialWhy::NoStart => "its suite had no start the helpers knew",
            SerialWhy::GaveUp => "its suite was not run ahead any more (too many wrong predictions or new starts)",
            SerialWhy::Closed => "first member from its suite's start, which the helpers do not run",
            SerialWhy::NotRun => "no helper had run it yet",
            SerialWhy::Guessed => "its run ahead rested on an earlier member's run ahead, which was wrong",
            SerialWhy::Early => {
                "its run ahead started before the members before it had run, and was not run again in time"
            }
            SerialWhy::Missed => "its run ahead started from another state than the walk's",
            SerialWhy::SingleState => "its run ahead read SingleInstance state other than the walk's",
            SerialWhy::NotRebuilt => "what its run ahead left could not be rebuilt",
        }
    }
}

/// One codeunit of the carry chain, or one that ran again after the parallel part.
#[derive(Debug, Clone)]
pub struct ChainEntry<W> {
    pub id: u32,
    pub name: String,
    pub why: W,
    pub took: std::time::Duration,
    /// It left state the later codeunits can start from (always false for a re-run).
    pub left: bool,
    /// A helper thread ran it ahead of the chain's walk, and the walk took that run (`chain_walk.rs`); false: it ran on
    /// the walk's thread, serially.
    pub ahead: bool,
}

/// The test codeunits of a run that ran serially (`Phases::chain`).
#[derive(Debug, Clone, Default)]
pub struct ChainReport {
    /// Test codeunits the run had (all passes).
    pub units: usize,
    /// The codeunits of the carry chain, in the order they ran.
    pub members: Vec<ChainEntry<ChainWhy>>,
    /// The codeunits that ran again after the parallel part, from what the chain left.
    pub reruns: Vec<ChainEntry<RerunWhy>>,
    /// The time each new start for the helper threads took to write, after a member left something (`chain_walk.rs`).
    pub starts: Vec<std::time::Duration>,
    /// The members that ran serially, with why they did not run ahead.
    pub not_ahead: Vec<ChainEntry<SerialWhy>>,
    /// For the runs ahead dropped because they started elsewhere: what differed between the predicted start and the
    /// walk's state (a sequence, a table, a counter), and how often.
    pub differed: std::collections::BTreeMap<String, usize>,
    /// The serial check of the SingleInstance carrying ran (`RunOptions::single_serial_check`, `single_walk.rs`).
    pub serial_check: bool,
    /// The codeunits it ran again in one session.
    pub serial_checked: usize,
    /// The tests whose outcome differed there: a failure of the check.
    pub serial_mismatches: Vec<String>,
}

impl ChainReport {
    /// A run ahead was dropped because its predicted start differed from the walk's state in `part`.
    pub fn missed_by(&mut self, part: &str) {
        *self.differed.entry(part.to_string()).or_default() += 1;
    }

    /// The `--timings` lines: the share of codeunits that ran serially and their time, by reason.
    pub fn lines(&self) -> Vec<String> {
        let serial: Vec<ChainEntry<ChainWhy>> = self.members.iter().filter(|m| !m.ahead).cloned().collect();
        let total = |es: &mut dyn Iterator<Item = &std::time::Duration>| secs(es.sum());
        let mut out = vec![format!(
            "carry chain: {} of {} test codeunits are in it: {} ran serially ({:.2} s), {} ran ahead on other threads ({:.2} s); {} ran again serially after the parallel part ({:.2} s)",
            self.members.len(),
            self.units,
            serial.len(),
            total(&mut serial.iter().map(|m| &m.took)),
            self.members.len() - serial.len(),
            total(&mut self.members.iter().filter(|m| m.ahead).map(|m| &m.took)),
            self.reruns.len(),
            total(&mut self.reruns.iter().map(|m| &m.took)),
        )];
        if !self.starts.is_empty() {
            out.push(format!(
                "carry chain: {} new starts written for the helper threads ({:.2} s)",
                self.starts.len(),
                total(&mut self.starts.iter())
            ));
        }
        let left: Vec<String> = self
            .members
            .iter()
            .filter(|m| m.left)
            .map(|m| format!("{} {}", m.id, m.name))
            .collect();
        if !self.members.is_empty() {
            let mut line = format!(
                "carry chain: {} of {} left state for the later codeunits",
                left.len(),
                self.members.len()
            );
            if !left.is_empty() {
                line.push_str(&format!(
                    ": {}",
                    left.iter().take(20).cloned().collect::<Vec<_>>().join("; ")
                ));
                if left.len() > 20 {
                    line.push_str(&format!("; {} more", left.len() - 20));
                }
            }
            out.push(line);
        }
        if let Some(line) = by_reason("serial carry chain by reason", &serial, ChainWhy::text) {
            out.push(line);
        }
        if let Some(line) = by_reason("serial re-runs by reason", &self.reruns, RerunWhy::text) {
            out.push(line);
        }
        if let Some(line) = by_reason("serial carry chain by cause", &self.not_ahead, SerialWhy::text) {
            out.push(line);
        }
        if self.serial_check {
            out.push(format!(
                "SingleInstance serial check: {} test codeunits ran again in one session, {} outcomes differ",
                self.serial_checked,
                self.serial_mismatches.len()
            ));
            out.extend(
                self.serial_mismatches
                    .iter()
                    .map(|m| format!("SingleInstance serial check: {m}")),
            );
        }
        if !self.differed.is_empty() {
            let mut rows: Vec<(&String, &usize)> = self.differed.iter().collect();
            rows.sort_by_key(|(part, n)| (std::cmp::Reverse(**n), *part));
            let parts: Vec<String> = rows.iter().take(15).map(|(part, n)| format!("{n} {part}")).collect();
            let more = if rows.len() > 15 {
                format!("; {} more", rows.len() - 15)
            } else {
                String::new()
            };
            out.push(format!(
                "carry chain wrong predictions by what differed: {}{more}",
                parts.join("; ")
            ));
        }
        out
    }
}

fn secs(d: std::time::Duration) -> f64 {
    d.as_secs_f64()
}

/// "label: n reason (s); ...", the reason with the most codeunits first; `None` when there are none.
fn by_reason<W: Copy + Ord>(label: &str, entries: &[ChainEntry<W>], text: fn(W) -> &'static str) -> Option<String> {
    let mut counts: std::collections::BTreeMap<W, (usize, std::time::Duration)> = Default::default();
    for e in entries {
        let c = counts.entry(e.why).or_default();
        c.0 += 1;
        c.1 += e.took;
    }
    let mut rows: Vec<_> = counts.into_iter().collect();
    rows.sort_by_key(|(w, (n, _))| (std::cmp::Reverse(*n), *w));
    (!rows.is_empty()).then(|| {
        let parts: Vec<String> = rows
            .iter()
            .map(|(w, (n, took))| format!("{n} {} ({:.2} s)", text(*w), secs(*took)))
            .collect();
        format!("{label}: {}", parts.join("; "))
    })
}

/// Whether a chain codeunit left state for the later codeunits (`CARRIED`): rows, numbers taken, storage, or nothing
/// known (`None`). A codeunit that left nothing new leaves `baseline: None` (`numbers_taken.rs`).
pub(super) fn left_something(left: Option<&super::run::Carried>) -> bool {
    left.is_none_or(|l| {
        l.baseline.is_some()
            || !l.tables.is_empty()
            || l.everything
            || l.unfinished.is_some()
            || l.storage_why.is_some()
    })
}

/// Why runner 130450 carries what test codeunit `oi` leaves to the later codeunits (`Unit::carry`); `None` when it
/// leaves nothing they can see. Not asked for a codeunit whose isolation is Disabled (it carries everything) or one
/// on the TestRunnerHub (`corpus.rs` `on_hub`).
pub(super) fn carry_why(engine: &Arc<al_vm::Engine>, lowered: &Lowered, oi: usize) -> Option<ChainWhy> {
    match onrun_writes(engine, lowered, oi) {
        Some(why) => Some(why),
        None if tests_take_numbers(engine, lowered, oi) => Some(ChainWhy::TestsTakeNumbers),
        None => None,
    }
}

/// Why a unit with isolation `isolation` and carry `carry` is in the chain.
pub(super) fn member_why(isolation: TestIsolation, carry: Option<ChainWhy>) -> ChainWhy {
    match carry {
        Some(why) if isolation != TestIsolation::Disabled => why,
        _ => ChainWhy::Disabled,
    }
}

/// Whether the `OnRun` trigger of test codeunit `oi` can write a table ([`ChainWhy::OnRunWritesTable`]) or isolated
/// storage or a number sequence ([`ChainWhy::OnRunWritesState`]). Under Microsoft's runner 130450 it runs outside the
/// codeunit isolation: what it writes, committed or not, reaches the later codeunits (codeunits/run-onrun/
/// RqIsolationModes.al:357-369, :493-505 and :616, the `#else` branches the BC 27 legs run), so such a codeunit carries
/// what its OnRun left to them (`Unit::carry`). On the TestRunnerHub, Codeunit and Function isolation roll it back
/// (:357-362, :493-498) and Disabled carries it with the rest. Isolated storage and number sequences OnRun changes,
/// committed and open, carry the same way (testlibraries test-framework/onrun-storage/Y35OnRunStorage.al
/// C301_B1_OnRunCommittedStorage, C301_B1_OnRunSequences, C301_B1_OnRunOpenWrites), so an OnRun whose reach can write
/// them counts too. An OnRun whose reach is not fully known counts only through what it is known to write, as for tables
/// (a refused OnRun that names nothing does not hold back the later codeunits).
fn onrun_writes(engine: &Arc<al_vm::Engine>, lowered: &Lowered, oi: usize) -> Option<ChainWhy> {
    use super::written::{ReachIndex, reach_facts, state};
    let o = &lowered.program.objects[oi];
    let r = o
        .routines
        .iter()
        .position(|r| r.is_trigger && r.name.eq_ignore_ascii_case("OnRun"))?;
    if !crate::baseline::tables_written(lowered, oi, r).is_empty() {
        return Some(ChainWhy::OnRunWritesTable);
    }
    let index = engine.derived(|| ReachIndex::new(&engine.events, lowered));
    let f = reach_facts(&engine.events, lowered, &index, oi, r);
    (f.state_writes & (state::ISOLATED_STORAGE | state::NUMBER_SEQUENCE) != 0).then_some(ChainWhy::OnRunWritesState)
}

/// Whether a test of test codeunit `oi` can take a number from a number sequence. Under runner 130450 the codeunit's rollback
/// does not give it back (testlibraries test-framework/onrun-storage/Y35OnRunStorage.al:30-31, C302_B2_TestWritesStorage :457), so
/// such a codeunit joins the carry chain as one whose OnRun writes (`numbers_taken.rs`). Counted only through what the
/// tests are known to write, as for OnRun.
fn tests_take_numbers(engine: &Arc<al_vm::Engine>, lowered: &Lowered, oi: usize) -> bool {
    use super::written::{ReachIndex, state, union_reach_has};
    let index = engine.derived(|| ReachIndex::new(&engine.events, lowered));
    let o = &lowered.program.objects[oi];
    // One walk of the tests' joined reach: a walk per test went over their shared reach once per test (11 % of the
    // full corpus run with Microsoft's apps).
    let tests: Vec<(usize, usize)> = (0..o.routines.len())
        .filter(|&r| {
            o.routines[r]
                .attributes
                .iter()
                .any(|a| a.name.eq_ignore_ascii_case("Test"))
        })
        .map(|r| (oi, r))
        .collect();
    union_reach_has(&engine.events, lowered, &index, &tests, &|f| {
        f.state_writes & state::NUMBER_SEQUENCE != 0
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn entry<W>(id: u32, why: W, ms: u64) -> ChainEntry<W> {
        ChainEntry {
            id,
            name: format!("C{id}"),
            why,
            took: Duration::from_millis(ms),
            left: id == 2,
            ahead: id == 3,
        }
    }

    #[test]
    fn the_report_counts_the_serial_codeunits_by_reason() {
        let r = ChainReport {
            units: 10,
            members: vec![
                entry(1, ChainWhy::TestsTakeNumbers, 1000),
                entry(2, ChainWhy::OnRunWritesTable, 500),
                entry(3, ChainWhy::TestsTakeNumbers, 1000),
            ],
            reruns: vec![entry(7, RerunWhy::ReadCarriedTable, 250)],
            ..Default::default()
        };
        assert_eq!(
            r.lines(),
            vec![
                "carry chain: 3 of 10 test codeunits are in it: 2 ran serially (1.50 s), 1 ran ahead on other threads (1.00 s); 1 ran again serially after the parallel part (0.25 s)",
                "carry chain: 1 of 3 left state for the later codeunits: 2 C2",
                "serial carry chain by reason: 1 OnRun can write a table (0.50 s); 1 tests can take NumberSequence numbers (1.00 s)",
                "serial re-runs by reason: 1 read a table a chain codeunit wrote (0.25 s)",
            ]
        );
    }

    /// Why each serial member did not run ahead, and for the wrong predictions what differed, so a slow chain can be read
    /// from `--timings` (al-runner issue 6924).
    #[test]
    fn the_report_says_why_a_member_did_not_run_ahead() {
        let mut r = ChainReport {
            units: 4,
            members: vec![
                entry(1, ChainWhy::TestsTakeNumbers, 1000),
                entry(2, ChainWhy::TestsTakeNumbers, 500),
                entry(4, ChainWhy::TestsTakeNumbers, 250),
            ],
            not_ahead: vec![
                entry(1, SerialWhy::Closed, 1000),
                entry(2, SerialWhy::Missed, 500),
                entry(4, SerialWhy::Missed, 250),
            ],
            ..Default::default()
        };
        r.missed_by("number sequence GLOBAL");
        r.missed_by("number sequence GLOBAL");
        r.missed_by("table 18 Customer");
        let lines = r.lines();
        assert_eq!(
            lines[lines.len() - 2..],
            [
                "serial carry chain by cause: 2 its run ahead started from another state than the walk's (0.75 s); 1 first member from its suite's start, which the helpers do not run (1.00 s)",
                "carry chain wrong predictions by what differed: 2 number sequence GLOBAL; 1 table 18 Customer",
            ]
        );
    }

    #[test]
    fn an_empty_chain_is_one_line() {
        let r = ChainReport {
            units: 4,
            ..Default::default()
        };
        assert_eq!(
            r.lines(),
            vec![
                "carry chain: 0 of 4 test codeunits are in it: 0 ran serially (0.00 s), 0 ran ahead on other threads (0.00 s); 0 ran again serially after the parallel part (0.00 s)"
            ]
        );
    }
}
