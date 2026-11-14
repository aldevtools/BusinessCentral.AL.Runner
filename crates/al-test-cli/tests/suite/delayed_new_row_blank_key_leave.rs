//! al-runner issue 6924 (`Price Formula Test PFIFC` of an outside reporter): a TestPage on a List page with
//! DelayedInsert opens with OpenEdit, `New()`, a SetValue on a field that is not the key, then `Next()` while the
//! NotBlank primary key was never entered, then `GetValidationError()`. On BC the text contains
//! 'code must be filled in.' (the reporter's run). The pinned corpus does not measure leaving such a row, so the
//! runner refuses the step instead of answering it.

use al_test_cli::report::Outcome;
use al_testkit::outcomes_in;

fn src(key_type: &str, not_blank: bool, collect: bool, set_value: bool, step: &str) -> String {
    src_on(key_type, not_blank, true, collect, set_value, step)
}

fn src_on(key_type: &str, not_blank: bool, delayed: bool, collect: bool, set_value: bool, step: &str) -> String {
    let di = if delayed { "DelayedInsert = true;" } else { "" };
    let nb = if not_blank { "NotBlank = true;" } else { "" };
    let eb = if collect {
        "[ErrorBehavior(ErrorBehavior::Collect)]"
    } else {
        ""
    };
    let sv = if set_value { "P.DescF.SetValue('abc');" } else { "" };
    format!(
        r#"
table 99700 "LT NB Row"
{{
    fields {{ field(1; "Code"; {key_type}) {{ {nb} }} field(2; Description; Text[50]) {{ }} }}
    keys {{ key(PK; "Code") {{ Clustered = true; }} }}
}}

page 99701 "LT NB List"
{{
    PageType = List;
    SourceTable = "LT NB Row";
    {di}
    layout {{ area(Content) {{ repeater(R) {{ field(CodeF; Rec."Code") {{ }} field(DescF; Rec.Description) {{ }} }} }} }}
    trigger OnOpenPage() begin CurrPage.Editable(true); end;
}}

codeunit 99702 T
{{
    Subtype = Test;

    [Test]
    {eb}
    procedure LeaveNewRow()
    var
        P: TestPage "LT NB List";
        ErrorTxt: Text;
    begin
        P.OpenEdit();
        P.New();
        {sv}
        {step}
        ErrorTxt := P.GetValidationError();
        if not ErrorTxt.ToLower().Contains('code must be filled in.') then
            Error('Error is not correct: %1', ErrorTxt);
    end;
}}
"#
    )
}

fn outcome(src: &str) -> Outcome {
    let o = outcomes_in(src, "T");
    assert_eq!(o.len(), 1, "{o:?}");
    o.into_iter().next().unwrap().1
}

fn assert_refused(o: &Outcome) {
    match o {
        Outcome::NotRun(r) => assert!(r.contains("DelayedInsert"), "{r}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// The reporter's shape: refused, not failed.
#[test]
fn reporter_shape_is_refused() {
    assert_refused(&outcome(&src("Code[20]", true, true, true, "P.Next();")));
}

/// The same without ErrorBehavior::Collect: refused too.
#[test]
fn without_collect_is_refused() {
    assert_refused(&outcome(&src("Code[20]", true, false, true, "P.Next();")));
}

/// Previous and Close off the same row: refused too.
#[test]
fn previous_and_close_are_refused() {
    assert_refused(&outcome(&src("Code[20]", true, true, true, "P.Previous();")));
    assert_refused(&outcome(&src("Code[20]", true, true, true, "P.Close(); exit;")));
}

/// An Integer key with NotBlank: refused as the Code key is.
#[test]
fn integer_key_is_refused() {
    assert_refused(&outcome(&src("Integer", true, true, true, "P.Next();")));
}

fn assert_main_kept(o: &Outcome) {
    // Main inserts the row and GetValidationError reads an empty text: the check in the test fails, nothing refuses.
    assert!(
        matches!(o, Outcome::Fail(m) if m.contains("Error is not correct")),
        "{o:?}"
    );
}

/// Neighbors outside the refused shape keep main's outcome: a key field without NotBlank, a page without DelayedInsert (the
/// row is inserted at the SetValue), a key that was typed, and a new row nothing was typed on.
#[test]
fn neighbors_keep_main_outcome() {
    assert_main_kept(&outcome(&src("Code[20]", false, true, true, "P.Next();")));
    assert_main_kept(&outcome(&src_on("Code[20]", true, false, true, true, "P.Next();")));
    assert_main_kept(&outcome(&src(
        "Code[20]",
        true,
        true,
        true,
        "P.CodeF.SetValue('A'); P.Next();",
    )));
    assert_main_kept(&outcome(&src("Code[20]", true, true, false, "P.Next();")));
}
