// The reporter's repro of al-runner issue 6114 (green on real BC), app and test files in one source.
table 50100 "View Order Line"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Group Code"; Code[20]) { }
        field(2; "Entry No."; Integer) { }
        field(3; "Sort No."; Integer) { }
        field(4; Description; Text[50]) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
        key(BySort; "Group Code", "Sort No.") { }
    }
}

page 50101 "View Order Asc List"
{
    PageType = List;
    SourceTable = "View Order Line";
    SourceTableView = sorting("Sort No.") order(ascending);
    UsageCategory = None;
    ApplicationArea = All;
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("Entry No."; Rec."Entry No.") { ApplicationArea = All; }
                field("Sort No."; Rec."Sort No.") { ApplicationArea = All; }
                field(Description; Rec.Description) { ApplicationArea = All; }
            }
        }
    }
}

page 50102 "View Order Desc List"
{
    PageType = List;
    SourceTable = "View Order Line";
    SourceTableView = sorting("Sort No.") order(descending);
    UsageCategory = None;
    ApplicationArea = All;
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("Entry No."; Rec."Entry No.") { ApplicationArea = All; }
                field("Sort No."; Rec."Sort No.") { ApplicationArea = All; }
                field(Description; Rec.Description) { ApplicationArea = All; }
            }
        }
    }
}

