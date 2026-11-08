//! CurrPage.Update(false) from a field's OnAssistEdit on a Card that declares OnNextRecord and OnAfterGetCurrRecord but no
//! OnFindRecord keeps the card on its row (al-runner issue 7268). Pinned by corpus pull request 1199 (measured on real BC 27.5,
//! 28.5 and 29.0): pages/currpage/UpdateOnFindRecord.al:357 CurrPage_Update_CardWithOnlyOnNextRecord_KeepsCurrentRow
//! (page :376). Only the row is pinned; the trigger order of the refresh is an assumption. A list, and a card with unsaved
//! edits, stay refused.

const SRC: &str = r#"
table 74821 "LT P168 Upd Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { Clustered = true; } } }
page 69998 "PAG Next Only Update Card"
{
    PageType = Card;
    SourceTable = "LT P168 Upd Row";
    layout
    {
        area(Content)
        {
            field(IdF; Rec.Id)
            {
                ApplicationArea = All;
                trigger OnAssistEdit()
                begin
                    CurrPage.Update(false);
                end;
            }
        }
    }
    trigger OnNextRecord(Steps: Integer): Integer
    begin
        exit(Rec.Next(Steps));
    end;
    trigger OnAfterGetCurrRecord()
    begin
        SeenId := Rec.Id;
    end;
    var
        SeenId: Integer;
}
codeunit 69999 C
{
    procedure F(): Text
    var
        Row: Record "LT P168 Upd Row";
        Card: TestPage "PAG Next Only Update Card";
        I: Integer;
        s: Text;
    begin
        for I := 1 to 3 do begin
            Row.Id := I;
            Row.Insert(false);
        end;
        Card.OpenEdit();
        Card.GoToKey(2);
        Card.IdF.AssistEdit();
        s := Card.IdF.Value();
        Card.Close();
        exit(s);
    end;
}
"#;

#[test]
fn update_from_assist_edit_keeps_the_card_on_its_row() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "F")), "2");
}
