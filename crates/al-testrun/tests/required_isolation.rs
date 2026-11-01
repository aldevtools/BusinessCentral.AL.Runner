//! `RunOptions::required_isolation`: each test codeunit runs under its own `RequiredTestIsolation`, as BC's TestRunnerHub
//! runs a test codeunit without handlers (the corpus CI on BC 28 and later). Pinned by
//! corpus tests/al-language/codeunit/lt-qu-commit-isolation/QuCommitIsolation.al (header lines 10-16 and 25-39).

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};

const ROWS: &str = r#"
table 50600 "Row" { fields { field(1; K; Code[20]) { } } keys { key(PK; K) { Clustered = true; } } }
codeunit 50601 "Rows"
{
    procedure Put(K: Text) var R: Record "Row"; begin R.K := CopyStr(K, 1, 20); R.Insert(); end;
    procedure Count(): Integer var R: Record "Row"; begin exit(R.Count()); end;
}
"#;

fn run(src: &str, required: bool) -> Vec<(u32, String, Outcome)> {
    let opts = RunOptions {
        required_isolation: required,
        ..Default::default()
    };
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), format!("{ROWS}\n{src}"))],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &opts,
    );
    let mut out: Vec<_> = rt
        .results()
        .iter()
        .map(|r| (r.codeunit, r.test.clone(), r.outcome.clone()))
        .collect();
    out.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    out
}

fn outcome<'a>(o: &'a [(u32, String, Outcome)], test: &str) -> &'a Outcome {
    &o.iter()
        .find(|(_, t, _)| t == test)
        .unwrap_or_else(|| panic!("no {test} in {o:?}"))
        .2
}

// QuCommitIsolation.al:466-488 (QU_F04_NextTestSees) and :530-551 (QU_F09): under RequiredTestIsolation = Function the
// next test sees nothing, the rows a Commit() wrote included.
const FUNCTION: &str = r#"
codeunit 50610 "F"
{
    Subtype = Test;
    RequiredTestIsolation = Function;
    var Rows: Codeunit "Rows";
    [Test] procedure F1_Commits() begin Rows.Put('F1'); Commit(); end;
    [Test] procedure F2_NoCommit() begin Rows.Put('F2'); end;
    [Test] procedure F3_SeesNothing() begin if Rows.Count() <> 0 then Error('rows: %1', Rows.Count()); end;
}
"#;

#[test]
fn function_isolation_from_the_property_rolls_back_a_commit_inside_a_test() {
    let o = run(FUNCTION, true);
    for t in ["F1_Commits", "F2_NoCommit", "F3_SeesNothing"] {
        assert_eq!(outcome(&o, t), &Outcome::Pass, "{t}: {o:?}");
    }
}

// QuCommitIsolation.al:10-16 and 36-39: under Microsoft's runner 130450 (the BC 27 legs) the property has no effect: the
// codeunit runs under Codeunit isolation and the next test sees the rows.
#[test]
fn without_the_hub_the_property_has_no_effect() {
    let o = run(FUNCTION, false);
    assert_eq!(outcome(&o, "F1_Commits"), &Outcome::Pass, "{o:?}");
    assert_eq!(outcome(&o, "F3_SeesNothing"), &Outcome::Fail("rows: 2".into()), "{o:?}");
}

// QuCommitIsolation.al:572-718 (QU_D04, QU_D09, QU_D11): under RequiredTestIsolation = Disabled the rows persist into
// the next codeunit, the write still open when a test ended included. A codeunit that runs before them does not see them;
// a later codeunit that reads the table does (codeunits run in ascending id order, QuCommitIsolation.al:18-20).
const DISABLED: &str = r#"
codeunit 50620 "Before"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure B0_SeesNone() begin if Rows.Count() <> 0 then Error('before: %1', Rows.Count()); end;
}
codeunit 50621 "DA"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    var Rows: Codeunit "Rows";
    [Test] procedure D1_NoCommit() begin Rows.Put('D1'); end;
    [Test] procedure D2_CommitThenWrite() begin Rows.Put('D2'); Commit(); Rows.Put('D3'); end;
    [Test] procedure D3_SeesThree() begin if Rows.Count() <> 3 then Error('in DA: %1', Rows.Count()); end;
}
codeunit 50622 "DB"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    var Rows: Codeunit "Rows";
    [Test] procedure D4_NextCodeunitSeesThree() begin if Rows.Count() <> 3 then Error('in DB: %1', Rows.Count()); end;
}
codeunit 50623 "After"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure A1_LaterCodeunitSeesThree() begin if Rows.Count() <> 3 then Error('after: %1', Rows.Count()); end;
}
codeunit 50624 "Unrelated"
{
    Subtype = Test;
    [Test] procedure U1_ReadsNothingOfIt() begin end;
}
"#;

