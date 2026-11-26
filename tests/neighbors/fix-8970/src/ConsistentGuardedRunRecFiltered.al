// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/record/record-consistent-method
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/codeunit/codeunit-run-method
// Scope: in-scope
// Fixtures used: Probe Observer (codeunit 69160); own objects 88984-88992 (GRQ X, GRQ Y, GRQ Z, GRQ Mode, GRQ Steps,
//   GRQ Inner X, GRQ Inner Y, GRQ Inner Z, GRQ Tests)
// Probe: from al-runner issue 8970, filtered record argument. Same shape as ConsistentGuardedRunRecFail.al (inner codeunit marks
//   tables with Consistent(false), then ends or fails), but the record argument of Codeunit.Run(Id, Rec) carries a filter. This is
//   the shape of the page-started posting previews: a page's record with filters set (one document of a header table).
// Area: the cells (RecCell: the guarded run under asserterror that ends in Error('END'), so a run that raises and one that returns
//   are both read; then a Commit in the caller; then the durable rows):
//   - Argument table: RecZ_ (never marked), RecX_ (marked first), RecY_ (marked second, after GRQ X), RecTempX_ (a temporary record of
//     GRQ X, the table marked first).
//   - Filter on the argument (the row 'caller', Id 1; each table also holds 'other', Id 2): NoFilter, SetRangeKey (SetRange on Id),
//     SetRangeName (SetRange on Name, not a key field), SetFilter (Name 'call*|zzz'), MarkedOnly (Mark on the row, then MarkedOnly).
//   - Marks (steps of the inner codeunit; startx = insert into GRQ X and Consistent(false) on the same variable, fya/fyc = Consistent(false)
//     on GRQ Y by two variables, fxa = on GRQ X, frec = Consistent(false) on the argument itself):
//       StartX_MarkYByA_Fails (startx,fya,err); StartX_MarkYByA_MarkYByC_Fails (startx,fya,fyc,err);
//       StartX_MarkYByA_Ends (startx,fya); MarkXByA_MarkYByA_PlainCommit (fxa,fya,Commit); MarkRec_Fails (frec,err).
//   - Plain_: the run as a plain statement, no guard of asserterror, for each table, with no filter, SetRange on Name and MarkedOnly.
//   Each cell reads: the run's error, the log of the caller (inner codeunit's view of the argument's filters and count, return value,
//   last error text and code, the argument's Name, filters, MarkedOnly and count after the run), the error of a later Commit, and the
//   rows that are durable. Every cell starts with two committed rows in each table.
// The values are what BC produced. What BC did, in short:
//   - The filter changes nothing. Every cell gives the same result for all five filter shapes, for the table marked first, the table
//     marked second, the unmarked table and the temporary record, and the same as the same cell without a filter and in
//     ConsistentGuardedRunRecFail.al.
//   - The inner codeunit sees the argument's filters and the filtered count (for example 'Id: 1', count 1; 'Marked: Yes', count 1).
//   - A guarded run whose inner codeunit fails with Error after marks returns No; the last error is the inner error (code Dialog);
//     the inner writes are rolled back; the argument keeps its Name, its filters, its MarkedOnly state and its filtered count; a later
//     Commit in the caller does not raise. All rows of the three tables stay durable. A plain statement gives the same.
//   - A mark still held when the inner codeunit ends normally makes the guarded run raise the Consistent error (DB:TableInConsistent).
//   - A plain Commit in the inner codeunit while it holds marks: the run returns No with the Consistent error text.
//   - Marking the argument itself (frec) and then failing: the same as above.
// Not measured (a runtime must refuse these): marks set by the caller before the run; a record argument of a table the caller
//   marked; a filtered temporary record of GRQ Y or GRQ Z; a filter set or changed by the inner codeunit; FieldError and TestField
//   after a mark; nested (not top-level) runs; a write in the caller after its Commit and before the run; more than two marked tables.
// local object: the tests must commit (a guarded Codeunit.Run commits), and the inner codeunit is the code under test.
// BC versions: 27.5, 28.5, 29.0

