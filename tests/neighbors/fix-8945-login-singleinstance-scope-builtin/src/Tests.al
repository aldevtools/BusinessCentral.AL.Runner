// Neighbors of al-runner issue 8945: what the test reads in SingleInstance codeunits of partner dependency apps that more than
// two OnAfterLogin subscribers changed (the shapes of corpus PR 1567, LsLoginSubscriberTests.al, with XApps A, B and C as
// dependency apps), and the shapes next to them that no probe cell measured. The app under test has no login subscriber of
// its own: one that reaches the memos the tests read makes the run raise the login through the stand-in login code.
// This copy runs on al-runner's builtin test runner (no row write before the tests) and adds two neighbors: State GT (first
// load in a "Global Triggers" setup subscriber) and the probe's load-once cache (Load Memo).
table 60924 "LS Watched Write"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { AutoIncrement = true; }
        field(2; Tag; Text[30]) { }
    }

    keys { key(PK; "Entry No.") { Clustered = true; } }
}

table 60925 "LS Fresh Write"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { AutoIncrement = true; }
        field(2; Tag; Text[30]) { }
    }

    keys { key(PK; "Entry No.") { Clustered = true; } }
}

codeunit 60921 "LS Check"
{
    procedure Eq(What: Text; Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('%1: got %2, want %3', What, Got, Want);
    end;

    procedure Windows(): Text
    var
        TrigMemo: Codeunit "LS XA Trig Memo";
        I: Integer;
        Entry: Text;
        Result: Text;
        Started: Boolean;
    begin
        for I := 1 to TrigMemo.EntryCount() do begin
            Entry := TrigMemo.EntryAt(I);
            if Entry.StartsWith('M') then begin
                Started := true;
                Result += '|' + Entry.Substring(2) + ':';
            end else
                if Started and ((Entry in ['T60931', 'T60932', 'T60933', 'T60950', 'T60960']) or Entry.StartsWith('W')) then
                    Result += Entry + ',';
        end;
        exit(Result);
    end;
}

codeunit 60922 "LS Login Tests A"
{
    Subtype = Test;

    [Test]
    procedure Order_ThreeApps()
    var
        OrderMemo: Codeunit "LS XA Order Memo";
        Check: Codeunit "LS Check";
    begin
        Check.Eq('order', OrderMemo.Joined(), 'A1;A2;A3;A4a;A4m;A4z;B1;B2;C1;C2;');
        Check.Eq('A1 runs', Format(OrderMemo.Occurrences('A1')), '1');
    end;

    [Test]
    procedure StateOne_ThreeSubscribersOfOneApp()
    var
        One: Codeunit "LS XA State One";
        Check: Codeunit "LS Check";
    begin
        Check.Eq('loaded', One.RawLoaded() + One.RawLoadedBy() + One.RawActivated(), 'YesA1Yes');
        Check.Eq('setup', One.SetupKeys(), '2:S1;S2;');
        Check.Eq('touches', One.RawTouches(), '3');
        Check.Eq('flag', One.RawFlag(), 'Yes');
        Check.Eq('txt', One.RawTxt(), 'A3');
        Check.Eq('num', One.RawNum(), '123');
        Check.Eq('log', One.LogRows(), '1=A1;2=A2;3=A3;');
    end;

    [Test]
    procedure StateMany_OneSubscriberOfEachApp()
    var
        Many: Codeunit "LS XA State Many";
        Check: Codeunit "LS Check";
    begin
        Check.Eq('loaded', Many.RawLoaded() + Many.RawLoadedBy() + Many.RawActivated(), 'YesA1Yes');
        Check.Eq('setup', Many.SetupKeys(), '2:S1;S2;');
        Check.Eq('txt', Many.RawTxt(), 'C1');
        Check.Eq('num', Many.RawNum(), '123');
        Check.Eq('log', Many.LogRows(), '1=A1;2=B1;3=C1;');
    end;

    [Test]
    procedure TrigMemo_FilledByGlobalTriggers()
    var
        TrigMemo: Codeunit "LS XA Trig Memo";
        Check: Codeunit "LS Check";
    begin
        Check.Eq('checked', TrigMemo.RawChecked(), 'Yes');
        Check.Eq('windows', Check.Windows(), '|A1:T60931,WI60931,|A2:T60932,WI60932,|A3:T60933,WI60933,|A4a:|A4m:|A4z:|B1:T60950,WI60950,|B2:|C1:T60960,WI60960,|C2:');
        Check.Eq('written', TrigMemo.WrittenRows(), '1=I60931;2=I60932;3=I60933;4=I60950;5=I60960;');
        Check.Eq('cache', TrigMemo.CacheKeys(), '5:T60931;T60932;T60933;T60950;T60960;');
    end;

    [Test]
    procedure StateDec_DecimalGlobal()
    var
        Dec: Codeunit "LS XA State Dec";
    begin
        if Dec.RawRate() <> 123 then
            Error('rate %1', Dec.RawRate());
    end;

    [Test]
    procedure StateNoOwn_NoSubscriberOfItsApp()
    var
        NoOwn: Codeunit "LS XA State NoOwn";
    begin
        if NoOwn.Count() <> 2 then
            Error('count %1', NoOwn.Count());
    end;

    [Test]
    procedure ReadsNothing()
    begin
    end;

    [Test]
    procedure StateOne_SecondTest_ThenTestTouches()
    var
        One: Codeunit "LS XA State One";
        Check: Codeunit "LS Check";
    begin
        Check.Eq('txt', One.RawTxt(), 'A3');
        One.Touch('T', 9);
        Check.Eq('num after', One.RawNum(), '1239');
    end;

    [Test]
    procedure StateGT_FirstLoadInGlobalTriggers()
    var
        GT: Codeunit "LS XA State GT";
        Check: Codeunit "LS Check";
    begin
        Check.Eq('loaded by', GT.RawLoadedBy(), 'GT');
        Check.Eq('touches', GT.RawTouches(), '3');
        Check.Eq('num', GT.RawNum(), '123');
        Check.Eq('summary', GT.Summary(), 'YesYesYesA3:2:S1;S2;1=A1;2=A2;3=A3;');
    end;

    [Test]
    procedure LoadMemo_AfterLogin()
    var
        LoadMemo: Codeunit "LS XA Load Memo";
        Check: Codeunit "LS Check";
    begin
        Check.Eq('load', LoadMemo.Summary(), 'YesYes1:6:T60924;T60931;T60932;T60933;T60950;T60960;:60931;60932;60933;60950;60960;');
    end;

    [Test]
    procedure LoadMemo_TestWritesFresh()
    var
        LoadMemo: Codeunit "LS XA Load Memo";
        Fresh: Record "LS Fresh Write";
        Check: Codeunit "LS Check";
        Before: Integer;
    begin
        Before := LoadMemo.AskCount();
        Fresh.DeleteAll();
        Check.Eq('asks by DeleteAll', Format(LoadMemo.AskCount() - Before), '1');
        Before := LoadMemo.AskCount();
        Fresh.Init();
        Fresh.Tag := 'F1';
        Fresh.Insert();
        Check.Eq('asks by insert', Format(LoadMemo.AskCount() - Before), '0');
        Check.Eq('load', LoadMemo.Summary(), 'YesYes1:6:T60924;T60931;T60932;T60933;T60950;T60960;:60931;60932;60933;60950;60960;');
    end;

    [Test]
    procedure LoadMemo_TestWritesWatched()
    var
        LoadMemo: Codeunit "LS XA Load Memo";
        Watched: Record "LS Watched Write";
        Check: Codeunit "LS Check";
    begin
        Watched.DeleteAll();
        Watched.Init();
        Watched.Tag := 'W1';
        Watched.Insert();
        Check.Eq('load', LoadMemo.Summary(), 'YesYes1:6:T60924;T60931;T60932;T60933;T60950;T60960;:60931;60932;60933;60950;60960;60924;');
    end;
}

codeunit 60923 "LS Login Tests B"
{
    Subtype = Test;

    [Test]
    procedure Order_SecondCodeunit()
    var
        OrderMemo: Codeunit "LS XA Order Memo";
        Check: Codeunit "LS Check";
    begin
        Check.Eq('order', OrderMemo.Joined(), 'A1;A2;A3;A4a;A4m;A4z;B1;B2;C1;C2;');
    end;

    [Test]
    procedure StateMany_SecondCodeunit()
    var
        Many: Codeunit "LS XA State Many";
        Check: Codeunit "LS Check";
    begin
        Check.Eq('log', Many.LogRows(), '1=A1;2=B1;3=C1;');
    end;

    [Test]
    procedure LoadMemo_SecondCodeunit()
    var
        LoadMemo: Codeunit "LS XA Load Memo";
        Check: Codeunit "LS Check";
    begin
        Check.Eq('load', LoadMemo.Summary(), 'YesYes1:6:T60924;T60931;T60932;T60933;T60950;T60960;:60931;60932;60933;60950;60960;60924;');
    end;
}
