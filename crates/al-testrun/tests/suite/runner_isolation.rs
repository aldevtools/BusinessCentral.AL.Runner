//! `RunOptions::isolation`: every test codeunit runs under one test runner's `TestIsolation` (`al-runner test --isolation
//! test` and `--isolation disabled`), as a runner codeunit with that property runs the whole suite. Measured by corpus
//! writer SD on BC 27.5, 28.5 and 29.0 (the same on all three) with the isolation probe app under four runners: corpus
//! branch al-runtime-rs-staging-pub-sd, commit 95c66e62 (corpus PR 735), tests/al-language-isolation-probe codeunits
//! 61302-61320. Under such a runner the codeunit's `RequiredTestIsolation` has no effect.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::isolation::TestIsolation;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};

const ROWS: &str = r#"
table 50700 "Row" { fields { field(1; K; Code[30]) { } } keys { key(PK; K) { Clustered = true; } } }
table 50701 "Counter" { fields { field(1; "Entry No."; Integer) { AutoIncrement = true; } field(2; Note; Text[30]) { } }
    keys { key(PK; "Entry No.") { Clustered = true; } } }
codeunit 50702 "Rows"
{
    procedure Put(K: Text) var R: Record "Row"; begin R.K := CopyStr(K, 1, 30); R.Insert(); end;
    procedure Has(K: Text): Boolean var R: Record "Row"; begin exit(R.Get(K)); end;
    procedure Count(): Integer var R: Record "Row"; begin exit(R.Count()); end;
}
"#;

fn results(src: &str, isolation: TestIsolation) -> Vec<(u32, String, Outcome, Vec<&'static str>)> {
    let opts = RunOptions {
        isolation: Some(isolation),
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
        .map(|r| {
            (
                r.codeunit,
                r.test.clone(),
                r.outcome.clone(),
                r.assumptions.iter().map(|a| a.info().id).collect(),
            )
        })
        .collect();
    out.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    out
}

fn run(src: &str, isolation: TestIsolation) -> Vec<(u32, String, Outcome)> {
    results(src, isolation)
        .into_iter()
        .map(|(c, t, o, _)| (c, t, o))
        .collect()
}

fn outcome<'a>(o: &'a [(u32, String, Outcome)], test: &str) -> &'a Outcome {
    &o.iter()
        .find(|(_, t, _)| t == test)
        .unwrap_or_else(|| panic!("no {test} in {o:?}"))
        .2
}

fn fails_with(o: &Outcome, part: &str) -> bool {
    matches!(o, Outcome::Fail(m) if m.contains(part))
}

fn not_run(o: &Outcome, part: &str) -> bool {
    matches!(o, Outcome::NotRun(m) if m.contains(part))
}

// IsolProbeCommit.Codeunit.al:145-166 (C1-C3) and SdIsolCommitError.Codeunit.al:43-68 (E2-E5), runner 61300: under
// TestIsolation = Function every test is rolled back, the part before its Commit() too, whether it passes or fails.
const COMMITS: &str = r#"
codeunit 50710 "Commits"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure C1_InsertsCommitsAndInsertsAgain() begin Rows.Put('COMMIT'); Commit(); Rows.Put('AFTER'); end;
    [Test] procedure C2_SeesNothing() begin if Rows.Count() <> 0 then Error('C2 rows: %1', Rows.Count()); end;
    [Test] procedure E2_CommitsThenFails() begin Rows.Put('E2-BEFORE'); Commit(); Rows.Put('E2-AFTER'); Error('E2 fails on purpose after Commit'); end;
    [Test] procedure E3_SeesNothing() begin if Rows.Count() <> 0 then Error('E3 rows: %1', Rows.Count()); end;
}
codeunit 50711 "Commit Reader"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure R1_NextCodeunitSeesNothing() begin if Rows.Count() <> 0 then Error('R1 rows: %1', Rows.Count()); end;
}
"#;

