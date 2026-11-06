//! A List an action opens through RunPageLink hands Rec, in OnAfterGetCurrRecord, to a procedure that takes it as a Variant
//! (RecordRef.GetTable, then Get(RecordId)) and to one that takes it by value (Get). al-runner issue 7138 (Base Application's
//! Purchase Order List and its attachment FactBox) and the probe testpage/R7138RunPageLinkCopyGet.al of the corpus.

use al_testkit::outcomes_in;

/// The reporter's repro of issue 7138, unchanged.
const ISSUE: &str = r##"
table 50100 "Repro Project"
{
    fields
    {
        field(1; Code; Code[20]) { }
    }
    keys
    {
        key(PK; Code) { Clustered = true; }
    }
}

table 50101 "Repro Order"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Project Code"; Code[20]) { }
    }
    keys
    {
        key(PK; "No.") { Clustered = true; }
        key(ByProject; "Project Code") { }
    }
}

codeunit 50100 "Repro Load Log"
{
    SingleInstance = true;

    var
        Loads: List of [Text];

    procedure Clear()
    begin
        System.Clear(Loads);
    end;

    procedure Add(Entry: Text)
    begin
        Loads.Add(Entry);
    end;

    procedure Joined(): Text
    var
        Entry: Text;
        Result: Text;
    begin
        foreach Entry in Loads do
            Result += Entry + ';';
        exit(Result);
    end;
}

page 50101 "Repro Project Card"
{
    PageType = Card;
    SourceTable = "Repro Project";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field(Code; Rec.Code) { ApplicationArea = All; }
        }
    }

    actions
    {
        area(Navigation)
        {
            action(Orders)
            {
                ApplicationArea = All;
                RunObject = page "Repro Order List";
                RunPageLink = "Project Code" = field(Code);
                RunPageView = sorting("Project Code");
            }
        }
    }
}

page 50102 "Repro Order List"
{
    PageType = List;
    SourceTable = "Repro Order";
    ApplicationArea = All;
    UsageCategory = None;
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field("Project Code"; Rec."Project Code") { ApplicationArea = All; }
            }
        }
    }

    trigger OnAfterGetCurrRecord()
    begin
        LoadFromRecord(Rec);
    end;

    local procedure LoadFromRecord(MainRecordVariant: Variant)
    var
        MainRecordRef: RecordRef;
        LoadLog: Codeunit "Repro Load Log";
    begin
        MainRecordRef.GetTable(MainRecordVariant);
        if MainRecordRef.Get(MainRecordRef.RecordId) then
            LoadLog.Add('found ' + Format(MainRecordRef.Field(1).Value))
        else
            LoadLog.Add('missing');
    end;
}
codeunit 50110 "Repro RunPageLink Copy Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        ListedOrders: Text;

    [Test]
    [HandlerFunctions('OrderListHandler')]
    procedure ListOpenedThroughLinkFindsItsRow()
    var
        LoadLog: Codeunit "Repro Load Log";
        ProjectCard: TestPage "Repro Project Card";
        Project: Record "Repro Project";
    begin
        Setup();
        LoadLog.Clear();
        ListedOrders := '';
        Project.Get('P2');
        ProjectCard.OpenView();
        ProjectCard.GoToRecord(Project);
        ProjectCard.Orders.Invoke();
        if ListedOrders <> 'O3;' then
            Error('The list showed %1.', ListedOrders);
        if LoadLog.Joined() <> 'found O3;' then
            Error('OnAfterGetCurrRecord loaded %1.', LoadLog.Joined());
        ProjectCard.Close();
    end;

    [PageHandler]
    procedure OrderListHandler(var OrderList: TestPage "Repro Order List")
    begin
        if OrderList.First() then
            repeat
                ListedOrders += OrderList."No.".Value() + ';';
            until not OrderList.Next();
    end;

    local procedure Setup()
    var
        Project: Record "Repro Project";
        "Order": Record "Repro Order";
    begin
        Project.DeleteAll();
        "Order".DeleteAll();
        Project.Code := 'P1';
        Project.Insert();
        Project.Code := 'P2';
        Project.Insert();
        InsertOrder('O1', 'P1');
        InsertOrder('O2', 'P1');
        InsertOrder('O3', 'P2');
    end;

    local procedure InsertOrder(No: Code[20]; ProjectCode: Code[20])
    var
        "Order": Record "Repro Order";
    begin
        "Order"."No." := No;
        "Order"."Project Code" := ProjectCode;
        "Order".Insert();
    end;
}
"##;

