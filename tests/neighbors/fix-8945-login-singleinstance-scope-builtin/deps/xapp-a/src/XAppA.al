// XApp A of al-runner issue 8945, a partner dependency app: the shapes of corpus PR 1567 (LsXaLoginSubscribers.al), renumbered,
// plus three neighbor memos and a copy of the probe's load-once cache. Several "System Initialization".OnAfterLogin subscribers change the same SingleInstance codeunits,
// directly and through this app's "Global Triggers" subscribers.
table 60930 "LS XA Setup"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; Description; Text[30]) { }
    }

    keys { key(PK; "Code") { Clustered = true; } }
}

table 60931 "LS XA Write 1"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { AutoIncrement = true; }
        field(2; Tag; Text[30]) { }
    }

    keys { key(PK; "Entry No.") { Clustered = true; } }
}

table 60932 "LS XA Write 2"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { AutoIncrement = true; }
        field(2; Tag; Text[30]) { }
    }

    keys { key(PK; "Entry No.") { Clustered = true; } }
}

table 60933 "LS XA Write 3"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { AutoIncrement = true; }
        field(2; Tag; Text[30]) { }
    }

    keys { key(PK; "Entry No.") { Clustered = true; } }
}

table 60934 "LS XA Log"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Tag; Text[30]) { }
    }

    keys { key(PK; "Entry No.") { Clustered = true; } }
}

// singleinstance-shared-on-purpose: the probe is what the login subscribers leave in these codeunits for the test session, read by
// two test codeunits (the second one also sees what the first one changed).
codeunit 60935 "LS XA Order Memo"
{
    SingleInstance = true;

    var
        Order: List of [Text];
        LoginGui: Text;

    procedure Add(Tag: Text)
    begin
        Order.Add(Tag);
        if LoginGui = '' then
            LoginGui := Format(GuiAllowed());
    end;

    procedure Joined(): Text
    var
        Tag: Text;
        Result: Text;
    begin
        foreach Tag in Order do
            Result += Tag + ';';
        exit(Result);
    end;

    procedure Count(): Integer
    begin
        exit(Order.Count());
    end;

    procedure Occurrences(Tag: Text): Integer
    var
        Entry: Text;
        N: Integer;
    begin
        foreach Entry in Order do
            if Entry = Tag then
                N += 1;
        exit(N);
    end;

    procedure RawLoginGui(): Text
    begin
        exit(LoginGui);
    end;
}

codeunit 60936 "LS XA State One"
{
    SingleInstance = true;

    var
        TempSetup: Record "LS XA Setup" temporary;
        TempLog: Record "LS XA Log" temporary;
        Loaded: Boolean;
        Activated: Boolean;
        Flag: Boolean;
        LoadedBy: Text;
        Txt: Text;
        Num: Integer;
        Touches: Integer;

    procedure Touch(Tag: Text; K: Integer)
    var
        Hub: Codeunit "LS XA Login Hub";
    begin
        if not Loaded then begin
            Loaded := true;
            LoadedBy := Tag;
            Activated := Hub.LoadSetup(TempSetup);
        end;
        Touches += 1;
        Flag := true;
        Txt := Tag;
        Num := Num * 10 + K;
        TempLog.Init();
        TempLog."Entry No." := TempLog.Count() + 1;
        TempLog.Tag := CopyStr(Tag, 1, MaxStrLen(TempLog.Tag));
        TempLog.Insert();
    end;

    procedure RawLoaded(): Text
    begin
        exit(Format(Loaded));
    end;

    procedure RawLoadedBy(): Text
    begin
        exit(LoadedBy);
    end;

    procedure RawActivated(): Text
    begin
        exit(Format(Activated));
    end;

    procedure SetupKeys(): Text
    var
        Hub: Codeunit "LS XA Login Hub";
    begin
        exit(Hub.SetupKeys(TempSetup));
    end;

    procedure RawFlag(): Text
    begin
        exit(Format(Flag));
    end;

    procedure RawTxt(): Text
    begin
        exit(Txt);
    end;

    procedure RawNum(): Text
    begin
        exit(Format(Num));
    end;

    procedure RawTouches(): Text
    begin
        exit(Format(Touches));
    end;

    procedure LogRows(): Text
    var
        Hub: Codeunit "LS XA Login Hub";
    begin
        exit(Hub.LogRows(TempLog));
    end;
}