#[test]
fn disabled_isolation_from_the_property_keeps_everything_for_the_later_codeunits() {
    let o = run(DISABLED, true);
    for t in [
        "B0_SeesNone",
        "D1_NoCommit",
        "D2_CommitThenWrite",
        "D3_SeesThree",
        "D4_NextCodeunitSeesThree",
        "A1_LaterCodeunitSeesThree",
        "U1_ReadsNothingOfIt",
    ] {
        assert_eq!(outcome(&o, t), &Outcome::Pass, "{t}: {o:?}");
    }
}

#[test]
fn without_the_hub_disabled_runs_as_codeunit_isolation() {
    let o = run(DISABLED, false);
    assert_eq!(outcome(&o, "D3_SeesThree"), &Outcome::Pass, "{o:?}");
    assert_eq!(
        outcome(&o, "D4_NextCodeunitSeesThree"),
        &Outcome::Fail("in DB: 0".into()),
        "{o:?}"
    );
}

fn not_run(o: &Outcome, part: &str) -> bool {
    matches!(o, Outcome::NotRun(w) if w.contains(part))
}

// What a Disabled codeunit leaves after a test that did not pass is not pinned (QuCommitIsolation.al:22: a test that fails
// after Commit() is not asked): a later codeunit that reads what it wrote is not run.
#[test]
fn a_reader_after_a_disabled_codeunit_with_a_failed_test_is_not_run() {
    let o = run(
        r#"
codeunit 50630 "DA"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    var Rows: Codeunit "Rows";
    [Test] procedure D1_Fails() begin Rows.Put('D1'); Commit(); Error('boom'); end;
}
codeunit 50631 "After"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure A1_Reads() begin if Rows.Count() <> 1 then Error('after: %1', Rows.Count()); end;
}
"#,
        true,
    );
    assert_eq!(outcome(&o, "D1_Fails"), &Outcome::Fail("boom".into()), "{o:?}");
    assert!(not_run(outcome(&o, "A1_Reads"), "Disabled"), "{o:?}");
}

// The hub runs only codeunits without handlers (QuCommitIsolation.al:11-13); one with handlers runs under runner 130450 as
// Codeunit isolation (RqIsolationModes.al:673-839). RequiredTestIsolation = None is not pinned, and neither is whether a
// later codeunit with handlers sees what a Disabled codeunit left.
#[test]
fn what_the_hub_does_not_pin_is_refused() {
    let o = run(
        r#"
codeunit 50640 "FH"
{
    Subtype = Test;
    RequiredTestIsolation = Function;
    [Test] [HandlerFunctions('MH')] procedure H1() begin Message('x'); end;
    [MessageHandler] procedure MH(M: Text[1024]) begin end;
}
codeunit 50641 "FOnRun"
{
    Subtype = Test;
    RequiredTestIsolation = Function;
    trigger OnRun() begin end;
    [Test] procedure R1() begin end;
}
codeunit 50642 "NoneIso"
{
    Subtype = Test;
    RequiredTestIsolation = None;
    [Test] procedure N1() begin end;
}
codeunit 50643 "DA"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    var Rows: Codeunit "Rows";
    [Test] procedure D1() begin Rows.Put('D1'); end;
}
codeunit 50644 "HandlerReader"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] [HandlerFunctions('MH')] procedure HR1() begin Message('%1', Rows.Count()); end;
    [MessageHandler] procedure MH(M: Text[1024]) begin end;
}
"#,
        true,
    );
    // A codeunit with handlers runs under runner 130450, as Codeunit isolation (RqIsolationModes.al:673-839).
    assert_eq!(outcome(&o, "H1"), &Outcome::Pass, "{o:?}");
    // A codeunit with an OnRun trigger is pinned now (RqIsolationModes.al:396-472).
    assert_eq!(outcome(&o, "R1"), &Outcome::Pass, "{o:?}");
    assert!(not_run(outcome(&o, "N1"), "None"), "{o:?}");
    assert_eq!(outcome(&o, "D1"), &Outcome::Pass, "{o:?}");
    assert!(not_run(outcome(&o, "HR1"), "Disabled"), "{o:?}");
}

