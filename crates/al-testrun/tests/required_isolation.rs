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

// The hub runs only codeunits without handlers (QuCommitIsolation.al:11-13); which runner BC's CI uses for one with
// handlers, and so its isolation and its place in the order, is not pinned.
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
    assert!(not_run(outcome(&o, "H1"), "RequiredTestIsolation"), "{o:?}");
    assert!(not_run(outcome(&o, "R1"), "OnRun"), "{o:?}");
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
    trigger OnRun() begin end;
    [Test] procedure D1() var R: Record "Row"; begin R.K := 'X'; R.Insert(); end;
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
    assert!(not_run(outcome(&o, "D1"), "OnRun"), "{o:?}");
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
"#,
        true,
    );
    assert_eq!(outcome(&o, "S1"), &Outcome::Pass, "{o:?}");
    assert!(not_run(outcome(&o, "L1"), "isolated storage"), "{o:?}");
}

// StartSession in a test raises TestStartSessionNotAllowed under a runner's TestIsolation = Codeunit
// (session/TestStartSessionRecord.al). Whether BC starts a session in a codeunit the TestRunnerHub runs under
// RequiredTestIsolation = Function or Disabled is not pinned (al-runner issue 5609): refused, never the Codeunit error.
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
fn start_session_under_the_hub_is_refused() {
    let o = run(START_SESSION, true);
    for t in ["SF1", "SD1"] {
        match outcome(&o, t) {
            Outcome::NotRun(why) => assert!(why.contains("StartSession"), "{t}: {why}"),
            other => panic!("{t}: {other:?}"),
        }
    }
}