codeunit 60937 "LS XA State Many"
{
    SingleInstance = true;

    var
        TempSetup: Record "LS XA Setup" temporary;
        TempLog: Record "LS XA Log" temporary;
        Loaded: Boolean;
        Activated: Boolean;
        Flag: Boolean;
        LoadedBy: Text;
        Txt: Text;
        Num: Integer;
        Touches: Integer;

    procedure Touch(Tag: Text; K: Integer)
    var
        Hub: Codeunit "LS XA Login Hub";
    begin
        if not Loaded then begin
            Loaded := true;
            LoadedBy := Tag;
            Activated := Hub.LoadSetup(TempSetup);
        end;
        Touches += 1;
        Flag := true;
        Txt := Tag;
        Num := Num * 10 + K;
        TempLog.Init();
        TempLog."Entry No." := TempLog.Count() + 1;
        TempLog.Tag := CopyStr(Tag, 1, MaxStrLen(TempLog.Tag));
        TempLog.Insert();
    end;

    procedure RawLoaded(): Text
    begin
        exit(Format(Loaded));
    end;

    procedure RawLoadedBy(): Text
    begin
        exit(LoadedBy);
    end;

    procedure RawActivated(): Text
    begin
        exit(Format(Activated));
    end;

    procedure SetupKeys(): Text
    var
        Hub: Codeunit "LS XA Login Hub";
    begin
        exit(Hub.SetupKeys(TempSetup));
    end;

    procedure RawFlag(): Text
    begin
        exit(Format(Flag));
    end;

    procedure RawTxt(): Text
    begin
        exit(Txt);
    end;

    procedure RawNum(): Text
    begin
        exit(Format(Num));
    end;

    procedure RawTouches(): Text
    begin
        exit(Format(Touches));
    end;

    procedure LogRows(): Text
    var
        Hub: Codeunit "LS XA Login Hub";
    begin
        exit(Hub.LogRows(TempLog));
    end;
}

codeunit 60938 "LS XA Trig Memo"
{
    SingleInstance = true;

    var
        TempCache: Record "LS XA Setup" temporary;
        TempWritten: Record "LS XA Log" temporary;
        Entries: List of [Text];
        Checked: Boolean;
        CheckedAt: Integer;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure OnSetup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean)
    begin
        Entries.Add('T' + Format(TableId));
        if not Checked then begin
            Checked := true;
            CheckedAt := Entries.Count();
            FillCache();
        end;
        if TempCache.Get('T' + Format(TableId)) then begin
            OnDatabaseInsert := true;
            OnDatabaseModify := true;
        end;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseInsert', '', false, false)]
    local procedure OnInsert(RecRef: RecordRef)
    begin
        NoteWrite('I', RecRef.Number);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseModify', '', false, false)]
    local procedure OnModify(RecRef: RecordRef)
    begin
        NoteWrite('M', RecRef.Number);
    end;

    local procedure FillCache()
    begin
        AddCache(Database::"LS XA Write 1");
        AddCache(Database::"LS XA Write 2");
        AddCache(Database::"LS XA Write 3");
        AddCache(60950); // XApp B's write table
        AddCache(60960); // XApp C's write table
    end;

    local procedure AddCache(TableId: Integer)
    begin
        TempCache.Init();
        TempCache.Code := CopyStr('T' + Format(TableId), 1, MaxStrLen(TempCache.Code));
        TempCache.Insert();
    end;

    local procedure NoteWrite(Kind: Text; TableId: Integer)
    begin
        if not TempCache.Get('T' + Format(TableId)) then
            exit;
        Entries.Add('W' + Kind + Format(TableId));
        TempWritten.Init();
        TempWritten."Entry No." := TempWritten.Count() + 1;
        TempWritten.Tag := CopyStr(Kind + Format(TableId), 1, MaxStrLen(TempWritten.Tag));
        TempWritten.Insert();
    end;

    procedure Mark(Tag: Text)
    begin
        Entries.Add('M' + Tag);
    end;

    procedure EntryCount(): Integer
    begin
        exit(Entries.Count());
    end;

    procedure EntryAt(Index: Integer): Text
    begin
        exit(Entries.Get(Index));
    end;

    // First index after After whose entry is Entry; 0 when there is none.
    procedure IndexOf(Entry: Text; After: Integer): Integer
    var
        I: Integer;
    begin
        for I := After + 1 to Entries.Count() do
            if Entries.Get(I) = Entry then
                exit(I);
        exit(0);
    end;

    procedure Occurrences(Entry: Text): Integer
    var
        E: Text;
        N: Integer;
    begin
        foreach E in Entries do
            if E = Entry then
                N += 1;
        exit(N);
    end;

    procedure RawChecked(): Text
    begin
        exit(Format(Checked));
    end;

    procedure RawCheckedAt(): Integer
    begin
        exit(CheckedAt);
    end;

    procedure CacheKeys(): Text
    var
        Hub: Codeunit "LS XA Login Hub";
    begin
        exit(Hub.SetupKeys(TempCache));
    end;

    procedure WrittenRows(): Text
    var
        Hub: Codeunit "LS XA Login Hub";
    begin
        exit(Hub.LogRows(TempWritten));
    end;
}

