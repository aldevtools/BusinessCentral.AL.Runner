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

/// A Worksheet like the reporter's, with a Rec field control, logging each read (`R:<id>`) and each OnValidate (`V:<id>/<term>`).
const NEIGHBORS: &str = r#"
table 50172 "R918 NB Line"
{
    fields { field(1; Id; Integer) { } field(2; Stored; Integer) { } field(3; Name; Text[30]) { } }
    keys { key(PK; Id) { } }
}
codeunit 50173 "R918 NB Log"
{
    SingleInstance = true;
    var L: Text;
    procedure Add(T: Text) begin L += T + ' '; end;
    procedure Get(): Text begin exit(L); end;
    procedure Clear() begin L := ''; end;
}
page 50172 "R918 NB Worksheet"
{
    PageType = Worksheet;
    SourceTable = "R918 NB Line";
    InsertAllowed = false;
    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Id; Rec.Id) { }
                field(NameCtrl; Rec.Name) { }
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
    procedure Bump() begin Log.Add(StrSubstNo('B:%1', Rec.Id)); Term := 99; end;
    var
        TempStore: Record "R918 NB Line" temporary;
        Log: Codeunit "R918 NB Log";
        Term: Integer;
}
codeunit 50174 "R918 NB Tests"
{
    var Log: Codeunit "R918 NB Log";

    local procedure Seed()
    var L: Record "R918 NB Line";
    begin
        L.DeleteAll();
        L.Id := 1; L.Name := 'a'; L.Insert();
        L.Id := 2; L.Name := 'b'; L.Insert();
        L.Id := 3; L.Name := 'c'; L.Insert();
    end;

    local procedure Report(): Text
    var L: Record "R918 NB Line"; T: Text;
    begin
        if L.FindSet() then repeat T += StrSubstNo('%1=%2 ', L.Id, L.Name); until L.Next() = 0;
        exit(Log.Get() + '| DB ' + T);
    end;

    procedure NameThenTermAfterNext(): Text
    var Sheet: TestPage "R918 NB Worksheet";
    begin
        Seed();
        Sheet.OpenEdit(); Sheet.First(); Sheet.Next();
        Log.Clear();
        Sheet.NameCtrl.SetValue('X');
        Sheet.TermCtrl.SetValue(5);
        Sheet.Close();
        exit(Report());
    end;

    procedure TermAfterNextNextPrevious(): Text
    var Sheet: TestPage "R918 NB Worksheet";
    begin
        Seed();
        Sheet.OpenEdit(); Sheet.First(); Sheet.Next(); Sheet.Next(); Sheet.Previous();
        Log.Clear();
        Sheet.TermCtrl.SetValue(5);
        Sheet.Close();
        exit(Report());
    end;

    procedure TermAfterNextGoToKey(): Text
    var Sheet: TestPage "R918 NB Worksheet";
    begin
        Seed();
        Sheet.OpenEdit(); Sheet.First(); Sheet.Next(); Sheet.GoToKey(3);
        Log.Clear();
        Sheet.TermCtrl.SetValue(5);
        Sheet.Close();
        exit(Report());
    end;

    procedure TermAfterNextFirst(): Text
    var Sheet: TestPage "R918 NB Worksheet";
    begin
        Seed();
        Sheet.OpenEdit(); Sheet.First(); Sheet.Next(); Sheet.First();
        Log.Clear();
        Sheet.TermCtrl.SetValue(5);
        Sheet.Close();
        exit(Report());
    end;

    procedure TermAfterNextOnTheOnlyRow(): Text
    var Sheet: TestPage "R918 NB Worksheet"; L: Record "R918 NB Line";
    begin
        Seed();
        L.SetFilter(Id, '>1'); L.DeleteAll();
        Sheet.OpenEdit(); Sheet.First(); Sheet.Next();
        Log.Clear();
        Sheet.TermCtrl.SetValue(5);
        Sheet.Close();
        exit(Report());
    end;

    procedure TermAfterLastNext(): Text
    var Sheet: TestPage "R918 NB Worksheet";
    begin
        Seed();
        Sheet.OpenEdit(); Sheet.Last(); Sheet.Next();
        Log.Clear();
        Sheet.TermCtrl.SetValue(5);
        Sheet.Close();
        exit(Report());
    end;

    procedure TermAfterNextRowModifiedElsewhere(): Text
    var Sheet: TestPage "R918 NB Worksheet"; L: Record "R918 NB Line";
    begin
        Seed();
        Sheet.OpenEdit(); Sheet.First(); Sheet.Next();
        Log.Clear();
        L.Get(2); L.Name := 'Z'; L.Modify();
        Sheet.TermCtrl.SetValue(5);
        Log.Add('name=' + Sheet.NameCtrl.Value());
        Sheet.Close();
        exit(Report());
    end;

    procedure TermAfterNextRowDeletedElsewhere(): Text
    var Sheet: TestPage "R918 NB Worksheet"; L: Record "R918 NB Line";
    begin
        Seed();
        Sheet.OpenEdit(); Sheet.First(); Sheet.Next();
        Log.Clear();
        L.Get(2); L.Delete();
        Sheet.TermCtrl.SetValue(5);
        Sheet.Close();
        exit(Report());
    end;

    procedure TermAfterNextAndControlRead(): Text
    var Sheet: TestPage "R918 NB Worksheet";
    begin
        Seed();
        Sheet.OpenEdit(); Sheet.First(); Sheet.Next();
        Log.Clear();
        Log.Add('name=' + Sheet.NameCtrl.Value());
        Sheet.TermCtrl.SetValue(5);
        Sheet.Close();
        exit(Report());
    end;

    procedure TermAfterNextThatSavedARow(): Text
    var Sheet: TestPage "R918 NB Worksheet";
    begin
        Seed();
        Sheet.OpenEdit(); Sheet.First(); Sheet.NameCtrl.SetValue('X'); Sheet.Next();
        Log.Clear();
        Sheet.TermCtrl.SetValue(5);
        Sheet.Close();
        exit(Report());
    end;

    procedure TermAfterNextOnTrappedRun(): Text
    var Sheet: TestPage "R918 NB Worksheet"; Pg: Page "R918 NB Worksheet";
    begin
        Seed();
        Sheet.Trap(); Pg.Run(); Sheet.First(); Sheet.Next();
        Log.Clear();
        Sheet.TermCtrl.SetValue(5);
        Sheet.Close();
        exit(Report());
    end;

    procedure TermAfterNextAndPageVariableCall(): Text
    var Sheet: TestPage "R918 NB Worksheet"; Pg: Page "R918 NB Worksheet";
    begin
        Seed();
        Sheet.Trap(); Pg.Run(); Sheet.First(); Sheet.Next();
        Log.Clear();
        Pg.Bump();
        Sheet.TermCtrl.SetValue(5);
        Sheet.Close();
        exit(Report());
    end;
}
"#;

