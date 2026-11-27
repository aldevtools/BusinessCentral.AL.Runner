// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#[cfg(test)]
use super::*;
use al_sema::workspace::AppSource;

// BC's TestRunnerHub with RequiredTestIsolation = Function: what a passing test leaves of a number sequence that the
// codeunit's OnRun inserted (al-vm `hub_sequences.rs`; corpus PR 1580, SeqNextHubFunction.al, assumption
// `hub-function-sequence`).

/// The outcomes of test codeunit 52504 of `src` under Function isolation on the hub.
fn outcomes(src: &str) -> Vec<(String, Outcome)> {
    outcomes_from(src, None)
}

/// [`outcomes`] on the data that codeunit `seed`'s OnRun left (the seed's code joins `src`).
fn outcomes_from(src: &str, seed: Option<&str>) -> Vec<(String, Outcome)> {
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
    let index = (l.program.objects.iter())
        .position(|o| o.id == Some(52504) && o.kind == al_ir::ObjKind::Codeunit)
        .unwrap();
    let baseline = seed.map(|seed| super::tests::seeded_baseline(&engine, &l, seed));
    let mut scratch = Scratch {
        hub: true,
        ..Scratch::new(crate::host::RunClock::begin())
    };
    run::run_codeunit_core(
        &engine,
        &l,
        index,
        baseline.as_ref(),
        TestIsolation::Function,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut Vec::new(),
        &mut scratch,
        None,
        false,
    )
}

/// Test a runs `change`; test b expects `expect` from Current (relative to the value after OnRun), or 'missing'.
fn codeunit(change: &str, expect: &str) -> String {
    format!(
        "codeunit 52504 \"Hub Seq\"
{{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    trigger OnRun()
    begin
        NumberSequence.Insert('HUBSEQ', 1, 1, false);
        if NumberSequence.Next('HUBSEQ', false) = 0 then;
        if NumberSequence.Next('HUBSEQ', false) = 0 then;
    end;

    [Test]
    procedure A()
    begin
        {change}
    end;

    [Test]
    procedure B()
    var
        Actual: Text;
    begin
        if NumberSequence.Exists('HUBSEQ', false) then
            Actual := 'cur=' + Format(NumberSequence.Current('HUBSEQ', false) - 2)
        else
            Actual := 'missing';
        if Actual <> '{expect}' then
            Error('expected {expect}, got %1', Actual);
    end;
}}"
    )
}

fn b(outcomes: &[(String, Outcome)]) -> &Outcome {
    &outcomes.iter().find(|(n, _)| n == "B").expect("test B").1
}

#[test]
fn a_restart_is_put_back_for_the_next_test() {
    let o = outcomes(&codeunit("NumberSequence.Restart('HUBSEQ', 1000, false);", "cur=0"));
    assert!(matches!(b(&o), Outcome::Pass), "{o:?}");
}

#[test]
fn a_delete_is_put_back_for_the_next_test() {
    let o = outcomes(&codeunit("NumberSequence.Delete('HUBSEQ', false);", "cur=0"));
    assert!(matches!(b(&o), Outcome::Pass), "{o:?}");
}

#[test]
fn a_sequence_that_existed_before_the_codeunit_began_refuses() {
    // The seed inserts HUBSEQ before codeunit 52504 begins; its OnRun then only takes Next twice. Not measured
    // (hub_test_sequences: "a sequence missing when the codeunit began"): the next test refuses.
    let seed = "codeunit 52503 Seed
{
    trigger OnRun()
    begin
        NumberSequence.Insert('HUBSEQ', 1, 1, false);
    end;
}
";
    let src = codeunit("NumberSequence.Restart('HUBSEQ', 1000, false);", "cur=0")
        .replace("NumberSequence.Insert('HUBSEQ', 1, 1, false);\n", "");
    assert!(!src.contains("Insert"), "{src}");
    let o = outcomes_from(&format!("{seed}{src}"), Some("Seed"));
    assert!(matches!(b(&o), Outcome::NotRun(_)), "{o:?}");
}
