//! A Worksheet in a key other than the primary key (OnOpenPage calls SetCurrentKey) whose field's OnDrillDown marks rows,
//! turns MarkedOnly on and calls `CurrPage.Update(false)`: the drilldown reads nothing and the page keeps showing the row it
//! stood on; the next GoToRecord first refills the rows around the row Rec was left on, then walks the marked rows (al-runner
//! issue 6008, the reporter's run on real BC 29.0.54011.55407; assumption drilldown-marked-refill-in-its-key until corpus
//! writer SU's pin lands). The fixture is the reporter's repro without Library Assert.

const SRC: &str = r##"table 50100 "Refill Probe Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Code[20]) { }
        field(3; Flag; Boolean) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
        key(ByName; Name) { }
    }
}

codeunit 50101 "Refill Probe Log"
{
    SingleInstance = true;

    var
        Entries: Text;

    procedure Add(Entry: Text)
    begin
        Entries += Entry + ';';
    end;

    procedure Take() Result: Text
    begin
        Result := Entries;
        Entries := '';
    end;
}

codeunit 50104 "Refill Probe Marks"
{
    procedure MarkFlagged(var Row: Record "Refill Probe Row")
    var
        RowsInView: Record "Refill Probe Row";
    begin
        Row.MarkedOnly(false);
        Row.ClearMarks();
        RowsInView.Copy(Row);
        if RowsInView.FindSet() then
            repeat
                if RowsInView.Flag then begin
                    Row.Get(RowsInView.Id);
                    Row.Mark(true);
                end;
            until RowsInView.Next() = 0;
        Row.MarkedOnly(true);
    end;
}

page 50102 "Refill Probe Name List"
{
    PageType = Worksheet;
    SourceTable = "Refill Probe Row";
    InsertAllowed = false;
    DeleteAllowed = false;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(Tiles)
            {
                field(FlaggedTile; Tile)
                {
                    Editable = false;

                    trigger OnDrillDown()
                    begin
                        Marks.MarkFlagged(Rec);
                        CurrPage.Update(false);
                    end;
                }
            }
            repeater(Rows)
            {
                Editable = false;
                field(Id; Rec.Id) { }
                field(Name; Rec.Name) { }
            }
        }
    }

    trigger OnOpenPage()
    begin
        Rec.SetCurrentKey(Name);
    end;

    trigger OnAfterGetRecord()
    begin
        Log.Add('R' + Format(Rec.Id));
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('C' + Format(Rec.Id));
    end;

    var
        Log: Codeunit "Refill Probe Log";
        Marks: Codeunit "Refill Probe Marks";
        Tile: Integer;
}

codeunit 50120 T
{
    var
        Log: Codeunit "Refill Probe Log";

    procedure Reported(): Text
    var
        Row: Record "Refill Probe Row";
        P: TestPage "Refill Probe Name List";
        Observed: Text;
    begin
        SeedRows();
        P.OpenView();
        Observed := 'open=' + P.Id.Value() + ':' + Log.Take();
        P.FlaggedTile.Drilldown();
        Observed += ' drilldown=' + P.Id.Value() + ':' + Log.Take();
        Row.Get(3);
        Observed += ' gotorecord3=' + Format(P.GoToRecord(Row)) + '/' + P.Id.Value() + ':' + Log.Take();
        Row.Get(1);
        Observed += ' gotorecord1=' + Format(P.GoToRecord(Row)) + '/' + P.Id.Value() + ':' + Log.Take();
        P.Close();
        exit(Observed);
    end;

    procedure GoToRecordOfRecsRowFirst()
    var
        Row: Record "Refill Probe Row";
        P: TestPage "Refill Probe Name List";
    begin
        SeedRows();
        P.OpenView();
        P.FlaggedTile.Drilldown();
        Row.Get(1);
        P.GoToRecord(Row);
    end;

    procedure NextAfterTheDrilldown()
    var
        P: TestPage "Refill Probe Name List";
    begin
        SeedRows();
        P.OpenView();
        P.FlaggedTile.Drilldown();
        P.Next();
    end;

    procedure GoToKeyAfterTheDrilldown()
    var
        P: TestPage "Refill Probe Name List";
    begin
        SeedRows();
        P.OpenView();
        P.FlaggedTile.Drilldown();
        P.GoToKey(3);
    end;

    procedure ThreeMarkedRows()
    var
        Row: Record "Refill Probe Row";
        P: TestPage "Refill Probe Name List";
    begin
        SeedRows();
        Row.Get(2);
        Row.Flag := true;
        Row.Modify();
        P.OpenView();
        P.FlaggedTile.Drilldown();
        Row.Get(3);
        P.GoToRecord(Row);
    end;

    local procedure SeedRows()
    var
        Row: Record "Refill Probe Row";
    begin
        Row.DeleteAll();
        AddRow(1, 'N4', true);
        AddRow(2, 'N3', false);
        AddRow(3, 'N2', true);
        AddRow(4, 'N1', false);
        Log.Take();
    end;

    local procedure AddRow(Id: Integer; Name: Code[20]; Flag: Boolean)
    var
        Row: Record "Refill Probe Row";
    begin
        Row.Init();
        Row.Id := Id;
        Row.Name := Name;
        Row.Flag := Flag;
        Row.Insert();
    end;
}
"##;

use al_builtins::assumptions::{Assumption, hit};
use al_builtins::native::Signal;

fn refusal(r: Result<al_builtins::value::Value, Signal>) -> String {
    match r {
        Err(Signal::Unsupported(why)) => why.to_string(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// BC's log, verbatim from the reporter's run on real BC (al-runner issue 6008).
#[test]
fn goto_record_after_a_marked_drilldown_in_another_key_refills_as_bc_does() {
    let mut b = al_testkit::Built::from_source(SRC);
    al_builtins::assumptions::clear_hits();
    let got = al_testkit::text(b.run_in("T", "Reported"));
    assert_eq!(
        got,
        "open=4:R4;R3;R2;R1;R4;R4;R4;C4; drilldown=4: gotorecord3=No/1:R1;R1;R3;R1;R1;R1;C1;R3;R3;R3;C3;R1;R1;R1;C1; \
         gotorecord1=Yes/1:R3;R3;R3;C3;R1;R1;R1;C1;"
    );
    assert!(hit(Assumption::DrilldownMarkedRefillInItsKey));
}

/// The shapes the reporter did not measure stay refused.
#[test]
fn unmeasured_moves_after_a_marked_drilldown_in_another_key_are_refused() {
    let mut b = al_testkit::Built::from_source(SRC);
    for p in [
        "GoToRecordOfRecsRowFirst",
        "NextAfterTheDrilldown",
        "GoToKeyAfterTheDrilldown",
        "ThreeMarkedRows",
    ] {
        let why = refusal(b.run_in("T", p));
        assert!(why.contains("not pinned"), "{p}: {why}");
    }
}