/// The neighbors of the reporter's shape keep the earlier behavior: no read before the SetValue, and the assumption is not hit.
/// A Rec field changed on the row first (al-runner PR 7787 review: the read cleared the change, and Close no longer saved `X`);
/// a Previous, a GoToKey or a First after the Next (the read ran on a row no Next had reached); a Next that did not move (one
/// row, or after Last); the row modified or deleted through another record variable after the Next; a control read between the Next
/// and the SetValue; a Next that saved the row it left; a page a Page variable ran
/// and the test trapped, with and without a procedure call through that variable after the Next.
#[test]
fn set_value_neighbors_of_the_held_row_read_keep_the_earlier_behavior() {
    use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
    let mut wrong = Vec::new();
    for (routine, want) in [
        ("NameThenTermAfterNext", "V:2/5 R:2 | DB 1=a 2=X 3=c "),
        ("TermAfterNextNextPrevious", "V:2/5 R:2 | DB 1=a 2=b 3=c "),
        ("TermAfterNextGoToKey", "V:3/5 R:3 | DB 1=a 2=b 3=c "),
        ("TermAfterNextFirst", "V:1/5 R:1 | DB 1=a 2=b 3=c "),
        ("TermAfterNextOnTheOnlyRow", "V:1/5 R:1 | DB 1=a "),
        ("TermAfterLastNext", "V:3/5 R:3 | DB 1=a 2=b 3=c "),
        (
            "TermAfterNextRowModifiedElsewhere",
            "V:2/5 R:2 name=Z | DB 1=a 2=Z 3=c ",
        ),
        ("TermAfterNextRowDeletedElsewhere", "V:2/5 | DB 1=a 3=c "),
        ("TermAfterNextAndControlRead", "name=b V:2/5 R:2 | DB 1=a 2=b 3=c "),
        ("TermAfterNextThatSavedARow", "V:2/5 R:2 | DB 1=X 2=b 3=c "),
        ("TermAfterNextOnTrappedRun", "V:2/5 R:2 | DB 1=a 2=b 3=c "),
        ("TermAfterNextAndPageVariableCall", "B:0 V:2/5 R:2 | DB 1=a 2=b 3=c "),
    ] {
        clear_hits();
        let got = text(run_in(NEIGHBORS, "R918 NB Tests", routine));
        let read = take_hits().contains(&Assumption::NextHeldRowReadBeforeSetValue);
        if got != want || read {
            wrong.push(format!("{routine}: {got} (assumption hit: {read})"));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
