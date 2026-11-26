// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/system/system-clearall-method
// Scope: in-scope
// Fixtures used: Probe Observer (codeunit 69160); "CKK Row" (table 86270) of ClearSharedCodeunitGlobalKinds.al; self-contained objects 86279 to 86287
// Probe: written against BC 27.5, 28.5 and 29.0 (al-runner issue 8964), observed first, then asserted.
// Facts: ClearAll in the Sales-Post shape resets the register codeunit the caller shares with it (entry counter,
// records, flag), but the register's nested tracking codeunit keeps its temporary rows. When the register had already
// registered entry 101 before the post, the post registers 101 again and fails with the duplicate-key error.
// BC versions: 27, 28, 29

codeunit 86279 "CKW UOM"
{
    var
        ItemUOM: Record "CKK Row";
        ResUOM: Record "CKK Row";

    procedure Prime()
    begin
        ItemUOM."No." := 8;
        ResUOM.SetRange("No.", 8);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('uom=%1|%2', ItemUOM."No.", ResUOM.GetFilters()));
    end;
}

codeunit 86280 "CKW Confirm"
{
    procedure Ping(): Integer
    begin
        exit(1);
    end;
}

codeunit 86281 "CKW WhseMgt"
{
    procedure Ping(): Integer
    begin
        exit(1);
    end;
}

// "Item Tracking Management" shape: three temporary records, two records, two nested codeunits, an Integer, a Date,
// four Booleans.
codeunit 86282 "CKW Tracking"
{
    var
        TempSource: Record "CKK Row" temporary;
        TempTracking: Record "CKK Row" temporary;
        TempGlobalWhse: Record "CKK Row" temporary;
        CachedItem: Record "CKK Row";
        CachedCode: Record "CKK Row";
        Confirm: Codeunit "CKW Confirm";
        UOM: Codeunit "CKW UOM";
        ProdLine: Integer;
        DueDate: Date;
        Registering: Boolean;
        IsConsume: Boolean;
        IsPick: Boolean;
        DeleteReserv: Boolean;

    procedure Prime()
    begin
        AddRow(TempSource, 1);
        AddRow(TempSource, 2);
        AddRow(TempTracking, 1);
        AddRow(TempGlobalWhse, 1);
        CachedItem."No." := 4;
        CachedCode.SetRange("No.", 4);
        UOM.Prime();
        ProdLine := 3;
        DueDate := DMY2Date(1, 1, 2021);
        Registering := true;
        IsConsume := true;
        IsPick := true;
        DeleteReserv := true;
    end;

    procedure AddWhseLine(V: Integer)
    begin
        AddRow(TempGlobalWhse, V);
    end;

    local procedure AddRow(var R: Record "CKK Row" temporary; V: Integer)
    begin
        R.Init();
        R."No." := V;
        R.Insert();
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('rows=%1/%2/%3 item=%4 code=%5 %6 prod=%7 due=%8 flags=%9',
            TempSource.Count(), TempTracking.Count(), TempGlobalWhse.Count(), CachedItem."No.", CachedCode.GetFilters(),
            UOM.State(), ProdLine, Format(DueDate, 0, 9),
            Format(Registering, 0, 9) + Format(IsConsume, 0, 9) + Format(IsPick, 0, 9) + Format(DeleteReserv, 0, 9)) +
            ' confirm=' + Format(Confirm.Ping()));
    end;
}

// "WMS Management" shape: five records, a nested tracking codeunit of its own, two nested codeunits, an Integer.
codeunit 86283 "CKW WMS"
{
    var
        ItemUOM: Record "CKK Row";
        Location: Record "CKK Row";
        Bin: Record "CKK Row";
        Item: Record "CKK Row";
        TrackingCode: Record "CKK Row";
        Tracking: Codeunit "CKW Tracking";
        WhseMgt: Codeunit "CKW WhseMgt";
        UOM: Codeunit "CKW UOM";
        NextLineNo: Integer;

    procedure Prime()
    begin
        ItemUOM."No." := 1;
        Location.SetRange("No.", 2);
        Bin."No." := 3;
        Item.Txt := 'IT';
        TrackingCode."No." := 5;
        Tracking.Prime();
        UOM.Prime();
        NextLineNo := 10000;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('recs=%1|%2|%3|%4|%5 next=%6 %7 [%8] whse=%9',
            ItemUOM."No.", Location.GetFilters(), Bin."No.", Item.Txt, TrackingCode."No.", NextLineNo, UOM.State(),
            Tracking.State(), WhseMgt.Ping()));
    end;
}