#[test]
fn under_a_function_runner_a_commit_inside_a_test_is_rolled_back() {
    let o = results(COMMITS, TestIsolation::Function);
    let plain: Vec<_> = o.iter().map(|(c, t, x, _)| (*c, t.clone(), x.clone())).collect();
    for t in [
        "C1_InsertsCommitsAndInsertsAgain",
        "C2_SeesNothing",
        "E3_SeesNothing",
        "R1_NextCodeunitSeesNothing",
    ] {
        assert_eq!(outcome(&plain, t), &Outcome::Pass, "{t}: {o:?}");
    }
    assert_eq!(
        outcome(&plain, "E2_CommitsThenFails"),
        &Outcome::Fail("E2 fails on purpose after Commit".into())
    );
    // The probe of corpus writer SD pins it: the test that committed is a clean pass.
    let c1 = o
        .iter()
        .find(|(_, t, _, _)| t == "C1_InsertsCommitsAndInsertsAgain")
        .unwrap();
    assert!(c1.3.is_empty(), "{c1:?}");
}

// IsolProbeAfterRollback.Codeunit.al:31-128 (S1-S9), runner 61300: the rollback after each test undoes an
// IsolatedStorage.Set and a NumberSequence.Insert (S7, S8 and S9 fail on BC); an AutoIncrement counter and the row
// version are not reset (S5, S6 pass).
const AFTER_ROLLBACK: &str = r#"
codeunit 50720 "After Rollback"
{
    Subtype = Test;
    var S1EntryNo: Integer; S1RowVersion: BigInteger; S3Next: BigInteger;
    [Test] procedure S1_InsertsCounterRow() var C: Record "Counter"; begin C.Note := 'S1'; C.Insert(); S1EntryNo := C."Entry No."; S1RowVersion := C.SystemRowVersion; end;
    [Test] procedure S2_SetsIsolatedStorage() begin IsolatedStorage.Set('K', 'V', DataScope::Module); end;
    [Test] procedure S3_InsertsNumberSequence() begin NumberSequence.Insert('SEQ', 1, 1); S3Next := NumberSequence.Next('SEQ'); end;
    [Test] procedure S5_AutoIncrementNotReset() var C: Record "Counter"; begin C.Note := 'S5'; C.Insert(); if C."Entry No." <> S1EntryNo + 1 then Error('S5 entry %1', C."Entry No."); end;
    [Test] procedure S6_RowVersionNotReset() var C: Record "Counter"; begin C.Note := 'S6'; C.Insert(); if C.SystemRowVersion <= S1RowVersion then Error('S6 row version'); end;
    [Test] procedure S7_IsolatedStorageGone() var V: Text; begin if IsolatedStorage.Get('K', DataScope::Module, V) then Error('S7 value %1', V); end;
    [Test] procedure S8_NumberSequenceGone() begin if NumberSequence.Exists('SEQ') then Error('S8 sequence there'); end;
}
"#;

#[test]
fn under_a_function_runner_isolated_storage_and_a_new_number_sequence_are_rolled_back() {
    let o = run(AFTER_ROLLBACK, TestIsolation::Function);
    for (_, t, x) in &o {
        assert_eq!(x, &Outcome::Pass, "{t}: {o:?}");
    }
}

