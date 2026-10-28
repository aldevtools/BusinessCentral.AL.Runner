//! A list in a key other than its primary key, and the same list in its primary key, after a TestPage filter change, after an
//! action that marks rows and turns MarkedOnly on, and after a field's OnDrillDown that does the same (corpus
//! pages/lt-qr-onopenpage-moves-rec/SuRefillKey.al and SyKeyMoves.al; al-runner issues 6008 and 5871). The fixture is the
//! corpus file's objects, renumbered; each routine returns the observations the corpus test compares.

const SRC: &str = r##"table 50130 "Rk Row"
{
    DataClassification = SystemMetadata;

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

codeunit 50131 "Rk Log"
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

codeunit 50132 "Rk Marks"
{
    procedure MarkFlagged(var Row: Record "Rk Row")
    var
        RowsInView: Record "Rk Row";
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

page 50133 "Rk Name Ws"
{
    PageType = Worksheet;
    SourceTable = "Rk Row";
    ApplicationArea = All;
    UsageCategory = None;
    InsertAllowed = false;
    DeleteAllowed = false;

    layout
    {
        area(Content)
        {
            group(Tiles)
            {
                field(FlaggedTile; Tile)
                {
                    ApplicationArea = All;
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
                field(Id; Rec.Id) { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(MarkAct)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Marks.MarkFlagged(Rec);
                    CurrPage.Update(false);
                end;
            }
            action(RefreshAct)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    CurrPage.Update(false);
                end;
            }
            action(MarkNoUpdate)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Marks.MarkFlagged(Rec);
                end;
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
        Log: Codeunit "Rk Log";
        Marks: Codeunit "Rk Marks";
        Tile: Integer;
}

page 50134 "Rk Id Ws"
{
    PageType = Worksheet;
    SourceTable = "Rk Row";
    ApplicationArea = All;
    UsageCategory = None;
    InsertAllowed = false;
    DeleteAllowed = false;

    layout
    {
        area(Content)
        {
            group(Tiles)
            {
                field(FlaggedTile; Tile)
                {
                    ApplicationArea = All;
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
                field(Id; Rec.Id) { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(MarkAct)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Marks.MarkFlagged(Rec);
                    CurrPage.Update(false);
                end;
            }
            action(RefreshAct)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    CurrPage.Update(false);
                end;
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('R' + Format(Rec.Id));
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('C' + Format(Rec.Id));
    end;

    var
        Log: Codeunit "Rk Log";
        Marks: Codeunit "Rk Marks";
        Tile: Integer;
}

codeunit 50140 T
{
    var
        Log: Codeunit "Rk Log";

    procedure NameFilterRefill(): Text
    var
        Row: Record "Rk Row";
        P: TestPage "Rk Name Ws";
        O: Text;
    begin
        Seed();
        P.OpenView();
        O := 'open=' + P.Id.Value() + ':' + Log.Take();
        P.Filter.SetFilter(Id, '2..4');
        O += ' filter=' + P.Id.Value() + ':' + Log.Take();
        Row.Get(2);
        O += ' goto2=' + Format(P.GoToRecord(Row)) + '/' + P.Id.Value() + ':' + Log.Take();
        Row.Get(4);
        O += ' goto4=' + Format(P.GoToRecord(Row)) + '/' + P.Id.Value() + ':' + Log.Take();
        Row.Get(1);
        O += ' goto1=' + Format(P.GoToRecord(Row)) + '/' + P.Id.Value() + ':' + Log.Take();
        O += ' last=' + Format(P.Last()) + '/' + P.Id.Value() + ':' + Log.Take();
        P.Close();
        exit(O);
    end;

    procedure IdFilterRefill(): Text
    var
        Row: Record "Rk Row";
        P: TestPage "Rk Id Ws";
        O: Text;
    begin
        Seed();
        P.OpenView();
        O := 'open=' + P.Id.Value() + ':' + Log.Take();
        P.Filter.SetFilter(Id, '2..4');
        O += ' filter=' + P.Id.Value() + ':' + Log.Take();
        Row.Get(2);
        O += ' goto2=' + Format(P.GoToRecord(Row)) + '/' + P.Id.Value() + ':' + Log.Take();
        Row.Get(4);
        O += ' goto4=' + Format(P.GoToRecord(Row)) + '/' + P.Id.Value() + ':' + Log.Take();
        Row.Get(1);
        O += ' goto1=' + Format(P.GoToRecord(Row)) + '/' + P.Id.Value() + ':' + Log.Take();
        O += ' last=' + Format(P.Last()) + '/' + P.Id.Value() + ':' + Log.Take();
        P.Close();
        exit(O);
    end;

    procedure NameActionMark(): Text
    var
        Row: Record "Rk Row";
        P: TestPage "Rk Name Ws";
        O: Text;
    begin
        Seed();
        P.OpenView();
        Log.Take();
        Row.Get(5);
        P.GoToRecord(Row);
        Log.Take();
        P.MarkAct.Invoke();
        O := 'mark=' + P.Id.Value() + ':' + Log.Take();
        Row.Get(3);
        O += ' goto3=' + Format(P.GoToRecord(Row)) + '/' + P.Id.Value() + ':' + Log.Take();
        Row.Get(1);
        O += ' goto1=' + Format(P.GoToRecord(Row)) + '/' + P.Id.Value() + ':' + Log.Take();
        P.Close();
        exit(O);
    end;

    procedure IdActionMark(): Text
    var
        Row: Record "Rk Row";
        P: TestPage "Rk Id Ws";
        O: Text;
    begin
        Seed();
        P.OpenView();
        Log.Take();
        Row.Get(5);
        P.GoToRecord(Row);
        Log.Take();
        P.MarkAct.Invoke();
        O := 'mark=' + P.Id.Value() + ':' + Log.Take();
        Row.Get(3);
        O += ' goto3=' + Format(P.GoToRecord(Row)) + '/' + P.Id.Value() + ':' + Log.Take();
        Row.Get(1);
        O += ' goto1=' + Format(P.GoToRecord(Row)) + '/' + P.Id.Value() + ':' + Log.Take();
        P.Close();
        exit(O);
    end;

    procedure IdDrillDownGoToRecord(): Text
    var
        Row: Record "Rk Row";
        P: TestPage "Rk Id Ws";
        O: Text;
    begin
        Seed();
        P.OpenView();
        Log.Take();
        P.FlaggedTile.Drilldown();
        O := 'drilldown=' + P.Id.Value() + ':' + Log.Take();
        Row.Get(3);
        O += ' goto3=' + Format(P.GoToRecord(Row)) + '/' + P.Id.Value() + ':' + Log.Take();
        Row.Get(5);
        O += ' goto5=' + Format(P.GoToRecord(Row)) + '/' + P.Id.Value() + ':' + Log.Take();
        Row.Get(1);
        O += ' goto1=' + Format(P.GoToRecord(Row)) + '/' + P.Id.Value() + ':' + Log.Take();
        O += ' next=' + Format(P.Next()) + '/' + P.Id.Value() + ':' + Log.Take();
        O += ' first=' + Format(P.First()) + '/' + P.Id.Value() + ':' + Log.Take();
        P.Close();
        exit(O);
    end;

    procedure NameDrillDownGoToOtherRow()
    var
        Row: Record "Rk Row";
        P: TestPage "Rk Name Ws";
    begin
        Seed();
        P.OpenView();
        P.FlaggedTile.Drilldown();
        Row.Get(5);
        P.GoToRecord(Row);
    end;

    procedure IdDrillDownGoToRecsRow()
    var
        Row: Record "Rk Row";
        P: TestPage "Rk Id Ws";
    begin
        Seed();
        P.OpenView();
        P.FlaggedTile.Drilldown();
        Row.Get(5);
        P.GoToRecord(Row);
    end;

    procedure NameActionMarkWithoutUpdate()
    var
        Row: Record "Rk Row";
        P: TestPage "Rk Name Ws";
    begin
        Seed();
        P.OpenView();
        P.MarkNoUpdate.Invoke();
        P.Next();
    end;

    local procedure Seed()
    var
        Row: Record "Rk Row";
    begin
        Row.DeleteAll();
        AddRow(1, 'M', false);
        AddRow(2, 'C', true);
        AddRow(3, 'T', true);
        AddRow(4, 'A', false);
        AddRow(5, 'P', true);
        Log.Take();
    end;

    local procedure AddRow(Id: Integer; Name: Code[20]; Flag: Boolean)
    var
        Row: Record "Rk Row";
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

fn run(p: &str) -> String {
    let mut b = al_testkit::Built::from_source(SRC);
    al_testkit::text(b.run_in("T", p))
}

/// A filter change refills the rows as the open does: in the key other than the primary key the first row is read three times
/// before OnAfterGetCurrRecord (SuRefillKey.al:299 Name_Filter_Refill, :317-322).
#[test]
fn filter_change_in_another_key_refills_as_the_open_reads() {
    assert_eq!(
        run("NameFilterRefill"),
        "open=4:R4;R2;R1;R5;R3;R4;R4;R4;C4; filter=4:R4;R2;R3;R4;R4;R4;C4; goto2=Yes/2:R2;R2;R2;C2; goto4=Yes/4:R4;R4;R4;C4; \
         goto1=No/4:R2;R2;R2;C2;R3;R3;R3;C3;R4;R4;R4;C4; last=Yes/3:R3;R3;R3;C3;"
    );
}

/// In the primary key the refill reads the first row twice; Last on the last row of a view the client holds whole reads nothing
/// (SuRefillKey.al:409 Id_Filter_Refill :427-432).
#[test]
fn filter_change_in_the_primary_key_then_last_on_the_last_row() {
    assert_eq!(
        run("IdFilterRefill"),
        "open=1:R1;R2;R3;R4;R5;R1;R1;C1; filter=2:R2;R3;R4;R2;R2;C2; goto2=Yes/2: goto4=Yes/4:R3;R3;C3;R4;R4;C4; \
         goto1=No/4:R2;R2;C2;R3;R3;C3;R4;R4;C4; last=Yes/4:"
    );
}

/// An action that marks rows, turns MarkedOnly on and calls CurrPage.Update(false): the client holds the marked rows it read
/// (SuRefillKey.al:353 Name_Action_Mark :372-374, :463 Id_Action_Mark :482-484).
#[test]
fn marking_action_holds_the_marked_rows() {
    assert_eq!(
        run("NameActionMark"),
        "mark=3:R5;R3;R3;R3;R5;R2;R3;R3;R3;C3; goto3=Yes/3:R2;R2;R2;C2;R5;R5;R5;C5;R3;R3;R3;C3; \
         goto1=No/3:R2;R2;R2;C2;R5;R5;R5;C5;R3;R3;R3;C3;"
    );
    assert_eq!(
        run("IdActionMark"),
        "mark=5:R5;R5;R5;R3;R2;R5;R5;C5; goto3=Yes/3:R2;R2;C2;R3;R3;C3; goto1=No/3:R2;R2;C2;R3;R3;C3;R5;R5;C5;R3;R3;C3;"
    );
}

/// A marking OnDrillDown in the primary key that left Rec on another row: the page still shows its row; the first GoToRecord
/// refills around Rec's row and, for another row, walks the marked rows and answers false (SuRefillKey.al:379
/// Id_DrillDown_GoToRecord :399-404).
#[test]
fn marking_drilldown_in_the_primary_key_then_goto_record() {
    assert_eq!(
        run("IdDrillDownGoToRecord"),
        "drilldown=1: goto3=No/5:R5;R3;R2;R5;R5;C5;R2;R2;C2;R3;R3;C3;R5;R5;C5; goto5=Yes/5:R2;R2;C2;R3;R3;C3;R5;R5;C5; \
         goto1=No/5:R2;R2;C2;R3;R3;C3;R5;R5;C5; next=No/5: first=Yes/2:R2;R2;C2;"
    );
}

/// What the corpus does not pin stays refused: after the drilldown a GoToRecord of another row than Rec's in the other key, of
/// Rec's row in the primary key; a marking action without CurrPage.Update.
#[test]
fn unpinned_shapes_after_marking_stay_refused() {
    let mut b = al_testkit::Built::from_source(SRC);
    for p in [
        "NameDrillDownGoToOtherRow",
        "IdDrillDownGoToRecsRow",
        "NameActionMarkWithoutUpdate",
    ] {
        match b.run_in("T", p) {
            Err(Signal::Unsupported(why)) => {
                assert!(why.contains("not pinned") || why.contains("not built"), "{p}: {why}")
            }
            other => panic!("{p}: expected a refusal, got {other:?}"),
        }
    }
}