// "Whse. Jnl.-Register Line" shape: four records, nested tracking and WMS codeunits, an Integer, two Labels, a Boolean.
codeunit 86284 "CKW Register"
{
    var
        Location: Record "CKK Row";
        WhseJnlLine: Record "CKK Row";
        Bin: Record "CKK Row";
        WhseReg: Record "CKK Row";
        Tracking: Codeunit "CKW Tracking";
        WMS: Codeunit "CKW WMS";
        GlobalWhseEntryNo: Integer;
        Text000: Label 'is not sufficient %1 %2';
        Text001: Label 'Serial No. %1 is found.';
        OnMovement: Boolean;

    procedure Prime()
    begin
        Location.SetRange("No.", 1);
        Location."No." := 1;
        WhseJnlLine.Txt := 'JNL';
        Bin."No." := 3;
        WhseReg.Init();
        WhseReg."No." := 50;
        WhseReg.Insert();
        Tracking.Prime();
        WMS.Prime();
        GlobalWhseEntryNo := 5;
        OnMovement := true;
    end;

    // Registers one entry: the first entry after a reset starts from 100 (the real codeunit reads the last entry).
    procedure RegisterEntry()
    begin
        if GlobalWhseEntryNo = 0 then
            GlobalWhseEntryNo := 100;
        GlobalWhseEntryNo += 1;
        OnMovement := false;
        Tracking.AddWhseLine(GlobalWhseEntryNo);
    end;

    procedure ShareTracking(var T: Codeunit "CKW Tracking")
    begin
        T := Tracking;
    end;

    procedure Entry(): Integer
    begin
        exit(GlobalWhseEntryNo);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1|%2 jnl=%3 bin=%4 reg=%5 entry=%6 mov=%7 lbl=%8',
            Location.GetFilters(), Location."No.", WhseJnlLine.Txt, Bin."No.", WhseReg."No.", GlobalWhseEntryNo,
            Format(OnMovement, 0, 9), StrLen(StrSubstNo(Text000, 1, 2)) + StrLen(Text001)));
    end;

    procedure TrackingState(): Text
    begin
        exit(Tracking.State());
    end;

    procedure WMSState(): Text
    begin
        exit(WMS.State());
    end;
}

// "Sales-Post" shape: TableNo, the register as a global handed in by SetWhseJnlRegisterCU (var), OnRun calls
// RunWithCheck, which saves two flags, runs ClearAllVariables (ClearAll, then DeleteAll of a temporary global), restores
// the flags and posts through the register.
codeunit 86285 "CKW Sales Post"
{
    TableNo = "CKK Row";

    var
        WhseJnlPostLine: Codeunit "CKW Register";
        TempSalesLineGlobal: Record "CKK Row" temporary;
        SuppressCommit: Boolean;
        PreviewMode: Boolean;
        StateAfterClear: Text;
        TrackingAfterClear: Text;
        WMSAfterClear: Text;
        EntryAfterPost: Integer;

    trigger OnRun()
    begin
        RunWithCheck(Rec);
    end;

    procedure RunWithCheck(var Row: Record "CKK Row")
    var
        SavedSuppressCommit: Boolean;
        SavedPreviewMode: Boolean;
    begin
        SavedSuppressCommit := SuppressCommit;
        SavedPreviewMode := PreviewMode;
        ClearAllVariables();
        SuppressCommit := SavedSuppressCommit;
        PreviewMode := SavedPreviewMode;
        StateAfterClear := WhseJnlPostLine.State();
        TrackingAfterClear := WhseJnlPostLine.TrackingState();
        WMSAfterClear := WhseJnlPostLine.WMSState();
        WhseJnlPostLine.RegisterEntry();
        EntryAfterPost := WhseJnlPostLine.Entry();
    end;

    local procedure ClearAllVariables()
    begin
        ClearAll();
        TempSalesLineGlobal.DeleteAll();
    end;

    procedure SetWhseJnlRegisterCU(var WhseJnlRegisterLine: Codeunit "CKW Register")
    begin
        WhseJnlPostLine := WhseJnlRegisterLine;
    end;

    procedure SetSuppressCommit(NewSuppressCommit: Boolean)
    begin
        SuppressCommit := NewSuppressCommit;
    end;

    procedure AddSalesLine(V: Integer)
    begin
        TempSalesLineGlobal.Init();
        TempSalesLineGlobal."No." := V;
        TempSalesLineGlobal.Insert();
    end;

    procedure ObservePost(var Observer: Codeunit "Probe Observer")
    begin
        Observer.Observe('post.reg', StateAfterClear);
        Observer.Observe('post.trk', TrackingAfterClear);
        Observer.Observe('post.wms', WMSAfterClear);
        Observer.Observe('post.entry', Format(EntryAfterPost));
        Observer.Observe('post.suppress', Format(SuppressCommit, 0, 9));
        Observer.Observe('post.saleslines', Format(TempSalesLineGlobal.Count()));
    end;
}