// A Disabled codeunit that is not run left something on BC that is not known here: a later codeunit that names a table
// it names is not run. One that changed isolated storage makes every later codeunit not run, as who reads that is not
// followed.
#[test]
fn what_a_disabled_codeunit_that_did_not_run_or_changed_other_state_left_is_not_trusted() {
    let o = run(
        r#"
codeunit 50650 "DOnRun"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    var Rows: Codeunit "Rows";
    [Test] procedure D1() var R: Record "Row"; begin R.K := 'X'; R.Insert(); Debugger.Deactivate(); end;
}
codeunit 50651 "Reader"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure R1() begin if Rows.Count() <> 0 then Error('rows'); end;
}
codeunit 50652 "Unrelated"
{
    Subtype = Test;
    [Test] procedure U1() begin end;
}
"#,
        true,
    );
    assert!(not_run(outcome(&o, "D1"), "Debugger"), "{o:?}");
    assert!(not_run(outcome(&o, "R1"), "was"), "{o:?}");
    assert_eq!(outcome(&o, "U1"), &Outcome::Pass, "{o:?}");
    let o = run(
        r#"
codeunit 50660 "DStorage"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure S1() begin IsolatedStorage.Set('k', 'v', DataScope::Module); end;
}
codeunit 50661 "Later"
{
    Subtype = Test;
    [Test] procedure L1() begin end;
}
codeunit 50662 "Later Reader"
{
    Subtype = Test;
    [Test] procedure L2() begin if not IsolatedStorage.Contains('k', DataScope::Module) then Error('gone'); end;
}
"#,
        true,
    );
    assert_eq!(outcome(&o, "S1"), &Outcome::Pass, "{o:?}");
    // Only a later codeunit whose reach can touch isolated storage or a number sequence depends on what was left there.
    assert_eq!(outcome(&o, "L1"), &Outcome::Pass, "{o:?}");
    assert!(not_run(outcome(&o, "L2"), "isolated storage"), "{o:?}");
}

// StartSession in a test raises TestStartSessionNotAllowed under a runner's TestIsolation = Codeunit
// (session/TestStartSessionRecord.al) and under RequiredTestIsolation = Function on the TestRunnerHub
// (RqIsolationModes.al:454-462). Under Disabled BC starts the session and it runs to its end (:585-614 RQ_D03, RQ_D04, BC 28
// and later): the session's codeunit runs inside the call.
const START_SESSION: &str = r#"
codeunit 50640 "Started" { trigger OnRun() begin end; }
codeunit 50641 "SF"
{
    Subtype = Test;
    RequiredTestIsolation = Function;
    [Test] procedure SF1() var Id: Integer; begin StartSession(Id, Codeunit::"Started"); end;
}
codeunit 50642 "SD"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure SD1() var Id: Integer; begin StartSession(Id, Codeunit::"Started"); end;
}
"#;

#[test]
fn start_session_under_the_hub_fails_under_function_and_runs_under_disabled() {
    let o = run(START_SESSION, true);
    assert_eq!(
        outcome(&o, "SF1"),
        &Outcome::Fail(
            "Sessions can only be started in tests that are run by a TestRunner that has TestIsolation set to Disabled.".into()
        ),
        "{o:?}"
    );
    assert_eq!(outcome(&o, "SD1"), &Outcome::Pass, "{o:?}");
}

// RqIsolationModes.al:396-472 (RQ_F01-RQ_F04, hub, BC 28+): under RequiredTestIsolation = Function a codeunit with an OnRun
// trigger runs: OnRun runs once, its writes (committed and left open) survive into every test, a test's own write does not
// reach the next test, and StartSession fails with TestStartSessionNotAllowed (:454-462).
const FUNCTION_ONRUN: &str = r#"
codeunit 50630 "FO"
{
    Subtype = Test;
    RequiredTestIsolation = Function;
    var Rows: Codeunit "Rows"; Calls: Integer;
    trigger OnRun() begin Calls += 1; Rows.Put('ORCM'); Commit(); Rows.Put('ORNC'); end;
    [Test] procedure FO1_Writes() begin if Rows.Count() <> 2 then Error('t1 rows: %1', Rows.Count()); Rows.Put('T1'); end;
    [Test] procedure FO2_SeesOnRunOnly() begin if Rows.Count() <> 2 then Error('t2 rows: %1', Rows.Count()); if Calls <> 1 then Error('calls %1', Calls); end;
    [Test] procedure FO3_StartSession()
    var Id: Integer; R: Record "Row";
    begin
        asserterror Session.StartSession(Id, Codeunit::"Rows", CompanyName(), R);
        if GetLastErrorCode() <> 'TestStartSessionNotAllowed' then Error('code %1', GetLastErrorCode());
        if GetLastErrorText() <> 'Sessions can only be started in tests that are run by a TestRunner that has TestIsolation set to Disabled.' then Error('text %1', GetLastErrorText());
    end;
}
codeunit 50631 "FO Next"
{
    Subtype = Test;
    RequiredTestIsolation = Function;
    var Rows: Codeunit "Rows";
    [Test] procedure FO11_NextCodeunitSeesNothing() begin if Rows.Count() <> 0 then Error('next rows: %1', Rows.Count()); end;
}
"#;

