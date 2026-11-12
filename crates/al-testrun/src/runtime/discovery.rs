//! The test driver (ADR-074, owner decision 2026-11-12, option 1) and BcContainerHelper's discovery pass.
//!
//! Two drivers start BC's tests, and the corpus pins both:
//! - [`TestDriver::BcLinux`], the corpus Linux legs (bc-linux's runner): OnRun runs once per test codeunit, a codeunit
//!   without handlers runs on the TestRunnerHub on BC 28 and later, and under runner 130450 what OnRun writes reaches the
//!   later codeunits (codeunits/run-onrun/RqIsolationModes.al:300-308, :357-369 `#else`). The default.
//! - [`TestDriver::BcContainerHelper`], `Run-TestsInBcContainer` as AL-Go runs tests (page 130455 in one client session,
//!   measured by the corpus Windows nightly run 43058375098, BC 28.4, and on BC 27.5, 28.5 and 29.0): there is no hub; a
//!   discovery pass first runs the OnRun trigger of every test codeunit of the range in the test session, without test
//!   isolation, and what it writes and the SingleInstance state it changes stay into the run
//!   (tests/al-language-isolation-probe/src/SwOnRunRunner.al:19-23, codeunit 97013 and the reader
//!   `OnRunWrites_DiscoveryRowAndRollback` :282-286). The run then calls OnRun again inside the runner's isolation: its
//!   rows are rolled back after the codeunit (:21-23). A codeunit whose OnRun raised in the discovery pass gets no method
//!   rows; the run still calls its OnRun and runs no method (:24-26, codeunit 97015 and the reader
//!   `OnRunRaisesInDiscovery_Rows` :289-294).
//!
//! This module owns every difference between the two: the hub ([`TestDriver::has_hub`]), whether the run's OnRun carries
//! ([`TestDriver::run_onrun_carries`]), the pass ([`run_pass`], a link of the carry chain at the front of each BC
//! session, `chain_walk.rs`) and what is refused because no probe pins it ([`Plan::refusal`] before the run,
//! [`refuse_after_pass`] after it).

use super::blocked::tables_touched;
use super::chain_walk::Link;
use super::run::Carried;
use super::written::{ReachFacts, ReachIndex, reach_facts};
use super::*;

/// Which driver's behavior the run models (ADR-074, `al-runner test --test-driver`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TestDriver {
    /// The corpus Linux legs (bc-linux's runner): OnRun once, the TestRunnerHub on BC 28 and later.
    #[default]
    BcLinux,
    /// BcContainerHelper and AL-Go: the discovery pass of page 130455, no TestRunnerHub.
    BcContainerHelper,
}

impl TestDriver {
    /// The values `--test-driver` takes, for messages.
    pub const VALUES: &'static str = "bc-linux or bccontainerhelper";

    /// The driver of a `--test-driver` value; `None` for an unknown one.
    pub fn parse(v: &str) -> Option<TestDriver> {
        match v.trim().to_ascii_lowercase().as_str() {
            "bc-linux" => Some(TestDriver::BcLinux),
            "bccontainerhelper" => Some(TestDriver::BcContainerHelper),
            _ => None,
        }
    }