// "Whse.-Activity-Post" shape: the register and Sales-Post are globals of the caller; the caller clears Sales-Post,
// hands its register in, runs Sales-Post and then keeps using its register.
codeunit 86286 "CKW Activity Post"
{
    var
        WhseJnlRegisterLine: Codeunit "CKW Register";
        SalesPost: Codeunit "CKW Sales Post";
        TrackingHandle: Codeunit "CKW Tracking";

    // Before: 1 primed (every global written), 2 one entry registered, 3 never used. Call: 1 SalesPost.Run(Row),
    // 2 SalesPost.RunWithCheck(Row) called directly. Handle: the caller also holds the register's nested tracking codeunit.
    procedure PostSourceDocument(Before: Integer; Call: Integer; Handle: Boolean; var Observer: Codeunit "Probe Observer")
    var
        Row: Record "CKK Row";
    begin
        Row.DeleteAll();
        case Before of
            1:
                WhseJnlRegisterLine.Prime();
            2:
                WhseJnlRegisterLine.RegisterEntry();
        end;
        if Handle then
            WhseJnlRegisterLine.ShareTracking(TrackingHandle);
        Clear(SalesPost);
        SalesPost.AddSalesLine(1);
        SalesPost.SetWhseJnlRegisterCU(WhseJnlRegisterLine);
        SalesPost.SetSuppressCommit(true);
        Row."No." := 1;
        if Call = 1 then
            SalesPost.Run(Row)
        else
            SalesPost.RunWithCheck(Row);
        SalesPost.ObservePost(Observer);
        Observer.Observe('caller.entry', Format(WhseJnlRegisterLine.Entry()));
        if Handle then
            Observer.Observe('handle.trk', TrackingHandle.State());
        WhseJnlRegisterLine.RegisterEntry();
        Observer.Observe('caller.reg', WhseJnlRegisterLine.State());
        Observer.Observe('caller.trk', WhseJnlRegisterLine.TrackingState());
        Observer.Observe('caller.wms', WhseJnlRegisterLine.WMSState());
        if Handle then
            Observer.Observe('handle.after', TrackingHandle.State());
        Observer.Observe('dbrows', Format(Row.Count()));
    end;
}

