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
    /// The sequential strict run (ADR-070): it leaves nothing the later codeunits can see, and runs in its place in BC's
    /// order for the single instances it takes over from the one before it and hands to the next.
    InOrder,
    /// BcContainerHelper's discovery pass (ADR-074, `discovery.rs`): the OnRun trigger of every test codeunit of the BC
    /// session, before the first one runs; what it leaves stays.
    Discovery,
}

impl ChainWhy {
    pub fn text(self) -> &'static str {
        match self {
            ChainWhy::Disabled => "isolation Disabled",
            ChainWhy::OnRunWritesTable => "OnRun can write a table",
            ChainWhy::OnRunWritesState => "OnRun can change isolated storage or a number sequence",
            ChainWhy::TestsTakeNumbers => "tests can take NumberSequence numbers",
            ChainWhy::NotRun => "isolation Disabled, not run",
            ChainWhy::InOrder => "in BC's order (sequential strict run)",
            ChainWhy::Discovery => "BcContainerHelper's discovery pass",
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
}

impl RerunWhy {
    pub fn text(self) -> &'static str {
        match self {
            RerunWhy::ReadCarriedTable => "read a table a chain codeunit wrote",
            RerunWhy::ReachesCarriedStorage => "can read the numbers or storage a chain codeunit changed",
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
    /// The SingleInstance lane (`single_lane.rs`, ADR-066 amendment 2).
    pub single_lane: super::LaneReport,
    /// ADR-058 amendment 1: the test codeunits that read SingleInstance state an earlier codeunit of their BC session
    /// left, with what they read (chain members from the walk, the others from the lane).
    pub carried: Vec<super::CarriedCodeunit>,
    /// ADR-066 amendment 8: members the walk ran with the single instances the members before them left.
    pub singles_kept: usize,
    /// Of `singles_kept`, those that read first a global an earlier member of their BC session wrote.
    pub singles_read_carried: usize,
    /// ADR-066 amendment 12, decision 3 (default run only): the test codeunits whose tests can take numbers after an
    /// earlier codeunit of their BC session that can take some too.
    pub numbers_order: Vec<super::NumbersOrder>,
    /// ADR-074: the test codeunits (suite, id) whose OnRun raised in BcContainerHelper's discovery pass. They get no
    /// method rows; the run still calls their OnRun (`discovery.rs`).
    pub discovery_raised: Vec<(usize, u32)>,
    /// ADR-074: the suites whose discovery pass changed session state the run does not carry, and why: every test of the
    /// suite is refused (`discovery::refuse_after_pass`).
    pub discovery_refused: Vec<(usize, String)>,
    /// ADR-074: the SingleInstance globals each suite's discovery pass changed (`discovery::refuse_after_pass`).
    pub discovery_singles: Vec<(usize, al_vm::single_track::GlobalKey)>,
    /// ADR-074: per suite, the test codeunits whose OnRun changed something in the discovery pass (id, tables, number
    /// sequences or isolated storage).
    pub discovery_wrote: Vec<(usize, u32, std::collections::BTreeSet<u32>, bool)>,
}

/// The most `number order:` lines the run output prints before it prints one summary line.
pub const NUMBERS_LINES_MAX: usize = 5;

impl ChainReport {
    /// Adds what another walk found (the sequential strict run's BC sessions each report on their own thread).
    pub(super) fn absorb(&mut self, other: ChainReport) {
        self.members.extend(other.members);
        self.reruns.extend(other.reruns);
        self.carried.extend(other.carried);
        self.singles_kept += other.singles_kept;
        self.singles_read_carried += other.singles_read_carried;
        self.discovery_raised.extend(other.discovery_raised);
        self.discovery_refused.extend(other.discovery_refused);
        self.discovery_singles.extend(other.discovery_singles);
        self.discovery_wrote.extend(other.discovery_wrote);
    }

    /// ADR-066 amendment 12, decision 3: the numbers list, one line per codeunit. The run output prints it with or
    /// without `--timings`.
    pub fn numbers_lines(&self) -> Vec<String> {
        self.numbers_order.iter().map(|n| n.line()).collect()
    }

    /// The order lines the run output prints with or without `--timings`: the skipped re-runs (ADR-066 amendment 7,
    /// option 2) and the numbers list.
    pub fn run_output_lines(&self) -> Vec<String> {
        let mut out = self.single_lane.skipped_lines();
        out.extend(self.numbers_summary());
        out
    }

    /// The numbers list for the run output: one line per codeunit up to [`NUMBERS_LINES_MAX`], else one summary line
    /// (`--timings` and the JUnit property `numberSequenceOrder` keep the detail). Diagnostics only (ADR-009).
    pub fn numbers_summary(&self) -> Vec<String> {
        let n = self.numbers_order.len();
        if n <= NUMBERS_LINES_MAX {
            return self.numbers_lines();
        }
        vec![format!(
            "number order: {n} test codeunits take NumberSequence numbers after another test codeunit took some on BC; \
             --timings lists them, the JUnit property numberSequenceOrder marks their tests, --strict-order chains the numbers as on BC"
        )]
    }

