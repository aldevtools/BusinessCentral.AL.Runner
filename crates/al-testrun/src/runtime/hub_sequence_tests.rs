// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#[cfg(test)]
use super::*;
use al_sema::workspace::AppSource;

// BC's TestRunnerHub with RequiredTestIsolation = Function: a number sequence the codeunit's OnRun inserted, or one a test
// changed, is refused in the later tests (al-vm `hub_sequences.rs`; BC reads it as missing now and then, corpus issues
// 1595 and 1614).

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

fn b_not_run(o: &[(String, Outcome)]) {
    assert!(matches!(b(o), Outcome::NotRun(_)), "{o:?}");
}

#[test]
fn a_restart_of_a_sequence_onrun_inserted_refuses() {
    b_not_run(&outcomes(&codeunit(
        "NumberSequence.Restart('HUBSEQ', 1000, false);",
        "cur=0",
    )));
}

#[test]
fn a_delete_of_a_sequence_onrun_inserted_refuses() {
    b_not_run(&outcomes(&codeunit("NumberSequence.Delete('HUBSEQ', false);", "cur=0")));
}

#[test]
fn a_next_of_a_sequence_onrun_inserted_refuses() {
    // O1 shape: OnRun only inserts it.
    let fresh = codeunit("if NumberSequence.Next('HUBSEQ', false) = 0 then;", "cur=-1").replace(
        "        if NumberSequence.Next('HUBSEQ', false) = 0 then;\n        if NumberSequence.Next('HUBSEQ', false) = 0 then;\n    end;",
        "    end;",
    );
    assert_eq!(fresh.matches("Next(").count(), 1, "{fresh}");
    b_not_run(&outcomes(&fresh));
    // E3 shape: OnRun inserts it and takes Next twice; the test takes Next and commits.
    b_not_run(&outcomes(&codeunit(
        "if NumberSequence.Next('HUBSEQ', false) = 0 then; Commit();",
        "cur=1",
    )));
}

#[test]
fn a_sequence_onrun_inserted_and_a_test_only_read_refuses() {
    b_not_run(&outcomes(&codeunit(
        "if NumberSequence.Exists('HUBSEQ', false) then;",
        "cur=0",
    )));
}

#[test]
fn a_sequence_inserted_in_a_test_refuses() {
    let src = codeunit(
        "NumberSequence.Insert('HUBNEW', 1, 1, false); if NumberSequence.Next('HUBNEW', false) = 0 then;",
        "cur=0",
    )
    // B reads only HUBNEW: a read of HUBSEQ, which OnRun inserted, would refuse B on its own (al-runner issue 9303).
    .replace(
        "if NumberSequence.Exists('HUBSEQ'",
        "if NumberSequence.Next('HUBNEW', false) = 0 then;\n        if NumberSequence.Exists('HUBNEW'",
    )
    .replace("Current('HUBSEQ'", "Current('HUBNEW'");
    assert!(!src[src.find("procedure B()").unwrap()..].contains("HUBSEQ"), "{src}");
    b_not_run(&outcomes(&src));
}

#[test]
fn a_sequence_that_existed_before_the_codeunit_began_refuses() {
    // The seed inserts HUBSEQ before codeunit 52504 begins; its OnRun then only takes Next twice. Not measured
    // (hub_sequences.rs: what a test leaves of a sequence it changed is not measured): the next test refuses.
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
