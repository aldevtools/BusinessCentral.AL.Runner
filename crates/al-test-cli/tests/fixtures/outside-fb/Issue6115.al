// The reporter's repro of al-runner issue 6115 (green on real BC), app and test files in one source.
table 50100 "Refill Probe Row"
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

table 50105 "Refill Probe Detail"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; "Line No."; Integer) { }
        field(3; Description; Text[50]) { }
    }

    keys
    {
        key(PK; Id, "Line No.") { Clustered = true; }
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

page 50106 "Refill Probe Detail Part"
{
    PageType = ListPart;
    SourceTable = "Refill Probe Detail";
    InsertAllowed = false;
    DeleteAllowed = false;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                Editable = false;
                field(Id; Rec.Id) { }
                field("Line No."; Rec."Line No.") { }
                field(Description; Rec.Description) { }
            }
        }
    }
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
            part(Details; "Refill Probe Detail Part")
            {
                SubPageLink = Id = field(Id);
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

page 50103 "Refill Probe Id List"
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
            part(Details; "Refill Probe Detail Part")
            {
                SubPageLink = Id = field(Id);
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
        Log: Codeunit "Refill Probe Log";
        Marks: Codeunit "Refill Probe Marks";
        Tile: Integer;
}

codeunit 50121 "Refill Probe Id Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit "Library Assert";
        Log: Codeunit "Refill Probe Log";
    [Test]
    procedure IdKeyWithPartGoToRecordOfRecsRow()
    var
        Row: Record "Refill Probe Row";
        P: TestPage "Refill Probe Id List";
        Observed: Text;
    begin
        SeedRows();
        P.OpenView();
        Observed := 'open=' + P.Id.Value() + ':' + Log.Take();
        P.FlaggedTile.Drilldown();
        Observed += ' drilldown=' + P.Id.Value() + ':' + Log.Take();
        Row.Get(3);
        Observed += ' gotorecord3=' + Format(P.GoToRecord(Row)) + '/' + P.Id.Value() + ':' + Log.Take();
        P.Close();
        Assert.AreEqual('open=1:R1;R2;R3;R4;R1;R1;C1; drilldown=1: gotorecord3=Yes/3:R3;R1;R3;R3;C3;', Observed, 'Primary key with a part, marked rows after OnDrillDown, then GoToRecord of the row Rec was left on.');
    end;
    [Test]
    procedure IdKeyWithPartGoToRecordOfUnmarkedRow()
    var
        Row: Record "Refill Probe Row";
        P: TestPage "Refill Probe Id List";
        Observed: Text;
    begin
        SeedRows();
        P.OpenView();
        Observed := 'open=' + P.Id.Value() + ':' + Log.Take();
        P.FlaggedTile.Drilldown();
        Observed += ' drilldown=' + P.Id.Value() + ':' + Log.Take();
        Row.Get(2);
        Observed += ' gotorecord2=' + Format(P.GoToRecord(Row)) + '/' + P.Id.Value() + ':' + Log.Take();
        P.Close();
        Assert.AreEqual('open=1:R1;R2;R3;R4;R1;R1;C1; drilldown=1: gotorecord2=No/1:R3;R1;R3;R3;C3;R1;R1;C1;', Observed, 'Primary key with a part, marked rows after OnDrillDown, then GoToRecord of an unmarked row.');
    end;

    local procedure SeedRows()
    var
        Row: Record "Refill Probe Row";
        Detail: Record "Refill Probe Detail";
    begin
        Row.DeleteAll();
        Detail.DeleteAll();
        AddRow(1, 'N4', true);
        AddRow(2, 'N3', false);
        AddRow(3, 'N2', true);
        AddRow(4, 'N1', false);
        Log.Take();
    end;

    local procedure AddRow(Id: Integer; Name: Code[20]; Flag: Boolean)
    var
        Row: Record "Refill Probe Row";
        Detail: Record "Refill Probe Detail";
    begin
        Row.Init();
        Row.Id := Id;
        Row.Name := Name;
        Row.Flag := Flag;
        Row.Insert();
        Detail.Init();
        Detail.Id := Id;
        Detail."Line No." := 10000;
        Detail.Description := 'D' + Format(Id);
        Detail.Insert();
    end;
}

codeunit 50120 "Refill Probe Key Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit "Library Assert";
        Log: Codeunit "Refill Probe Log";
    [Test]
    procedure NameKeyWithPartGoToRecordOfRecsRow()
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
        Row.Get(1);
        Observed += ' gotorecord1=' + Format(P.GoToRecord(Row)) + '/' + P.Id.Value() + ':' + Log.Take();
        P.Close();
        Assert.AreEqual('open=4:R4;R3;R2;R1;R4;R4;R4;C4; drilldown=4: gotorecord1=Yes/1:R1;R1;R3;R1;R1;R1;C1;', Observed, 'Name key with a part, marked rows after OnDrillDown, then GoToRecord of the row Rec was left on.');
    end;
    [Test]
    procedure NameKeyWithPartGoToRecordOfUnmarkedRow()
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
        Row.Get(2);
        Observed += ' gotorecord2=' + Format(P.GoToRecord(Row)) + '/' + P.Id.Value() + ':' + Log.Take();
        P.Close();
        Assert.AreEqual('open=4:R4;R3;R2;R1;R4;R4;R4;C4; drilldown=4: gotorecord2=No/1:R1;R1;R3;R1;R1;R1;C1;R3;R3;R3;C3;R1;R1;R1;C1;', Observed, 'Name key with a part, marked rows after OnDrillDown, then GoToRecord of an unmarked row.');
    end;

    local procedure SeedRows()
    var
        Row: Record "Refill Probe Row";
        Detail: Record "Refill Probe Detail";
    begin
        Row.DeleteAll();
        Detail.DeleteAll();
        AddRow(1, 'N4', true);
        AddRow(2, 'N3', false);
        AddRow(3, 'N2', true);
        AddRow(4, 'N1', false);
        Log.Take();
    end;

    local procedure AddRow(Id: Integer; Name: Code[20]; Flag: Boolean)
    var
        Row: Record "Refill Probe Row";
        Detail: Record "Refill Probe Detail";
    begin
        Row.Init();
        Row.Id := Id;
        Row.Name := Name;
        Row.Flag := Flag;
        Row.Insert();
        Detail.Init();
        Detail.Id := Id;
        Detail."Line No." := 10000;
        Detail.Description := 'D' + Format(Id);
        Detail.Insert();
    end;
}