codeunit 60939 "LS XA Login Hub"
{
    // What every login subscriber does first: its tag into the order memo and its marker into the trigger memo.
    procedure Enter(Tag: Text)
    var
        OrderMemo: Codeunit "LS XA Order Memo";
        TrigMemo: Codeunit "LS XA Trig Memo";
    begin
        OrderMemo.Add(Tag);
        TrigMemo.Mark(Tag);
    end;

    procedure CanWrite(): Boolean
    begin
        exit(GuiAllowed());
    end;

    // Seeds the setup rows (S1, S2) when the session may write, then reports whether the setup table has rows and copies them.
    procedure LoadSetup(var TempSetup: Record "LS XA Setup" temporary): Boolean
    var
        Setup: Record "LS XA Setup";
    begin
        if CanWrite() then begin
            SeedOne('S1');
            SeedOne('S2');
        end;
        TempSetup.Reset();
        TempSetup.DeleteAll();
        if Setup.FindSet() then
            repeat
                TempSetup := Setup;
                TempSetup.Insert();
            until Setup.Next() = 0;
        exit(not Setup.IsEmpty());
    end;

    local procedure SeedOne(SetupCode: Code[20])
    var
        Setup: Record "LS XA Setup";
    begin
        if Setup.Get(SetupCode) then
            exit;
        Setup.Init();
        Setup.Code := SetupCode;
        Setup.Description := 'seeded by the login';
        if Setup.Insert() then;
    end;

    procedure SetupKeys(var TempSetup: Record "LS XA Setup" temporary): Text
    var
        Result: Text;
    begin
        TempSetup.Reset();
        if TempSetup.FindSet() then
            repeat
                Result += TempSetup.Code + ';';
            until TempSetup.Next() = 0;
        exit(Format(TempSetup.Count()) + ':' + Result);
    end;

    procedure LogRows(var TempLog: Record "LS XA Log" temporary): Text
    var
        Result: Text;
    begin
        TempLog.Reset();
        if TempLog.FindSet() then
            repeat
                Result += Format(TempLog."Entry No.") + '=' + TempLog.Tag + ';';
            until TempLog.Next() = 0;
        exit(Result);
    end;

    procedure WriteRow1(Tag: Text)
    var
        Row: Record "LS XA Write 1";
    begin
        if not CanWrite() then
            exit;
        Row.Init();
        Row.Tag := CopyStr(Tag, 1, MaxStrLen(Row.Tag));
        Row.Insert(true);
    end;

    procedure WriteRow2(Tag: Text)
    var
        Row: Record "LS XA Write 2";
    begin
        if not CanWrite() then
            exit;
        Row.Init();
        Row.Tag := CopyStr(Tag, 1, MaxStrLen(Row.Tag));
        Row.Insert(true);
    end;

    procedure WriteRow3(Tag: Text)
    var
        Row: Record "LS XA Write 3";
    begin
        if not CanWrite() then
            exit;
        Row.Init();
        Row.Tag := CopyStr(Tag, 1, MaxStrLen(Row.Tag));
        Row.Insert(true);
    end;
}

