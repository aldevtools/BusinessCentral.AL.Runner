//! `ApplicationArea := <text>` is the call `ApplicationArea(<text>)` (al-runner issue 7597, the reporter's repro: BC 29
//! compiles and runs both, and `ApplicationArea()` reads the assigned text). The call form is pinned by
//! pages/application-area/TestPageApplicationAreaControlRemoval.al:87-93; the assignment form marks assumption
//! `application-area-assign-is-call` until the corpus probe lands.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
codeunit 50100 "Repro AA"
{
    Subtype = Test;

    [Test]
    procedure A1_SetterAssignment()
    begin
        ApplicationArea := '#All,#Basic';
        Error('RESULT A1 setter assignment: [%1]', ApplicationArea());
    end;

    [Test]
    procedure A2_SetterAppend()
    begin
        ApplicationArea('#All');
        ApplicationArea := ApplicationArea() + ',#Basic';
        Error('RESULT A2 setter append: [%1]', ApplicationArea());
    end;

    [Test]
    procedure A3_CallForm()
    begin
        ApplicationArea('#All,#Basic');
        Error('RESULT A3 call form: [%1]', ApplicationArea());
    end;
}
"#;

#[test]
fn assigning_the_application_area_calls_its_setter() {
    let got = outcomes_in(SRC, "Repro AA");
    let want = |name: &str, text: &str| (name.to_string(), Outcome::Fail(text.to_string()));
    assert_eq!(
        got,
        vec![
            want("A1_SetterAssignment", "RESULT A1 setter assignment: [#All,#Basic]"),
            want("A2_SetterAppend", "RESULT A2 setter append: [#All,#Basic]"),
            want("A3_CallForm", "RESULT A3 call form: [#All,#Basic]"),
        ]
    );
}

const MARKED: &str = r#"
codeunit 50101 M
{
    procedure Assigned(): Text
    begin
        ApplicationArea := '#All,#Basic';
        exit(ApplicationArea());
    end;

    procedure Called(): Text
    begin
        ApplicationArea('#All,#Basic');
        exit(ApplicationArea());
    end;
}
"#;

fn hits() -> Vec<&'static str> {
    al_testrun::assumptions::take_hits()
        .iter()
        .map(|a| a.info().id)
        .collect()
}

#[test]
fn only_the_assignment_form_marks_the_assumption() {
    al_testrun::assumptions::clear_hits();
    assert_eq!(
        al_testkit::text(al_testkit::run_in(MARKED, "M", "Called")),
        "#All,#Basic"
    );
    assert!(!hits().contains(&"application-area-assign-is-call"));
    assert_eq!(
        al_testkit::text(al_testkit::run_in(MARKED, "M", "Assigned")),
        "#All,#Basic"
    );
    assert!(hits().contains(&"application-area-assign-is-call"));
}
