//! The rows Microsoft's runner reads (ADR-066 decision 2), and the call that starts it the way BcContainerHelper and
//! AL-Go do.
//!
//! A runner codeunit with `TableNo = "Test Method Line"` (Microsoft's 130450 and 130451, and custom runners of that
//! shape such as the isolation-probe runners) reads its test codeunits and methods from the rows of table 130450 "Test
//! Method Line", and its suite from table 130451 "AL Test Suite" (Test Runner app, `TestRunnerMgt.Codeunit.al:25-58`).
//! BcContainerHelper fills them through page 130455 "Command Line Test Tool" and then calls its action `RunNextTest`
//! once per codeunit (`CommandLineTestTool.Page.al:353-375`), which calls `Test Suite Mgt.`.RunNextTest
//! (`TestSuiteMgt.Codeunit.al:97-121`): that picks the first codeunit line without a result and runs
//! `RunSelectedTests`, which calls `Codeunit.Run(ALTestSuite."Test Runner Id", Lines)` (`:218-253`, `:646-649`).
//!
//! The rows are written natively in the shape discovery writes them (`TestSuiteMgt.Codeunit.al:624-636` for the
//! codeunit line, `TestRunnerGetMethods.Codeunit.al:59-79` for a method line), with the methods a selection leaves out
//! at `Run = false`, as the procedure filter of page 130455 leaves them (ADR-066 decision 4). Discovery itself (codeunit
//! 130452, which runs the test codeunit's OnRun) is not run (owner decision 2 of ADR-066; probe P6).

use super::*;
use al_builtins::value::Value;

/// Table 130450 "Test Method Line" of Microsoft's Test Runner app.
pub(crate) const TEST_METHOD_LINE: u32 = 130_450;
/// Table 130451 "AL Test Suite".
pub(crate) const AL_TEST_SUITE: u32 = 130_451;
/// Codeunit 130456 "Test Suite Mgt.".
pub(crate) const TEST_SUITE_MGT: u32 = 130_456;
/// The suite BcContainerHelper uses when none is named (`Run-TestsInBcContainer -testSuite`, default `DEFAULT`).
const SUITE: &str = "DEFAULT";
/// Discovery numbers the lines of a new suite in steps of 10000 (`TestRunnerGetMethods.Codeunit.al:64-67`).
const LINE_STEP: i32 = 10_000;

/// Whether runner codeunit `runner` reads its tests from "Test Method Line" rows (its TableNo is table 130450).
pub(crate) fn reads_rows(program: &al_ir::Program, runner: usize) -> bool {
    program.objects[runner]
        .rec_slot
        .and_then(|slot| match program.objects[runner].globals.get(slot).map(|g| &g.ty) {
            Some(al_builtins::value::RtType::Record(t, _)) => program.objects[*t as usize].id,
            _ => None,
        })
        == Some(TEST_METHOD_LINE)
}

/// The table objects (indices) of "Test Method Line" and "AL Test Suite" that the program holds.
pub(crate) fn tables(program: &al_ir::Program) -> std::collections::BTreeSet<u32> {
    [TEST_METHOD_LINE, AL_TEST_SUITE]
        .into_iter()
        .filter_map(|id| object_of(program, al_ir::ObjKind::Table, id))
        .map(|o| o as u32)
        .collect()
}