codeunit 60940 "LS XA Zulu Subs"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure OnAfterLogin()
    var
        Hub: Codeunit "LS XA Login Hub";
        One: Codeunit "LS XA State One";
        Many: Codeunit "LS XA State Many";
        StateGT: Codeunit "LS XA State GT";
        Dec: Codeunit "LS XA State Dec";
    begin
        Hub.Enter('A1');
        One.Touch('A1', 1);
        Dec.Touch(1);
        Many.Touch('A1', 1);
        Hub.WriteRow1('A1');
        StateGT.Touch('A1', 1);
    end;
}

codeunit 60941 "LS XA Alpha Subs"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure OnAfterLogin()
    var
        Hub: Codeunit "LS XA Login Hub";
        One: Codeunit "LS XA State One";
        StateGT: Codeunit "LS XA State GT";
        Dec: Codeunit "LS XA State Dec";
    begin
        Hub.Enter('A2');
        One.Touch('A2', 2);
        Dec.Touch(2);
        Hub.WriteRow2('A2');
        StateGT.Touch('A2', 2);
    end;
}

codeunit 60942 "LS XA Mike Subs"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure OnAfterLogin()
    var
        Hub: Codeunit "LS XA Login Hub";
        One: Codeunit "LS XA State One";
        StateGT: Codeunit "LS XA State GT";
        Dec: Codeunit "LS XA State Dec";
    begin
        Hub.Enter('A3');
        One.Touch('A3', 3);
        Dec.Touch(3);
        Hub.WriteRow3('A3');
        StateGT.Touch('A3', 3);
    end;
}

codeunit 60943 "LS XA Two Procs"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure ZuluLogin()
    var
        Hub: Codeunit "LS XA Login Hub";
    begin
        Hub.Enter('A4z');
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure AlphaLogin()
    var
        Hub: Codeunit "LS XA Login Hub";
    begin
        Hub.Enter('A4a');
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure MikeLogin()
    var
        Hub: Codeunit "LS XA Login Hub";
    begin
        Hub.Enter('A4m');
    end;
}

// Neighbor: the shape of State One with a Decimal global (a type no probe cell measured).
codeunit 60944 "LS XA State Dec"
{
    SingleInstance = true;

    var
        Rate: Decimal;
        Touches: Integer;

    procedure Touch(K: Integer)
    begin
        Touches += 1;
        Rate := Rate * 10 + K;
    end;

    procedure RawRate(): Decimal
    begin
        exit(Rate);
    end;
}

// Neighbor: a memo of this app that only XApp B's and XApp C's subscribers change (none of its own app).
codeunit 60945 "LS XA State NoOwn"
{
    SingleInstance = true;

    var
        Order: List of [Text];

    procedure Touch(Tag: Text)
    begin
        Order.Add(Tag);
    end;

    procedure Count(): Integer
    begin
        exit(Order.Count());
    end;
}

