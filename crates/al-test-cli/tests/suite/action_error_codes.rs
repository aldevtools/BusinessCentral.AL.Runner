//! The code an error of a TestPage action's OnAction carries after `Invoke`: the test framework wraps it
//! (corpus errors/lt-az-error-texts/AzErrorTexts.al:139-185, PageAction_ErrorCodes and PageAction_DuplicateInsert).
//! A code outside the pinned set has no pinned wrapped form: reading it refuses the test.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 69990 "AE Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[20]) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
page 69991 "AE Card"
{
    PageType = Card; SourceTable = "AE Row"; UsageCategory = None;
    layout { area(Content) { field(Id; Rec.Id) { ApplicationArea = All; } field(Name; Rec.Name) { ApplicationArea = All; } } }
    actions
    {
        area(Processing)
        {
            action(DbError) { ApplicationArea = All; trigger OnAction() var Row: Record "AE Row"; begin Row.Get(-77); end; }
            action(DupInsert) { ApplicationArea = All; trigger OnAction() var Row: Record "AE Row"; begin Row.Id := Rec.Id; Row.Insert(); end; }
            action(TestFieldError) { ApplicationArea = All; trigger OnAction() begin Rec.TestField(Name); end; }
            action(PlainError) { ApplicationArea = All; trigger OnAction() begin Error('ae plain'); end; }
            action(OtherCode) { ApplicationArea = All; trigger OnAction() var I: Integer; begin I := 1 div 0; end; }
        }
    }
}
codeunit 69992 T
{
    Subtype = Test;
    local procedure Check(Want: Text; Got: Text) begin if Want <> Got then Error('want [%1] got [%2]', Want, Got); end;
    local procedure Open(var Card: TestPage "AE Card")
    var Row: Record "AE Row";
    begin
        Row.DeleteAll(); Row.Id := 1; Row.Insert(); Commit();
        Card.OpenEdit(); Card.GoToRecord(Row);
    end;
    [Test] procedure Codes()
    var Card: TestPage "AE Card";
    begin
        Open(Card);
        asserterror Card.DbError.Invoke();
        Check('TestWrapped:CSideRecordNotFound', GetLastErrorCode());
        Check('The AE Row does not exist. Identification fields and values: Id=''-77''', GetLastErrorText());
        asserterror Card.TestFieldError.Invoke();
        Check('TestWrapped:TestField', GetLastErrorCode());
        asserterror Card.PlainError.Invoke();
        Check('TestWrapped:Dialog', GetLastErrorCode());
        asserterror Card.DupInsert.Invoke();
        Check('TestWrapped:CSideDuplicateKey', GetLastErrorCode());
        Check('The record in table AE Row already exists. Identification fields and values: Id=''1''', GetLastErrorText());
        Card.Close();
    end;
    [Test] procedure OtherCodeRefused()
    var Card: TestPage "AE Card";
    begin
        Open(Card);
        asserterror Card.OtherCode.Invoke();
        Check('x', GetLastErrorCode());
    end;
}
"#;

#[test]
fn action_errors_carry_the_wrapped_codes() {
    let out = outcomes_in(SRC, "T");
    let get = |n: &str| out.iter().find(|(t, _)| t == n).map(|(_, o)| o.clone()).unwrap();
    assert_eq!(get("Codes"), Outcome::Pass);
    assert!(
        matches!(get("OtherCodeRefused"), Outcome::NotRun(_)),
        "{:?}",
        get("OtherCodeRefused")
    );
}