// What the rollback does to a key or a sequence that existed before the test is not measured (the probe changes only
// new ones): the tests after such a change are not run.
#[test]
fn under_a_function_runner_a_change_to_storage_that_existed_before_the_test_is_not_followed() {
    let src = r#"
codeunit 50725 "Old Storage"
{
    Subtype = Test;
    trigger OnRun() begin IsolatedStorage.Set('OLD', 'V', DataScope::Module); NumberSequence.Insert('OLDSEQ', 1, 1); end;
    [Test] procedure A_DeletesTheOldKey() begin IsolatedStorage.Delete('OLD', DataScope::Module); end;
    [Test] procedure B_ReadsIt() begin if not IsolatedStorage.Contains('OLD', DataScope::Module) then Error('gone'); end;
}
codeunit 50726 "Old Sequence"
{
    Subtype = Test;
    trigger OnRun() begin NumberSequence.Insert('OLDSEQ2', 1, 1); end;
    [Test] procedure A_DrawsFromTheOldSequence() begin NumberSequence.Next('OLDSEQ2'); end;
    [Test] procedure B_DrawsAgain() begin if NumberSequence.Next('OLDSEQ2') <> 1 then Error('not reset'); end;
}
"#;
    let o = run(src, TestIsolation::Function);
    assert_eq!(outcome(&o, "A_DeletesTheOldKey"), &Outcome::Pass, "{o:?}");
    assert!(not_run(outcome(&o, "B_ReadsIt"), "not measured"), "{o:?}");
    assert_eq!(outcome(&o, "A_DrawsFromTheOldSequence"), &Outcome::Pass, "{o:?}");
    assert!(not_run(outcome(&o, "B_DrawsAgain"), "not measured"), "{o:?}");
}

// IsolProbeWriter.Codeunit.al:23-77 (W1-W5) and IsolProbeReader.Codeunit.al:16-41 (R1-R3), SdIsolCommitError.Codeunit.al
// :34-68 (E1-E5) and SdIsolCommitReader.Codeunit.al:21-35 (R1), runners 61304 (Disabled) and 61305 (no property): nothing
// is rolled back but a failed test's writes since its last Commit(); the next codeunit (a higher id) sees the rest.
const DISABLED: &str = r#"
codeunit 50730 "Writer"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    trigger OnRun() begin Rows.Put('W-ONRUN'); end;
    [Test] procedure W1_InsertsPassRow() begin Rows.Put('W-PASS'); end;
    [Test] procedure W2_InsertsFailRowThenFails() begin Rows.Put('W-FAIL'); Error('W2 fails on purpose after inserting W-FAIL'); end;
    [Test] procedure W3_PassRowStillThere() begin if not Rows.Has('W-PASS') then Error('W3: W-PASS gone'); end;
    [Test] procedure W4_FailRowRolledBack() begin if Rows.Has('W-FAIL') then Error('W4: W-FAIL kept'); end;
    [Test] procedure W5_OnRunRowStillThere() begin if not Rows.Has('W-ONRUN') then Error('W5: W-ONRUN gone'); end;
}
codeunit 50731 "Commit Error"
{
    Subtype = Test;
    RequiredTestIsolation = Function;
    var Rows: Codeunit "Rows";
    [Test] procedure E2_CommitsThenFails() begin Rows.Put('E2-BEFORE'); Commit(); Rows.Put('E2-AFTER'); Error('E2 fails on purpose after Commit'); end;
    [Test] procedure E3_SeesTheCommittedRow() begin if not Rows.Has('E2-BEFORE') or Rows.Has('E2-AFTER') then Error('E3'); end;
    [Test] procedure E4_CommitsAndPasses() begin Rows.Put('E4-COMMITTED'); Commit(); Rows.Put('E4-OPEN'); end;
}
codeunit 50732 "Reader"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure R1_SeesWhatStayed()
    begin
        if not (Rows.Has('W-ONRUN') and Rows.Has('W-PASS') and Rows.Has('E2-BEFORE') and Rows.Has('E4-COMMITTED') and Rows.Has('E4-OPEN')) then
            Error('R1: a row is gone');
        if Rows.Has('W-FAIL') or Rows.Has('E2-AFTER') then
            Error('R1: a failed test''s open row is there');
    end;
}
"#;

#[test]
fn under_a_disabled_runner_what_passed_or_was_committed_stays_for_the_later_tests_and_codeunits() {
    let o = results(DISABLED, TestIsolation::Disabled);
    let plain: Vec<_> = o.iter().map(|(c, t, x, _)| (*c, t.clone(), x.clone())).collect();
    for (_, t, x) in &plain {
        if t == "W2_InsertsFailRowThenFails" || t == "E2_CommitsThenFails" {
            assert!(fails_with(x, "fails on purpose"), "{t}: {o:?}");
        } else {
            assert_eq!(x, &Outcome::Pass, "{t}: {o:?}");
        }
    }
    let r1 = o.iter().find(|(_, t, _, _)| t == "R1_SeesWhatStayed").unwrap();
    assert!(r1.3.is_empty(), "{r1:?}");
}

