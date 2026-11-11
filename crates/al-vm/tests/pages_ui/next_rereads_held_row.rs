//! al-runner issue 7783: after `TestPage.Next()` onto a row the list already holds (no read trigger runs), a SetValue on a control
//! of that row first reads the row again, so OnAfterGetRecord sets the page's per-line variables before the control's OnValidate
//! (Microsoft's Subscription Billing test `Contract Renewal Test.RenewalTermEnteredValueIsRetainedOnPage`).

use al_testkit::{run_in, text};

/// The reporter's objects from the issue, unchanged; the test procedure returns the text its `Error` calls would raise.
const SRC: &str = r#"
table 50170 "R918 DF Line"
{
    fields { field(1; Id; Integer) { } field(2; Stored; DateFormula) { } }
    keys { key(PK; Id) { } }
}

page 50170 "R918 DF Worksheet"
{
    PageType = Worksheet;
    SourceTable = "R918 DF Line";
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
        if not TempStore.Get(Rec.Id) then begin
            TempStore.Init();
            TempStore.Id := Rec.Id;
            TempStore.Insert();
        end;
        Term := TempStore.Stored;
    end;
    var
        TempStore: Record "R918 DF Line" temporary;
        Term: DateFormula;
}

codeunit 50171 "R918 DF Tests"
{
    procedure TermEnteredOnEachLineIsKept(): Text
    var
        L: Record "R918 DF Line";
        Sheet: TestPage "R918 DF Worksheet";
        Entered: DateFormula;
        Shown: DateFormula;
    begin
        L.Id := 1; L.Insert();
        L.Id := 2; L.Insert();
        Evaluate(Entered, '<17D>');

        Sheet.OpenEdit();
        Sheet.First();
        Sheet.TermCtrl.SetValue(Entered);
        Evaluate(Shown, Sheet.TermCtrl.Value());
        if Shown <> Entered then exit(StrSubstNo('first line shows "%1"', Shown));

        Sheet.Next();
        Sheet.TermCtrl.SetValue(Entered);
        Evaluate(Shown, Sheet.TermCtrl.Value());
        if Shown <> Entered then exit(StrSubstNo('next line shows "%1"', Shown));
        exit('ok');
    end;
}
"#;

/// The reporter's repro; the path is the assumption `next-held-row-read-before-set-value` (flagged pass).
#[test]
fn set_value_after_next_reads_the_held_row_first() {
    use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
    clear_hits();
    assert_eq!(text(run_in(SRC, "R918 DF Tests", "TermEnteredOnEachLineIsKept")), "ok");
    assert!(take_hits().contains(&Assumption::NextHeldRowReadBeforeSetValue));
}
