//! What the default isolated run says about BC's order instead of following it (ADR-070, decisions 3 and 4).
//!
//! By default every test codeunit starts with fresh SingleInstance state and from the install baseline's
//! NumberSequence values, and nothing runs again in BC's order (`RunOptions::strict_order` runs it). The reports say
//! where the result may differ from BC's, from what the parallel run noted (`single_lane.rs` `Lane::run`, per BC
//! session of `streams`):
//!
//! * [`OrderHint`] on a failed test: the SingleInstance globals it read first that an earlier test codeunit of its BC
//!   session would have left with another value (`hints_for`, from `Found::changed`, `Found::last` and
//!   `SingleOut::read_in`), and the number sequences it took numbers from that an earlier codeunit of its session also
//!   took from in this run (decision 3, `numbers_hints`, from `Scratch::numbers_taken`, noted by [`note_taken`]). Runner
//!   130450 does not give taken numbers back (test-framework/onrun-storage/Y35OnRunStorage.al:30-31), so on BC the test
//!   took later numbers.
//! * The same reads on a passing test (decision 4): the JUnit property `singleInstanceCarried` and one summary line
//!   (`ChainReport::run_output_lines`).
//!
//! Under `--strict-order` the numbers chain as on BC, except under TestIsolation = Function, where what a later codeunit
//! sees of them is not measured: [`mark_unmeasured_numbers`] marks every later codeunit (`Unit::numbers_unmeasured`) so
//! that each NumberSequence call of its session is refused (al-vm `storage_checkpoint.rs`; al-runner issue 8152).
//! Front end: `alrunner-cli` `output.rs` (JUnit `orderHint` and `singleInstanceCarried`, JSON `orderHint`, the
//! `order hint:` line under a failure).

use super::single_fresh::CarriedRead;
use al_vm::single_track::GlobalKey;
use std::collections::{BTreeMap, BTreeSet};

/// The flag that runs the test codeunits in BC's order.
pub const STRICT_FLAG: &str = "--strict-order";

/// Why a test's result may differ in BC's order: the SingleInstance globals it read first that an earlier test codeunit
/// would have left with another value, and the number sequences it took numbers from after an earlier codeunit took some.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OrderHint {
    pub reads: Vec<CarriedRead>,
    pub numbers: Vec<NumbersTaken>,
}

/// One test's numbers taken after an earlier test codeunit (codeunit id, test, numbers): `hint_numbers` writes one per
/// test that took any, whatever its outcome (decision 7).
pub type TakenAfter = (u32, String, Vec<NumbersTaken>);

/// A number sequence a failed test took numbers from that an earlier test codeunit of its BC session also took from
/// (decision 3).
#[derive(Debug, Clone, PartialEq)]
pub struct NumbersTaken {
    /// The sequence's name.
    pub sequence: String,
    /// The last earlier test codeunit that took numbers from it (id, name).
    pub after: (u32, String),
}

impl NumbersTaken {
    /// `took numbers from sequence S after test codeunit N "Name"`: the core of the console hint and of the cause of a
    /// stability flip (`alrunner-cli` `stability_si.rs`).
    pub fn text(&self) -> String {
        format!(
            "took numbers from sequence {} after test codeunit {} \"{}\"",
            self.sequence, self.after.0, self.after.1
        )
    }
}

impl OrderHint {
    /// The text the console, the JUnit property `orderHint`, JSON and `serve` show.
    pub fn text(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if !self.reads.is_empty() {
            parts.push(format!(
                "this test read {}; each test codeunit starts with fresh SingleInstance state here",
                self.reads_text()
            ));
        }
        for n in &self.numbers {
            parts.push(format!(
                "this test {} took some on BC; each test codeunit starts from the install baseline's NumberSequence \
                 values here",
                n.text()
            ));
        }
        parts.push(format!("{STRICT_FLAG} runs the test codeunits in BC's order"));
        parts.join("; ")
    }

    /// The SingleInstance globals read, each with its codeunit, app and the codeunit that sets it on BC: the value of
    /// the JUnit property `singleInstanceCarried` of a passing test (decision 4).
    pub fn reads_text(&self) -> String {
        let reads: Vec<String> = self
            .reads
            .iter()
            .map(|r| {
                let writer = match &r.last_writer {
                    Some((id, name)) => format!("test codeunit {id} \"{name}\""),
                    None => "an earlier test codeunit".to_string(),
                };
                format!(
                    "global {} of SingleInstance codeunit {} \"{}\" (app {} by {}), which {writer} sets before this test on BC",
                    r.global, r.single_id, r.single_name, r.app, r.publisher
                )
            })
            .collect();
        reads.join("; ")
    }
}

/// Moves the number sequences the session took numbers from since the last call into `Scratch::numbers_taken`, under
/// `test` (`""`: OnRun and the runner steps outside a test). Decision 3.
pub(super) fn note_taken(scratch: &mut super::Scratch, session: &mut al_vm::Session, test: &str) {
    let taken = session.take_numbers_taken();
    if !taken.is_empty() {
        scratch.numbers_taken.push((test.to_string(), taken));
    }
}