// SdIsolStartSession.Codeunit.al:22-30 and :38-63 (S1-S3): under the Disabled runner (61304, and 61305 with no property)
// BC starts the session, it ends within the test's wait, `Session.IsSessionActive` then answers false, and its committed row
// is seen by the test and by the next one. Under Function the start raises BC's text (S2 under 61300).
const START_SESSION: &str = r#"
codeunit 50740 "Session Writer" { trigger OnRun() var R: Record "Row"; begin R.K := 'SESSION'; if R.Insert() then; end; }
codeunit 50741 "Starter"
{
    var Id: Integer;
    trigger OnRun() begin Session.StartSession(Id, Codeunit::"Session Writer"); end;
    procedure GetSessionId(): Integer begin exit(Id); end;
}
codeunit 50742 "Start Session"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure S2_StartSession()
    var Starter: Codeunit "Starter";
    begin
        if not Starter.Run() then Error('%1', GetLastErrorText());
        if Starter.GetSessionId() <= 0 then Error('session id %1', Starter.GetSessionId());
        if Session.IsSessionActive(Starter.GetSessionId()) then Error('still active');
        if not Rows.Has('SESSION') then Error('no row');
    end;
    [Test] procedure S3_NextTestSeesTheSessionRow() begin if not Rows.Has('SESSION') then Error('S3: no row'); end;
}
"#;

#[test]
fn start_session_fails_under_a_function_runner_and_runs_the_session_under_a_disabled_one() {
    let o = run(START_SESSION, TestIsolation::Function);
    assert_eq!(
        outcome(&o, "S2_StartSession"),
        &Outcome::Fail(
            "Sessions can only be started in tests that are run by a TestRunner that has TestIsolation set to Disabled.".into()
        )
    );
    let o = run(START_SESSION, TestIsolation::Disabled);
    assert_eq!(outcome(&o, "S2_StartSession"), &Outcome::Pass, "{o:?}");
    assert_eq!(outcome(&o, "S3_NextTestSeesTheSessionRow"), &Outcome::Pass, "{o:?}");
}

// What no probe measures is refused: a start while the test holds an uncommitted write (whether the new session waits for
// it, and what it sees), an error in the new session, and what a new session reads of the session state (here WorkDate).
// A new session has its own SingleInstance codeunits and no manual subscriptions of the test (both are per session).
const START_SESSION_HELPERS: &str = r#"
codeunit 50740 "Session Writer" { trigger OnRun() var R: Record "Row"; begin R.K := 'SESSION'; if R.Insert() then; end; }
codeunit 50750 "Failing Writer" { trigger OnRun() begin Error('session fails'); end; }
codeunit 50751 "WorkDate Reader" { trigger OnRun() var D: Date; begin D := WorkDate(); end; }
codeunit 50752 "Single" { SingleInstance = true; var N: Integer; procedure Bump(): Integer begin N += 1; exit(N); end; }
codeunit 50753 "Single Writer"
{
    trigger OnRun() var S: Codeunit "Single"; R: Record "Row"; begin R.K := Format(S.Bump()); R.Insert(); end;
}
"#;

/// One test codeunit `body` with the helpers, run on its own under the Disabled runner: the outcome of test `U`.
fn start_unpinned(body: &str) -> Outcome {
    let src = format!(
        "{START_SESSION_HELPERS}\ncodeunit 50754 \"Start Unpinned\"\n{{\n    Subtype = Test;\n    var Rows: Codeunit \"Rows\";\n    [Test] procedure U() {body}\n}}\n"
    );
    outcome(&run(&src, TestIsolation::Disabled), "U").clone()
}