#[test]
fn issue_7138_get_of_the_record_id_finds_the_row() {
    let o = outcomes_in(ISSUE, "Repro RunPageLink Copy Tests");
    assert_eq!(format!("{o:?}"), r#"[("ListOpenedThroughLinkFindsItsRow", Pass)]"#);
}

/// The objects of testpage/R7138RunPageLinkCopyGet.al with the Probe Observer replaced by Error on a difference.
const PROBE: &str = r##"
table 95980 "TP R7138 Project"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Code; Code[20]) { }
    }

    keys
    {
        key(PK; Code) { Clustered = true; }
    }
}

table 95981 "TP R7138 Order"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Project Code"; Code[20]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
        key(ByProject; "Project Code") { }
    }
}

page 95982 "TP R7138 Project Card"
{
    PageType = Card;
    SourceTable = "TP R7138 Project";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field(Code; Rec.Code) { ApplicationArea = All; }
        }
    }

    actions
    {
        area(Navigation)
        {
            action(Orders)
            {
                ApplicationArea = All;
                RunObject = page "TP R7138 Order List";
                RunPageLink = "Project Code" = field(Code);
                RunPageView = sorting("Project Code");
            }
        }
    }
}

page 95983 "TP R7138 Order List"
{
    PageType = List;
    SourceTable = "TP R7138 Order";
    ApplicationArea = All;
    UsageCategory = None;
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field("Project Code"; Rec."Project Code") { ApplicationArea = All; }
            }
        }
    }

    trigger OnOpenPage()
    var
        Log: Codeunit "TP R7138 Log";
    begin
        Log.Add('open[' + Rec.GetFilters() + '] n=' + Format(Rec.Count()) + ' rec=' + Rec."No.");
    end;

    trigger OnAfterGetCurrRecord()
    var
        Log: Codeunit "TP R7138 Log";
    begin
        Log.Add('agcr ' + Rec."No.");
        FromVariant(Rec);
        FromCopy(Rec);
    end;

    local procedure FromVariant(MainRecordVariant: Variant)
    var
        MainRecordRef: RecordRef;
        Outside: Record "TP R7138 Order";
        Log: Codeunit "TP R7138 Log";
    begin
        MainRecordRef.GetTable(MainRecordVariant);
        Log.Add('rr[' + MainRecordRef.GetFilters() + '] n=' + Format(MainRecordRef.Count()));
        if MainRecordRef.Get(MainRecordRef.RecordId) then
            Log.Add('rr.get own ' + Format(MainRecordRef.Field(1).Value))
        else
            Log.Add('rr.get own missing');
        Outside.Get('O1');
        if MainRecordRef.Get(Outside.RecordId) then
            Log.Add('rr.get O1 ' + Format(MainRecordRef.Field(1).Value))
        else
            Log.Add('rr.get O1 missing');
    end;

    local procedure FromCopy(Copy: Record "TP R7138 Order")
    var
        Own: Code[20];
        Log: Codeunit "TP R7138 Log";
    begin
        Own := Copy."No.";
        Log.Add('copy[' + Copy.GetFilters() + '] n=' + Format(Copy.Count()));
        if Copy.Get(Own) then
            Log.Add('copy.get own ' + Copy."No.")
        else
            Log.Add('copy.get own missing');
        if Copy.Get('O1') then
            Log.Add('copy.get O1 ' + Copy."No.")
        else
            Log.Add('copy.get O1 missing');
    end;
}