/// Decision 3: the numbers each of `tests` of a codeunit took (`taken`, per test) from a sequence that an earlier codeunit
/// of its BC session took from (`before`: per sequence, the unit index of the last earlier codeunit that took from it
/// and whose numbers did not reach this codeunit, [`hint_numbers`]).
pub(super) fn numbers_hints(
    lowered: &al_sema::lower::Lowered,
    units: &[super::Unit],
    taken: &[(String, BTreeSet<(String, String)>)],
    before: &BTreeMap<(String, String), usize>,
    tests: &[String],
) -> Vec<(String, Vec<NumbersTaken>)> {
    let name = |k: usize| (units[k].id, lowered.program.objects[units[k].oi].name.clone());
    tests
        .iter()
        .filter_map(|test| {
            let numbers: Vec<NumbersTaken> = taken
                .iter()
                .filter(|(t, _)| t == test)
                .flat_map(|(_, keys)| keys)
                .filter_map(|key| {
                    before.get(key).map(|&w| NumbersTaken {
                        sequence: key.1.clone(),
                        after: name(w),
                    })
                })
                .collect();
            (!numbers.is_empty()).then(|| (test.clone(), numbers))
        })
        .collect()
}

/// The hint of each failed test of a dependent codeunit: `changed` its globals read first that the earlier codeunits
/// left otherwise, `writers` the last writer of each (as unit index), `read_in` the test each global was first read in
/// (0: before the first test), `failed` each failed or unfinished test with the number `Session::single_test_begins`
/// gave it (`None`: not numbered, it counts as the last test). A test gets the globals read first in it or in a test
/// before it (`single_lane.rs` `failure_saw_change`); a changed global without a note counts for every test.
pub(super) fn hints_for(
    lowered: &al_sema::lower::Lowered,
    units: &[super::Unit],
    changed: &BTreeSet<GlobalKey>,
    writers: &[(GlobalKey, usize)],
    read_in: Option<&BTreeMap<GlobalKey, u32>>,
    failed: &[(String, Option<u32>)],
) -> Vec<(String, OrderHint)> {
    failed
        .iter()
        .filter_map(|(test, n)| {
            let bound = n.unwrap_or(u32::MAX);
            let seen: Vec<(GlobalKey, usize)> = writers
                .iter()
                .filter(|(g, _)| changed.contains(g))
                .filter(|(g, _)| read_in.and_then(|r| r.get(g)).is_none_or(|&t| t <= bound))
                .copied()
                .collect();
            (!seen.is_empty()).then(|| {
                let reads = super::single_fresh::name_reads(lowered, units, &seen);
                (
                    test.clone(),
                    OrderHint {
                        reads,
                        ..Default::default()
                    },
                )
            })
        })
        .collect()
}

/// Decision 3: by default (`strict` false) a codeunit that is in the carry chain only for the numbers its tests take
/// (`ChainWhy::TestsTakeNumbers`) leaves it and starts from the install baseline's sequence values. Under
/// `--strict-order` it stays in the chain.
pub(super) fn numbers_unchained(units: &mut [super::Unit], strict: bool) {
    for u in units.iter_mut().filter(|_| !strict) {
        if u.carry == Some(super::chain::ChainWhy::TestsTakeNumbers) {
            u.carry = None;
        }
    }
}

/// al-runner issue 8152: under `--strict-order` with TestIsolation = Function, every NumberSequence call of a test
/// codeunit after a codeunit of its BC session that is in the carry chain for the numbers its tests take
/// (`ChainWhy::TestsTakeNumbers`) is refused, from any code of its session (`Unit::numbers_unmeasured`, al-vm
/// `storage_checkpoint.rs`). Only runner 130450's Codeunit isolation is measured to leave the taken numbers to the later
/// codeunits (Y35OnRunStorage.al:30-31); under Function BC's answer is not measured. By default (no `--strict-order`)
/// nothing is marked: each codeunit starts from the install baseline's sequence values.
pub(super) fn mark_unmeasured_numbers(
    lowered: &al_sema::lower::Lowered,
    units: &mut [super::Unit],
    place: &dyn Fn(usize, u32) -> u32,
    opts: &super::RunOptions,
) {
    if !opts.strict_order {
        return;
    }
    let function_taker = |u: &super::Unit| {
        u.isolation == super::TestIsolation::Function && u.carry == Some(super::chain::ChainWhy::TestsTakeNumbers)
    };
    let objs = &lowered.program.objects;
    let mut marks: Vec<(usize, std::sync::Arc<str>)> = Vec::new();
    for ks in super::single_lane::streams(units, place, &|_| true, opts.test_suites.as_deref()) {
        let mut last: Option<usize> = None;
        for k in ks {
            if let Some(w) = last {
                let why = format!(
                    "the tests of an earlier test codeunit, {} \"{}\", can take NumberSequence numbers: what a later \
                     codeunit sees of them under TestIsolation = Function is not measured (al-runner issue 8152)",
                    units[w].id, objs[units[w].oi].name
                );
                marks.push((k, why.into()));
            }
            if function_taker(&units[k]) {
                last = Some(k);
            }
        }
    }
    for (k, why) in marks {
        units[k].numbers_unmeasured.get_or_insert(why);
    }
}