    /// The value `--test-driver` takes for this driver.
    pub fn name(self) -> &'static str {
        match self {
            TestDriver::BcLinux => "bc-linux",
            TestDriver::BcContainerHelper => "bccontainerhelper",
        }
    }

    /// Whether a test codeunit can run on the TestRunnerHub (`corpus.rs` `on_hub`). BcContainerHelper runs every test
    /// codeunit through the runner codeunit (`-testRunnerCodeunitId`).
    pub(super) fn has_hub(self) -> bool {
        self == TestDriver::BcLinux
    }

    /// Whether the run models one BC session for the discovery pass and every test codeunit, so that the SingleInstance
    /// state the pass and each codeunit leave carries to the next codeunit: the front ends then run in BC's order
    /// (`RunOptions::strict_order` and `strict_sequential`). BcContainerHelper's page 130455 runs the pass and the run in
    /// the one client session of `Run-TestsInBcContainer`, and the reader sees the OnRun calls of the pass and of the run
    /// in a later codeunit (SwOnRunRunner.al:18-20, the reader `OnRunRaisesInDiscovery_Rows` :289-294). Without it every
    /// test codeunit would start with fresh SingleInstance state, and a test that reads one the pass changed (Microsoft's
    /// own caches included) would be refused (`refuse_after_pass`).
    pub fn one_session(self) -> bool {
        self == TestDriver::BcContainerHelper
    }

    /// Whether each test codeunit measures the tables it reads (`Scratch::read_tables`): an earlier codeunit whose static
    /// reach cannot name its tables (a `RecordRef`, a part not known) is checked against what the discovery pass wrote
    /// by what it read (`unpinned_reach`).
    pub(super) fn watches_reads(self) -> bool {
        self == TestDriver::BcContainerHelper
    }

    /// Whether what the run's call of OnRun writes reaches the later codeunits under runner 130450 (`chain.rs`
    /// `carry_why`, `Scratch::onrun_carry`). Under BcContainerHelper the run's OnRun runs inside the runner's isolation
    /// and its rows are rolled back after the codeunit (SwOnRunRunner.al:21-23).
    pub(super) fn run_onrun_carries(self) -> bool {
        self == TestDriver::BcLinux
    }
}

/// The driver of the test codeunits of `suite`: the suite's own (the corpus harness, from its oracle's format), else the
/// run's.
pub(super) fn driver_of(suite: &SuiteRun, opts: &RunOptions) -> TestDriver {
    suite.driver.unwrap_or(opts.test_driver)
}

/// Codeunit 130452 "Test Runner - Get Methods", which runs each test codeunit in the discovery pass and raises
/// `OnGetTestMethods`, `OnBeforeAddTestMethodLine` and `OnAfterAddTestMethodLine` per method.
const GET_METHODS: u32 = 130_452;

/// The discovery pass of one BC session (a suite) under BcContainerHelper.
pub(super) struct Plan {
    si: usize,
    /// The selected test codeunits with an OnRun trigger, in id order: (object, id, routine, reach of the OnRun).
    onruns: Vec<(usize, u32, usize, ReachFacts)>,
    /// Why no test codeunit of the session runs.
    all: Option<String>,
    /// `RunOptions::strict_order`: the single instances move from codeunit to codeunit.
    strict: bool,
}

/// The discovery pass a chain link runs (`Link::Discovery`): `unit` lends the session's settings, `objects` are the
/// test codeunits whose OnRun the pass runs, in id order.
pub(super) struct Pass {
    pub unit: usize,
    pub objects: Arc<Vec<usize>>,
}

impl Plan {
    /// The pass of suite `si`; `None` when the suite's driver is bc-linux or the pass can do nothing (no selected test
    /// codeunit has an OnRun trigger, nothing subscribes to the events of codeunit 130452, and the runner is one the pass
    /// was measured under).
    pub(super) fn new(
        engine: &Arc<al_vm::Engine>,
        lowered: &Lowered,
        si: usize,
        suite: &SuiteRun,
        opts: &RunOptions,
        only: &Option<Vec<(usize, u32)>>,
    ) -> Option<Plan> {
        if opts.execute || driver_of(suite, opts) != TestDriver::BcContainerHelper {
            return None;
        }
        let heads = lowered.program.objects.heads();
        let selected: Vec<usize> = (0..heads.len())
            .filter(|&oi| is_selected_test(lowered, oi, suite, only))
            .collect();
        let index = engine.derived(|| ReachIndex::new(&engine.events, lowered));
        let mut onruns: Vec<(usize, u32, usize, ReachFacts)> = selected
            .iter()
            .filter_map(|&oi| {
                let r = on_run_of(&lowered.program.objects[oi])?;
                // The reach serves only the check of single instances under --strict-order (`Plan::refusal`).
                let facts = match opts.strict_order {
                    true => reach_facts(&engine.events, lowered, &index, oi, r),
                    false => ReachFacts::default(),
                };
                Some((oi, heads[oi].id?, r, facts))
            })
            .collect();
        onruns.sort_by_key(|o| o.1);
        let subscribed = heads
            .iter()
            .position(|h| h.is_kind(al_ir::ObjKind::Codeunit) && h.id == Some(GET_METHODS))
            .is_some_and(|g| foreign_subscriber(engine, heads, g));
        let unmeasured = unmeasured(suite, opts);
        if onruns.is_empty() && !subscribed && unmeasured.is_none() {
            return None;
        }
        let mut apps: Vec<u32> = selected.iter().map(|&oi| heads[oi].app).collect();
        apps.sort_unstable();
        apps.dedup();
        let all = if let Some(why) = unmeasured {
            Some(why)
        } else if subscribed {
            Some("BcContainerHelper's discovery pass raises the events of codeunit 130452 \"Test Runner - Get Methods\" for each test method, and a subscriber to them runs there: not pinned".to_string())
        } else if apps.len() > 1 {
            Some("BcContainerHelper's discovery pass over the test codeunits of several apps in one BC session: not pinned".to_string())
        } else if opts.test_suites.is_some() {
            Some("BcContainerHelper's discovery pass with test suites (--test-suites): which test codeunits each suite's discovery pass runs is not pinned".to_string())
        } else if opts.strict_order && !opts.strict_sequential {
            Some("BcContainerHelper's discovery pass under --strict-order needs each BC session run in one walk (strict_sequential)".to_string())
        } else {
            None
        };
        Some(Plan {
            si,
            onruns,
            all,
            strict: opts.strict_order,
        })
    }

