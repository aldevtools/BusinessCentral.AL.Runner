// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/record/record-consistent-method
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/codeunit/codeunit-run-method
// Scope: in-scope
// Fixtures used: Probe Observer (codeunit 69160); own objects 88975-88979, 88981-88983 (GRR X, GRR Y, GRR Z, GRR Mode, GRR Steps,
//   GRR Inner Z, GRR Inner X, GRR Tests)
// Probe: from al-runner issue 8970, record argument. Same cells as ConsistentGuardedRunFail.al (inner codeunit marks tables with
//   Consistent(false), then ends or fails), but the guarded run passes a record: Ok := Codeunit.Run(Id, Rec).
// Area: Codeunit.Run(Id, Rec) with a record argument, while the inner codeunit marks a table with Consistent(false).
//   - RecZ_*: the argument is a committed row of GRR Z (a table the marks do not name); the inner codeunit has TableNo GRR Z.
//   - RecX_*: the argument is a committed row of GRR X (the table the inner codeunit marks); the inner codeunit has TableNo GRR X.
//   - RecTemp_*: the argument is a temporary GRR X record.
//   - Plain_*: the same run as a plain statement, without the guard.
//   - Cells: the 13 mark/ends/fails sequences and the 6 Commit cells (plain Commit, CommitBehavior::Error, ::Ignore) of
//     ConsistentGuardedRunFail.al; the inner codeunit marking or modifying its own argument (MarkRec, ModifyRec, TrueRec).
//   Each cell reads the run's error, the log (return value, last error text and code, the argument's Name after the run), whether
//   a Commit in the caller raises, and the rows that are durable. Every cell starts with one committed row in GRR X and GRR Z.
//   RecCell runs the guarded run and ends with Error('END') under asserterror, so a run that raises and one that returns are both read.
// The values are what BC produced. What BC did, in short:
//   - The record argument changes nothing. Every cell gives the same result as the same cell without an argument in
//     ConsistentGuardedRunFail.al, for a committed row of another table, a committed row of the marked table, and a temporary record.
//   - A mark still held when the inner codeunit ends normally makes the guarded Codeunit.Run raise the Consistent error
//     (DB:TableInConsistent) naming the marked table; the run does not return.
//   - When the inner codeunit fails with Error after marks, the marks are dropped: the run returns No, the last error is the
//     inner error (code Dialog), the argument's Name is unchanged, the inner writes are rolled back, and a later Commit in the
//     caller does not raise. A plain statement gives the same.
//   - Marking or modifying the argument itself in the inner codeunit follows the same rules: a mark held at the end raises, a
//     mark cleared with Consistent(true) on the same variable lets the run return Yes, a failure drops it.
//   - A plain Commit in the inner codeunit while it holds a mark: the run returns No with the Consistent error text and code
//     DB:TableInConsistent. Under CommitBehavior::Error the run returns No with the commit-prohibited error. Under
//     CommitBehavior::Ignore the commit is skipped and the end of the run decides.
// Not measured (a runtime must refuse these): marks set by the caller before the run; FieldError and TestField after a mark; a
//   record argument of a table the caller marked; a record argument with filters, or read after the run beyond its Name; nested
//   (not top-level) runs; a write in the caller after its Commit and before the run.
// local object: the tests must commit (a guarded Codeunit.Run commits), and the inner codeunit is the code under test.
// BC versions: 27.5, 28.5, 29.0

table 88975 "GRR X"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { AutoIncrement = true; }
        field(2; Name; Text[30]) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 88976 "GRR Y"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Code; Code[10]) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

// The table of the record argument that no mark names (unless a cell marks the argument itself).
table 88977 "GRR Z"
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
codeunit 88978 "GRR Mode"
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

