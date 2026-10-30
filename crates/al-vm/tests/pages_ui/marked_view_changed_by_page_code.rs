//! After a marking OnDrillDown with `CurrPage.Update(false)` on a Worksheet with a ListPart, GoToRecord follows the marked view
//! that the page code later changes and refreshes with `CurrPage.Update(false)`:
//! - an action that turns MarkedOnly off and clears the marks: GoToRecord finds an unmarked row again (al-runner issue 6289;
//!   corpus writer Y1, branch al-runtime-rs-staging-y1, pages/lt-y1-6289/Y1ClearMarkedOnly.al:362, :390, :418, :446);
//! - OnAfterGetCurrRecord that marks again after code outside the page cleared a row's flag: GoToRecord of that row is false
//!   (al-runner issue 6288; pages/lt-y1-6288/Y1ExternalReapplyMarks.al:223, :254, :278).
//!
//! The fixtures are the corpus files with a local "Probe Observer" whose Verify raises the observations as the error text.

const CLEAR: &str = r##"table 90930 "LT Y1 Clear Row"
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

table 90931 "LT Y1 Clear Detail"
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

page 90933 "LT Y1 Clear Detail Part"
{
    PageType = ListPart;
    SourceTable = "LT Y1 Clear Detail";
    InsertAllowed = false;
    DeleteAllowed = false;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                Editable = false;
                field(Id; Rec.Id) { ApplicationArea = All; }
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
            }
        }
    }
}