std::thread_local! {
    /// Whether the rows the two tables held before [`write`] emptied them stayed unseen in every session of this thread
    /// since [`take_unseen`] (`None`: no session wrote them).
    static UNSEEN: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

/// Notes whether what the two tables held before [`write`] emptied them stayed unseen in this session ([`take_unseen`]).
pub(crate) fn note_unseen(unseen: bool) {
    UNSEEN.with(|c| c.set(Some(c.get().unwrap_or(true) && unseen)));
}

/// Whether every session of this thread that wrote the rows since the last call ([`write`]) started without anything
/// seeing what the two tables held before, and at least one did; then resets. A run ahead of the carry chain
/// (`chain_walk.rs`) whose sessions all did may be taken from a start whose rows of the two tables differ: the run
/// cannot tell (`al_vm::Baseline::fingerprint_apart`).
pub(crate) fn take_unseen() -> bool {
    UNSEEN.with(|c| c.take()).unwrap_or(false)
}

/// Starts watching the two tables in a new session whose codeunit `runner` will run, when it reads its tests from
/// their rows, before the session runs any AL code (install code, the login code, Company Triggers): [`seen_before`]
/// then says whether that code touched them. The value is that runner, if it reads rows. `al_runner_off`: the session
/// runs its tests without the runner (a scratch run, or a codeunit with data-driven tests, `run.rs`).
pub(crate) fn watch_before(session: &mut Session, runner: Option<usize>, al_runner_off: bool) -> Option<usize> {
    let runner = runner.filter(|&r| !al_runner_off && reads_rows(&session.engine.program, r))?;
    let tables = tables(&session.engine.program);
    session.store_mut().watch(tables);
    Some(runner)
}

/// Whether the AL code since [`watch_before`] (which gave `runner`) read or wrote the two tables; stops the watch. A
/// touch makes the session's start depend on what they held ([`take_unseen`]).
pub(crate) fn seen_before(session: &mut Session, runner: Option<usize>) {
    if runner.is_none() {
        return;
    }
    let seen = session.store_mut().watch_hit().is_some();
    session.store_mut().watch(Default::default());
    if seen {
        note_unseen(false);
    }
}

fn object_of(program: &al_ir::Program, kind: al_ir::ObjKind, id: u32) -> Option<usize> {
    program
        .objects
        .heads()
        .iter()
        .position(|o| o.kind == kind && o.id == Some(id))
}

/// Writes the suite row for runner `runner` and the lines of test codeunit `object`: one codeunit line and one line per
/// `[Test]` method in declaration order, `Run` true only for `selected` (routine indices). The value is a "Test Method
/// Line" record whose "Test Suite" names the suite, as page 130455 passes its record to `RunNextTest`.
pub(crate) fn write(session: &mut Session, runner: usize, object: usize, selected: &[usize]) -> Result<Value, String> {
    let engine = session.engine.clone();
    let program = &engine.program;
    let (Some(lines), Some(suites)) = (
        object_of(program, al_ir::ObjKind::Table, TEST_METHOD_LINE),
        object_of(program, al_ir::ObjKind::Table, AL_TEST_SUITE),
    ) else {
        return Err(
            "the test runner reads its tests from the Test Runner app's tables, which this run does not have".into(),
        );
    };
    let obj = &program.objects[object];
    let text = |s: &str| Value::Text(s.to_string().into());
    let id = obj.id.unwrap_or(0) as i32;
    let refused = |e: Signal| format!("writing the test runner's rows was refused: {}", e.describe());
    // A unit that starts from what an earlier codeunit left (Disabled isolation, an OnRun under runner 130450) finds
    // that codeunit's rows: the suite starts over, one codeunit per suite (what earlier codeunits' rows hold on BC is
    // ADR-066 probe P7).
    //
    // What they held before is unseen when nothing reads them before this (`watch_before`), and the reset itself shows
    // nothing of them: no subscriber listens to an event of either table (a DeleteAll with delete events deletes row by
    // row, `records/op_inner.rs`), the Global Triggers setup the reset asks first turns on no database event for them
    // (and its subscribers do not read them), and the SQL counters do not count it: the reset runs before any AL code
    // of the unit can read a counter, a read before it does not reach the unit, and AL sees only the difference of two
    // reads (al-builtins `sqlcount.rs`: an absolute value is refused, two reads of different epochs subtract to
    // `SqlForm::Unknown`), so no difference a test takes spans the reset. The commit below empties the insert buffer
    // either way (al-vm `sqlcount.rs` `SqlCounters::committed`). The runner's own AL reads only the current codeunit's
    // lines while its tests run, so it cannot see them either: Microsoft's Test Runner (BC 29, `Test Runner.Source.zip`)
    // calls `PlatformBeforeTestRun`/`PlatformAfterTestRun` from runner 130450's OnBeforeTestRun/OnAfterTestRun with the
    // suite and the codeunit's "Line No." filter (TestRunnerIsolCodeunit.Codeunit.al:28-40), and those read lines only
    // through `GetTestCodeunit`, `GetTestFunction` and `UpdateCodeunitLine`, each filtered on "Test Suite", "Test
    // Codeunit" and that filter (TestRunnerMgt.Codeunit.al:146-158, :215-241). The test's own code may not touch the
    // tables at all (`al_runner.rs` `Steps::rows_touched`, ADR-066 decision 4).
    let mut unseen = !(engine.events.any_subscriber_of(lines) || engine.events.any_subscriber_of(suites));
    let both: std::collections::BTreeSet<u32> = [lines as u32, suites as u32].into();
    for t in [lines, suites] {
        session.store_mut().watch(both.clone());
        let on = session.database_triggers_on(t);
        unseen &= session.store_mut().watch_hit().is_none();
        session.store_mut().watch(Default::default());
        unseen &= !on.map_err(refused)?;
        session.delete_all_rows(t).map_err(refused)?;
    }
    note_unseen(unseen);
    session
        .insert_row(
            suites,
            &[
                (1, text(SUITE)),
                (8, Value::Integer(program.objects[runner].id.unwrap_or(0) as i32)),
            ],
        )
        .map_err(refused)?;
    session
        .insert_row(
            lines,
            &[
                (1, text(SUITE)),
                (2, Value::Integer(LINE_STEP)),
                (3, Value::Integer(0)),
                (4, Value::Integer(id)),
                (5, text(&obj.name)),
                (7, Value::Boolean(true)),
                (12, Value::Integer(0)),
            ],
        )
        .map_err(refused)?;
    let methods = (0..obj.routines.len()).filter(|&r| {
        obj.routines[r]
            .attributes
            .iter()
            .any(|a| a.name.eq_ignore_ascii_case("Test"))
    });
    for (n, r) in methods.enumerate() {
        let name = &obj.routines[r].name;
        session
            .insert_row(
                lines,
                &[
                    (1, text(SUITE)),
                    (2, Value::Integer(LINE_STEP * (n as i32 + 2))),
                    (3, Value::Integer(1)),
                    (4, Value::Integer(id)),
                    (5, text(name)),
                    (6, text(name)),
                    (7, Value::Boolean(selected.contains(&r))),
                    (12, Value::Integer(1)),
                ],
            )
            .map_err(refused)?;
    }
    // The page's record: blank but for the suite (`CommandLineTestTool.Page.al:353-375`).
    let rec = session.record_with(lines, &[(1, text(SUITE))]).map_err(refused)?;
    // The rows are there before the run starts, as BcContainerHelper's are.
    session.commit_transaction();
    Ok(rec)
}

/// `Test Suite Mgt.`.RunNextTest(`rec`), as page 130455's action calls it.
pub(crate) fn run_next_test(session: &mut Session, rec: Value) -> Result<(), Signal> {
    let engine = session.engine.clone();
    let Some(object) = engine.compiled_codeunit(Some(TEST_SUITE_MGT)) else {
        return Err(Signal::Unsupported(format!(
            "codeunit {TEST_SUITE_MGT} \"Test Suite Mgt.\" of the Test Runner app was not compiled"
        )));
    };
    let obj = &engine.program.objects[object];
    let Some(routine) = obj
        .routines
        .iter()
        .position(|r| !r.is_trigger && r.name.eq_ignore_ascii_case("RunNextTest"))
    else {
        return Err(Signal::Unsupported(format!(
            "codeunit {TEST_SUITE_MGT} \"{}\" has no RunNextTest procedure",
            obj.name
        )));
    };
    let instance = session.codeunit_instance(object);
    session.call(object, routine, instance, vec![rec]).map(|_| ())
}
