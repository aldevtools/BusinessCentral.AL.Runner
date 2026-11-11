//! al-runner issue 7783, a neighbor of its shape (al-runner PR 7787 review): on a Worksheet that Page.RunModal runs and a
//! ModalPageHandler drives, a SetValue after Next() does not read the row first (the read is limited to a page the test opened
//! with OpenEdit, `held_row_read_pinned` in al-vm pages/next_held_row_read.rs).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 69970 "HRN Line"
{
    fields { field(1; Id; Integer) { } field(2; Stored; Integer) { } }
    keys { key(PK; Id) { } }
}
codeunit 69971 "HRN Log"
{
    SingleInstance = true;
    var L: Text;
    procedure Add(T: Text) begin L += T + ' '; end;
    procedure Get(): Text begin exit(L); end;
    procedure Clear() begin L := ''; end;
}
page 69972 "HRN Worksheet"
{
    PageType = Worksheet;
    SourceTable = "HRN Line";
    InsertAllowed = false;
    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Id; Rec.Id) { }
                field(TermCtrl; Term)
                {
                    trigger OnValidate()
                    begin
                        Log.Add(StrSubstNo('V:%1/%2', Rec.Id, Term));
                        TempStore.Stored := Term;
                        TempStore.Modify(false);
                        CurrPage.Update(false);
                    end;
                }
            }
        }
    }
    trigger OnAfterGetRecord()
    begin
        Log.Add(StrSubstNo('R:%1', Rec.Id));
        if not TempStore.Get(Rec.Id) then begin
            TempStore.Init();
            TempStore.Id := Rec.Id;
            TempStore.Insert();
        end;
        Term := TempStore.Stored;
    end;
    var
        TempStore: Record "HRN Line" temporary;
        Log: Codeunit "HRN Log";
        Term: Integer;
}
codeunit 69973 T
{
    Subtype = Test;
    var Log: Codeunit "HRN Log";

    [Test] [HandlerFunctions('Modal')] procedure TermAfterNextInModalHandler()
    var L: Record "HRN Line"; I: Integer;
    begin
        for I := 1 to 3 do begin L.Id := I; L.Insert(); end;
        Page.RunModal(Page::"HRN Worksheet");
        if Log.Get() <> 'V:2/5 R:2 ' then Error('got [%1]', Log.Get());
    end;
    [ModalPageHandler] procedure Modal(var Sheet: TestPage "HRN Worksheet")
    begin
        Sheet.First(); Sheet.Next(); Log.Clear();
        Sheet.TermCtrl.SetValue(5); Sheet.OK().Invoke();
    end;
}
"#;

#[test]
fn set_value_after_next_in_a_modal_handler_keeps_the_earlier_behavior() {
    let out = outcomes_in(SRC, "T");
    assert!(matches!(out[0].1, Outcome::Pass), "{out:#?}");
}