    /// The text for the tests of codeunit `codeunit` of run `suite` when it is on the numbers list.
    pub fn numbers_text(&self, suite: usize, codeunit: u32) -> Option<String> {
        self.numbers_order
            .iter()
            .find(|n| n.suite == suite && n.codeunit == codeunit)
            .map(|n| n.text())
    }

    /// The `--timings` lines: the codeunits of the chain and their time, by reason.
    pub fn lines(&self) -> Vec<String> {
        let total = |es: &mut dyn Iterator<Item = &std::time::Duration>| secs(es.sum());
        let mut out = vec![format!(
            "carry chain: {} of {} test codeunits are in it, all ran serially ({:.2} s); {} ran again serially after the parallel part ({:.2} s)",
            self.members.len(),
            self.units,
            total(&mut self.members.iter().map(|m| &m.took)),
            self.reruns.len(),
            total(&mut self.reruns.iter().map(|m| &m.took)),
        )];
        out.extend(self.numbers_lines());
        if self.singles_kept > 0 {
            out.push(format!(
                "carry chain: {} members ran with the SingleInstance state of the members before them (ADR-066 amendment 8); {} of them read first a global an earlier member wrote",
                self.singles_kept, self.singles_read_carried
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
        if let Some(line) = by_reason("serial carry chain by reason", &self.members, ChainWhy::text) {
            out.push(line);
        }
        if let Some(line) = by_reason("serial re-runs by reason", &self.reruns, RerunWhy::text) {
            out.push(line);
        }
        out.extend(self.single_lane.lines());
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
/// on the TestRunnerHub (`corpus.rs` `on_hub`). Under BcContainerHelper the run's OnRun rows are rolled back
/// (`TestDriver::run_onrun_carries`): only an OnRun that can change a number sequence or isolated storage, and tests that
/// take numbers, put the codeunit in the chain (`discovery::run_onrun_left`).
pub(super) fn carry_why(
    engine: &Arc<al_vm::Engine>,
    lowered: &Lowered,
    oi: usize,
    driver: super::TestDriver,
) -> Option<ChainWhy> {
    let onrun =
        onrun_writes(engine, lowered, oi).filter(|w| driver.run_onrun_carries() || *w == ChainWhy::OnRunWritesState);
    match onrun {
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

/// Whether a chain member with isolation `isolation` and carry `carry` is in the chain only for the numbers its tests
/// take ([`ChainWhy::TestsTakeNumbers`]). ADR-066 amendment 11, option 1: such a member runs with fresh single instances
/// in the walk, and its SingleInstance order dependence is handled as for codeunits outside the chain
/// (amendments 4, 5 and 7). A member with Disabled isolation or an OnRun that writes keeps amendment 8.
pub(super) fn numbers_only(isolation: TestIsolation, carry: Option<ChainWhy>) -> bool {
    isolation != TestIsolation::Disabled && carry == Some(ChainWhy::TestsTakeNumbers)
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
        }
    }

    #[test]
    fn a_long_numbers_list_is_one_summary_line_in_the_run_output() {
        let order = |n: usize| ChainReport {
            numbers_order: (0..n as u32)
                .map(|i| super::super::NumbersOrder {
                    suite: 0,
                    codeunit: i,
                    name: format!("C{i}"),
                    after: (i + 1, "X".into()),
                })
                .collect(),
            ..Default::default()
        };
        assert_eq!(order(NUMBERS_LINES_MAX).run_output_lines().len(), NUMBERS_LINES_MAX);
        let long = order(NUMBERS_LINES_MAX + 1);
        let out = long.run_output_lines();
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(out[0].contains(&format!("number order: {} test codeunits", NUMBERS_LINES_MAX + 1)));
        // `--timings` keeps the detail.
        assert_eq!(long.numbers_lines().len(), NUMBERS_LINES_MAX + 1);
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
                "carry chain: 3 of 10 test codeunits are in it, all ran serially (2.50 s); 1 ran again serially after the parallel part (0.25 s)",
                "carry chain: 1 of 3 left state for the later codeunits: 2 C2",
                "serial carry chain by reason: 2 tests can take NumberSequence numbers (2.00 s); 1 OnRun can write a table (0.50 s)",
                "serial re-runs by reason: 1 read a table a chain codeunit wrote (0.25 s)",
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
                "carry chain: 0 of 4 test codeunits are in it, all ran serially (0.00 s); 0 ran again serially after the parallel part (0.00 s)"
            ]
        );
    }
}
