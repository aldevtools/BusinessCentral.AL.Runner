// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#[cfg(test)]
use super::*;
use al_sema::workspace::AppSource;

// Database.MinimumActiveRowVersion across the tests of a test codeunit, as the corpus probe
// tests/al-language/metadata/platform-tables/MinimumActiveRowVersionStages.al (corpus branch
// al-runtime-rs-staging-minrowver, MRV Sequence.Run, lines 47-84) observed it on BC 28.5 and 29.0: the first read of the
// codeunit returns LastUsedRowVersion() + 1; the value then stays the same through an open Insert and Modify, Commit,
// further writes and a DeleteAll, also in the next test of the codeunit, under RequiredTestIsolation Codeunit and Function
// alike; the first row inserted after the first read carries the value as its SystemRowVersion, later rows a greater one.

/// The outcomes of test codeunit `test` under `isolation`, through test runner codeunit `runner` or al-runner's own runner,
/// on BC's TestRunnerHub when `hub`.
fn outcomes(src: &str, test: u32, runner: Option<u32>, isolation: TestIsolation, hub: bool) -> Vec<(String, Outcome)> {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let engine = Engine::with_target(l.program.clone(), Vec::new().into_iter().collect(), None);
    let index = |id: u32| {
        l.program
            .objects
            .iter()
            .position(|o| o.id == Some(id) && o.kind == al_ir::ObjKind::Codeunit)
            .unwrap()
    };
    let mut scratch = Scratch {
        runner: runner.map(index),
        hub,
        ..Default::default()
    };
    run::run_codeunit_core(
        &engine,
        &l,
        index(test),
        None,
        isolation,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut Vec::new(),
        &mut scratch,
        None,
        false,
    )
}

/// A table for the tests' rows, a log table and a runner whose triggers write a log row each (the platform commits after
/// OnBeforeTestRun), as Microsoft's runner does.
const FIXTURE: &str = "
table 60340 \"MRV Row\"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[10]) { }
    }
    keys { key(PK; Id) { } }
}

table 60341 \"MRV Log\"
{
    fields
    {
        field(1; No; Integer) { AutoIncrement = true; }
        field(2; FunctionName; Text[100]) { }
    }
    keys { key(PK; No) { } }
}

codeunit 60342 \"MRV Runner\"
{
    Subtype = TestRunner;

    trigger OnRun()
    begin
        Codeunit.Run(60343);
    end;

    trigger OnBeforeTestRun(CodeunitId: Integer; CodeunitName: Text; FunctionName: Text; FunctionTestPermissions: TestPermissions): Boolean
    var
        L: Record \"MRV Log\";
    begin
        L.Init();
        L.FunctionName := FunctionName;
        L.Insert();
        exit(true);
    end;

    trigger OnAfterTestRun(CodeunitId: Integer; CodeunitName: Text; FunctionName: Text; FunctionTestPermissions: TestPermissions; IsSuccess: Boolean)
    var
        L: Record \"MRV Log\";
    begin
        L.Init();
        L.FunctionName := FunctionName;
        L.Insert();
    end;
}

codeunit 60345 \"MRV Runner Write First\"
{
    Subtype = TestRunner;

    trigger OnRun()
    begin
        Codeunit.Run(60344);
    end;

    trigger OnBeforeTestRun(CodeunitId: Integer; CodeunitName: Text; FunctionName: Text; FunctionTestPermissions: TestPermissions): Boolean
    var
        L: Record \"MRV Log\";
    begin
        L.Init();
        L.FunctionName := FunctionName;
        L.Insert();
        exit(true);
    end;

    trigger OnAfterTestRun(CodeunitId: Integer; CodeunitName: Text; FunctionName: Text; FunctionTestPermissions: TestPermissions; IsSuccess: Boolean)
    var
        L: Record \"MRV Log\";
    begin
        L.Init();
        L.FunctionName := FunctionName;
        L.Insert();
    end;
}