// The step interpreter: runs the steps of "GRR Mode", comma separated. Same steps as "GRF Inner" of ConsistentGuardedRunFail.al:
//   x / y: table GRR X / GRR Y. f = Consistent(false), t = Consistent(true).
//   a, b, c: three local variables of DoSteps (alive until the run ends); p: a local variable of a procedure (gone when it returns).
//   rec: the record argument of the run (frec, trec: Consistent on it; modrec: Name := 'modified' and Modify).
codeunit 88979 "GRR Steps"
{
    procedure DoSteps(var RecX: Record "GRR X"; var RecZ: Record "GRR Z"; IsX: Boolean)
    var
        Mode: Codeunit "GRR Mode";
        XA: Record "GRR X";
        XB: Record "GRR X";
        XC: Record "GRR X";
        YA: Record "GRR Y";
        YB: Record "GRR Y";
        YC: Record "GRR Y";
        Step: Text;
    begin
        foreach Step in Mode.Get().Split(',') do
            case Step of
                'insx':
                    InsertX('inner');
                'insy':
                    InsertY(5);
                'startx':
                    StartX();
                'fyp':
                    MarkYInProcedure();
                'fxa':
                    XA.Consistent(false);
                'fxc':
                    XC.Consistent(false);
                'txa':
                    XA.Consistent(true);
                'txb':
                    XB.Consistent(true);
                'txc':
                    XC.Consistent(true);
                'fya':
                    YA.Consistent(false);
                'fyb':
                    YB.Consistent(false);
                'fyc':
                    YC.Consistent(false);
                'tya':
                    YA.Consistent(true);
                'frec':
                    if IsX then
                        RecX.Consistent(false)
                    else
                        RecZ.Consistent(false);
                'trec':
                    if IsX then
                        RecX.Consistent(true)
                    else
                        RecZ.Consistent(true);
                'modrec':
                    if IsX then begin
                        RecX.Name := 'modified';
                        RecX.Modify();
                    end else begin
                        RecZ.Name := 'modified';
                        RecZ.Modify();
                    end;
                'commit':
                    Commit();
                'cbcommit':
                    CommitUnderError();
                'cbignore':
                    CommitUnderIgnore();
                'err':
                    Error('inner failed');
            end;
    end;

    local procedure InsertX(Name: Text[30])
    var
        X: Record "GRR X";
    begin
        X.Init();
        X.Id := 0;
        X.Name := Name;
        X.Insert();
    end;

    local procedure InsertY(Id: Integer)
    var
        Y: Record "GRR Y";
    begin
        Y.Init();
        Y.Id := Id;
        Y.Insert();
    end;

    // The preview state shape: insert a row and mark the table from the same local variable.
    local procedure StartX()
    var
        X: Record "GRR X";
    begin
        X.Init();
        X.Id := 0;
        X.Name := 'state';
        X.Insert();
        X.Consistent(false);
    end;

    local procedure MarkYInProcedure()
    var
        Y: Record "GRR Y";
    begin
        Y.Consistent(false);
    end;

    [CommitBehavior(CommitBehavior::Error)]
    local procedure CommitUnderError()
    begin
        Commit();
    end;

    [CommitBehavior(CommitBehavior::Ignore)]
    local procedure CommitUnderIgnore()
    begin
        Commit();
    end;
}

// The inner codeunit for a record of GRR Z (a table no mark names).
codeunit 88981 "GRR Inner Z"
{
    TableNo = "GRR Z";

    trigger OnRun()
    var
        Steps: Codeunit "GRR Steps";
        NoX: Record "GRR X";
    begin
        Steps.DoSteps(NoX, Rec, false);
    end;
}

// The inner codeunit for a record of GRR X (the table the steps mark).
codeunit 88982 "GRR Inner X"
{
    TableNo = "GRR X";

    trigger OnRun()
    var
        Steps: Codeunit "GRR Steps";
        NoZ: Record "GRR Z";
    begin
        Steps.DoSteps(Rec, NoZ, true);
    end;
}