/// Whether the numbers a codeunit took that left the carry at place `at` of the default walk reached a run that started
/// from the carry at place `from` (`Scratch::carry_at`, `Scratch::carry_from`): the walk's carries of a suite build on
/// each other, so a run started from a carry sees every carry before it.
fn reached(at: Option<usize>, from: Option<usize>) -> bool {
    matches!((at, from), (Some(a), Some(f)) if a <= f)
}

/// Decision 3: the numbers each test of BC session `ks` (unit indices in BC's order) took from a sequence that an earlier
/// codeunit of the session took from, when that codeunit's numbers did not reach it here ([`reached`]; a codeunit outside
/// the carry chain never hands them on). Every test, whatever its outcome, goes into `taken_after` (codeunit id, test,
/// numbers: the cause of a stability flip, decision 7, never printed); each failed test also gets the numbers hint
/// ([`numbers_hints`]), added to its SingleInstance hint when it has one. A test of a codeunit in two sessions
/// (`--test-suites`) is hinted in the first session that names an earlier taker for it only (`numbered`, per unit index
/// and test, so another test of the codeunit keeps its own first session). Returns how many failed tests got a hint they
/// did not have.
pub(super) fn hint_numbers(
    lowered: &al_sema::lower::Lowered,
    units: &[super::Unit],
    ks: &[usize],
    numbered: &mut BTreeSet<(usize, String)>,
    slots: &mut [Option<super::UnitResult>],
    taken_after: &mut Vec<TakenAfter>,
) -> usize {
    // Per sequence, each earlier codeunit that took from it, in order: (unit index, `Scratch::carry_at`).
    type Takers = Vec<(usize, Option<usize>)>;
    let mut before: BTreeMap<(String, String), Takers> = BTreeMap::new();
    let mut n = 0;
    for &k in ks {
        let Some(Ok(done)) = slots[k].as_mut().map(|r| &mut r.result) else {
            continue;
        };
        if !before.is_empty() {
            let from = done.5.carry_from;
            let unseen: BTreeMap<(String, String), usize> = before
                .iter()
                .filter_map(|(key, ws)| {
                    let w = ws.iter().rev().find(|(_, at)| !reached(*at, from))?;
                    Some((key.clone(), w.0))
                })
                .collect();
            let tests: Vec<String> = done
                .0
                .iter()
                .map(|(t, _)| t.clone())
                .filter(|t| !numbered.contains(&(k, t.clone())))
                .collect();
            let failed: BTreeSet<String> = done
                .0
                .iter()
                .filter(|(_, o)| matches!(o, crate::Outcome::Fail(_)))
                .map(|(t, _)| t.clone())
                .collect();
            let hints = numbers_hints(lowered, units, &done.5.numbers_taken, &unseen, &tests);
            for (test, numbers) in hints {
                numbered.insert((k, test.clone()));
                taken_after.push((units[k].id, test.clone(), numbers.clone()));
                if !failed.contains(&test) {
                    continue;
                }
                match done.5.order_hints.iter_mut().find(|(t, _)| *t == test) {
                    Some((_, h)) => h.numbers = numbers,
                    None => {
                        done.5.order_hints.push((
                            test,
                            OrderHint {
                                numbers,
                                ..Default::default()
                            },
                        ));
                        n += 1;
                    }
                }
            }
        }
        for (_, keys) in &done.5.numbers_taken {
            for key in keys {
                before.entry(key.clone()).or_default().push((k, done.5.carry_at));
            }
        }
    }
    n
}

impl super::UnitResult {
    /// The default walk's places of the carry this run started from and of the carry it left (`Scratch::carry_from`,
    /// `Scratch::carry_at`): set by the walk for a member and by the run again from a carry (`corpus.rs`).
    pub(super) fn note_carry(&mut self, from: Option<usize>, at: Option<usize>) {
        if let Ok(done) = &mut self.result {
            done.5.carry_from = from;
            done.5.carry_at = at;
        }
    }
}

impl super::CorpusRuntime {
    /// ADR-070 decision 7: per test (codeunit id, name), the numbers it took after an earlier codeunit of its BC session
    /// took from the same sequence in this default run, whatever its outcome ([`hint_numbers`]). Empty under
    /// `--strict-order`.
    pub fn numbers_taken_after(&self) -> &[TakenAfter] {
        &self.phases.chain.single_lane.numbers_taken_after
    }
}