codeunit 60343 \"MRV Stages\"
{
    Subtype = Test;

    var
        Start: BigInteger;

    [Test]
    procedure A_FirstReadThenWrites()
    var
        Row: Record \"MRV Row\";
    begin
        Start := MinimumActiveRowVersion();
        if Start <> LastUsedRowVersion() + 1 then Error('first: not last + 1');
        Row.Id := 1;
        Row.Insert();
        Row.Get(1);
        if MinimumActiveRowVersion() <> Start then Error('ins: min moved');
        if Row.SystemRowVersion <> Start then Error('ins: row is not min');
        if Row.SystemRowVersion > LastUsedRowVersion() then Error('ins: row above last');
        Row.Name := 'x';
        Row.Modify();
        Row.Get(1);
        if MinimumActiveRowVersion() <> Start then Error('mod: min moved');
        if Row.SystemRowVersion <= Start then Error('mod: row not above min');
        Commit();
        if MinimumActiveRowVersion() <> Start then Error('com: min moved');
        Row.Id := 2;
        Row.Insert();
        Row.Get(2);
        if MinimumActiveRowVersion() <> Start then Error('ins2: min moved');
        if Row.SystemRowVersion <= Start then Error('ins2: row not above min');
        Commit();
        Row.DeleteAll();
        if MinimumActiveRowVersion() <> Start then Error('del: min moved');
        Commit();
        if MinimumActiveRowVersion() <> Start then Error('delcom: min moved');
    end;

    [Test]
    procedure B_NextTestSeesTheSameValue()
    var
        Row: Record \"MRV Row\";
    begin
        if MinimumActiveRowVersion() <> Start then Error('b: min moved');
        if MinimumActiveRowVersion() >= LastUsedRowVersion() + 1 then Error('b: min not below last + 1');
        Row.Id := 3;
        Row.Insert();
        Row.Get(3);
        if Row.SystemRowVersion <= Start then Error('b: row not above min');
        if MinimumActiveRowVersion() <> Start then Error('b ins: min moved');
    end;
}

codeunit 60344 \"MRV Write First\"
{
    Subtype = Test;

    [Test]
    procedure WritesThenReads()
    var
        Row: Record \"MRV Row\";
        Min: BigInteger;
    begin
        Row.Id := 10;
        Row.Insert();
        Commit();
        Min := MinimumActiveRowVersion();
        if MinimumActiveRowVersion() <> Min then Error('min moved');
    end;
}
";

fn passes(names: &[&str]) -> Vec<(String, Outcome)> {
    names.iter().map(|n| (n.to_string(), Outcome::Pass)).collect()
}

// After the runner's codeunit-level OnBeforeTestRun wrote and committed a row, the first read of the codeunit still
// returns LastUsedRowVersion() + 1 and the value holds through the stages and into the next test (Codeunit isolation).
#[test]
fn min_active_row_version_after_runner_triggers_wrote_follows_the_probe_under_codeunit_isolation() {
    let got = outcomes(FIXTURE, 60343, Some(60342), TestIsolation::Codeunit, false);
    assert_eq!(got, passes(&["A_FirstReadThenWrites", "B_NextTestSeesTheSameValue"]));
}

// RequiredTestIsolation = Function on the hub: the same relations, also after the first test's rows were rolled back. (The
// runner's rows keep the database from being empty: with no row version handed out yet no value is taken.)
#[test]
fn min_active_row_version_follows_the_probe_under_function_isolation() {
    let got = outcomes(FIXTURE, 60343, Some(60342), TestIsolation::Function, true);
    assert_eq!(got, passes(&["A_FirstReadThenWrites", "B_NextTestSeesTheSameValue"]));
}

// Under Codeunit isolation a test that wrote before its first read reads the value taken when the codeunit's tests began,
// also when the runner's codeunit-level OnBeforeTestRun wrote and committed a row before them (as before that call was
// made, PR 7336).
#[test]
fn min_active_row_version_read_after_own_writes_under_codeunit_isolation_with_a_writing_runner() {
    let got = outcomes(FIXTURE, 60344, Some(60345), TestIsolation::Codeunit, false);
    assert_eq!(got, passes(&["WritesThenReads"]));
}

// A first read after the codeunit's tests wrote rows is not covered by the probe's observations under Function isolation:
// refused.
#[test]
fn min_active_row_version_first_read_after_own_writes_under_function_isolation_is_refused() {
    let got = outcomes(FIXTURE, 60344, Some(60345), TestIsolation::Function, true);
    assert!(
        matches!(&got[..], [(n, Outcome::NotRun(_))] if n == "WritesThenReads"),
        "{got:?}"
    );
}
