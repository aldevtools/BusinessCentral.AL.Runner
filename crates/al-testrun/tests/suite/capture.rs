//! Value capture and loop tracking (ADR-030): what a scratch run's code did, statement by statement.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_vm::{CaptureConfig, ClosedBy, Rendered};

fn run(code: &str, capture: CaptureConfig) -> al_testrun::runtime::TestResult {
    let apps = vec![AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![],
        files: vec![("a.al".into(), code.into())],
    }];
    let rt = CorpusRuntime::try_run_suites_with(
        apps,
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun {
            name: String::new(),
            app: Some(0),
            runner: None,
        }],
        &RunOptions {
            execute: true,
            capture,
            ..Default::default()
        },
    )
    .unwrap();
    rt.results()[0].clone()
}

const BOTH: CaptureConfig = CaptureConfig {
    values: true,
    loops: true,
};
const VALUES: CaptureConfig = CaptureConfig {
    values: true,
    loops: false,
};

fn series(r: &al_testrun::runtime::TestResult) -> Vec<(String, String, Rendered)> {
    r.captured
        .as_ref()
        .unwrap()
        .captured
        .values
        .iter()
        .map(|v| (v.scope.clone(), v.variable.clone(), v.value.clone()))
        .collect()
}

#[test]
fn every_execution_of_a_statement_that_writes_a_local_is_a_record_changed_or_not() {
    let r = run(
        "codeunit 50100 X { trigger OnRun() var Msg: Text; N: Integer; Untouched: Integer; begin Msg := 'hi'; Msg := 'bye'; N := 5; N := 5; end; }",
        VALUES,
    );
    let s = series(&r);
    let text = |t: &str| Rendered::Text(t.into());
    assert_eq!(
        s,
        vec![
            ("OnRun".into(), "Msg".into(), text("hi")),
            ("OnRun".into(), "Msg".into(), text("bye")),
            ("OnRun".into(), "N".into(), Rendered::Int(5)),
            ("OnRun".into(), "N".into(), Rendered::Int(5)),
        ],
        "a local never written has no record, a rewrite of the same value has one"
    );
    let ids: Vec<u32> = r
        .captured
        .as_ref()
        .unwrap()
        .captured
        .values
        .iter()
        .map(|v| v.statement)
        .collect();
    assert_eq!(ids.len(), 4);
    assert!(
        ids.windows(2).all(|w| w[0] != w[1]),
        "four statements, four ids: {ids:?}"
    );
    // Every id names a place in the source.
    for id in ids {
        assert!(r.captured.as_ref().unwrap().locations.contains_key(&id));
    }
}

#[test]
fn a_parameter_at_entry_is_no_record_and_a_callee_has_its_own_scope() {
    let r = run(
        "codeunit 50100 X { procedure Twice(N: Integer): Integer var R: Integer; begin R := N * 2; exit(R); end; trigger OnRun() var T: Integer; begin T := Twice(21); end; }",
        VALUES,
    );
    let s = series(&r);
    assert_eq!(
        s,
        vec![
            ("Twice".into(), "R".into(), Rendered::Int(42)),
            ("OnRun".into(), "T".into(), Rendered::Int(42)),
        ]
    );
}

#[test]
fn a_for_loop_has_its_passes_and_tags_what_they_produced() {
    let r = run(
        "codeunit 50100 X { trigger OnRun() var i: Integer; total: Integer; begin total := 0; for i := 1 to 3 do begin total := total + i; Message(Format(total)); end; end; }",
        BOTH,
    );
    let c = &r.captured.as_ref().unwrap().captured;
    assert_eq!(c.loops.len(), 1, "{c:?}");
    let l = &c.loops[0];
    assert_eq!(
        (l.scope.as_str(), l.parent, l.closed_by),
        ("OnRun", None, ClosedBy::Exit)
    );
    assert_eq!(l.iterations.len(), 3);
    // Each pass ran the same two statements of the body (the loop statement itself is outside its pass).
    assert!(l.iterations.iter().all(|p| p == &l.iterations[0]), "{:?}", l.iterations);
    assert_eq!(l.iterations[0].len(), 2);
    let tagged: Vec<(String, Option<u32>, Option<u32>)> = c
        .values
        .iter()
        .map(|v| (v.variable.clone(), v.loop_id, v.iteration))
        .collect();
    assert_eq!(
        tagged,
        vec![
            ("total".into(), None, None),
            ("i".into(), Some(0), Some(1)),
            ("total".into(), Some(0), Some(1)),
            ("i".into(), Some(0), Some(2)),
            ("total".into(), Some(0), Some(2)),
            ("i".into(), Some(0), Some(3)),
            ("total".into(), Some(0), Some(3)),
        ]
    );
    let m: Vec<(&str, Option<u32>, Option<u32>)> = c
        .messages
        .iter()
        .map(|m| (m.text.as_str(), m.loop_id, m.iteration))
        .collect();
    assert_eq!(
        m,
        vec![
            ("1", Some(0), Some(1)),
            ("3", Some(0), Some(2)),
            ("6", Some(0), Some(3))
        ]
    );
    assert_eq!(r.messages, vec!["1", "3", "6"]);
}