table 88984 "GRQ X"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 88985 "GRQ Y"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 88986 "GRQ Z"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

// The steps the inner codeunit runs, and a log the caller writes (a SingleInstance codeunit is not rolled back).
codeunit 88987 "GRQ Mode"
{
    SingleInstance = true;

    var
        Steps: Text;
        LogText: Text;

    procedure Set(NewSteps: Text)
    begin
        Steps := NewSteps;
        LogText := '';
    end;

    procedure Get(): Text
    begin
        exit(Steps);
    end;

    procedure Log(Line: Text)
    begin
        LogText += Line + ';';
    end;

    procedure GetLog(): Text
    begin
        exit(LogText);
    end;
}

// The step interpreter: runs the steps of "GRQ Mode", comma separated.
//   startx: insert a row into GRQ X and mark GRQ X from the same local variable (the preview state shape).
//   fxa, fya, fyc: Consistent(false) on a local variable of DoSteps for GRQ X, GRQ Y (a), GRQ Y (c).
//   frec: Consistent(false) on the record argument of the run. commit: Commit. err: Error('inner failed').
codeunit 88988 "GRQ Steps"
{
    procedure DoSteps(var RecX: Record "GRQ X"; var RecY: Record "GRQ Y"; var RecZ: Record "GRQ Z"; Which: Text)
    var
        Mode: Codeunit "GRQ Mode";
        XA: Record "GRQ X";
        YA: Record "GRQ Y";
        YC: Record "GRQ Y";
        Step: Text;
    begin
        foreach Step in Mode.Get().Split(',') do
            case Step of
                'startx':
                    StartX();
                'fxa':
                    XA.Consistent(false);
                'fya':
                    YA.Consistent(false);
                'fyc':
                    YC.Consistent(false);
                'frec':
                    case Which of
                        'x':
                            RecX.Consistent(false);
                        'y':
                            RecY.Consistent(false);
                        'z':
                            RecZ.Consistent(false);
                    end;
                'commit':
                    Commit();
                'err':
                    Error('inner failed');
            end;
    end;

    local procedure StartX()
    var
        X: Record "GRQ X";
    begin
        X.Init();
        X.Id := 50;
        X.Name := 'state';
        X.Insert();
        X.Consistent(false);
    end;
}

// The inner codeunit for a record of GRQ X. It logs the filters and the row count it sees on its argument.
codeunit 88989 "GRQ Inner X"
{
    TableNo = "GRQ X";

    trigger OnRun()
    var
        Steps: Codeunit "GRQ Steps";
        Mode: Codeunit "GRQ Mode";
        NoY: Record "GRQ Y";
        NoZ: Record "GRQ Z";
    begin
        Mode.Log('inner filters=' + Rec.GetFilters() + ' count=' + Format(Rec.Count()));
        Steps.DoSteps(Rec, NoY, NoZ, 'x');
    end;
}

// The inner codeunit for a record of GRQ Y. It logs the filters and the row count it sees on its argument.
codeunit 88990 "GRQ Inner Y"
{
    TableNo = "GRQ Y";

    trigger OnRun()
    var
        Steps: Codeunit "GRQ Steps";
        Mode: Codeunit "GRQ Mode";
        NoX: Record "GRQ X";
        NoZ: Record "GRQ Z";
    begin
        Mode.Log('inner filters=' + Rec.GetFilters() + ' count=' + Format(Rec.Count()));
        Steps.DoSteps(NoX, Rec, NoZ, 'y');
    end;
}

// The inner codeunit for a record of GRQ Z. It logs the filters and the row count it sees on its argument.
codeunit 88991 "GRQ Inner Z"
{
    TableNo = "GRQ Z";

    trigger OnRun()
    var
        Steps: Codeunit "GRQ Steps";
        Mode: Codeunit "GRQ Mode";
        NoX: Record "GRQ X";
        NoY: Record "GRQ Y";
    begin
        Mode.Log('inner filters=' + Rec.GetFilters() + ' count=' + Format(Rec.Count()));
        Steps.DoSteps(NoX, NoY, Rec, 'z');
    end;
}

