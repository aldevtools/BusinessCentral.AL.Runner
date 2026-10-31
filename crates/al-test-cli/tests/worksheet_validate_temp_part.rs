//! A Worksheet without a source table whose field's OnValidate hands the new value to its unlinked ListPart (a temporary
//! source table filled in the host's OnOpenPage) and calls `CurrPage.Update(false)`: the part's next First shows the value its
//! OnAfterGetRecord computes from the new value, `(A)` (al-runner issue 6465, the reporter's repro; BC 29: the test passes).

use al_testkit::outcomes_in;

const SRC: &str = r##"
enum 50100 "Repro Choice"
{
    Extensible = false;
    value(0; Initial) { }
    value(1; Changed) { }
}

table 50100 "Repro Row"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; Description; Text[30]) { }
    }
    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

page 50101 "Repro Rows"
{
    PageType = ListPart;
    SourceTable = "Repro Row";
    SourceTableTemporary = true;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(Summary; Summary)
                {
                    ApplicationArea = All;
                }
            }
        }
    }
    trigger OnAfterGetRecord()
    begin
        Summary := Rec.Description;
        if (Summary <> '') and (CurrentChoice = CurrentChoice::Changed) then
            Summary := '(' + Summary + ')';
    end;
    procedure SetChoice(NewChoice: Enum "Repro Choice")
    begin
        CurrentChoice := NewChoice;
    end;
    procedure SetRows()
    begin
        Rec.DeleteAll();
        Rec.Init();
        Rec."No." := 1;
        Rec.Description := 'A';
        Rec.Insert();
    end;
    var
        CurrentChoice: Enum "Repro Choice";
        Summary: Text[100];
}

page 50102 "Repro Worksheet"
{
    PageType = Worksheet;
    layout
    {
        area(Content)
        {
            group(Header)
            {
                field(Choice; CurrentChoice)
                {
                    ApplicationArea = All;
                    trigger OnValidate()
                    begin
                        CurrPage.Rows.Page.SetChoice(CurrentChoice);
                        CurrPage.Update(false);
                    end;
                }
            }
            part(Rows; "Repro Rows")
            {
                ApplicationArea = All;
            }
        }
    }
    trigger OnOpenPage()
    begin
        CurrPage.Rows.Page.SetChoice(CurrentChoice);
        CurrPage.Rows.Page.SetRows();
    end;
    var
        CurrentChoice: Enum "Repro Choice";
}

codeunit 50103 "Repro Part Refresh Tests"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('WorksheetHandler')]
    procedure PartValueAfterParentValidate()
    var
        Worksheet: Page "Repro Worksheet";
    begin
        Worksheet.RunModal();
    end;
    [ModalPageHandler]
    procedure WorksheetHandler(var Worksheet: TestPage "Repro Worksheet")
    begin
        Worksheet.Rows.First();
        if Worksheet.Rows.Summary.Value <> 'A' then
            Error('Before: expected A, got %1', Worksheet.Rows.Summary.Value);
        Worksheet.Choice.SetValue('Changed');
        Worksheet.Rows.First();
        if Worksheet.Rows.Summary.Value <> '(A)' then
            Error('After: expected (A), got %1', Worksheet.Rows.Summary.Value);
        Worksheet.OK().Invoke();
    end;
}
"##;

#[test]
fn a_temporary_part_shows_the_value_its_host_validate_handed_it() {
    let o = outcomes_in(SRC, "Repro Part Refresh Tests");
    let got = format!(
        "{:?}",
        o.iter()
            .find(|(n, _)| n == "PartValueAfterParentValidate")
            .map(|(_, x)| x)
    );
    assert_eq!(got, "Some(Pass)");
}