codeunit 86287 "CKW Whse Shape Test"
{
    Subtype = Test;

    [Test]
    procedure WhseRegister_Primed_Run()
    var
        Caller: Codeunit "CKW Activity Post";
        Observer: Codeunit "Probe Observer";
    begin
        Caller.PostSourceDocument(1, 1, false, Observer);
        Observer.Expect('post.reg', 'loc=|0 jnl= bin=0 reg=0 entry=0 mov=false lbl=44');
        Observer.Expect('post.trk', 'rows=2/1/1 item=0 code= uom=0| prod=0 due= flags=falsefalsefalsefalse confirm=1');
        Observer.Expect('post.wms', 'recs=0||0||0 next=0 uom=0| [rows=2/1/1 item=0 code= uom=0| prod=0 due= flags=falsefalsefalsefalse confirm=1] whse=1');
        Observer.Expect('post.entry', '101');
        Observer.Expect('post.suppress', 'true');
        Observer.Expect('post.saleslines', '0');
        Observer.Expect('caller.entry', '101');
        Observer.Expect('caller.reg', 'loc=|0 jnl= bin=0 reg=0 entry=102 mov=false lbl=44');
        Observer.Expect('caller.trk', 'rows=2/1/3 item=0 code= uom=0| prod=0 due= flags=falsefalsefalsefalse confirm=1');
        Observer.Expect('caller.wms', 'recs=0||0||0 next=0 uom=0| [rows=2/1/1 item=0 code= uom=0| prod=0 due= flags=falsefalsefalsefalse confirm=1] whse=1');
        Observer.Expect('dbrows', '1');
        Observer.Verify();
    end;

    [Test]
    procedure WhseRegister_OneEntry_Run()
    var
        Caller: Codeunit "CKW Activity Post";
        Observer: Codeunit "Probe Observer";
    begin
        // The post stops with an error here; the observations made before it and the error text are kept.
        asserterror Caller.PostSourceDocument(2, 1, false, Observer);
        Observer.Observe('error', GetLastErrorText());
        Observer.Expect('error', 'The record in table CKK Row already exists. Identification fields and values: No.=''101''');
        Observer.Verify();
    end;

    [Test]
    procedure WhseRegister_NeverUsed_Run()
    var
        Caller: Codeunit "CKW Activity Post";
        Observer: Codeunit "Probe Observer";
    begin
        Caller.PostSourceDocument(3, 1, false, Observer);
        Observer.Expect('post.reg', 'loc=|0 jnl= bin=0 reg=0 entry=0 mov=false lbl=44');
        Observer.Expect('post.trk', 'rows=0/0/0 item=0 code= uom=0| prod=0 due= flags=falsefalsefalsefalse confirm=1');
        Observer.Expect('post.wms', 'recs=0||0||0 next=0 uom=0| [rows=0/0/0 item=0 code= uom=0| prod=0 due= flags=falsefalsefalsefalse confirm=1] whse=1');
        Observer.Expect('post.entry', '101');
        Observer.Expect('post.suppress', 'true');
        Observer.Expect('post.saleslines', '0');
        Observer.Expect('caller.entry', '101');
        Observer.Expect('caller.reg', 'loc=|0 jnl= bin=0 reg=0 entry=102 mov=false lbl=44');
        Observer.Expect('caller.trk', 'rows=0/0/2 item=0 code= uom=0| prod=0 due= flags=falsefalsefalsefalse confirm=1');
        Observer.Expect('caller.wms', 'recs=0||0||0 next=0 uom=0| [rows=0/0/0 item=0 code= uom=0| prod=0 due= flags=falsefalsefalsefalse confirm=1] whse=1');
        Observer.Expect('dbrows', '0');
        Observer.Verify();
    end;

    [Test]
    procedure WhseRegister_Primed_DirectCall()
    var
        Caller: Codeunit "CKW Activity Post";
        Observer: Codeunit "Probe Observer";
    begin
        Caller.PostSourceDocument(1, 2, false, Observer);
        Observer.Expect('post.reg', 'loc=|0 jnl= bin=0 reg=0 entry=0 mov=false lbl=44');
        Observer.Expect('post.trk', 'rows=2/1/1 item=0 code= uom=0| prod=0 due= flags=falsefalsefalsefalse confirm=1');
        Observer.Expect('post.wms', 'recs=0||0||0 next=0 uom=0| [rows=2/1/1 item=0 code= uom=0| prod=0 due= flags=falsefalsefalsefalse confirm=1] whse=1');
        Observer.Expect('post.entry', '101');
        Observer.Expect('post.suppress', 'true');
        Observer.Expect('post.saleslines', '0');
        Observer.Expect('caller.entry', '101');
        Observer.Expect('caller.reg', 'loc=|0 jnl= bin=0 reg=0 entry=102 mov=false lbl=44');
        Observer.Expect('caller.trk', 'rows=2/1/3 item=0 code= uom=0| prod=0 due= flags=falsefalsefalsefalse confirm=1');
        Observer.Expect('caller.wms', 'recs=0||0||0 next=0 uom=0| [rows=2/1/1 item=0 code= uom=0| prod=0 due= flags=falsefalsefalsefalse confirm=1] whse=1');
        Observer.Expect('dbrows', '1');
        Observer.Verify();
    end;

    [Test]
    procedure WhseRegister_Primed_NestedHandle()
    var
        Caller: Codeunit "CKW Activity Post";
        Observer: Codeunit "Probe Observer";
    begin
        Caller.PostSourceDocument(1, 1, true, Observer);
        Observer.Expect('post.reg', 'loc=|0 jnl= bin=0 reg=0 entry=0 mov=false lbl=44');
        Observer.Expect('post.trk', 'rows=2/1/1 item=0 code= uom=0| prod=0 due= flags=falsefalsefalsefalse confirm=1');
        Observer.Expect('post.wms', 'recs=0||0||0 next=0 uom=0| [rows=2/1/1 item=0 code= uom=0| prod=0 due= flags=falsefalsefalsefalse confirm=1] whse=1');
        Observer.Expect('post.entry', '101');
        Observer.Expect('post.suppress', 'true');
        Observer.Expect('post.saleslines', '0');
        Observer.Expect('caller.entry', '101');
        Observer.Expect('handle.trk', 'rows=2/1/2 item=0 code= uom=0| prod=0 due= flags=falsefalsefalsefalse confirm=1');
        Observer.Expect('caller.reg', 'loc=|0 jnl= bin=0 reg=0 entry=102 mov=false lbl=44');
        Observer.Expect('caller.trk', 'rows=2/1/3 item=0 code= uom=0| prod=0 due= flags=falsefalsefalsefalse confirm=1');
        Observer.Expect('caller.wms', 'recs=0||0||0 next=0 uom=0| [rows=2/1/1 item=0 code= uom=0| prod=0 due= flags=falsefalsefalsefalse confirm=1] whse=1');
        Observer.Expect('handle.after', 'rows=2/1/3 item=0 code= uom=0| prod=0 due= flags=falsefalsefalsefalse confirm=1');
        Observer.Expect('dbrows', '1');
        Observer.Verify();
    end;
}