#[test]
fn function_isolation_on_the_hub_keeps_what_onrun_wrote_for_each_test() {
    let o = run(FUNCTION_ONRUN, true);
    for t in [
        "FO1_Writes",
        "FO2_SeesOnRunOnly",
        "FO3_StartSession",
        "FO11_NextCodeunitSeesNothing",
    ] {
        assert_eq!(outcome(&o, t), &Outcome::Pass, "{t}: {o:?}");
    }
}

// RqIsolationModes.al:532-568 and :616 (RQ_D01, RQ_D02, RQ_D11, hub, BC 28+): under RequiredTestIsolation = Disabled a
// codeunit with an OnRun trigger runs: the tests see OnRun's writes and each other's, and so does the next codeunit.
const DISABLED_ONRUN: &str = r#"
codeunit 50640 "DO"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    var Rows: Codeunit "Rows";
    trigger OnRun() begin Rows.Put('ORCM'); Commit(); Rows.Put('ORNC'); end;
    [Test] procedure DO1_Writes() begin if Rows.Count() <> 2 then Error('t1 rows: %1', Rows.Count()); Rows.Put('T1'); end;
    [Test] procedure DO2_SeesAll() begin if Rows.Count() <> 3 then Error('t2 rows: %1', Rows.Count()); end;
}
codeunit 50641 "DO Next"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    var Rows: Codeunit "Rows";
    [Test] procedure DO11_NextCodeunitSeesAll() begin if Rows.Count() <> 3 then Error('next rows: %1', Rows.Count()); end;
}
"#;

#[test]
fn disabled_isolation_on_the_hub_keeps_what_onrun_wrote_for_the_later_codeunits() {
    let o = run(DISABLED_ONRUN, true);
    for t in ["DO1_Writes", "DO2_SeesAll", "DO11_NextCodeunitSeesAll"] {
        assert_eq!(outcome(&o, t), &Outcome::Pass, "{t}: {o:?}");
    }
}

// RqIsolationModes.al:357-369 #else (RQ_C11 on BC 27): under Microsoft's runner 130450 a test codeunit's OnRun runs outside
// the codeunit isolation, so what it wrote, committed or not, reaches the later codeunits; the tests' writes do not.
const ONRUN_130450: &str = r#"
codeunit 50650 "CO"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var Rows: Codeunit "Rows";
    trigger OnRun() begin Rows.Put('ORCM'); end;
    [Test] procedure CO1_SeesOnRun() begin if Rows.Count() <> 1 then Error('rows: %1', Rows.Count()); Rows.Put('T1'); end;
}
codeunit 50651 "CO Next"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var Rows: Codeunit "Rows";
    [Test] procedure CO11_NextCodeunitReads() begin if Rows.Count() <> 1 then Error('next rows: %1', Rows.Count()); end;
    [Test] procedure CO12_ReadsNothing() begin end;
}
"#;

#[test]
fn under_runner_130450_what_onrun_wrote_reaches_the_later_codeunits() {
    let o = run(ONRUN_130450, false);
    for t in ["CO1_SeesOnRun", "CO11_NextCodeunitReads", "CO12_ReadsNothing"] {
        assert_eq!(outcome(&o, t), &Outcome::Pass, "{t}: {o:?}");
    }
    // A codeunit on the TestRunnerHub (BC 28+, no handlers, TestPermissions = Disabled) rolls OnRun's writes back
    // (RqIsolationModes.al:357-362).
    let o = run(ONRUN_130450, true);
    assert_eq!(
        outcome(&o, "CO11_NextCodeunitReads"),
        &Outcome::Fail("next rows: 0".into()),
        "{o:?}"
    );
}