codeunit 95984 "TP R7138 Probe"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Listed: Text;
        Walk: Boolean;

    local procedure Initialize()
    var
        Project: Record "TP R7138 Project";
        "Order": Record "TP R7138 Order";
        Log: Codeunit "TP R7138 Log";
    begin
        Project.DeleteAll();
        "Order".DeleteAll();
        Project.Code := 'P1';
        Project.Insert();
        Project.Code := 'P2';
        Project.Insert();
        InsertOrder('O1', 'P1');
        InsertOrder('O2', 'P1');
        InsertOrder('O3', 'P2');
        InsertOrder('O4', 'P2');
        Listed := '';
        Log.Reset();
    end;

    local procedure InsertOrder(No: Code[20]; ProjectCode: Code[20])
    var
        "Order": Record "TP R7138 Order";
    begin
        "Order"."No." := No;
        "Order"."Project Code" := ProjectCode;
        "Order".Insert();
    end;

    local procedure Run(ExpectListed: Text; ExpectLog: Text)
    var
        Project: Record "TP R7138 Project";
        Card: TestPage "TP R7138 Project Card";
        Log: Codeunit "TP R7138 Log";
    begin
        Project.Get('P2');
        Card.OpenView();
        Card.GoToRecord(Project);
        Log.Reset();
        Card.Orders.Invoke();
        if Listed <> ExpectListed then
            Error('listed %1', Listed);
        if Log.Get() <> ExpectLog then
            Error('log %1', Log.Get());
        Card.Close();
    end;

    [Test]
    [HandlerFunctions('OrderListHandler')]
    procedure TP7138_RunPageLinkList_WalkFirstNext_CopyGet()
    begin
        Initialize();
        Walk := true;
        Run('O3;O4;', 'open[Project Code: P2] n=2 rec=;agcr O3;rr[Project Code: P2] n=2;rr.get own O3;rr.get O1 O1;copy[] n=4;copy.get own O3;copy.get O1 O1;handler;agcr O4;rr[Project Code: P2] n=2;rr.get own O4;rr.get O1 O1;copy[] n=4;copy.get own O4;copy.get O1 O1');
    end;

    [Test]
    [HandlerFunctions('OrderListHandler')]
    procedure TP7138_RunPageLinkList_ShownRowOnly_CopyGet()
    begin
        Initialize();
        Walk := false;
        Run('O3;', 'open[Project Code: P2] n=2 rec=;agcr O3;rr[Project Code: P2] n=2;rr.get own O3;rr.get O1 O1;copy[] n=4;copy.get own O3;copy.get O1 O1;handler');
    end;

    [PageHandler]
    procedure OrderListHandler(var OrderList: TestPage "TP R7138 Order List")
    var
        Log: Codeunit "TP R7138 Log";
    begin
        Log.Add('handler');
        if not Walk then begin
            Listed := OrderList."No.".Value() + ';';
            exit;
        end;
        if OrderList.First() then
            repeat
                Listed += OrderList."No.".Value() + ';';
            until not OrderList.Next();
    end;
}

codeunit 95985 "TP R7138 Log"
{
    SingleInstance = true;

    var
        Entries: Text;

    procedure Reset()
    begin
        Entries := '';
    end;

    procedure Add(Entry: Text)
    begin
        if Entries <> '' then
            Entries += ';';
        Entries += Entry;
    end;

    procedure Get(): Text
    begin
        exit(Entries);
    end;
}
"##;

/// R7138RunPageLinkCopyGet.al:210 TP7138_RunPageLinkList_WalkFirstNext_CopyGet and :224 TP7138_RunPageLinkList_ShownRowOnly_CopyGet.
#[test]
fn probe_7138_copies_of_rec_and_their_get() {
    let o = outcomes_in(PROBE, "TP R7138 Probe");
    assert_eq!(
        format!("{o:?}"),
        r#"[("TP7138_RunPageLinkList_WalkFirstNext_CopyGet", Pass), ("TP7138_RunPageLinkList_ShownRowOnly_CopyGet", Pass)]"#
    );
}