// Neighbor: the shape of State One whose first load runs in this codeunit's "Global Triggers" setup subscriber (asked by A1's
// write), not in a login subscriber; A1, A2 and A3 then touch it.
codeunit 60946 "LS XA State GT"
{
    SingleInstance = true;

    var
        TempSetup: Record "LS XA Setup" temporary;
        TempLog: Record "LS XA Log" temporary;
        Loaded: Boolean;
        Activated: Boolean;
        Flag: Boolean;
        LoadedBy: Text;
        Txt: Text;
        Num: Integer;
        Touches: Integer;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure OnSetup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean)
    begin
        LoadOnce('GT');
    end;

    local procedure LoadOnce(Tag: Text)
    var
        Hub: Codeunit "LS XA Login Hub";
    begin
        if Loaded then
            exit;
        Loaded := true;
        LoadedBy := Tag;
        Activated := Hub.LoadSetup(TempSetup);
    end;

    procedure Touch(Tag: Text; K: Integer)
    begin
        LoadOnce(Tag);
        Touches += 1;
        Flag := true;
        Txt := Tag;
        Num := Num * 10 + K;
        TempLog.Init();
        TempLog."Entry No." := TempLog.Count() + 1;
        TempLog.Tag := CopyStr(Tag, 1, MaxStrLen(TempLog.Tag));
        TempLog.Insert();
    end;

    procedure RawLoadedBy(): Text
    begin
        exit(LoadedBy);
    end;

    procedure RawTouches(): Text
    begin
        exit(Format(Touches));
    end;

    procedure RawNum(): Text
    begin
        exit(Format(Num));
    end;

    procedure Summary(): Text
    var
        Hub: Codeunit "LS XA Login Hub";
    begin
        exit(Format(Loaded) + Format(Activated) + Format(Flag) + Txt + ':' + Hub.SetupKeys(TempSetup) + Hub.LogRows(TempLog));
    end;
}

// A copy of the probe's load-once cache (corpus PR 1567, "LT LS XA Load Memo" 98538), on "Global Triggers" instead of
// GlobalTriggerManagement (the stand-in platform has no Base Application): its setup subscriber does the one-time load on the
// first ask whatever table is asked, reads TableId only after that, and notes the inserts of the watched tables. No login
// subscriber calls it.
codeunit 60947 "LS XA Load Memo"
{
    SingleInstance = true;

    var
        TempWatched: Record "LS XA Setup" temporary;
        Checked: Boolean;
        Activated: Boolean;
        WatchedLoaded: Boolean;
        Loads: Integer;
        Asks: Integer;
        Written: Text;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure AfterSetup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean)
    begin
        Asks += 1;
        if not Checked then begin
            Checked := true;
            Activated := IsActivated();
        end;
        if not Activated then
            exit;
        if not WatchedLoaded then begin
            WatchedLoaded := true;
            Loads += 1;
            LoadWatched();
        end;
        if TempWatched.Get('T' + Format(TableId)) then begin
            OnDatabaseInsert := true;
            OnDatabaseModify := true;
        end;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseInsert', '', false, false)]
    local procedure AfterInsert(RecRef: RecordRef)
    begin
        if TempWatched.Get('T' + Format(RecRef.Number)) then
            Written += Format(RecRef.Number) + ';';
    end;

    local procedure IsActivated(): Boolean
    var
        AllObj: Record AllObj;
    begin
        AllObj.SetRange("Object Type", AllObj."Object Type"::Table);
        AllObj.SetRange("Object ID", Database::"LS XA Setup");
        exit(not AllObj.IsEmpty());
    end;

    local procedure LoadWatched()
    begin
        AddWatched(Database::"LS XA Write 1");
        AddWatched(Database::"LS XA Write 2");
        AddWatched(Database::"LS XA Write 3");
        AddWatched(60950); // XApp B's write table
        AddWatched(60960); // XApp C's write table
        AddWatched(60924); // "LS Watched Write" of the app under test, written by a test only
    end;

    local procedure AddWatched(TableId: Integer)
    begin
        TempWatched.Init();
        TempWatched.Code := CopyStr('T' + Format(TableId), 1, MaxStrLen(TempWatched.Code));
        TempWatched.Insert();
    end;

    procedure Summary(): Text
    var
        Hub: Codeunit "LS XA Login Hub";
    begin
        exit(Format(Checked) + Format(Activated) + Format(Loads) + ':' + Hub.SetupKeys(TempWatched) + ':' + Written);
    end;

    procedure AskCount(): Integer
    begin
        exit(Asks);
    end;
}