    /// Why test codeunit `oi` (id `id`) of this session is not run under BcContainerHelper, decided before the run; `None`:
    /// it runs. What the pass changes in the database, number sequences and isolated storage is checked once it ran
    /// ([`refuse_after_pass`]).
    pub(super) fn refusal(&self, engine: &Arc<al_vm::Engine>, lowered: &Lowered, oi: usize, id: u32) -> Option<String> {
        if let Some(why) = &self.all {
            return Some(why.clone());
        }
        if !self.strict {
            return None;
        }
        let own = super::strict_taint::unit_facts(engine, lowered, oi);
        let objs = &lowered.program.objects;
        for (v, vid, _, f) in &self.onruns {
            let name = &objs[*v].name;
            // Under --strict-order the single instances the pass left reach the run.
            if *vid > id
                && let Some(&(o, g)) = f
                    .globals_written
                    .intersection(&own.globals_read)
                    .find(|(o, _)| single_instance(&objs[*o]))
            {
                return Some(format!(
                    "BcContainerHelper's discovery pass runs the OnRun trigger of the later codeunit {name} before this codeunit runs, and it can change the SingleInstance global {}.{} that this codeunit reads: whether this codeunit sees that is not pinned",
                    objs[o].name, objs[o].globals[g].name
                ));
            }
        }
        None
    }

    /// The chain link that runs the pass, at the front of the session (id 0 sorts first; `chain_walk.rs`); `None` when no
    /// unit of the session runs or no codeunit has an OnRun.
    pub(super) fn link(&self, units: &[Unit]) -> Option<(usize, u8, u32, Link)> {
        let unit = units.iter().position(|u| u.si == self.si)?;
        let objects: Vec<usize> = self.onruns.iter().map(|o| o.0).collect();
        (!objects.is_empty()).then(|| {
            (
                self.si,
                0,
                0,
                Link::Discovery(Arc::new(Pass {
                    unit,
                    objects: Arc::new(objects),
                })),
            )
        })
    }
}