#[test]
fn loops_nest_across_calls_and_say_how_they_ended() {
    let r = run(
        "codeunit 50100 X {
            procedure Inner(): Integer var j: Integer; begin for j := 1 to 5 do if j = 2 then exit(j); end;
            trigger OnRun() var i: Integer; k: Integer; w: Integer;
            begin
                for i := 1 to 2 do k := Inner();
                while true do begin w += 1; if w = 3 then break; end;
                for i := 1 to 0 do k := 1;
            end;
        }",
        BOTH,
    );
    let c = &r.captured.as_ref().unwrap().captured;
    type Shape<'a> = (&'a str, Option<u32>, Option<u32>, usize, ClosedBy);
    let by_scope: Vec<Shape> = c
        .loops
        .iter()
        .map(|l| {
            (
                l.scope.as_str(),
                l.parent,
                l.parent_iteration,
                l.iterations.len(),
                l.closed_by,
            )
        })
        .collect();
    assert_eq!(
        by_scope,
        vec![
            ("OnRun", None, None, 2, ClosedBy::Exit),
            ("Inner", Some(0), Some(1), 2, ClosedBy::ScopeExit),
            ("Inner", Some(0), Some(2), 2, ClosedBy::ScopeExit),
            ("OnRun", None, None, 3, ClosedBy::Exit),
            ("OnRun", None, None, 0, ClosedBy::Exit),
        ],
        "{:?}",
        c.loops
    );
}

#[test]
fn a_loop_an_error_goes_through_is_unfinished_and_the_values_before_it_stay() {
    let r = run(
        "codeunit 50100 X { trigger OnRun() var i: Integer; begin for i := 1 to 3 do if i = 2 then Error('stop'); end; }",
        BOTH,
    );
    assert!(matches!(r.outcome, al_testrun::Outcome::Fail(_)));
    let c = &r.captured.as_ref().unwrap().captured;
    assert_eq!(c.loops[0].closed_by, ClosedBy::Unfinished);
    assert_eq!(c.loops[0].iterations.len(), 2);
    assert_eq!(c.values.len(), 2, "{:?}", c.values);
}

#[test]
fn without_a_capture_nothing_is_kept() {
    let r = run(
        "codeunit 50100 X { trigger OnRun() var N: Integer; begin N := 1; end; }",
        CaptureConfig::default(),
    );
    assert!(r.captured.is_none());
}

#[test]
fn loops_alone_keep_no_values_and_values_alone_keep_no_loops() {
    let code = "codeunit 50100 X { trigger OnRun() var i: Integer; begin for i := 1 to 2 do ; end; }";
    let only_loops = run(
        code,
        CaptureConfig {
            values: false,
            loops: true,
        },
    );
    let c = &only_loops.captured.unwrap().captured;
    assert!(c.values.is_empty() && c.loops.len() == 1, "{c:?}");
    let only_values = run(code, VALUES);
    let c = &only_values.captured.unwrap().captured;
    assert!(c.loops.is_empty() && !c.values.is_empty(), "{c:?}");
}

#[test]
fn a_capture_keeps_a_bounded_number_of_records_and_says_when_it_dropped_some() {
    let r = run(
        "codeunit 50100 X { trigger OnRun() var i: Integer; x: Integer; begin for i := 1 to 150000 do x := i; end; }",
        VALUES,
    );
    let c = &r.captured.as_ref().unwrap().captured;
    assert!(c.truncated);
    assert_eq!(c.values.len(), al_vm::MAX_RECORDS);
    assert!(
        matches!(r.outcome, al_testrun::Outcome::Pass),
        "the run itself is not cut short"
    );
}
