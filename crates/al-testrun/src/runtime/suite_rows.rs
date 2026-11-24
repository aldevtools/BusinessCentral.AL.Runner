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
use al_sema::check::demand::TEST_SUITE_MGT;

/// Table 130450 "Test Method Line" of Microsoft's Test Runner app.
pub(crate) const TEST_METHOD_LINE: u32 = 130_450;
/// Table 130451 "AL Test Suite".
pub(crate) const AL_TEST_SUITE: u32 = 130_451;
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
    // The reset itself shows nothing of what they held: no subscriber listens to an event of either table (a DeleteAll
    // with delete events deletes row by row, `records/op_inner.rs`), the Global Triggers setup the reset asks first turns on no database event for them
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
    // tables at all (`al_runner.rs` `Steps::rows_touched`, ADR-066 decision 4). A SingleInstance global that the
    // setup asks or the events of these writes change refuses its readers ([`mark_changed`]).
    let before = session.left_stamp();
    for t in [lines, suites] {
        session.database_triggers_on(t).map_err(refused)?;
        session.delete_all_rows(t).map_err(refused)?;
    }
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
    mark_changed(session, &before);
    // The rows are there before the run starts, as BcContainerHelper's are.
    session.commit_transaction();
    Ok(rec)
}

/// Marks each global of a SingleInstance codeunit that the row write changed since `before` (its Global Triggers setup
/// asks for 130450 and 130451, and the database events of its writes when a subscriber turned them on): every read of it
/// is refused. BC asks the setup for 130450 once or not at all, depending on the test runner, and for 130451 never
/// (corpus PR 1567, LsLoginSubscriberTests.al:53-57, cell D), so what this code leaves is not BC's value (al-runner issue
/// 8945). The mark comes after the whole write: a second ask that reads what the first one changed runs, and what it
/// changes is marked too.
fn mark_changed(session: &mut Session, before: &al_vm::LeftStamp) {
    let engine = session.engine.clone();
    for (o, globals) in before.changed(&session.left_stamp()).singles {
        let obj = &engine.program.objects[o];
        for g in globals {
            let why = format!(
                "a read of global {} of SingleInstance codeunit {}: the test runner's writes of its tables 130450 and 130451 (the \
                 Global Triggers setup asks and events they raise) changed it before the first test; BC asks the setup for 130450 once \
                 or not at all, depending on the test runner, and for 130451 never: what BC leaves there is not measured",
                obj.globals.get(g).map_or("?", |v| v.name.as_str()),
                obj.name
            );
            session.mark_single_global_unmeasured(o, g, why.into());
        }
    }
}

/// `Test Suite Mgt.`.RunNextTest(`rec`), as page 130455's action calls it.
pub(crate) fn run_next_test(session: &mut Session, rec: Value) -> Result<(), Signal> {
    let engine = session.engine.clone();
    let Some(object) = engine.compiled_codeunit(Some(TEST_SUITE_MGT)) else {
        return Err(Signal::unsupported(format!(
            "codeunit {TEST_SUITE_MGT} \"Test Suite Mgt.\" of the Test Runner app was not compiled"
        )));
    };
    let obj = &engine.program.objects[object];
    let Some(routine) = obj
        .routines
        .iter()
        .position(|r| !r.is_trigger && r.name.eq_ignore_ascii_case("RunNextTest"))
    else {
        return Err(Signal::unsupported(format!(
            "codeunit {TEST_SUITE_MGT} \"{}\" has no RunNextTest procedure",
            obj.name
        )));
    };
    let instance = session.codeunit_instance(object);
    session.call(object, routine, instance, vec![rec]).map(|_| ())
}
