// The reporter's repro of al-runner issue 6139 (green on real BC 29.0.54011.55407), app and test files in one source.
table 50100 "Reapply Probe Row"
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

table 50101 "Reapply Probe Detail"
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

codeunit 50102 "Reapply Probe Marks"
{
    procedure MarkFlagged(var Row: Record "Reapply Probe Row")
    var
        RowsInView: Record "Reapply Probe Row";
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

page 50103 "Reapply Probe Detail Part"
{
    PageType = ListPart;
    SourceTable = "Reapply Probe Detail";
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

page 50104 "Reapply Probe List"
{
    PageType = Worksheet;
    SourceTable = "Reapply Probe Row";
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
            part(Details; "Reapply Probe Detail Part")
            {
                SubPageLink = Id = field(Id);
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(Reapply)
            {
                trigger OnAction()
                begin
                    Marks.MarkFlagged(Rec);
                    CurrPage.Update(false);
                end;
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        RowText := Format(Rec.Id);
    end;

    trigger OnAfterGetCurrRecord()
    begin
        CurrText := Format(Rec.Id);
    end;

    var
        Marks: Codeunit "Reapply Probe Marks";
        Tile: Integer;
        RowText: Text;
        CurrText: Text;
}
codeunit 50120 "Reapply Probe Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit "Library Assert";

    [Test]
    procedure DrillDownAgainAfterTheRowLostItsFlag()
    var
        Row: Record "Reapply Probe Row";
        P: TestPage "Reapply Probe List";
        Observed: Text;
    begin
        SeedRows();
        P.OpenView();
        P.FlaggedTile.Drilldown();
        Row.Get(2);
        Observed := 'first=' + Format(P.GoToRecord(Row), 0, 9) + '/' + P.Id.Value();
        Unflag(2);
        P.FlaggedTile.Drilldown();
        Observed += ' second=' + Format(P.GoToRecord(Row), 0, 9) + '/' + P.Id.Value();
        P.Close();
        Assert.AreEqual('first=true/2 second=false/1', Observed, 'Primary key with a part, drilldown again after the row lost its flag.');
    end;

    [Test]
    procedure ActionAfterTheRowLostItsFlag()
    var
        Row: Record "Reapply Probe Row";
        P: TestPage "Reapply Probe List";
        Observed: Text;
    begin
        SeedRows();
        P.OpenView();
        P.FlaggedTile.Drilldown();
        Row.Get(2);
        Observed := 'first=' + Format(P.GoToRecord(Row), 0, 9) + '/' + P.Id.Value();
        Unflag(2);
        P.Reapply.Invoke();
        Observed += ' second=' + Format(P.GoToRecord(Row), 0, 9) + '/' + P.Id.Value();
        P.Close();
        Assert.AreEqual('first=true/2 second=false/1', Observed, 'Primary key with a part, action after the row lost its flag.');
    end;

    [Test]
    procedure DrillDownAgainWhileTheRowKeepsItsFlag()
    var
        Row: Record "Reapply Probe Row";
        P: TestPage "Reapply Probe List";
        Observed: Text;
    begin
        SeedRows();
        P.OpenView();
        P.FlaggedTile.Drilldown();
        Row.Get(2);
        Observed := 'first=' + Format(P.GoToRecord(Row), 0, 9) + '/' + P.Id.Value();
        P.FlaggedTile.Drilldown();
        Observed += ' second=' + Format(P.GoToRecord(Row), 0, 9) + '/' + P.Id.Value();
        P.Close();
        Assert.AreEqual('first=true/2 second=true/2', Observed, 'Primary key with a part, drilldown again while the row keeps its flag.');
    end;

    local procedure SeedRows()
    var
        Row: Record "Reapply Probe Row";
        Detail: Record "Reapply Probe Detail";
    begin
        Row.DeleteAll();
        Detail.DeleteAll();
        AddRow(1, 'N4', true);
        AddRow(2, 'N3', true);
        AddRow(3, 'N2', false);
        AddRow(4, 'N1', false);
    end;

    local procedure AddRow(Id: Integer; Name: Code[20]; Flag: Boolean)
    var
        Row: Record "Reapply Probe Row";
        Detail: Record "Reapply Probe Detail";
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

    local procedure Unflag(Id: Integer)
    var
        Row: Record "Reapply Probe Row";
    begin
        Row.Get(Id);
        Row.Flag := false;
        Row.Modify();
    end;
}