codeunit 50120 "View Order Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit "Library Assert";
        Log: Text;

    [Test]
    [HandlerFunctions('AscModalHandler')]
    [TransactionModel(TransactionModel::AutoCommit)]
    procedure AscViewAscRecPositioned()
    var
        Line: Record "View Order Line";
    begin
        Initialize();
        Line.SetRange("Group Code", 'G1');
        Line.FindFirst();
        Page.RunModal(Page::"View Order Asc List", Line);
        Assert.AreEqual('filter=G1 entry=E1S20 rows=E2S10,E1S20,E3S30,', Log, 'RunModal, view sorting(Sort No.) order(ascending), record ascending on primary key after FindFirst');
    end;

    [Test]
    [HandlerFunctions('AscModalHandler')]
    [TransactionModel(TransactionModel::AutoCommit)]
    procedure AscViewAscRec()
    var
        Line: Record "View Order Line";
    begin
        Initialize();
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"View Order Asc List", Line);
        Assert.AreEqual('filter=G1 entry=E2S10 rows=E2S10,E1S20,E3S30,', Log, 'RunModal, view order(ascending), record ascending, not positioned');
    end;

    [Test]
    [HandlerFunctions('DescModalHandler')]
    [TransactionModel(TransactionModel::AutoCommit)]
    procedure DescViewAscRec()
    var
        Line: Record "View Order Line";
    begin
        Initialize();
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"View Order Desc List", Line);
        Assert.AreEqual('filter=G1 entry=E2S10 rows=E3S30,E1S20,E2S10,', Log, 'RunModal, view order(descending), record ascending, not positioned');
    end;

    [Test]
    [HandlerFunctions('AscModalHandler')]
    [TransactionModel(TransactionModel::AutoCommit)]
    procedure AscViewDescRec()
    var
        Line: Record "View Order Line";
    begin
        Initialize();
        Line.SetRange("Group Code", 'G1');
        Line.Ascending(false);
        Page.RunModal(Page::"View Order Asc List", Line);
        Assert.AreEqual('filter=G1 entry=E2S10 rows=E2S10,E1S20,E3S30,', Log, 'RunModal, view order(ascending), record descending, not positioned');
    end;

    [Test]
    [HandlerFunctions('DescModalHandler')]
    [TransactionModel(TransactionModel::AutoCommit)]
    procedure DescViewAscRecPositioned()
    var
        Line: Record "View Order Line";
    begin
        Initialize();
        Line.SetRange("Group Code", 'G1');
        Line.FindFirst();
        Page.RunModal(Page::"View Order Desc List", Line);
        Assert.AreEqual('filter=G1 entry=E1S20 rows=E3S30,E1S20,E2S10,', Log, 'RunModal, view order(descending), record ascending on primary key after FindFirst');
    end;

    [Test]
    [HandlerFunctions('DescPageHandler')]
    [TransactionModel(TransactionModel::AutoCommit)]
    procedure DescViewAscRecPageRun()
    var
        Line: Record "View Order Line";
    begin
        Initialize();
        Line.SetRange("Group Code", 'G1');
        Page.Run(Page::"View Order Desc List", Line);
        Assert.AreEqual('filter=G1 entry=E3S30 rows=E3S30,E1S20,E2S10,', Log, 'Page.Run, view order(descending), record ascending, not positioned');
    end;

    [Test]
    [HandlerFunctions('AscPageHandler')]
    [TransactionModel(TransactionModel::AutoCommit)]
    procedure AscViewDescRecPageRun()
    var
        Line: Record "View Order Line";
    begin
        Initialize();
        Line.SetRange("Group Code", 'G1');
        Line.Ascending(false);
        Page.Run(Page::"View Order Asc List", Line);
        Assert.AreEqual('filter=G1 entry=E2S10 rows=E2S10,E1S20,E3S30,', Log, 'Page.Run, view order(ascending), record descending, not positioned');
    end;

    [Test]
    [HandlerFunctions('DescPageHandler')]
    [TransactionModel(TransactionModel::AutoCommit)]
    procedure DescViewDescRecPageRun()
    var
        Line: Record "View Order Line";
    begin
        Initialize();
        Line.SetRange("Group Code", 'G1');
        Line.Ascending(false);
        Page.Run(Page::"View Order Desc List", Line);
        Assert.AreEqual('filter=G1 entry=E3S30 rows=E3S30,E1S20,E2S10,', Log, 'Page.Run, view order(descending), record descending, not positioned');
    end;

    [Test]
    [HandlerFunctions('DescPageHandler')]
    [TransactionModel(TransactionModel::AutoCommit)]
    procedure DescViewAscRecPositionedPageRun()
    var
        Line: Record "View Order Line";
    begin
        Initialize();
        Line.SetRange("Group Code", 'G1');
        Line.FindFirst();
        Page.Run(Page::"View Order Desc List", Line);
        Assert.AreEqual('filter=G1 entry=E1S20 rows=E3S30,E1S20,E2S10,', Log, 'Page.Run, view order(descending), record ascending on primary key after FindFirst');
    end;

    [Test]
    [HandlerFunctions('AscPageHandler')]
    [TransactionModel(TransactionModel::AutoCommit)]
    procedure AscViewAscRecPageRun()
    var
        Line: Record "View Order Line";
    begin
        Initialize();
        Line.SetRange("Group Code", 'G1');
        Page.Run(Page::"View Order Asc List", Line);
        Assert.AreEqual('filter=G1 entry=E2S10 rows=E2S10,E1S20,E3S30,', Log, 'Page.Run, view order(ascending), record ascending, not positioned');
    end;

    [Test]
    [HandlerFunctions('DescPageHandler')]
    [TransactionModel(TransactionModel::AutoCommit)]
    procedure DescViewNoRecPageRun()
    begin
        Initialize();
        Page.Run(Page::"View Order Desc List");
        Assert.AreEqual('filter= entry=E3S30 rows=E3S30,E1S20,E2S10,G2E1,', Log, 'Page.Run, view order(descending), no record');
    end;

    local procedure Initialize()
    var
        Line: Record "View Order Line";
    begin
        Log := '';
        Line.DeleteAll();
        InsertLine('G1', 1, 20, 'E1S20');
        InsertLine('G1', 2, 10, 'E2S10');
        InsertLine('G1', 3, 30, 'E3S30');
        InsertLine('G2', 1, 5, 'G2E1');
        Commit();
    end;

    local procedure InsertLine(GroupCode: Code[20]; EntryNo: Integer; SortNo: Integer; Desc: Text[50])
    var
        Line: Record "View Order Line";
    begin
        Line."Group Code" := GroupCode;
        Line."Entry No." := EntryNo;
        Line."Sort No." := SortNo;
        Line.Description := Desc;
        Line.Insert();
    end;

    [ModalPageHandler]
    procedure AscModalHandler(var ListPage: TestPage "View Order Asc List")
    begin
        Log := 'filter=' + ListPage.Filter.GetFilter("Group Code") + ' entry=' + ListPage.Description.Value() + ' rows=';
        if ListPage.First() then
            repeat
                Log += ListPage.Description.Value() + ',';
            until not ListPage.Next();
    end;

    [ModalPageHandler]
    procedure DescModalHandler(var ListPage: TestPage "View Order Desc List")
    begin
        Log := 'filter=' + ListPage.Filter.GetFilter("Group Code") + ' entry=' + ListPage.Description.Value() + ' rows=';
        if ListPage.First() then
            repeat
                Log += ListPage.Description.Value() + ',';
            until not ListPage.Next();
    end;

    [PageHandler]
    procedure AscPageHandler(var ListPage: TestPage "View Order Asc List")
    begin
        Log := 'filter=' + ListPage.Filter.GetFilter("Group Code") + ' entry=' + ListPage.Description.Value() + ' rows=';
        if ListPage.First() then
            repeat
                Log += ListPage.Description.Value() + ',';
            until not ListPage.Next();
    end;

    [PageHandler]
    procedure DescPageHandler(var ListPage: TestPage "View Order Desc List")
    begin
        Log := 'filter=' + ListPage.Filter.GetFilter("Group Code") + ' entry=' + ListPage.Description.Value() + ' rows=';
        if ListPage.First() then
            repeat
                Log += ListPage.Description.Value() + ',';
            until not ListPage.Next();
    end;
}