/// Why the runner and isolation the test codeunits of `suite` run under are not what BcContainerHelper's discovery pass was
/// measured under; `None`: they are. Measured (an allowlist; everything else is refused):
/// - runner 130450 "Test Runner - Isol. Codeunit", TestIsolation = Codeunit (SwOnRunRunner.al:3-6, run with
///   `-testRunnerCodeunitId 130450`), which al-runner's own runner and Microsoft's runner run when no runner codeunit,
///   `--isolation` other than `codeunit` or `--isolation required` is asked for;
/// - the runner codeunit of a corpus harness suite whose oracle BcContainerHelper recorded through exactly that runner
///   (`SuiteRun::driver` from the oracle's format, `SuiteRun::runner` from its `runner`: the isolation suites, runners
///   61300 and 61301, Windows nightly run 43058375098), with no `--isolation` on top (`required_isolation` does not
///   reach a suite with a runner). The harness compares every case of such a suite with that oracle.
fn unmeasured(suite: &SuiteRun, opts: &RunOptions) -> Option<String> {
    use crate::isolation::TestIsolation;
    let recorded = suite.driver.is_some();
    let runner = suite.runner.filter(|_| !recorded).or(opts.test_runner.codeunit());
    let isolation = opts.isolation.filter(|i| recorded || *i != TestIsolation::Codeunit);
    let what = match (runner, isolation) {
        (Some(r), _) => format!("the runner codeunit {r}"),
        (None, Some(i)) => format!("TestIsolation = {i:?}"),
        // A recorded suite's runner codeunit decides its isolation; `required_isolation` only picks the TestRunnerHub of a
        // suite without one (`corpus.rs` `hub_path`), as the harness sets it for the main app on BC 28 and later.
        (None, None) if opts.required_isolation && !recorded => "--isolation required".to_string(),
        (None, None) => return None,
    };
    Some(format!(
        "BcContainerHelper's discovery pass is pinned only under runner 130450 \"Test Runner - Isol. Codeunit\" (TestIsolation = Codeunit) and under the runner a suite's oracle was recorded with: under {what} it is not pinned"
    ))
}

/// Whether an object outside the Test Runner app (the app of codeunit 130452, object `g`) subscribes to an event 130452
/// raises in the discovery pass. The Test Runner app's own subscribers are part of what the nightly measured.
fn foreign_subscriber(engine: &al_vm::Engine, heads: &[al_ir::ObjectHead], g: usize) -> bool {
    [
        "ongettestmethods",
        "onbeforeaddtestmethodline",
        "onafteraddtestmethodline",
    ]
    .iter()
    .filter_map(|e| engine.events.subscribers.get(&(g, (*e).to_string())))
    .flatten()
    .any(|s| heads[s.object].app != heads[g].app)
}

/// The OnRun trigger of `o`.
fn on_run_of(o: &al_ir::Object) -> Option<usize> {
    o.routines
        .iter()
        .position(|r| r.is_trigger && r.name.eq_ignore_ascii_case("OnRun"))
}

/// Whether `o` is a codeunit with `SingleInstance = true`.
fn single_instance(o: &al_ir::Object) -> bool {
    o.properties
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("SingleInstance") && v.trim().eq_ignore_ascii_case("true"))
}

/// What the last discovery pass on this thread found besides what it left in the database ([`run_pass`]).
#[derive(Default)]
pub(super) struct Found {
    /// The ids of the test codeunits whose OnRun raised.
    pub raised: Vec<u32>,
    /// Why every test of the session is refused: the pass changed session state the run does not carry.
    pub refused: Option<String>,
    /// The SingleInstance globals the pass left other than it found them.
    pub singles: BTreeSet<al_vm::single_track::GlobalKey>,
    /// Per test codeunit whose OnRun changed something: its id, the tables it changed and whether it changed a number
    /// sequence or isolated storage.
    pub wrote: Vec<(u32, BTreeSet<u32>, bool)>,
}

std::thread_local! {
    static FOUND: std::cell::RefCell<Found> = std::cell::RefCell::new(Found::default());
}

/// What the last discovery pass on this thread found.
pub(super) fn take_found() -> Found {
    FOUND.with(|r| std::mem::take(&mut *r.borrow_mut()))
}

