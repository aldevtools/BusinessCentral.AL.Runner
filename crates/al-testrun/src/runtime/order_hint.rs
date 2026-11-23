//! ADR-066 amendment 12 (owner, 2026-11-09): what the default run says about BC's order instead of following it.
//!
//! By default every test codeunit starts with fresh SingleInstance state and from the install baseline's
//! NumberSequence values, and nothing runs again in BC's order (`RunOptions::strict_order` restores that). Two reports
//! say where the result may differ from BC's:
//!
//! * [`OrderHint`] (decision 2, owner answer 1): a failed test that read first a SingleInstance global an earlier test
//!   codeunit of its BC session would have left with another value. It names the SingleInstance codeunit, its app, the
//!   global, the codeunit that would have written it and the flag. From the lane's analysis (`single_lane.rs`
//!   `Found::changed`, `Found::last`) and the test each global was first read in (`SingleOut::read_in`, ADR-066
//!   amendment 7).
//! * [`NumbersOrder`] (decision 3): a test codeunit whose tests can take NumberSequence numbers, or that only reads
//!   them, and that comes after a codeunit whose tests can take some in BC's order: on BC it would see the numbers after
//!   the ones the earlier codeunit took (runner 130450 does not give them back:
//!   test-framework/onrun-storage/Y35OnRunStorage.al:30-31). A report line and a JUnit property, not a failure.
//!
//! Under `--strict-order` the numbers chain as on BC, except under TestIsolation = Function, where what a later codeunit
//! sees of them is not measured: [`mark_unmeasured_numbers`] refuses every NumberSequence call of the later codeunits
//! (al-runner issue 8152).
//!
//! ADR-066 amendment 12, the default run (no `--strict-order`): every test codeunit starts with fresh
//! SingleInstance state and the install baseline's NumberSequence values (`corpus.rs` drops
//! `ChainWhy::TestsTakeNumbers` from `Unit::carry` and notes `Unit::takes_numbers`; the walk keeps no
//! single instances), and `single_lane.rs` `Lane::run` with `Lane::bc_order` false flags and re-runs
//! nothing. `OrderHint` (`hints_for`, called by `Lane::hint`): a failed test that read first a global
//! of `Found::changed` gets the SingleInstance codeunit, app, global and last writer
//! (`TestResult::order_hint`, through `Scratch::order_hints`); `NumbersOrder` (`numbers_order`):
//! codeunits that can take numbers, or only read them, after another one that can take some in their
//! BC session (`ChainReport::numbers_order`, `numbers_lines`, `numbers_text`); under `--strict-order`
//! with TestIsolation = Function `mark_unmeasured_numbers` marks every later codeunit
//! (`Unit::numbers_unmeasured`) so that each NumberSequence call of its session is refused (al-vm
//! `storage_checkpoint.rs`; al-runner issue 8152). Front end: `alrunner-cli` `output.rs`
//! `order_properties` (JUnit `orderHint`, `numberSequenceOrder`), `json_tdd` (`orderHint`), `run.rs`
//! (the `order hint:` and `number order:` lines). `RunOptions::strict_order` turns all of it off and
//! the amendments 4 to 11 on; `RunOptions::replay_every_dependent` (stability base variant with
//! `isolation.singleInstance`) replays every dependent codeunit.

use super::single_fresh::CarriedRead;
use al_vm::single_track::GlobalKey;
use std::collections::{BTreeMap, BTreeSet};

/// The flag that runs the test codeunits in BC's order.
pub const STRICT_FLAG: &str = "--strict-order";

/// Why a failed test may have passed in BC's order: the SingleInstance globals it read first that an earlier test
/// codeunit would have left with another value.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderHint {
    pub reads: Vec<CarriedRead>,
}

impl OrderHint {
    /// The text the console, the JUnit property `orderHint`, JSON and `serve` show.
    pub fn text(&self) -> String {
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
        format!(
            "this test read {}; each test codeunit starts with fresh SingleInstance state here; {STRICT_FLAG} runs the test codeunits in BC's order",
            reads.join("; ")
        )
    }
}

/// A test codeunit whose tests can take NumberSequence numbers, or that only reads them, after an earlier codeunit of
/// its BC session whose tests can take some (decision 3).
#[derive(Debug, Clone, PartialEq)]
pub struct NumbersOrder {
    /// The index of the run (`TestResult::suite`) and the codeunit.
    pub suite: usize,
    pub codeunit: u32,
    pub name: String,
    /// The last codeunit before it in BC's order that can take numbers (id, name).
    pub after: (u32, String),
    /// Its tests and OnRun trigger only read the numbers (`NumberSequence.Current` or `Exists`), they take none.
    pub reads_only: bool,
}