codeunit 88992 "GRQ Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mode: Codeunit "GRQ Mode";

    // Every cell starts with two committed rows in each table: Id 1 'caller' and Id 2 'other'. The run gets the first one.
    local procedure Initialize(Steps: Text)
    var
        X: Record "GRQ X";
        Y: Record "GRQ Y";
        Z: Record "GRQ Z";
    begin
        X.Consistent(true);
        Y.Consistent(true);
        Z.Consistent(true);
        X.DeleteAll();
        Y.DeleteAll();
        Z.DeleteAll();
        X.Init();
        X.Id := 1;
        X.Name := 'caller';
        X.Insert();
        X.Id := 2;
        X.Name := 'other';
        X.Insert();
        Y.Init();
        Y.Id := 1;
        Y.Name := 'caller';
        Y.Insert();
        Y.Id := 2;
        Y.Name := 'other';
        Y.Insert();
        Z.Init();
        Z.Id := 1;
        Z.Name := 'caller';
        Z.Insert();
        Z.Id := 2;
        Z.Name := 'other';
        Z.Insert();
        Commit();
        ClearLastError();
        Mode.Set(Steps);
    end;

    local procedure Rows(): Text
    var
        X: Record "GRQ X";
        Y: Record "GRQ Y";
        Z: Record "GRQ Z";
        XNames: Text;
        YNames: Text;
        ZNames: Text;
    begin
        if X.FindSet() then
            repeat
                XNames += X.Name + ';';
            until X.Next() = 0;
        if Y.FindSet() then
            repeat
                YNames += Y.Name + ';';
            until Y.Next() = 0;
        if Z.FindSet() then
            repeat
                ZNames += Z.Name + ';';
            until Z.Next() = 0;
        exit('x=[' + XNames + '] y=[' + YNames + '] z=[' + ZNames + ']');
    end;

    // The rows a commit made durable: an unrelated error undoes every write since the last commit.
    local procedure DurableRows(): Text
    begin
        asserterror Error('unrelated');
        ClearLastError();
        exit(Rows());
    end;

    local procedure Describe(var RecRef: RecordRef): Text
    begin
        exit('filters=' + RecRef.GetFilters() + ' markedonly=' + Format(RecRef.MarkedOnly()) + ' count=' + Format(RecRef.Count()));
    end;

    // Runs the inner codeunit. Kind: z, x, y = a database record of GRQ Z / X / Y; tx = a temporary GRQ X record with the same two rows.
    // Filter: none, key, name, flt (SetFilter), marked (Mark the row, then MarkedOnly). The argument is on the row 'caller'.
    local procedure RunRec(Kind: Text; Filter: Text; var ArgName: Text; var ArgState: Text) Ok: Boolean
    var
        X: Record "GRQ X";
        Y: Record "GRQ Y";
        Z: Record "GRQ Z";
        TempX: Record "GRQ X" temporary;
        RecRef: RecordRef;
    begin
        case Kind of
            'z':
                begin
                    PrepareZ(Z, Filter);
                    Ok := Codeunit.Run(Codeunit::"GRQ Inner Z", Z);
                    ArgName := Z.Name;
                    RecRef.GetTable(Z);
                end;
            'x':
                begin
                    PrepareX(X, Filter);
                    Ok := Codeunit.Run(Codeunit::"GRQ Inner X", X);
                    ArgName := X.Name;
                    RecRef.GetTable(X);
                end;
            'y':
                begin
                    PrepareY(Y, Filter);
                    Ok := Codeunit.Run(Codeunit::"GRQ Inner Y", Y);
                    ArgName := Y.Name;
                    RecRef.GetTable(Y);
                end;
            'tx':
                begin
                    TempX.Init();
                    TempX.Id := 1;
                    TempX.Name := 'caller';
                    TempX.Insert();
                    TempX.Id := 2;
                    TempX.Name := 'other';
                    TempX.Insert();
                    PrepareX(TempX, Filter);
                    Ok := Codeunit.Run(Codeunit::"GRQ Inner X", TempX);
                    ArgName := TempX.Name;
                    RecRef.GetTable(TempX);
                end;
        end;
        ArgState := Describe(RecRef);
    end;

    local procedure PrepareX(var Rec: Record "GRQ X"; Filter: Text)
    begin
        case Filter of
            'key':
                Rec.SetRange(Id, 1);
            'name':
                Rec.SetRange(Name, 'caller');
            'flt':
                Rec.SetFilter(Name, 'call*|zzz');
            'marked':
                begin
                    Rec.FindFirst();
                    Rec.Mark(true);
                    Rec.MarkedOnly(true);
                end
        end;
        Rec.FindFirst();
    end;

    local procedure PrepareY(var Rec: Record "GRQ Y"; Filter: Text)
    begin
        case Filter of
            'key':
                Rec.SetRange(Id, 1);
            'name':
                Rec.SetRange(Name, 'caller');
            'flt':
                Rec.SetFilter(Name, 'call*|zzz');
            'marked':
                begin
                    Rec.FindFirst();
                    Rec.Mark(true);
                    Rec.MarkedOnly(true);
                end
        end;
        Rec.FindFirst();
    end;

    local procedure PrepareZ(var Rec: Record "GRQ Z"; Filter: Text)
    begin
        case Filter of
            'key':
                Rec.SetRange(Id, 1);
            'name':
                Rec.SetRange(Name, 'caller');
            'flt':
                Rec.SetFilter(Name, 'call*|zzz');
            'marked':
                begin
                    Rec.FindFirst();
                    Rec.Mark(true);
                    Rec.MarkedOnly(true);
                end
        end;
        Rec.FindFirst();
    end;

    local procedure RunThenEnd(Kind: Text; Filter: Text)
    var
        Ok: Boolean;
        ArgName: Text;
        ArgState: Text;
    begin
        Ok := RunRec(Kind, Filter, ArgName, ArgState);
        Mode.Log('run=' + Format(Ok));
        Mode.Log('text=' + GetLastErrorText());
        Mode.Log('code=' + GetLastErrorCode());
        Mode.Log('arg=' + ArgName);
        Mode.Log('argstate ' + ArgState);
        ClearLastError();
        Error('END');
    end;

    local procedure CommitThenEnd()
    begin
        Commit();
        Error('END');
    end;

    local procedure RecCell(var Observer: Codeunit "Probe Observer"; Kind: Text; Filter: Text; Steps: Text)
    begin
        Initialize(Steps);
        asserterror RunThenEnd(Kind, Filter);
        Observer.ObserveError('run end');
        Observer.Observe('log', Mode.GetLog());
        asserterror CommitThenEnd();
        Observer.ObserveError('commit');
        Observer.Observe('durable rows', DurableRows());
    end;

    // The run as a plain statement (no asserterror around it).
    local procedure PlainCell(var Observer: Codeunit "Probe Observer"; Kind: Text; Filter: Text; Steps: Text)
    var
        Ok: Boolean;
        ArgName: Text;
        ArgState: Text;
    begin
        Initialize(Steps);
        Ok := RunRec(Kind, Filter, ArgName, ArgState);
        Observer.Observe('run', Format(Ok));
        Observer.ObserveError('last error');
        Observer.Observe('arg', ArgName);
        Observer.Observe('argstate', ArgState);
        Observer.Observe('rows after run', Rows());
        asserterror CommitThenEnd();
        Observer.ObserveError('commit');
        Observer.Observe('durable rows', DurableRows());
    end;

    [Test]
    procedure RecZ_NoFilter_StartX_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'none', 'startx,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters= count=2;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters= markedonly=No count=2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_NoFilter_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'none', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters= count=2;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters= markedonly=No count=2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_NoFilter_StartX_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'none', 'startx,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', 'inner filters= count=2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_NoFilter_MarkXByA_MarkYByA_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'none', 'fxa,fya,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters= count=2;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;argstate filters= markedonly=No count=2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_NoFilter_MarkRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'none', 'frec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters= count=2;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters= markedonly=No count=2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_SetRangeKey_StartX_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'key', 'startx,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Id: 1 count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Id: 1 markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_SetRangeKey_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'key', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Id: 1 count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Id: 1 markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_SetRangeKey_StartX_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'key', 'startx,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', 'inner filters=Id: 1 count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_SetRangeKey_MarkXByA_MarkYByA_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'key', 'fxa,fya,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Id: 1 count=1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;argstate filters=Id: 1 markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_SetRangeKey_MarkRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'key', 'frec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Id: 1 count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Id: 1 markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_SetRangeName_StartX_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'name', 'startx,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: caller count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: caller markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_SetRangeName_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'name', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: caller count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: caller markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_SetRangeName_StartX_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'name', 'startx,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', 'inner filters=Name: caller count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_SetRangeName_MarkXByA_MarkYByA_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'name', 'fxa,fya,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: caller count=1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;argstate filters=Name: caller markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_SetRangeName_MarkRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'name', 'frec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: caller count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: caller markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_SetFilter_StartX_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'flt', 'startx,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: call*|zzz count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: call*|zzz markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_SetFilter_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'flt', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: call*|zzz count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: call*|zzz markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_SetFilter_StartX_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'flt', 'startx,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', 'inner filters=Name: call*|zzz count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_SetFilter_MarkXByA_MarkYByA_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'flt', 'fxa,fya,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: call*|zzz count=1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;argstate filters=Name: call*|zzz markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_SetFilter_MarkRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'flt', 'frec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: call*|zzz count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: call*|zzz markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_MarkedOnly_StartX_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'marked', 'startx,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Marked: Yes count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Marked: Yes markedonly=Yes count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_MarkedOnly_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'marked', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Marked: Yes count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Marked: Yes markedonly=Yes count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_MarkedOnly_StartX_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'marked', 'startx,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', 'inner filters=Marked: Yes count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_MarkedOnly_MarkXByA_MarkYByA_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'marked', 'fxa,fya,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Marked: Yes count=1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;argstate filters=Marked: Yes markedonly=Yes count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_MarkedOnly_MarkRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'marked', 'frec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Marked: Yes count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Marked: Yes markedonly=Yes count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_NoFilter_StartX_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'none', 'startx,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters= count=2;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters= markedonly=No count=2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_NoFilter_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'none', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters= count=2;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters= markedonly=No count=2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_NoFilter_StartX_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'none', 'startx,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', 'inner filters= count=2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_NoFilter_MarkXByA_MarkYByA_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'none', 'fxa,fya,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters= count=2;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;argstate filters= markedonly=No count=2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_NoFilter_MarkRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'none', 'frec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters= count=2;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters= markedonly=No count=2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_SetRangeKey_StartX_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'key', 'startx,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Id: 1 count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Id: 1 markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_SetRangeKey_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'key', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Id: 1 count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Id: 1 markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_SetRangeKey_StartX_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'key', 'startx,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', 'inner filters=Id: 1 count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_SetRangeKey_MarkXByA_MarkYByA_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'key', 'fxa,fya,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Id: 1 count=1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;argstate filters=Id: 1 markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_SetRangeKey_MarkRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'key', 'frec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Id: 1 count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Id: 1 markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_SetRangeName_StartX_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'name', 'startx,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: caller count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: caller markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_SetRangeName_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'name', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: caller count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: caller markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_SetRangeName_StartX_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'name', 'startx,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', 'inner filters=Name: caller count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_SetRangeName_MarkXByA_MarkYByA_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'name', 'fxa,fya,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: caller count=1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;argstate filters=Name: caller markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_SetRangeName_MarkRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'name', 'frec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: caller count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: caller markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_SetFilter_StartX_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'flt', 'startx,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: call*|zzz count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: call*|zzz markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_SetFilter_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'flt', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: call*|zzz count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: call*|zzz markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_SetFilter_StartX_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'flt', 'startx,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', 'inner filters=Name: call*|zzz count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_SetFilter_MarkXByA_MarkYByA_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'flt', 'fxa,fya,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: call*|zzz count=1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;argstate filters=Name: call*|zzz markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_SetFilter_MarkRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'flt', 'frec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: call*|zzz count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: call*|zzz markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_MarkedOnly_StartX_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'marked', 'startx,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Marked: Yes count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Marked: Yes markedonly=Yes count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_MarkedOnly_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'marked', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Marked: Yes count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Marked: Yes markedonly=Yes count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_MarkedOnly_StartX_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'marked', 'startx,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', 'inner filters=Marked: Yes count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_MarkedOnly_MarkXByA_MarkYByA_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'marked', 'fxa,fya,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Marked: Yes count=1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;argstate filters=Marked: Yes markedonly=Yes count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_MarkedOnly_MarkRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'marked', 'frec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Marked: Yes count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Marked: Yes markedonly=Yes count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_NoFilter_StartX_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'none', 'startx,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters= count=2;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters= markedonly=No count=2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_NoFilter_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'none', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters= count=2;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters= markedonly=No count=2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_NoFilter_StartX_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'none', 'startx,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', 'inner filters= count=2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_NoFilter_MarkXByA_MarkYByA_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'none', 'fxa,fya,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters= count=2;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;argstate filters= markedonly=No count=2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_NoFilter_MarkRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'none', 'frec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters= count=2;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters= markedonly=No count=2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_SetRangeKey_StartX_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'key', 'startx,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Id: 1 count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Id: 1 markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_SetRangeKey_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'key', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Id: 1 count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Id: 1 markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_SetRangeKey_StartX_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'key', 'startx,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', 'inner filters=Id: 1 count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_SetRangeKey_MarkXByA_MarkYByA_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'key', 'fxa,fya,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Id: 1 count=1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;argstate filters=Id: 1 markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_SetRangeKey_MarkRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'key', 'frec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Id: 1 count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Id: 1 markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_SetRangeName_StartX_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'name', 'startx,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: caller count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: caller markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_SetRangeName_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'name', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: caller count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: caller markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_SetRangeName_StartX_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'name', 'startx,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', 'inner filters=Name: caller count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_SetRangeName_MarkXByA_MarkYByA_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'name', 'fxa,fya,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: caller count=1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;argstate filters=Name: caller markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_SetRangeName_MarkRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'name', 'frec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: caller count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: caller markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_SetFilter_StartX_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'flt', 'startx,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: call*|zzz count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: call*|zzz markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_SetFilter_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'flt', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: call*|zzz count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: call*|zzz markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_SetFilter_StartX_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'flt', 'startx,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', 'inner filters=Name: call*|zzz count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_SetFilter_MarkXByA_MarkYByA_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'flt', 'fxa,fya,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: call*|zzz count=1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;argstate filters=Name: call*|zzz markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_SetFilter_MarkRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'flt', 'frec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: call*|zzz count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: call*|zzz markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_MarkedOnly_StartX_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'marked', 'startx,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Marked: Yes count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Marked: Yes markedonly=Yes count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_MarkedOnly_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'marked', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Marked: Yes count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Marked: Yes markedonly=Yes count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_MarkedOnly_StartX_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'marked', 'startx,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', 'inner filters=Marked: Yes count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_MarkedOnly_MarkXByA_MarkYByA_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'marked', 'fxa,fya,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Marked: Yes count=1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;argstate filters=Marked: Yes markedonly=Yes count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecY_MarkedOnly_MarkRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'y', 'marked', 'frec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Marked: Yes count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Marked: Yes markedonly=Yes count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_NoFilter_StartX_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'none', 'startx,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters= count=2;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters= markedonly=No count=2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_NoFilter_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'none', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters= count=2;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters= markedonly=No count=2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_NoFilter_StartX_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'none', 'startx,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', 'inner filters= count=2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_NoFilter_MarkXByA_MarkYByA_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'none', 'fxa,fya,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters= count=2;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;argstate filters= markedonly=No count=2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_NoFilter_MarkRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'none', 'frec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters= count=2;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters= markedonly=No count=2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_SetRangeKey_StartX_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'key', 'startx,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Id: 1 count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Id: 1 markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_SetRangeKey_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'key', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Id: 1 count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Id: 1 markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_SetRangeKey_StartX_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'key', 'startx,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', 'inner filters=Id: 1 count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_SetRangeKey_MarkXByA_MarkYByA_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'key', 'fxa,fya,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Id: 1 count=1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;argstate filters=Id: 1 markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_SetRangeKey_MarkRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'key', 'frec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Id: 1 count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Id: 1 markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_SetRangeName_StartX_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'name', 'startx,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: caller count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: caller markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_SetRangeName_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'name', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: caller count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: caller markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_SetRangeName_StartX_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'name', 'startx,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', 'inner filters=Name: caller count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_SetRangeName_MarkXByA_MarkYByA_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'name', 'fxa,fya,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: caller count=1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;argstate filters=Name: caller markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_SetRangeName_MarkRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'name', 'frec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: caller count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: caller markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_SetFilter_StartX_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'flt', 'startx,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: call*|zzz count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: call*|zzz markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_SetFilter_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'flt', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: call*|zzz count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: call*|zzz markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_SetFilter_StartX_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'flt', 'startx,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', 'inner filters=Name: call*|zzz count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_SetFilter_MarkXByA_MarkYByA_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'flt', 'fxa,fya,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: call*|zzz count=1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;argstate filters=Name: call*|zzz markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_SetFilter_MarkRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'flt', 'frec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Name: call*|zzz count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Name: call*|zzz markedonly=No count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_MarkedOnly_StartX_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'marked', 'startx,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Marked: Yes count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Marked: Yes markedonly=Yes count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_MarkedOnly_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'marked', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Marked: Yes count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Marked: Yes markedonly=Yes count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_MarkedOnly_StartX_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'marked', 'startx,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', 'inner filters=Marked: Yes count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_MarkedOnly_MarkXByA_MarkYByA_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'marked', 'fxa,fya,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Marked: Yes count=1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRQ X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;argstate filters=Marked: Yes markedonly=Yes count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure RecTempX_MarkedOnly_MarkRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tx', 'marked', 'frec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'inner filters=Marked: Yes count=1;run=No;text=inner failed;code=Dialog;arg=caller;argstate filters=Marked: Yes markedonly=Yes count=1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure Plain_RecZ_NoFilter_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PlainCell(Observer, 'z', 'none', 'startx,fya,fyc,err');
        Observer.Expect('run', 'No');
        Observer.Expect('last error|text', 'inner failed');
        Observer.Expect('last error|code', 'Dialog');
        Observer.Expect('arg', 'caller');
        Observer.Expect('argstate', 'filters= markedonly=No count=2');
        Observer.Expect('rows after run', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure Plain_RecZ_SetRangeName_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PlainCell(Observer, 'z', 'name', 'startx,fya,fyc,err');
        Observer.Expect('run', 'No');
        Observer.Expect('last error|text', 'inner failed');
        Observer.Expect('last error|code', 'Dialog');
        Observer.Expect('arg', 'caller');
        Observer.Expect('argstate', 'filters=Name: caller markedonly=No count=1');
        Observer.Expect('rows after run', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure Plain_RecZ_MarkedOnly_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PlainCell(Observer, 'z', 'marked', 'startx,fya,fyc,err');
        Observer.Expect('run', 'No');
        Observer.Expect('last error|text', 'inner failed');
        Observer.Expect('last error|code', 'Dialog');
        Observer.Expect('arg', 'caller');
        Observer.Expect('argstate', 'filters=Marked: Yes markedonly=Yes count=1');
        Observer.Expect('rows after run', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure Plain_RecX_NoFilter_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PlainCell(Observer, 'x', 'none', 'startx,fya,fyc,err');
        Observer.Expect('run', 'No');
        Observer.Expect('last error|text', 'inner failed');
        Observer.Expect('last error|code', 'Dialog');
        Observer.Expect('arg', 'caller');
        Observer.Expect('argstate', 'filters= markedonly=No count=2');
        Observer.Expect('rows after run', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure Plain_RecX_SetRangeName_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PlainCell(Observer, 'x', 'name', 'startx,fya,fyc,err');
        Observer.Expect('run', 'No');
        Observer.Expect('last error|text', 'inner failed');
        Observer.Expect('last error|code', 'Dialog');
        Observer.Expect('arg', 'caller');
        Observer.Expect('argstate', 'filters=Name: caller markedonly=No count=1');
        Observer.Expect('rows after run', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure Plain_RecX_MarkedOnly_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PlainCell(Observer, 'x', 'marked', 'startx,fya,fyc,err');
        Observer.Expect('run', 'No');
        Observer.Expect('last error|text', 'inner failed');
        Observer.Expect('last error|code', 'Dialog');
        Observer.Expect('arg', 'caller');
        Observer.Expect('argstate', 'filters=Marked: Yes markedonly=Yes count=1');
        Observer.Expect('rows after run', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure Plain_RecY_NoFilter_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PlainCell(Observer, 'y', 'none', 'startx,fya,fyc,err');
        Observer.Expect('run', 'No');
        Observer.Expect('last error|text', 'inner failed');
        Observer.Expect('last error|code', 'Dialog');
        Observer.Expect('arg', 'caller');
        Observer.Expect('argstate', 'filters= markedonly=No count=2');
        Observer.Expect('rows after run', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure Plain_RecY_SetRangeName_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PlainCell(Observer, 'y', 'name', 'startx,fya,fyc,err');
        Observer.Expect('run', 'No');
        Observer.Expect('last error|text', 'inner failed');
        Observer.Expect('last error|code', 'Dialog');
        Observer.Expect('arg', 'caller');
        Observer.Expect('argstate', 'filters=Name: caller markedonly=No count=1');
        Observer.Expect('rows after run', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure Plain_RecY_MarkedOnly_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PlainCell(Observer, 'y', 'marked', 'startx,fya,fyc,err');
        Observer.Expect('run', 'No');
        Observer.Expect('last error|text', 'inner failed');
        Observer.Expect('last error|code', 'Dialog');
        Observer.Expect('arg', 'caller');
        Observer.Expect('argstate', 'filters=Marked: Yes markedonly=Yes count=1');
        Observer.Expect('rows after run', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure Plain_RecTempX_NoFilter_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PlainCell(Observer, 'tx', 'none', 'startx,fya,fyc,err');
        Observer.Expect('run', 'No');
        Observer.Expect('last error|text', 'inner failed');
        Observer.Expect('last error|code', 'Dialog');
        Observer.Expect('arg', 'caller');
        Observer.Expect('argstate', 'filters= markedonly=No count=2');
        Observer.Expect('rows after run', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure Plain_RecTempX_SetRangeName_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PlainCell(Observer, 'tx', 'name', 'startx,fya,fyc,err');
        Observer.Expect('run', 'No');
        Observer.Expect('last error|text', 'inner failed');
        Observer.Expect('last error|code', 'Dialog');
        Observer.Expect('arg', 'caller');
        Observer.Expect('argstate', 'filters=Name: caller markedonly=No count=1');
        Observer.Expect('rows after run', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;

    [Test]
    procedure Plain_RecTempX_MarkedOnly_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PlainCell(Observer, 'tx', 'marked', 'startx,fya,fyc,err');
        Observer.Expect('run', 'No');
        Observer.Expect('last error|text', 'inner failed');
        Observer.Expect('last error|code', 'Dialog');
        Observer.Expect('arg', 'caller');
        Observer.Expect('argstate', 'filters=Marked: Yes markedonly=Yes count=1');
        Observer.Expect('rows after run', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'x=[caller;other;] y=[caller;other;] z=[caller;other;]');
        Observer.Verify();
    end;
}