/// Refuses the tests a discovery pass makes unknown (`ChainReport`): every test of a session whose pass changed session
/// state the run does not carry, and without `strict` (ADR-066 amendment 12: each test codeunit starts with fresh single
/// instances) every test of a codeunit whose reach reads a SingleInstance global the pass changed; on BC the run sees
/// what the pass left there (SwOnRunRunner.al:19-23, the reader expects two OnRun calls).
pub(super) fn refuse_after_pass(
    slots: &mut [Option<UnitResult>],
    report: &super::ChainReport,
    engine: &Arc<al_vm::Engine>,
    lowered: &Lowered,
    strict: bool,
) {
    let objs = &lowered.program.objects;
    for r in slots.iter_mut().flatten() {
        let (si, cu, oi) = (r.si, r.id, r.oi);
        let Ok(done) = &mut r.result else { continue };
        if let Some((_, why)) = report.discovery_refused.iter().find(|(s, _)| *s == si) {
            super::corpus::refuse_all(&mut done.0, why);
            continue;
        }
        if let Some(why) = unpinned_reach(report, (si, cu, oi), &done.5.read_tables, engine, lowered) {
            super::corpus::refuse_all(&mut done.0, &why);
            continue;
        }
        let changed: BTreeSet<_> = report
            .discovery_singles
            .iter()
            .filter(|(s, _)| *s == si)
            .map(|(_, g)| *g)
            .collect();
        if strict || changed.is_empty() {
            continue;
        }
        let reads = super::strict_taint::unit_facts(engine, lowered, oi).globals_read;
        if let Some(&(o, g)) = reads.intersection(&changed).next() {
            let why = format!(
                "BcContainerHelper's discovery pass changed the SingleInstance global {}.{} that this codeunit reads; without --strict-order every test codeunit starts with fresh SingleInstance state",
                objs[o].name, objs[o].globals[g].name
            );
            super::corpus::refuse_all(&mut done.0, &why);
        }
    }
}

/// Why codeunit `r` cannot be run after what the discovery pass of its session changed. Only the rows and the
/// SingleInstance state the pass leaves are pinned (SwOnRunRunner.al:19-26), and only for the codeunits the Test Runner
/// runs after the writer. So the codeunit is refused when the pass changed a number sequence or isolated storage (or the
/// session language or WorkDate) and its reach can read them, whichever codeunit's OnRun changed them (its own included);
/// and when the OnRun of a later codeunit wrote a table the codeunit can read: one its static reach touches, or, when its
/// reach opens tables through a RecordRef or is partly unknown, one its run used (`read`, measured by every record
/// operation, `Scratch::read_tables`; a part of the reach that is not compiled refuses when it runs, so a run that ended
/// read nothing through it). The Test Runner discovers the whole range before it runs the first codeunit (ADR-074); no
/// corpus test reads that order yet.
fn unpinned_reach(
    report: &super::ChainReport,
    (si, cu, oi): (usize, u32, usize),
    read: &BTreeSet<u32>,
    engine: &Arc<al_vm::Engine>,
    lowered: &Lowered,
) -> Option<String> {
    let objs = &lowered.program.objects;
    let session: Vec<_> = report.discovery_wrote.iter().filter(|w| w.0 == si).collect();
    if session.is_empty() {
        return None;
    }
    if let Some((_, id, _, _)) = session.iter().find(|w| w.3)
        && super::corpus::reaches_session_storage(engine, lowered, oi)
    {
        return Some(format!(
            "BcContainerHelper's discovery pass ran the OnRun trigger of codeunit {id}, and it changed a number sequence or isolated storage, which this codeunit can read: only the rows and SingleInstance state the pass leaves are pinned"
        ));
    }
    let facts = super::strict_taint::unit_facts(engine, lowered, oi);
    let table = |t: u32| objs.get(t as usize).map_or("?", |o| o.name.as_str()).to_string();
    let dynamic = facts.unknown.is_some() || facts.reads_any;
    session.iter().filter(|w| w.1 > cu).find_map(|(_, id, tables, _)| {
        let what = match tables.intersection(&facts.touched).next() {
            Some(&t) => format!("the table {}, which this codeunit can read", table(t)),
            None if dynamic => format!(
                "the table {}, which this codeunit read (through a RecordRef or a part of its reach not known statically)",
                table(*tables.intersection(read).next()?)
            ),
            None => return None,
        };
        Some(format!(
            "BcContainerHelper's discovery pass ran the OnRun trigger of the later codeunit {id} before this codeunit, and it changed {what}: whether this codeunit sees that is not pinned"
        ))
    })
}

