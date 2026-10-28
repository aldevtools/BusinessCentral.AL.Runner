//! A Worksheet in a key other than the primary key (OnOpenPage calls SetCurrentKey) whose field's OnDrillDown marks rows,
//! turns MarkedOnly on and calls `CurrPage.Update(false)`: the drilldown reads nothing and the page keeps showing the row it
//! stood on; a GoToRecord of the row Rec was left on first refills the rows around it and answers true on it (corpus
//! pages/lt-qr-onopenpage-moves-rec/SuRefillKey.al:269 Name_DrillDown_GoToRecord, three marked rows; al-runner issue 6008).
//! The fixture is the reporter's repro without Library Assert; its GoToRecord of another marked row is not pinned (the
//! reporter's Windows run and a SaaS sandbox disagree) and is refused.

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

    procedure ThreeMarkedRowsGoToRecsRow(): Text
    var
        Row: Record "Refill Probe Row";
        P: TestPage "Refill Probe Name List";
        Observed: Text;
    begin
        SeedRows();
        Row.Get(2);
        Row.Flag := true;
        Row.Modify();
        P.OpenView();
        Log.Take();
        P.FlaggedTile.Drilldown();
        Observed := 'drilldown=' + P.Id.Value() + ':' + Log.Take();
        Row.Get(1);
        Observed += ' goto1=' + Format(P.GoToRecord(Row)) + '/' + P.Id.Value() + ':' + Log.Take();
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

use al_builtins::native::Signal;

fn refusal(r: Result<al_builtins::value::Value, Signal>) -> String {
    match r {
        Err(Signal::Unsupported(why)) => why.to_string(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// The corpus pin's shape on the reporter's page: three marked rows in the Name key (rows 3, 2, 1), Rec left on the last
/// (row 1); the refill reads row 1 twice, the rows above it from the nearest up, row 1 three times and OnAfterGetCurrRecord
/// (SuRefillKey.al:269 `goto 3` `Yes/3:R3;R3;R5;R2;R3;R3;R3;C3;`).
#[test]
fn goto_record_of_recs_row_after_a_marked_drilldown_refills_as_the_corpus_pins() {
    let mut b = al_testkit::Built::from_source(SRC);
    al_builtins::assumptions::clear_hits();
    let got = al_testkit::text(b.run_in("T", "ThreeMarkedRowsGoToRecsRow"));
    assert_eq!(got, "drilldown=4: goto1=Yes/1:R1;R1;R2;R3;R1;R1;R1;C1;");
    assert!(al_builtins::assumptions::take_hits().is_empty());
}

/// The shapes the corpus does not pin stay refused: the reporter's GoToRecord of the first of two marked rows, two marked
/// rows, another row than Rec's, and moves other than GoToRecord.
#[test]
fn unmeasured_moves_after_a_marked_drilldown_in_another_key_are_refused() {
    let mut b = al_testkit::Built::from_source(SRC);
    for p in [
        "Reported",
        "GoToRecordOfRecsRowFirst",
        "NextAfterTheDrilldown",
        "GoToKeyAfterTheDrilldown",
        "ThreeMarkedRows",
    ] {
        let why = refusal(b.run_in("T", p));
        assert!(why.contains("not pinned"), "{p}: {why}");
    }
}