page 90934 "LT Y1 Clear Name Part"
{
    PageType = Worksheet;
    SourceTable = "LT Y1 Clear Row";
    InsertAllowed = false;
    DeleteAllowed = false;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(Filter)
            {
                field(Flagged; Tile)
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
            part(Details; "LT Y1 Clear Detail Part")
            {
                ApplicationArea = All;
                SubPageLink = Id = field(Id);
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ClearFilters)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Marks.Clear(Rec);
                    CurrPage.Update(false);
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
        RowText := Format(Rec.Id);
    end;

    trigger OnAfterGetCurrRecord()
    begin
        CurrentText := Format(Rec.Id);
    end;

    var
        Marks: Codeunit "LT Y1 Clear Marks";
        Tile: Integer;
        RowText: Text;
        CurrentText: Text;
}

page 90935 "LT Y1 Clear Id Part"
{
    PageType = Worksheet;
    SourceTable = "LT Y1 Clear Row";
    InsertAllowed = false;
    DeleteAllowed = false;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(Filter)
            {
                field(Flagged; Tile)
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
            part(Details; "LT Y1 Clear Detail Part")
            {
                ApplicationArea = All;
                SubPageLink = Id = field(Id);
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ClearFilters)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Marks.Clear(Rec);
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
        CurrentText := Format(Rec.Id);
    end;

    var
        Marks: Codeunit "LT Y1 Clear Marks";
        Tile: Integer;
        RowText: Text;
        CurrentText: Text;
}

page 90936 "LT Y1 Clear No Triggers"
{
    PageType = Worksheet;
    SourceTable = "LT Y1 Clear Row";
    InsertAllowed = false;
    DeleteAllowed = false;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(Filter)
            {
                field(Flagged; Tile)
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
            part(Details; "LT Y1 Clear Detail Part")
            {
                ApplicationArea = All;
                SubPageLink = Id = field(Id);
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ClearFilters)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Marks.Clear(Rec);
                    CurrPage.Update(false);
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Rec.SetCurrentKey(Name);
    end;

    var
        Marks: Codeunit "LT Y1 Clear Marks";
        Tile: Integer;
}

codeunit 90932 "LT Y1 Clear Marks"
{
    procedure MarkFlagged(var Row: Record "LT Y1 Clear Row")
    var
        RowsInView: Record "LT Y1 Clear Row";
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

    procedure Clear(var Row: Record "LT Y1 Clear Row")
    begin
        Row.MarkedOnly(false);
        Row.ClearMarks();
    end;
}

codeunit 90937 "LT Y1 Clear Marked Tests"
{

    var
        Observer: Codeunit "Probe Observer";

    local procedure SeedRows()
    var
        Row: Record "LT Y1 Clear Row";
        Detail: Record "LT Y1 Clear Detail";
    begin
        Row.DeleteAll();
        Detail.DeleteAll();
        AddRow(1, 'N4', true);
        AddRow(2, 'N3', false);
        AddRow(3, 'N2', true);
        AddRow(4, 'N1', false);
    end;

    local procedure AddRow(Id: Integer; Name: Code[20]; Flag: Boolean)
    var
        Row: Record "LT Y1 Clear Row";
        Detail: Record "LT Y1 Clear Detail";
    begin
        Row.Init();
        Row.Id := Id;
        Row.Name := Name;
        Row.Flag := Flag;
        Row.Insert();
        Detail.Init();
        Detail.Id := Id;
        Detail."Line No." := 10000;
        Detail.Insert();
    end;

    procedure Y1_Clear_NameKeyWithPartAndTriggers()
    var
        Row: Record "LT Y1 Clear Row";
        ClearPage: TestPage "LT Y1 Clear Name Part";
    begin
        SeedRows();
        Row.Get(2);
        ClearPage.OpenView();
        Observer.Observe('open', ClearPage.Id.Value());
        ClearPage.Flagged.Drilldown();
        Observer.Observe('after drilldown', ClearPage.Id.Value());
        Observer.Observe('goto 2 filtered', Format(ClearPage.GoToRecord(Row), 0, 9) + '/' + ClearPage.Id.Value());
        ClearPage.ClearFilters.Invoke();
        Observer.Observe('after clear', ClearPage.Id.Value());
        Observer.Observe('goto 2 cleared', Format(ClearPage.GoToRecord(Row), 0, 9) + '/' + ClearPage.Id.Value());
        Observer.Observe('first cleared', Format(ClearPage.First(), 0, 9) + '/' + ClearPage.Id.Value());
        ClearPage.Close();
        Observer.Verify();
    end;

    procedure Y1_Clear_NameKeyWithPartAndTriggers_Row4()
    var
        Row: Record "LT Y1 Clear Row";
        ClearPage: TestPage "LT Y1 Clear Name Part";
    begin
        SeedRows();
        Row.Get(4);
        ClearPage.OpenView();
        Observer.Observe('open', ClearPage.Id.Value());
        ClearPage.Flagged.Drilldown();
        Observer.Observe('after drilldown', ClearPage.Id.Value());
        Observer.Observe('goto 4 filtered', Format(ClearPage.GoToRecord(Row), 0, 9) + '/' + ClearPage.Id.Value());
        ClearPage.ClearFilters.Invoke();
        Observer.Observe('after clear', ClearPage.Id.Value());
        Observer.Observe('goto 4 cleared', Format(ClearPage.GoToRecord(Row), 0, 9) + '/' + ClearPage.Id.Value());
        Observer.Observe('first cleared', Format(ClearPage.First(), 0, 9) + '/' + ClearPage.Id.Value());
        ClearPage.Close();
        Observer.Verify();
    end;

    procedure Y1_Clear_PrimaryKeyWithPartAndTriggers()
    var
        Row: Record "LT Y1 Clear Row";
        ClearPage: TestPage "LT Y1 Clear Id Part";
    begin
        SeedRows();
        Row.Get(4);
        ClearPage.OpenView();
        Observer.Observe('open', ClearPage.Id.Value());
        ClearPage.Flagged.Drilldown();
        Observer.Observe('after drilldown', ClearPage.Id.Value());
        Observer.Observe('goto 4 filtered', Format(ClearPage.GoToRecord(Row), 0, 9) + '/' + ClearPage.Id.Value());
        ClearPage.ClearFilters.Invoke();
        Observer.Observe('after clear', ClearPage.Id.Value());
        Observer.Observe('goto 4 cleared', Format(ClearPage.GoToRecord(Row), 0, 9) + '/' + ClearPage.Id.Value());
        Observer.Observe('first cleared', Format(ClearPage.First(), 0, 9) + '/' + ClearPage.Id.Value());
        ClearPage.Close();
        Observer.Verify();
    end;

    procedure Y1_Clear_NameKeyWithPartNoTriggers()
    var
        Row: Record "LT Y1 Clear Row";
        ClearPage: TestPage "LT Y1 Clear No Triggers";
    begin
        SeedRows();
        Row.Get(2);
        ClearPage.OpenView();
        Observer.Observe('open', ClearPage.Id.Value());
        ClearPage.Flagged.Drilldown();
        Observer.Observe('after drilldown', ClearPage.Id.Value());
        Observer.Observe('goto 2 filtered', Format(ClearPage.GoToRecord(Row), 0, 9) + '/' + ClearPage.Id.Value());
        ClearPage.ClearFilters.Invoke();
        Observer.Observe('after clear', ClearPage.Id.Value());
        Observer.Observe('goto 2 cleared', Format(ClearPage.GoToRecord(Row), 0, 9) + '/' + ClearPage.Id.Value());
        Observer.Observe('first cleared', Format(ClearPage.First(), 0, 9) + '/' + ClearPage.Id.Value());
        ClearPage.Close();
        Observer.Verify();
    end;
}

codeunit 69160 "Probe Observer"
{
    var
        Log: Text;

    procedure Observe(Name: Text; Value: Text)
    begin
        Log += Name + '=' + Value + ';';
    end;

    procedure Expect(Name: Text; Value: Text)
    begin
    end;

    procedure Verify()
    begin
        Error(Log);
    end;
}
"##;

#[test]
fn clear_name_key_with_part_and_triggers() {
    assert_eq!(
        al_testkit::error_message(al_testkit::run_in(
            CLEAR,
            "LT Y1 Clear Marked Tests",
            "Y1_Clear_NameKeyWithPartAndTriggers"
        )),
        "open=4;after drilldown=4;goto 2 filtered=false/1;after clear=1;goto 2 cleared=true/2;first cleared=true/4;"
    );
}

#[test]
fn clear_name_key_with_part_and_triggers_row4() {
    assert_eq!(
        al_testkit::error_message(al_testkit::run_in(
            CLEAR,
            "LT Y1 Clear Marked Tests",
            "Y1_Clear_NameKeyWithPartAndTriggers_Row4"
        )),
        "open=4;after drilldown=4;goto 4 filtered=false/1;after clear=1;goto 4 cleared=true/4;first cleared=true/4;"
    );
}

#[test]
fn clear_primary_key_with_part_and_triggers() {
    assert_eq!(
        al_testkit::error_message(al_testkit::run_in(
            CLEAR,
            "LT Y1 Clear Marked Tests",
            "Y1_Clear_PrimaryKeyWithPartAndTriggers"
        )),
        "open=1;after drilldown=1;goto 4 filtered=false/1;after clear=1;goto 4 cleared=true/4;first cleared=true/1;"
    );
}

#[test]
fn clear_name_key_with_part_no_triggers() {
    assert_eq!(
        al_testkit::error_message(al_testkit::run_in(
            CLEAR,
            "LT Y1 Clear Marked Tests",
            "Y1_Clear_NameKeyWithPartNoTriggers"
        )),
        "open=4;after drilldown=4;goto 2 filtered=false/1;after clear=1;goto 2 cleared=true/2;first cleared=true/4;"
    );
}

const REAPPLY: &str = r##"table 90920 "LT Y1 Reapply Row"
{
    
    fields
    {
        field(1; Id; Integer) { }
        field(2; Flag; Boolean) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 90921 "LT Y1 Reapply Detail"
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

page 90924 "LT Y1 Reapply Part"
{
    PageType = ListPart;
    SourceTable = "LT Y1 Reapply Detail";
    InsertAllowed = false;
    DeleteAllowed = false;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Id; Rec.Id) { ApplicationArea = All; }
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
            }
        }
    }
}

page 90925 "LT Y1 Reapply List"
{
    PageType = Worksheet;
    SourceTable = "LT Y1 Reapply Row";
    InsertAllowed = false;
    DeleteAllowed = false;
    ApplicationArea = All;
    UsageCategory = None;

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
                field(Id; Rec.Id) { ApplicationArea = All; }
            }
            part(Details; "LT Y1 Reapply Part")
            {
                ApplicationArea = All;
                SubPageLink = Id = field(Id);
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
        if ExternalRefresh.ConsumeRefreshCompleted() then begin
            Marks.MarkFlagged(Rec);
            CurrPage.Update(false);
        end;
    end;

    var
        Marks: Codeunit "LT Y1 Reapply Marks";
        ExternalRefresh: Codeunit "LT Y1 External Refresh";
        Tile: Integer;
        RowText: Text;
        CurrText: Text;
}

codeunit 90922 "LT Y1 Reapply Marks"
{
    procedure MarkFlagged(var Row: Record "LT Y1 Reapply Row")
    var
        RowsInView: Record "LT Y1 Reapply Row";
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

codeunit 90923 "LT Y1 External Refresh"
{
    SingleInstance = true;

    procedure InvokeRefresh()
    var
        Row: Record "LT Y1 Reapply Row";
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

    procedure Reset()
    begin
        RefreshCompleted := false;
    end;

    var
        RefreshCompleted: Boolean;
}

codeunit 90926 "LT Y1 Reapply Tests"
{

    var
        Observer: Codeunit "Probe Observer";
        ExternalRefresh: Codeunit "LT Y1 External Refresh";

    local procedure SeedRows()
    var
        Row: Record "LT Y1 Reapply Row";
        Detail: Record "LT Y1 Reapply Detail";
    begin
        ExternalRefresh.Reset();
        Row.DeleteAll();
        Detail.DeleteAll();
        AddRow(1, true);
        AddRow(2, true);
        AddRow(3, false);
    end;

    local procedure AddRow(Id: Integer; Flag: Boolean)
    var
        Row: Record "LT Y1 Reapply Row";
        Detail: Record "LT Y1 Reapply Detail";
    begin
        Row.Init();
        Row.Id := Id;
        Row.Flag := Flag;
        Row.Insert();
        Detail.Init();
        Detail.Id := Id;
        Detail."Line No." := 10000;
        Detail.Insert();
    end;

    procedure Y1_Reapply_FirstAfterExternalRefresh()
    var
        Row: Record "LT Y1 Reapply Row";
        ReapplyPage: TestPage "LT Y1 Reapply List";
    begin
        SeedRows();
        ReapplyPage.OpenView();
        ReapplyPage.FlaggedTile.Drilldown();
        Observer.Observe('after drilldown', ReapplyPage.Id.Value());
        Row.Get(2);
        Observer.Observe('goto 2 before', Format(ReapplyPage.GoToRecord(Row), 0, 9) + '/' + ReapplyPage.Id.Value());
        ExternalRefresh.InvokeRefresh();
        Observer.Observe('first', Format(ReapplyPage.First(), 0, 9) + '/' + ReapplyPage.Id.Value());
        Observer.Observe('goto 2 after', Format(ReapplyPage.GoToRecord(Row), 0, 9) + '/' + ReapplyPage.Id.Value());
        Observer.Observe('next after', Format(ReapplyPage.Next(), 0, 9) + '/' + ReapplyPage.Id.Value());
        Row.Get(1);
        Observer.Observe('goto 1 after', Format(ReapplyPage.GoToRecord(Row), 0, 9) + '/' + ReapplyPage.Id.Value());
        Observer.Observe('refresh still pending', Format(ExternalRefresh.ConsumeRefreshCompleted(), 0, 9));
        ReapplyPage.Close();
        Observer.Verify();
    end;

    procedure Y1_Reapply_FirstWithoutRefresh()
    var
        Row: Record "LT Y1 Reapply Row";
        ReapplyPage: TestPage "LT Y1 Reapply List";
    begin
        SeedRows();
        ReapplyPage.OpenView();
        ReapplyPage.FlaggedTile.Drilldown();
        Row.Get(2);
        Observer.Observe('goto 2 before', Format(ReapplyPage.GoToRecord(Row), 0, 9) + '/' + ReapplyPage.Id.Value());
        Observer.Observe('first', Format(ReapplyPage.First(), 0, 9) + '/' + ReapplyPage.Id.Value());
        Observer.Observe('goto 2 after', Format(ReapplyPage.GoToRecord(Row), 0, 9) + '/' + ReapplyPage.Id.Value());
        Row.Get(3);
        Observer.Observe('goto 3 after', Format(ReapplyPage.GoToRecord(Row), 0, 9) + '/' + ReapplyPage.Id.Value());
        ReapplyPage.Close();
        Observer.Verify();
    end;

    procedure Y1_Reapply_GoToRecordAfterRefreshWithoutFirst()
    var
        Row: Record "LT Y1 Reapply Row";
        ReapplyPage: TestPage "LT Y1 Reapply List";
    begin
        SeedRows();
        ReapplyPage.OpenView();
        ReapplyPage.FlaggedTile.Drilldown();
        Observer.Observe('after drilldown', ReapplyPage.Id.Value());
        ExternalRefresh.InvokeRefresh();
        Row.Get(2);
        Observer.Observe('goto 2', Format(ReapplyPage.GoToRecord(Row), 0, 9) + '/' + ReapplyPage.Id.Value());
        Observer.Observe('refresh still pending', Format(ExternalRefresh.ConsumeRefreshCompleted(), 0, 9));
        Observer.Observe('goto 2 again', Format(ReapplyPage.GoToRecord(Row), 0, 9) + '/' + ReapplyPage.Id.Value());
        ReapplyPage.Close();
        Observer.Verify();
    end;
}

codeunit 69160 "Probe Observer"
{
    var
        Log: Text;

    procedure Observe(Name: Text; Value: Text)
    begin
        Log += Name + '=' + Value + ';';
    end;

    procedure Expect(Name: Text; Value: Text)
    begin
    end;

    procedure Verify()
    begin
        Error(Log);
    end;
}
"##;

#[test]
fn reapply_first_after_external_refresh() {
    assert_eq!(
        al_testkit::error_message(al_testkit::run_in(
            REAPPLY,
            "LT Y1 Reapply Tests",
            "Y1_Reapply_FirstAfterExternalRefresh"
        )),
        "after drilldown=1;goto 2 before=true/2;first=true/1;goto 2 after=false/1;next after=false/1;goto 1 after=true/1;refresh still pending=false;"
    );
}

#[test]
fn reapply_first_without_refresh() {
    assert_eq!(
        al_testkit::error_message(al_testkit::run_in(
            REAPPLY,
            "LT Y1 Reapply Tests",
            "Y1_Reapply_FirstWithoutRefresh"
        )),
        "goto 2 before=true/2;first=true/1;goto 2 after=true/2;goto 3 after=false/2;"
    );
}

#[test]
fn reapply_go_to_record_after_refresh_without_first() {
    assert_eq!(
        al_testkit::error_message(al_testkit::run_in(
            REAPPLY,
            "LT Y1 Reapply Tests",
            "Y1_Reapply_GoToRecordAfterRefreshWithoutFirst"
        )),
        "after drilldown=1;goto 2=false/1;refresh still pending=false;goto 2 again=false/1;"
    );
}