/// What the run's call of a codeunit's OnRun leaves under BcContainerHelper (`run.rs`; `left` is what it left, as
/// bc-linux carries it, `start` the state before it): its rows are rolled back with the codeunit (SwOnRunRunner.al:21-23),
/// so the later codeunits start from `start`. Whether a number sequence or isolated storage it changed stays is not
/// pinned: a later codeunit whose reach can touch them is refused (`Carried::storage_why`).
pub(super) fn run_onrun_left(mut left: Carried, start: &al_vm::Baseline, name: &str) -> Carried {
    let mut b = start.clone();
    if let Some(l) = &left.baseline {
        b.workspace_installs.clone_from(&l.workspace_installs);
        b.install_rows.clone_from(&l.install_rows);
    }
    left.baseline = Some(b);
    left.tables.clear();
    left.as_started = false;
    if left.everything {
        left.storage_why = Some(format!(
            "under BcContainerHelper the run called the OnRun trigger of the earlier codeunit {name} inside the runner's isolation, and it changed a number sequence or isolated storage: whether that change stays for the later codeunits is not pinned"
        ));
    }
    left
}

/// Runs the discovery pass in `session` (opened as a test codeunit's session is, `run.rs`): the OnRun trigger of each of
/// `objects`, in order, each committed after it (130452 has no test isolation, SwOnRunRunner.al:21-22). Sets `CARRIED` to
/// what the pass left and `FOUND` ([`take_found`]) to the codeunits whose OnRun raised and what else it changed. `company_open_wrote`: the session's
/// Company Triggers subscribers wrote when it opened the company; each unit opens it again, so how those writes meet
/// what the pass left is not pinned.
pub(super) fn run_pass(
    session: &mut Session,
    lowered: &Lowered,
    objects: &[usize],
    skipped_installs: &[(BTreeSet<u32>, String)],
    company_open_wrote: bool,
    limit: Option<std::time::Duration>,
) {
    let start = session.capture_baseline();
    let stamp = session.state_stamp();
    session.track_single_instances();
    let changes_from = session.store_mut().change_counter();
    let mut raised = Vec::new();
    let mut wrote = Vec::new();
    let mut unfinished: Option<String> = company_open_wrote.then(|| {
        "a Company Triggers subscriber writes when the test session opens the company: how its writes meet what BcContainerHelper's discovery pass left is not pinned".to_string()
    });
    let mut touched = BTreeSet::new();
    for &oi in objects {
        let obj = &lowered.program.objects[oi];
        let Some(r) = on_run_of(obj) else { continue };
        let name = &obj.name;
        let before = (session.store_mut().change_counter(), session.state_stamp());
        let why = match blocked_reason(lowered, oi, r) {
            Some(why) => Some(why),
            None => {
                session.clear_stop();
                let watchdog = limit.map(|l| super::run::Watchdog::start_paused(session.stop_flag(), l, None, None));
                session.start_sleep_budget(limit);
                al_builtins::assumptions::clear_hits();
                let instance = session.codeunit_instance(oi);
                let ran = session.call(oi, r, instance, Vec::new());
                let timed_out = watchdog.is_some_and(super::run::Watchdog::finish);
                let marks = al_builtins::assumptions::take_hits();
                match ran {
                    _ if timed_out => Some("it was stopped by the time limit".to_string()),
                    _ if !marks.is_empty() => Some(format!(
                        "it went through unprobed assumptions ({})",
                        marks.iter().map(|a| a.info().id).collect::<Vec<_>>().join(", ")
                    )),
                    Ok(_) if session.inconsistent_table().is_some() => {
                        Some("it ends while Consistent(false) marks a table".to_string())
                    }
                    Err(Signal::Unsupported(why)) => Some(why.to_string()),
                    // The codeunit gets no method rows (SwOnRunRunner.al:24-26). What the pass keeps of what the OnRun
                    // changed before it raised is not pinned; the single instances it changed stay (the reader expects
                    // two OnRun calls, :289-294).
                    Err(Signal::Error(_)) => {
                        raised.push(obj.id.unwrap_or(0));
                        let wrote = session.store_mut().has_uncommitted()
                            || !session.store_mut().tables_changed_since(before.0).is_empty()
                            || before.1.changed(&session.state_stamp()).is_some();
                        session.rollback_transaction();
                        wrote.then(|| "it raised an error after it changed the database or session state".to_string())
                    }
                    _ => None,
                }
            }
        };
        match why {
            None => {
                session.commit_transaction();
                let tables = session.store_mut().tables_changed_since(before.0);
                let state = before.1.changed(&session.state_stamp()).is_some();
                if !tables.is_empty() || state {
                    wrote.push((obj.id.unwrap_or(0), tables, state));
                }
            }
            Some(why) => {
                session.rollback_transaction();
                touched.extend(tables_touched(lowered, oi, r));
                unfinished.get_or_insert_with(|| {
                    format!(
                        "BcContainerHelper's discovery pass runs the OnRun trigger of {name}, and {why}: what the pass left for the tests is not known"
                    )
                });
            }
        }
    }
    let mut tables = session.store_mut().tables_changed_since(changes_from);
    tables.extend(touched);
    let everything = session.storage_or_sequences_differ(&start) || unfinished.is_some();
    let mut baseline = session.capture_baseline();
    // The install code already ran for this state; the tests must not run it again on top (as `run.rs` carry_disabled).
    baseline.workspace_installs = Some(skipped_installs.to_vec());
    let carried = Carried {
        baseline: Some(baseline),
        tables,
        storage_why: unfinished.clone().filter(|_| everything),
        unfinished,
        everything,
        as_started: false,
    };
    super::run::CARRIED.with(|c| *c.borrow_mut() = Some(carried));
    // Each test codeunit has a session of its own here, which the session language and WorkDate the pass set do not
    // reach; on BC the run goes on in the pass's session. Number sequences and isolated storage are in `baseline`.
    let session_state = stamp
        .changed(&session.state_stamp())
        .filter(|what| matches!(*what, "the session language" | "WorkDate"))
        .map(|what| {
            format!("BcContainerHelper's discovery pass changed {what}, which the runtime does not carry into the run")
        });
    let singles = session.single_out().map(|o| o.wrote).unwrap_or_default();
    FOUND.with(|r| {
        *r.borrow_mut() = Found {
            raised,
            refused: session_state,
            singles,
            wrote,
        }
    });
}