impl NumbersOrder {
    /// The text of the report line and of the JUnit property `numberSequenceOrder`.
    pub fn text(&self) -> String {
        let (what, verb) = match self.reads_only {
            true => ("only read NumberSequence values", "read them"),
            false => ("can take NumberSequence numbers", "take them"),
        };
        format!(
            "its tests {what} and start from the install baseline's values here; on BC they {verb} after test codeunit {} \"{}\" took some; {STRICT_FLAG} chains the numbers as on BC",
            self.after.0, self.after.1
        )
    }

    /// The report line.
    pub fn line(&self) -> String {
        format!(
            "number order: test codeunit {} \"{}\": {}",
            self.codeunit,
            self.name,
            self.text()
        )
    }
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
                (test.clone(), OrderHint { reads })
            })
        })
        .collect()
}

/// Decision 3: per BC session (`sessions`, unit indices in BC's order), each codeunit that can take numbers
/// (`takes(k)`) or only reads them (`reads(k)`, asked only of a codeunit that takes none) after an earlier one that can
/// take some.
pub(super) fn numbers_order(
    lowered: &al_sema::lower::Lowered,
    units: &[super::Unit],
    sessions: &[Vec<usize>],
    takes: &dyn Fn(usize) -> bool,
    reads: &dyn Fn(usize) -> bool,
) -> Vec<NumbersOrder> {
    let name = |k: usize| lowered.program.objects[units[k].oi].name.clone();
    let mut out = Vec::new();
    for ks in sessions {
        let mut last: Option<usize> = None;
        for &k in ks {
            let took = takes(k);
            if let Some(w) = last.filter(|_| took || reads(k)) {
                out.push(NumbersOrder {
                    suite: units[k].si,
                    codeunit: units[k].id,
                    name: name(k),
                    after: (units[w].id, name(w)),
                    reads_only: !took,
                });
            }
            if took {
                last = Some(k);
            }
        }
    }
    out
}

/// Decision 3: by default (`strict` false) a codeunit that is in the carry chain only for the numbers its tests take
/// (`ChainWhy::TestsTakeNumbers`) leaves it and starts from the install baseline's sequence values; `Unit::takes_numbers`
/// notes it for the numbers list. Under `--strict-order` it stays in the chain.
pub(super) fn numbers_unchained(units: &mut [super::Unit], strict: bool) {
    for u in units.iter_mut().filter(|_| !strict) {
        if u.carry == Some(super::chain::ChainWhy::TestsTakeNumbers) {
            u.carry = None;
            u.takes_numbers = true;
        }
    }
}

/// Decision 3: the numbers list of a default run, per BC session (`single_lane.rs` `streams`: the whole run, or each
/// suite of `--test-suites`). Under `--strict-order` there is no list; [`mark_unmeasured_numbers`] marks the units instead.
pub(super) fn note_numbers_order(
    engine: &std::sync::Arc<al_vm::Engine>,
    lowered: &al_sema::lower::Lowered,
    units: &mut [super::Unit],
    place: &dyn Fn(usize, u32) -> u32,
    opts: &super::RunOptions,
    report: &mut super::ChainReport,
) {
    if opts.strict_order {
        return mark_unmeasured_numbers(lowered, units, place, opts);
    }
    let units = &*units;
    let sessions = super::single_lane::streams(units, place, &|_| true, opts.test_suites.as_deref());
    report.numbers_order.extend(numbers_order(
        lowered,
        units,
        &sessions,
        &|k| units[k].takes_numbers,
        &|k| super::chain::reads_numbers(engine, lowered, units[k].oi),
    ));
}

/// al-runner issue 8152: under `--strict-order` with TestIsolation = Function, every NumberSequence call of a test
/// codeunit after a codeunit of its BC session that is in the carry chain for the numbers its tests take
/// (`ChainWhy::TestsTakeNumbers`) is refused, from any code of its session (`Unit::numbers_unmeasured`, al-vm
/// `storage_checkpoint.rs`). Only runner 130450's Codeunit isolation is measured to leave the taken numbers to the later
/// codeunits (Y35OnRunStorage.al:30-31); under Function BC's answer is not measured.
fn mark_unmeasured_numbers(
    lowered: &al_sema::lower::Lowered,
    units: &mut [super::Unit],
    place: &dyn Fn(usize, u32) -> u32,
    opts: &super::RunOptions,
) {
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