// Under runner 130450 only what a test codeunit's OnRun writes reaches the later codeunits; its tests' writes are rolled
// back with the codeunit (RqIsolationModes.al:357-369 #else, RQ_C11 on BC 27). When OnRun does not run to its end here,
// a later codeunit that reads a table OnRun can write is not run; one that reads only a table the tests name runs.
#[test]
fn an_unfinished_onrun_under_runner_130450_blocks_only_readers_of_what_onrun_writes() {
    let o = run(
        r#"
table 50602 "Other" { fields { field(1; K; Code[20]) { } } keys { key(PK; K) { Clustered = true; } } }
codeunit 50670 "OnRun Unfinished"
{
    Subtype = Test;
    TestPermissions = Disabled;
    trigger OnRun() var R: Record "Row"; begin R.K := 'OR'; R.Insert(); R.Consistent(false); end;
    [Test] procedure OU1() var X: Record "Other"; begin X.K := 'T'; X.Insert(); end;
}
codeunit 50671 "Reads Row"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var Rows: Codeunit "Rows";
    [Test] procedure RR1() begin if Rows.Count() <> 1 then Error('rows: %1', Rows.Count()); end;
}
codeunit 50672 "Reads Other"
{
    Subtype = Test;
    TestPermissions = Disabled;
    [Test] procedure RO1() var X: Record "Other"; begin if not X.IsEmpty() then Error('other'); end;
}
"#,
        false,
    );
    assert!(not_run(outcome(&o, "OU1"), "Consistent"), "{o:?}");
    assert!(
        not_run(outcome(&o, "RR1"), "OnRun Unfinished, whose OnRun trigger"),
        "{o:?}"
    );
    assert_eq!(outcome(&o, "RO1"), &Outcome::Pass, "{o:?}");
}

// Under runner 130450 what a test codeunit's OnRun leaves in isolated storage and in the number sequences, committed and
// open, reaches the later codeunits as its rows do; the TestRunnerHub (BC 28+) drops it with the rows
// (testlibraries lt-y35-onrun-storage/Y35OnRunStorage.al C301_B1_OnRunCommittedStorage, C301_B1_OnRunSequences,
// C301_B1_OnRunOpenWrites; C306_B6_OnRunStorage for the hub).
const ONRUN_STORAGE_130450: &str = r#"
codeunit 50680 "OnRun Storage"
{
    Subtype = Test;
    TestPermissions = Disabled;
    trigger OnRun()
    var V: BigInteger;
    begin
        IsolatedStorage.Set('C', 'committed', DataScope::Module);
        NumberSequence.Insert('S', 100, 1, false);
        V := NumberSequence.Next('S', false);
        Commit();
        IsolatedStorage.Set('O', 'open', DataScope::Module);
    end;
    [Test] procedure OS1() begin end;
}
codeunit 50681 "Reads Storage"
{
    Subtype = Test;
    TestPermissions = Disabled;
    [Test] procedure RS1()
    var V: Text;
    begin
        if not IsolatedStorage.Get('C', DataScope::Module, V) then Error('no committed key');
        if not IsolatedStorage.Get('O', DataScope::Module, V) then Error('no open key');
        if not NumberSequence.Exists('S', false) then Error('no sequence');
        if NumberSequence.Next('S', false) <> 101 then Error('next');
    end;
}
"#;

#[test]
fn under_runner_130450_what_onrun_left_in_storage_and_sequences_reaches_the_later_codeunits() {
    let o = run(ONRUN_STORAGE_130450, false);
    assert_eq!(outcome(&o, "RS1"), &Outcome::Pass, "{o:?}");
    let o = run(ONRUN_STORAGE_130450, true);
    assert_eq!(outcome(&o, "RS1"), &Outcome::Fail("no committed key".into()), "{o:?}");
}

// An OnRun the lowering refuses (its reach is not known) that statically writes no table and no storage does not join the
// carry chain, as main's rule for tables has it (`onrun_writes`): a later codeunit that reads a table the refused
// codeunit's tests write still runs, the tests' own writes being rolled back with the codeunit. With `--no-deps` on BC 27
// every OnRun that reaches a Microsoft app is refused so.
const ONRUN_REFUSED_130450: &str = r#"
codeunit 50682 "OnRun Refused"
{
    Subtype = Test;
    TestPermissions = Disabled;
    trigger OnRun()
    var V: Variant; T: Text; L: Label 'abc', Locked = true; B: Boolean;
    begin
        V := B ? L : T;
    end;
    [Test] procedure OR1() var Rows: Codeunit "Rows"; begin Rows.Put('own'); end;
}
codeunit 50683 "Reads Rows"
{
    Subtype = Test;
    TestPermissions = Disabled;
    [Test] procedure RR1()
    var Rows: Codeunit "Rows";
    begin
        if Rows.Count() <> 0 then Error('rows');
    end;
}
"#;

#[test]
fn under_runner_130450_a_refused_onrun_that_writes_nothing_known_does_not_hold_back_a_later_codeunit() {
    let o = run(ONRUN_REFUSED_130450, false);
    assert!(matches!(outcome(&o, "OR1"), Outcome::NotRun(_)), "{o:?}");
    assert_eq!(outcome(&o, "RR1"), &Outcome::Pass, "{o:?}");
}
