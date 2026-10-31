//! A Worksheet in a key other than the primary key (OnOpenPage calls SetCurrentKey) with a linked ListPart: an OnDrillDown marks
//! the flagged rows; after an outside change of a row's flag, `First` runs OnAfterGetCurrRecord, which marks the rows again and
//! calls `CurrPage.Update(false)`. The page then holds the marked rows of the new view: a GoToRecord of the row the refresh left
//! out is false and the page stays on row 1 (al-runner issue 6467, the reporter's repro without Library Assert; BC 29:
//! `before=false/1 after=false/1`). The primary key is pinned by pages/lt-y1-6288/Y1ExternalReapplyMarks.al.

use al_testkit::outcomes_in;

const SRC: &str = r##"
table 50100 "External Reapply Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Flag; Boolean) { }
        field(3; SortOrder; Integer) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
        key(BySortOrder; SortOrder) { }
    }
}

table 50101 "External Reapply Detail"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; "Line No."; Integer) { }
    }
    keys
    {
        key(PK; Id, "Line No.") { Clustered = true; }
    }
}

codeunit 50102 "External Reapply Marks"
{
    procedure MarkFlagged(var Row: Record "External Reapply Row")
    var
        RowsInView: Record "External Reapply Row";
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

codeunit 50103 "External Refresh"
{
    SingleInstance = true;

    procedure InvokeRefresh()
    var
        Row: Record "External Reapply Row";
    begin
        Row.Get(2);
        Row.Flag := false;
        Row.Modify();
        RefreshCompleted := true;
    end;

    procedure ConsumeRefreshCompleted(): Boolean
    begin
        if not RefreshCompleted then
            exit(false);
        RefreshCompleted := false;
        exit(true);
    end;

    procedure NoteReapplied()
    begin
        RefreshReapplied := true;
    end;

    procedure WasReapplied(): Boolean
    begin
        exit(RefreshReapplied);
    end;

    var
        RefreshCompleted: Boolean;
        RefreshReapplied: Boolean;
}

page 50104 "External Reapply Part"
{
    PageType = ListPart;
    SourceTable = "External Reapply Detail";
    InsertAllowed = false;
    DeleteAllowed = false;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Id; Rec.Id) { }
                field("Line No."; Rec."Line No.") { }
            }
        }
    }
}

page 50105 "External Reapply List"
{
    PageType = Worksheet;
    SourceTable = "External Reapply Row";
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
                field(Id; Rec.Id) { }
            }
            part(Details; "External Reapply Part")
            {
                SubPageLink = Id = field(Id);
            }
        }
    }

    trigger OnOpenPage()
    begin
        Rec.SetCurrentKey(SortOrder);
    end;

    trigger OnAfterGetRecord()
    begin
        RowText := Format(Rec.Id);
    end;

    trigger OnAfterGetCurrRecord()
    begin
        CurrText := Format(Rec.Id);
        if ExternalRefresh.ConsumeRefreshCompleted() then begin
            Marks.MarkFlagged(Rec);
            CurrPage.Update(false);
            ExternalRefresh.NoteReapplied();
        end;
    end;

    var
        Marks: Codeunit "External Reapply Marks";
        ExternalRefresh: Codeunit "External Refresh";
        Tile: Integer;
        RowText: Text;
        CurrText: Text;
}

codeunit 50120 "External Reapply Tests"
{
    Subtype = Test;

    [Test]
    procedure FirstAfterExternalRefreshReappliesMarks()
    var
        Row: Record "External Reapply Row";
        ExternalRefresh: Codeunit "External Refresh";
        ReapplyPage: TestPage "External Reapply List";
        Observed: Text;
    begin
        SeedRows();
        ReapplyPage.OpenView();
        ReapplyPage.FlaggedTile.Drilldown();
        Row.Get(2);
        if not Row.Flag then
            Error('Row 2 starts flagged.');
        Observed := 'before=' + Format(ReapplyPage.GoToRecord(Row), 0, 9) + '/' + ReapplyPage.Id.Value();

        if ExternalRefresh.WasReapplied() then
            Error('The refresh has not yet been reapplied.');
        ExternalRefresh.InvokeRefresh();
        Row.Get(2);
        if Row.Flag then
            Error('The refresh removes the flag of row 2.');
        ReapplyPage.First();
        if not ExternalRefresh.WasReapplied() then
            Error('OnAfterGetCurrRecord re-applies the marked view.');

        Observed += ' after=' + Format(ReapplyPage.GoToRecord(Row), 0, 9) + '/' + ReapplyPage.Id.Value();
        ReapplyPage.Close();
        if Observed <> 'before=false/1 after=false/1' then
            Error('Observed %1', Observed);
    end;

    local procedure SeedRows()
    var
        Row: Record "External Reapply Row";
        Detail: Record "External Reapply Detail";
    begin
        Row.DeleteAll();
        Detail.DeleteAll();
        AddRow(1, true);
        AddRow(2, true);
        AddRow(3, false);
    end;

    local procedure AddRow(Id: Integer; Flag: Boolean)
    var
        Row: Record "External Reapply Row";
        Detail: Record "External Reapply Detail";
    begin
        Row.Init();
        Row.Id := Id;
        Row.SortOrder := 4 - Id;
        Row.Flag := Flag;
        Row.Insert();
        Detail.Init();
        Detail.Id := Id;
        Detail."Line No." := 10000;
        Detail.Insert();
    end;
}
"##;

#[test]
fn first_after_an_outside_refresh_reapplies_the_marks_in_another_key() {
    let o = outcomes_in(SRC, "External Reapply Tests");
    let got = format!(
        "{:?}",
        o.iter()
            .find(|(n, _)| n == "FirstAfterExternalRefreshReappliesMarks")
            .map(|(_, x)| x)
    );
    assert_eq!(got, "Some(Pass)");
}