codeunit 88983 "GRR Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mode: Codeunit "GRR Mode";

    // Every cell starts with one committed row in GRR X ('caller') and in GRR Z ('caller'); the run gets one of them.
    local procedure Initialize(Steps: Text)
    var
        X: Record "GRR X";
        Y: Record "GRR Y";
        Z: Record "GRR Z";
    begin
        X.Consistent(true);
        Y.Consistent(true);
        Z.Consistent(true);
        X.DeleteAll();
        Y.DeleteAll();
        Z.DeleteAll();
        X.Init();
        X.Id := 0;
        X.Name := 'caller';
        X.Insert();
        Z.Init();
        Z.Id := 1;
        Z.Name := 'caller';
        Z.Insert();
        Commit();
        ClearLastError();
        Mode.Set(Steps);
    end;

    local procedure Rows(): Text
    var
        X: Record "GRR X";
        Y: Record "GRR Y";
        Z: Record "GRR Z";
        Names: Text;
    begin
        if X.FindSet() then
            repeat
                Names += X.Name + ';';
            until X.Next() = 0;
        Z.Get(1);
        exit('[' + Names + '] y=' + Format(Y.Count()) + ' z=' + Z.Name);
    end;

    // The rows a commit made durable: an unrelated error undoes every write since the last commit.
    local procedure DurableRows(): Text
    begin
        asserterror Error('unrelated');
        ClearLastError();
        exit(Rows());
    end;

    // Runs the inner codeunit with the record kind: z = the GRR Z row, x = the GRR X row, tmp = a temporary GRR X record with one row.
    local procedure RunRec(Kind: Text; var ArgName: Text) Ok: Boolean
    var
        X: Record "GRR X";
        Z: Record "GRR Z";
        TempX: Record "GRR X" temporary;
    begin
        case Kind of
            'z':
                begin
                    Z.Get(1);
                    Ok := Codeunit.Run(Codeunit::"GRR Inner Z", Z);
                    ArgName := Z.Name;
                end;
            'x':
                begin
                    X.FindFirst();
                    Ok := Codeunit.Run(Codeunit::"GRR Inner X", X);
                    ArgName := X.Name;
                end;
            'tmp':
                begin
                    TempX.Init();
                    TempX.Id := 1;
                    TempX.Name := 'temp';
                    TempX.Insert();
                    Ok := Codeunit.Run(Codeunit::"GRR Inner X", TempX);
                    ArgName := TempX.Name;
                end;
        end;
    end;

    local procedure RunThenEnd(Kind: Text)
    var
        Ok: Boolean;
        ArgName: Text;
    begin
        Ok := RunRec(Kind, ArgName);
        Mode.Log('run=' + Format(Ok));
        Mode.Log('text=' + GetLastErrorText());
        Mode.Log('code=' + GetLastErrorCode());
        Mode.Log('arg=' + ArgName);
        ClearLastError();
        Error('END');
    end;

    local procedure CommitThenEnd()
    begin
        Commit();
        Error('END');
    end;

    local procedure RecCell(var Observer: Codeunit "Probe Observer"; Kind: Text; Steps: Text)
    begin
        Initialize(Steps);
        asserterror RunThenEnd(Kind);
        Observer.ObserveError('run end');
        Observer.Observe('log', Mode.GetLog());
        asserterror CommitThenEnd();
        Observer.ObserveError('commit');
        Observer.Observe('durable rows', DurableRows());
    end;

    // The run as a plain statement (no asserterror around it).
    local procedure PlainCell(var Observer: Codeunit "Probe Observer"; Kind: Text; Steps: Text)
    var
        Ok: Boolean;
        ArgName: Text;
    begin
        Initialize(Steps);
        Ok := RunRec(Kind, ArgName);
        Observer.Observe('run', Format(Ok));
        Observer.ObserveError('last error');
        Observer.Observe('arg', ArgName);
        Observer.Observe('rows after run', Rows());
        asserterror CommitThenEnd();
        Observer.ObserveError('commit');
        Observer.Observe('durable rows', DurableRows());
    end;

    // ---- the section (2) and (3) cells of ConsistentGuardedRunFail.al, record argument: RecZ ----

    [Test]
    procedure RecZ_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_StartX_InsertY_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'startx,insy,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_StartX_MarkYInProcTwice_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'startx,fyp,fyp,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_StartX_MarkYByA_MarkYByC_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'startx,fya,fyc');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRR X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_StartX_TrueXByB_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'startx,txb,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_StartX_TrueXByB_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'startx,txb,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRR X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_StartX_TrueXByB_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'startx,txb');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRR X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_MarkXByA_TrueXByB_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'fxa,txb,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_MarkXByA_TrueXByB_MarkXByC_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'fxa,txb,fxc,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_MarkXByA_TrueXByB_MarkXByC_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'fxa,txb,fxc,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRR X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_MarkXByA_TrueXByB_MarkXByC_TrueXByA_TrueYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'fxa,txb,fxc,fya,txa,tya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRR X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_MarkXByA_TrueXByB_MarkXByC_TrueXByC_TrueYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'fxa,txb,fxc,fya,txc,tya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRR X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_MarkYByA_MarkXByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'fya,fxa');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRR Y table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_MarkXMarkY_CommitUnderCommitBehaviorError()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'fxa,fyb,cbcommit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=Commit is prohibited in the current scope. The operation cannot continue. Contact your system administrator.;code=DB:Ok;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_InsertX_CommitUnderCommitBehaviorError()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'insx,cbcommit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=Commit is prohibited in the current scope. The operation cannot continue. Contact your system administrator.;code=DB:Ok;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_MarkXMarkY_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'fxa,fyb,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRR X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_MarkXInsertX_CommitUnderCommitBehaviorIgnore_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'fxa,insx,cbignore');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRR X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_MarkXInsertX_CommitUnderCommitBehaviorIgnore_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'fxa,insx,cbignore,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_MarkXTrueX_InsertX_CommitUnderCommitBehaviorIgnore_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'fxa,txa,insx,cbignore');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=Yes;text=;code=;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;inner;] y=0 z=caller');
        Observer.Verify();
    end;

    // ---- the section (2) and (3) cells of ConsistentGuardedRunFail.al, record argument: RecX ----

    [Test]
    procedure RecX_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_StartX_InsertY_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'startx,insy,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_StartX_MarkYInProcTwice_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'startx,fyp,fyp,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_StartX_MarkYByA_MarkYByC_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'startx,fya,fyc');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRR X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_StartX_TrueXByB_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'startx,txb,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_StartX_TrueXByB_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'startx,txb,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRR X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_StartX_TrueXByB_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'startx,txb');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRR X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_MarkXByA_TrueXByB_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'fxa,txb,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_MarkXByA_TrueXByB_MarkXByC_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'fxa,txb,fxc,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_MarkXByA_TrueXByB_MarkXByC_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'fxa,txb,fxc,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRR X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_MarkXByA_TrueXByB_MarkXByC_TrueXByA_TrueYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'fxa,txb,fxc,fya,txa,tya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRR X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_MarkXByA_TrueXByB_MarkXByC_TrueXByC_TrueYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'fxa,txb,fxc,fya,txc,tya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRR X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_MarkYByA_MarkXByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'fya,fxa');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRR Y table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_MarkXMarkY_CommitUnderCommitBehaviorError()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'fxa,fyb,cbcommit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=Commit is prohibited in the current scope. The operation cannot continue. Contact your system administrator.;code=DB:Ok;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_InsertX_CommitUnderCommitBehaviorError()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'insx,cbcommit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=Commit is prohibited in the current scope. The operation cannot continue. Contact your system administrator.;code=DB:Ok;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_MarkXMarkY_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'fxa,fyb,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRR X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_MarkXInsertX_CommitUnderCommitBehaviorIgnore_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'fxa,insx,cbignore');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRR X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_MarkXInsertX_CommitUnderCommitBehaviorIgnore_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'fxa,insx,cbignore,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_MarkXTrueX_InsertX_CommitUnderCommitBehaviorIgnore_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'fxa,txa,insx,cbignore');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=Yes;text=;code=;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;inner;] y=0 z=caller');
        Observer.Verify();
    end;

    // ---- the inner codeunit marks or modifies the record argument itself: RecZ ----

    [Test]
    procedure RecZ_MarkRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'frec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_MarkRec_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'frec');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRR Z table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_ModifyRec_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'modrec,startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_ModifyRec_MarkRec_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'modrec,frec,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_MarkRec_TrueRec_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'frec,trec');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=Yes;text=;code=;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecZ_MarkRec_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'z', 'frec,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRR Z table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    // ---- the inner codeunit marks or modifies the record argument itself: RecX ----

    [Test]
    procedure RecX_MarkRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'frec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_MarkRec_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'frec');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRR X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_ModifyRec_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'modrec,startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_ModifyRec_MarkRec_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'modrec,frec,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_MarkRec_TrueRec_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'frec,trec');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=Yes;text=;code=;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecX_MarkRec_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'x', 'frec,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRR X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=caller;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    // ---- a temporary record of GRR X as the argument ----

    [Test]
    procedure RecTemp_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tmp', 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;arg=temp;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecTemp_StartX_MarkYByA_MarkYByC_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tmp', 'startx,fya,fyc');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRR X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecTemp_MarkXMarkY_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tmp', 'fxa,fyb,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRR X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg=temp;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure RecTemp_MarkRec_ModifyRec_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RecCell(Observer, 'tmp', 'frec,modrec,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;arg=temp;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure Plain_RecZ_StartX_InsertY_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PlainCell(Observer, 'z', 'startx,insy,fya,fyc,err');
        Observer.Expect('run', 'No');
        Observer.Expect('last error|text', 'inner failed');
        Observer.Expect('last error|code', 'Dialog');
        Observer.Expect('arg', 'caller');
        Observer.Expect('rows after run', '[caller;] y=0 z=caller');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;

    [Test]
    procedure Plain_RecX_StartX_InsertY_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PlainCell(Observer, 'x', 'startx,insy,fya,fyc,err');
        Observer.Expect('run', 'No');
        Observer.Expect('last error|text', 'inner failed');
        Observer.Expect('last error|code', 'Dialog');
        Observer.Expect('arg', 'caller');
        Observer.Expect('rows after run', '[caller;] y=0 z=caller');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[caller;] y=0 z=caller');
        Observer.Verify();
    end;
}
