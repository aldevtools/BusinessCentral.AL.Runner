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
//! * [`NumbersOrder`] (decision 3): a test codeunit whose tests can take NumberSequence numbers and that comes after
//!   another such codeunit in BC's order: on BC it would take the numbers after the ones the earlier codeunit took
//!   (runner 130450 does not give them back: test-framework/onrun-storage/Y35OnRunStorage.al:30-31). A report line and
//!   a JUnit property, not a failure.

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

/// A test codeunit whose tests can take NumberSequence numbers after an earlier codeunit of its BC session that can
/// take some too (decision 3).
#[derive(Debug, Clone, PartialEq)]
pub struct NumbersOrder {
    /// The index of the run (`TestResult::suite`) and the codeunit.
    pub suite: usize,
    pub codeunit: u32,
    pub name: String,
    /// The last codeunit before it in BC's order that can take numbers (id, name).
    pub after: (u32, String),
}

impl NumbersOrder {
    /// The text of the report line and of the JUnit property `numberSequenceOrder`.
    pub fn text(&self) -> String {
        format!(
            "its tests can take NumberSequence numbers and start from the install baseline's values here; on BC they take them after test codeunit {} \"{}\"; {STRICT_FLAG} chains the numbers as on BC",
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
/// (`takes(k)`) after an earlier one that can.
pub(super) fn numbers_order(
    lowered: &al_sema::lower::Lowered,
    units: &[super::Unit],
    sessions: &[Vec<usize>],
    takes: &dyn Fn(usize) -> bool,
) -> Vec<NumbersOrder> {
    let name = |k: usize| lowered.program.objects[units[k].oi].name.clone();
    let mut out = Vec::new();
    for ks in sessions {
        let mut last: Option<usize> = None;
        for &k in ks.iter().filter(|&&k| takes(k)) {
            if let Some(w) = last {
                out.push(NumbersOrder {
                    suite: units[k].si,
                    codeunit: units[k].id,
                    name: name(k),
                    after: (units[w].id, name(w)),
                });
            }
            last = Some(k);
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

/// Decision 3: the numbers list of a default run (none under `--strict-order`), per BC session (`single_lane.rs`
/// `streams`: the whole run, or each suite of `--test-suites`).
pub(super) fn note_numbers_order(
    lowered: &al_sema::lower::Lowered,
    units: &[super::Unit],
    place: &dyn Fn(usize, u32) -> u32,
    opts: &super::RunOptions,
    report: &mut super::ChainReport,
) {
    if opts.strict_order {
        return;
    }
    let sessions = super::single_lane::streams(units, place, &|_| true, opts.test_suites.as_deref());
    report
        .numbers_order
        .extend(numbers_order(lowered, units, &sessions, &|k| units[k].takes_numbers));
}
