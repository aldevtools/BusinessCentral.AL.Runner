//! CurrPage.Update(true) or Update() called by a procedure of a Page variable that never ran, after the procedure set the fields
//! of a new record (the Base Application "Table Filter" page fills its lines this way; al-runner issue 6951). On BC it inserts
//! the record (OnInsertRecord, the table's OnInsert, AutoSplitKey numbers 10000, 20000): corpus branch
//! al-runtime-rs-staging-r6951 format-evaluate/tablefilter-neutral/TableFilterEnumForms.al
//! UpdateNeverRunPage_NewRecord_UpdateTrue, _UpdateNoArgument. Update(false) and no Update insert nothing (_UpdateFalse,
//! _NoUpdate); with a blank record Update(true) does nothing (pages/currpage/UnopenedCurrPage.al Direct_Part_UpdateTrue).
//! The insert is not implemented: refused, not run silently.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

fn outcome(mode: &str, set_fields: bool) -> Outcome {
    let set = if set_fields {
        "Rec.\"Field Number\" := 1; Rec.\"Field Filter\" := 'A..B';"
    } else {
        ""
    };
    let src = format!(
        r#"
table 62970 "UN Line" {{ fields {{ field(1; "Table Number"; Integer) {{ }} field(2; "Line No."; Integer) {{ }} field(3; "Field Number"; Integer) {{ }} field(4; "Field Filter"; Text[250]) {{ }} }}
    keys {{ key(PK; "Table Number", "Line No.") {{ Clustered = true; }} }} }}
page 62971 "UN Lines" {{
    PageType = List; SourceTable = "UN Line"; SourceTableTemporary = true; AutoSplitKey = true; DelayedInsert = true;
    layout {{ area(Content) {{ repeater(R) {{ field("Field Number"; Rec."Field Number") {{ }} }} }} }}
    procedure AddLine(Mode: Text)
    begin
        Rec.Init();
        {set}
        case Mode of
            'true': CurrPage.Update(true);
            'false': CurrPage.Update(false);
            'none': CurrPage.Update();
        end;
    end;
    procedure Count(): Integer begin Rec.Reset(); exit(Rec.Count()); end;
}}
codeunit 62972 T
{{
    Subtype = Test;
    [Test] procedure Run()
    var P: Page "UN Lines";
    begin
        P.AddLine('{mode}');
        Error('rows=%1', P.Count());
    end;
}}"#
    );
    outcomes_in(&src, "T")[0].1.clone()
}

#[test]
fn update_true_or_without_argument_over_a_new_record_is_refused() {
    assert!(
        matches!(outcome("true", true), Outcome::NotRun(_)),
        "{:?}",
        outcome("true", true)
    );
    assert!(matches!(outcome("none", true), Outcome::NotRun(_)));
}

#[test]
fn update_false_and_a_blank_record_insert_nothing() {
    assert_eq!(outcome("false", true), Outcome::Fail("rows=0".into()));
    assert_eq!(outcome("skip", true), Outcome::Fail("rows=0".into()));
    assert_eq!(outcome("true", false), Outcome::Fail("rows=0".into()));
}
