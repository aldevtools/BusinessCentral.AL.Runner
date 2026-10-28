//! `--explain`: where a test was refused or failed. AL source -> compile -> run as a test codeunit
//! with fault tracing, then read the site the runtime recorded.

use al_sema::workspace::AppSource;
use al_test_cli::explain::{self, ErrorInfo};
use al_test_cli::isolation::TestIsolation;
use al_test_cli::report::Outcome;
use al_test_cli::runtime::{Observed, Observer, blocked_site, run_codeunit_observed};
use al_vm::Engine;

const SRC: &str = r#"
codeunit 63100 T
{
    Subtype = Test;

    [Test]
    procedure FailsTwoRoutinesDeep()
    begin
        Outer();
    end;

    [Test]
    procedure ReadsTheUndefinedDate()
    var
        a: Integer; d: Date;
    begin
        a := 0;
        a := Date2DMY(d, 2);
    end;

    [Test]
    procedure AssertErrorIsCaught_ThenSecondErrorEscapes()
    begin
        asserterror Error('first, caught');
        Error('second, escapes');
    end;

    [Test]
    procedure NeedsAnUnimplementedBuiltin()
    var
        v: Text;
    begin
        v := Database.SerialNumber();
    end;

    [Test]
    procedure Passes()
    begin
    end;

    local procedure Outer()
    begin
        Inner();
    end;

    local procedure Inner()
    begin
        Error('boom from Inner');
    end;
}
"#;

/// For each test: its outcome and the detail `--explain` would print.
fn run() -> (al_ir::Program, Vec<(String, Outcome, explain::TestDetail)>) {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("src/t.al".into(), SRC.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let mut details = Vec::new();
    let mut observe = |name: &str, _: &Outcome, seen: &Observed| {
        let mut d = explain::detail(
            &l.program,
            seen.session.fault_site(),
            seen.error.map(|e| ErrorInfo {
                message: e.message.clone(),
                code: e.code.clone(),
                call_stack: e.call_stack.clone(),
            }),
        );
        d.refused_in = seen.refused_in.map(|(o, r)| explain::describe_refused(&l, o, r));
        details.push((name.to_string(), d));
    };
    let observer: &mut Observer = &mut observe;
    let outcomes = run_codeunit_observed(&engine, &l, oi, None, TestIsolation::Codeunit, Some(observer));
    let rows = outcomes
        .into_iter()
        .zip(details)
        .map(|((name, outcome), (n, d))| {
            assert_eq!(name, n);
            (name, outcome, d)
        })
        .collect();
    ((*l.program).clone(), rows)
}

fn of<'a>(
    rows: &'a [(String, Outcome, explain::TestDetail)],
    name: &str,
) -> &'a (String, Outcome, explain::TestDetail) {
    rows.iter().find(|r| r.0 == name).unwrap()
}

#[test]
fn a_failure_names_the_error_and_every_routine_it_left_with_its_line() {
    let (_, rows) = run();
    let (_, outcome, d) = of(&rows, "FailsTwoRoutinesDeep");
    assert_eq!(outcome, &Outcome::Fail("boom from Inner".into()));
    assert_eq!(d.error.as_ref().unwrap().message, "boom from Inner");
    let who: Vec<&str> = d.raised_in.iter().map(|l| l.who.as_str()).collect();
    assert_eq!(
        who,
        [
            "codeunit 63100 \"T\".Inner",
            "codeunit 63100 \"T\".Outer",
            "codeunit 63100 \"T\".FailsTwoRoutinesDeep"
        ]
    );
    // The statement that was running in each routine, innermost first (the file starts with a
    // newline, so the `codeunit` line is line 2).
    let at: Vec<&str> = d.raised_in.iter().map(|l| l.at.as_str()).collect();
    assert_eq!(at, ["a: src/t.al:48:9", "a: src/t.al:43:9", "a: src/t.al:9:9"]);
    let text = explain::render("head", outcome, "passes", Some(d));
    assert!(
        text.contains("runtime: FAIL")
            && text.contains("error: boom from Inner")
            && text.contains("raised in (innermost first):"),
        "{text}"
    );
}

#[test]
fn a_refusal_while_running_names_the_statement() {
    let (_, rows) = run();
    let (_, outcome, d) = of(&rows, "ReadsTheUndefinedDate");
    assert!(
        matches!(outcome, Outcome::NotRun(r) if r.contains("Date2DMY of an undefined date")),
        "{outcome:?}"
    );
    assert_eq!(d.refused_in, None);
    assert_eq!(d.raised_in.len(), 1);
    assert_eq!(d.raised_in[0].who, "codeunit 63100 \"T\".ReadsTheUndefinedDate");
    assert_eq!(d.raised_in[0].at, "a: src/t.al:18:9");
    let text = explain::render("head", outcome, "passes", Some(d));
    assert!(
        text.contains("refused while running, in:") && text.contains("src/t.al:18:9"),
        "{text}"
    );
}

#[test]
fn an_error_that_was_caught_does_not_stand_for_a_later_one() {
    let (_, rows) = run();
    let (_, outcome, d) = of(&rows, "AssertErrorIsCaught_ThenSecondErrorEscapes");
    // ReadsTheUndefinedDate, which runs before it, was refused while it ran, but its reach changes nothing this test
    // reads (only locals), so the failure is trusted (runtime/written.rs); the error is the second one, and
    // `--explain` shows where it was raised.
    assert_eq!(outcome, &Outcome::Fail("second, escapes".into()), "{outcome:?}");
    assert_eq!(d.raised_in.len(), 1);
    assert_eq!(
        d.raised_in[0].at, "a: src/t.al:25:9",
        "the second Error, not the asserterror line"
    );
}

#[test]
fn a_reason_found_before_the_run_names_the_statement_it_came_from() {
    let (_, rows) = run();
    let (_, outcome, d) = of(&rows, "NeedsAnUnimplementedBuiltin");
    assert!(
        matches!(outcome, Outcome::NotRun(r) if r.contains("SerialNumber")),
        "{outcome:?}"
    );
    let site = d.refused_in.as_ref().expect("a lowering reason has a routine");
    assert_eq!(site.who, "codeunit 63100 \"T\".NeedsAnUnimplementedBuiltin");
    // al-runner issue 6061: the statement of the refused call, not the routine header.
    assert_eq!(site.at, "a: src/t.al:33:9 (the statement the refusal came from)");
    assert!(d.raised_in.is_empty());
}

#[test]
fn a_passing_test_leaves_no_site_and_rendering_says_so() {
    let (_, rows) = run();
    let (_, outcome, d) = of(&rows, "Passes");
    assert_eq!(outcome, &Outcome::Pass);
    assert!(d.raised_in.is_empty() && d.error.is_none());
    assert!(explain::render("head", outcome, "passes", Some(d)).contains("runtime: PASS"));
}

#[test]
fn blocked_site_finds_the_routine_a_reason_sits_in_through_the_call_graph() {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![(
                "f.al".into(),
                "codeunit 1 C { [Test] procedure T() begin Helper(); end; local procedure Helper() var v: Text; begin v := Database.SerialNumber(); end; }".into(),
            )],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty());
    let t = l.program.objects[0]
        .routines
        .iter()
        .position(|r| r.name == "T")
        .unwrap();
    let helper = l.program.objects[0]
        .routines
        .iter()
        .position(|r| r.name == "Helper")
        .unwrap();
    let b = blocked_site(&l, 0, t).expect("blocked through Helper");
    assert_eq!((b.object, b.routine), (0, helper));
    assert!(b.text.starts_with("C.Helper: "), "{}", b.text);
}
