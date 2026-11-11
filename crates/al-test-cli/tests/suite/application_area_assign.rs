//! `ApplicationArea := <text>` is the call `ApplicationArea(<text>)` (al-runner issue 7597, the reporter's repro: BC 29
//! compiles and runs both, and `ApplicationArea()` reads the assigned text). The call form is pinned by
//! pages/application-area/TestPageApplicationAreaControlRemoval.al:87-93, the assignment form by
//! pages/application-area/ApplicationAreaAssignForm.al:36-107. This keeps the reporter's repro unchanged.

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