#[test]
fn start_session_under_a_disabled_runner_refuses_what_no_probe_measures() {
    let o = start_unpinned(
        "var Id: Integer; begin Rows.Put('OPEN'); Session.StartSession(Id, Codeunit::\"Session Writer\"); end;",
    );
    assert!(not_run(&o, "uncommitted"), "{o:?}");
    let o = start_unpinned("var Id: Integer; begin Session.StartSession(Id, Codeunit::\"Failing Writer\"); end;");
    assert!(not_run(&o, "an error in the new session"), "{o:?}");
    let o = start_unpinned("var Id: Integer; begin Session.StartSession(Id, Codeunit::\"WorkDate Reader\"); end;");
    assert!(not_run(&o, "WorkDate"), "{o:?}");
    // A new session has SingleInstance codeunits of its own: they are per session.
    let o = start_unpinned(
        "var S: Codeunit \"Single\"; Id: Integer; begin S.Bump(); S.Bump(); Session.StartSession(Id, Codeunit::\"Single Writer\"); \
         if not Rows.Has('1') then Error('the session used the test session''s SingleInstance'); \
         if S.Bump() <> 3 then Error('the test session''s SingleInstance changed'); end;",
    );
    assert_eq!(o, Outcome::Pass);
}

// SdIsolReqFunction.Codeunit.al / SdIsolReqDisabled.Codeunit.al:16-17 (Q1, Q2 and the readers): under a runner the
// codeunit's RequiredTestIsolation changes nothing.
#[test]
fn under_a_runner_required_test_isolation_has_no_effect() {
    let src = r#"
codeunit 50750 "Req Disabled"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    var Rows: Codeunit "Rows";
    [Test] procedure Q1_Inserts() begin Rows.Put('RD-1'); end;
    [Test] procedure Q2_Sees() begin if not Rows.Has('RD-1') then Error('Q2 under RequiredTestIsolation = Disabled: RD-1 gone'); end;
}
codeunit 50751 "Req Disabled Reader"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure N1_Sees() begin if not Rows.Has('RD-1') then Error('N1: RD-1 gone'); end;
}
"#;
    let o = run(src, TestIsolation::Function);
    assert!(fails_with(outcome(&o, "Q2_Sees"), "RD-1 gone"), "{o:?}");
    assert!(fails_with(outcome(&o, "N1_Sees"), "RD-1 gone"), "{o:?}");
    let o = run(src, TestIsolation::Disabled);
    assert_eq!(outcome(&o, "Q2_Sees"), &Outcome::Pass, "{o:?}");
    assert_eq!(outcome(&o, "N1_Sees"), &Outcome::Pass, "{o:?}");
}

// IsolProbeAfterRollback.Codeunit.al S2 and S3 under runner 61304 change isolated storage and a number sequence; what a
// later codeunit sees of them is not measured. A later codeunit whose reach cannot touch either runs (the probe's
// codeunits 61310-61318 run after 61309 on BC); one that reads isolated storage is not run.
#[test]
fn under_a_disabled_runner_only_a_later_codeunit_that_reaches_isolated_storage_depends_on_it() {
    let src = r#"
codeunit 50760 "Sets Storage"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure SetsIt() begin IsolatedStorage.Set('K', 'V', DataScope::Module); Rows.Put('SET'); end;
}
codeunit 50761 "Reads Rows"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure SeesTheRow() begin if not Rows.Has('SET') then Error('gone'); end;
}
codeunit 50762 "Reads Storage"
{
    Subtype = Test;
    [Test] procedure SeesTheKey() begin if not IsolatedStorage.Contains('K', DataScope::Module) then Error('gone'); end;
}
"#;
    let o = run(src, TestIsolation::Disabled);
    assert_eq!(outcome(&o, "SetsIt"), &Outcome::Pass, "{o:?}");
    assert_eq!(outcome(&o, "SeesTheRow"), &Outcome::Pass, "{o:?}");
    assert!(not_run(outcome(&o, "SeesTheKey"), "isolated storage"), "{o:?}");
}