/// A codeunit whose OnRun raised in the discovery pass (`raised`, from `ChainReport::discovery_raised`) gets no method
/// rows: the run calls its OnRun and runs no method (SwOnRunRunner.al:24-26).
pub(super) fn skip_methods_if_raised(u: &mut Unit, raised: &[(usize, u32)]) {
    if raised.contains(&(u.si, u.id)) {
        u.run_only = Some(Arc::new(BTreeSet::new()));
    }
}

/// What a discovery pass that did not run to its end (a panic) leaves: nothing known.
pub(super) fn not_known(why: &str) -> Carried {
    let why = format!(
        "BcContainerHelper's discovery pass did not run to its end ({why}): what it left for the tests is not known"
    );
    Carried {
        baseline: None,
        tables: BTreeSet::new(),
        unfinished: Some(why.clone()),
        everything: true,
        storage_why: Some(why),
        as_started: false,
    }
}

#[cfg(test)]
mod tests {
    use super::TestDriver;

    #[test]
    fn the_driver_names_parse_back_and_bc_linux_is_the_default() {
        assert_eq!(TestDriver::default(), TestDriver::BcLinux);
        for d in [TestDriver::BcLinux, TestDriver::BcContainerHelper] {
            assert_eq!(TestDriver::parse(d.name()), Some(d));
        }
        assert_eq!(
            TestDriver::parse("BcContainerHelper"),
            Some(TestDriver::BcContainerHelper)
        );
        assert_eq!(TestDriver::parse("al-go"), None);
        assert!(TestDriver::BcLinux.has_hub() && !TestDriver::BcContainerHelper.has_hub());
        assert!(TestDriver::BcLinux.run_onrun_carries() && !TestDriver::BcContainerHelper.run_onrun_carries());
    }
}
