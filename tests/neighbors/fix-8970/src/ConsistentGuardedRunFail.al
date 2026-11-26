// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/record/record-consistent-method
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/codeunit/codeunit-run-method
// Scope: in-scope
// Fixtures used: Probe Observer (codeunit 69160); own objects 88970-88974 (GRF X, GRF Y, GRF Mode, GRF Inner, GRF Tests)
// Probe: from al-runner issue 8970, second round (posting preview: the inner codeunit marks tables with Consistent(false) through
//   several record variables and then fails on purpose). Neighbors of ConsistentGuardedRun.al and ConsistentGuardedRunEnd.al.
// Area: a guarded Codeunit.Run (Ok := Codeunit.Run(...)) while Consistent(false) marks a table, by the inner codeunit or by the caller.
//   - the inner codeunit ends (a) with Error after writes, (b) with Error and no writes, (c) normally, (d) with Commit and then Error;
//   - the mark is set by the caller before the run on the table the inner codeunit writes, on another table, on a temporary record,
//     or by the inner codeunit;
//   - mark sequences through several variables of one table: false by A then false by C; false by A, true by B; false by A, true by B,
//     false by C; each with a second table marked, the run failing and succeeding;
//   - a Commit inside the inner codeunit under a CommitBehavior::Error and a CommitBehavior::Ignore procedure while marks are held;
//   - a FieldError and a TestField after a mark set on a codeunit global.
//   Each cell reads the run's return value and last error (text and code), whether a Commit in the caller raises (and its text), the
//   same after Consistent(true) on the caller's marking variable, and the rows a commit made durable.
//   How the cells are run: RunThenEnd runs the guarded run, writes the return value and the last error to the log of "GRF Mode",
//   and ends with Error('END'); the test calls it under asserterror, so a run that raises and a run that returns are both read.
//   CommitThenEnd does the same for Commit(). The rows the run committed stay; asserterror rolls back only what came after.
//   Some cells repeat a shape as a plain statement (no asserterror around the run) to show the wrapper does not change the result.
// The values are what BC produced. What BC did, in short:
//   - A mark the caller sets before the run (on the table the inner codeunit writes, on another table, or on a temporary record,
//     which names the database table) makes the guarded Codeunit.Run itself raise the Consistent error, however the inner
//     codeunit ends: the run does not return a value. After that error (caught by asserterror), a Commit in the caller does not
//     raise, although the caller's variable never called Consistent(true).
//   - A mark the inner codeunit still holds when it ends normally makes the run raise the Consistent error. A mark is held per
//     record variable: Consistent(true) on another variable of the same table does not clear it, only the same variable does
//     (fxa,txa clears; fxa,txb does not). A mark set by a procedure's local variable stays after the procedure returns. When two
//     tables are held, the error names the table marked first (in the cells measured here).
//   - When the inner codeunit fails, its marks are dropped: the run returns No with the inner error (text and code), and a later
//     Commit in the caller does not raise. This holds for a FieldError, a TestField and an Error after the marks.
//   - A plain Commit inside the inner codeunit while it holds a mark ends the run with No and DB:TableInConsistent; under
//     CommitBehavior::Error the run returns No with the commit-prohibited error (code DB:Ok); under CommitBehavior::Ignore the
//     commit is skipped and the end of the run decides.
// local object: the tests must commit (a guarded Codeunit.Run commits), and the inner codeunit is the code under test.
// BC versions: 27.5, 28.5, 29.0

table 88970 "GRF X"
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

table 88971 "GRF Y"
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

// The steps the inner codeunit runs, and a log the caller writes (a SingleInstance codeunit is not rolled back).
codeunit 88972 "GRF Mode"
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

// The inner codeunit: runs the steps of "GRF Mode", comma separated.
//   x / y: table GRF X / GRF Y. f = Consistent(false), t = Consistent(true).
//   a, b, c: three local variables of DoSteps (alive until the run ends); p: a local variable of a procedure (gone when it returns);
//   g: a codeunit global; tmp: a temporary record of GRF X.
codeunit 88973 "GRF Inner"
{
    trigger OnRun()
    begin
        DoSteps();
    end;

    var
        GlobalX: Record "GRF X";

    procedure DoSteps()
    var
        Mode: Codeunit "GRF Mode";
        XA: Record "GRF X";
        XB: Record "GRF X";
        XC: Record "GRF X";
        YA: Record "GRF Y";
        YB: Record "GRF Y";
        YC: Record "GRF Y";
        TempX: Record "GRF X" temporary;
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
                'fxp':
                    MarkXInProcedure();
                'fyp':
                    MarkYInProcedure();
                'fxa':
                    XA.Consistent(false);
                'fxb':
                    XB.Consistent(false);
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
                'tyb':
                    YB.Consistent(true);
                'fxg':
                    GlobalX.Consistent(false);
                'fxtmp':
                    TempX.Consistent(false);
                'commit':
                    Commit();
                'cbcommit':
                    CommitUnderError();
                'cbignore':
                    CommitUnderIgnore();
                'err':
                    Error('inner failed');
                'fielderr':
                    begin
                        YA.Init();
                        YA.FieldError(Code, 'must be set');
                    end;
                'testfield':
                    begin
                        YA.Init();
                        YA.TestField(Code);
                    end;
            end;
    end;

    local procedure InsertX(Name: Text[30])
    var
        X: Record "GRF X";
    begin
        X.Init();
        X.Id := 0;
        X.Name := Name;
        X.Insert();
    end;

    local procedure InsertY(Id: Integer)
    var
        Y: Record "GRF Y";
    begin
        Y.Init();
        Y.Id := Id;
        Y.Insert();
    end;

    // The preview state shape: insert a row and mark the table from the same local variable.
    local procedure StartX()
    var
        X: Record "GRF X";
    begin
        X.Init();
        X.Id := 0;
        X.Name := 'state';
        X.Insert();
        X.Consistent(false);
    end;

    local procedure MarkXInProcedure()
    var
        X: Record "GRF X";
    begin
        X.Consistent(false);
    end;

    local procedure MarkYInProcedure()
    var
        Y: Record "GRF Y";
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

codeunit 88974 "GRF Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mode: Codeunit "GRF Mode";

    local procedure Initialize(Steps: Text)
    var
        X: Record "GRF X";
        Y: Record "GRF Y";
    begin
        X.Consistent(true);
        Y.Consistent(true);
        X.DeleteAll();
        Y.DeleteAll();
        Commit();
        ClearLastError();
        Mode.Set(Steps);
    end;

    local procedure Rows(): Text
    var
        X: Record "GRF X";
        Y: Record "GRF Y";
        Names: Text;
    begin
        if X.FindSet() then
            repeat
                Names += X.Name + ';';
            until X.Next() = 0;
        exit('[' + Names + '] y=' + Format(Y.Count()));
    end;

    // The rows a commit made durable: an unrelated error undoes every write since the last commit.
    local procedure DurableRows(): Text
    begin
        asserterror Error('unrelated');
        ClearLastError();
        exit(Rows());
    end;

    local procedure RunThenEnd()
    var
        Ok: Boolean;
    begin
        Ok := Codeunit.Run(Codeunit::"GRF Inner");
        Mode.Log('run=' + Format(Ok));
        Mode.Log('text=' + GetLastErrorText());
        Mode.Log('code=' + GetLastErrorCode());
        ClearLastError();
        Error('END');
    end;

    local procedure CommitThenEnd()
    begin
        Commit();
        Error('END');
    end;

    // ---- (1) the caller marks before the run; the inner codeunit ends in four ways ----

    local procedure CallerCell(var Observer: Codeunit "Probe Observer"; Mark: Text; Steps: Text)
    var
        MarkX: Record "GRF X";
        MarkY: Record "GRF Y";
        TempX: Record "GRF X" temporary;
    begin
        Initialize(Steps);
        case Mark of
            'x':
                MarkX.Consistent(false);
            'y':
                MarkY.Consistent(false);
            'tmp':
                TempX.Consistent(false);
        end;
        asserterror RunThenEnd();
        Observer.ObserveError('run end');
        Observer.Observe('log', Mode.GetLog());
        asserterror CommitThenEnd();
        Observer.ObserveError('commit');
        case Mark of
            'x':
                MarkX.Consistent(true);
            'y':
                MarkY.Consistent(true);
            'tmp':
                TempX.Consistent(true);
        end;
        asserterror CommitThenEnd();
        Observer.ObserveError('commit after true');
        Observer.Observe('durable rows', DurableRows());
    end;

    [Test]
    procedure CallerMarksX_InnerInsertsXAndFails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        CallerCell(Observer, 'x', 'insx,err');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRF X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('commit after true|text', 'END');
        Observer.Expect('commit after true|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure CallerMarksX_InnerFailsNoWrite()
    var
        Observer: Codeunit "Probe Observer";
    begin
        CallerCell(Observer, 'x', 'err');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRF X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('commit after true|text', 'END');
        Observer.Expect('commit after true|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure CallerMarksX_InnerInsertsXAndEnds()
    var
        Observer: Codeunit "Probe Observer";
    begin
        CallerCell(Observer, 'x', 'insx');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRF X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('commit after true|text', 'END');
        Observer.Expect('commit after true|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure CallerMarksX_InnerInsertsXCommitsAndFails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        CallerCell(Observer, 'x', 'insx,commit,err');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRF X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('commit after true|text', 'END');
        Observer.Expect('commit after true|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure CallerMarksY_InnerInsertsXAndFails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        CallerCell(Observer, 'y', 'insx,err');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRF Y table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('commit after true|text', 'END');
        Observer.Expect('commit after true|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure CallerMarksY_InnerFailsNoWrite()
    var
        Observer: Codeunit "Probe Observer";
    begin
        CallerCell(Observer, 'y', 'err');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRF Y table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('commit after true|text', 'END');
        Observer.Expect('commit after true|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure CallerMarksY_InnerInsertsXAndEnds()
    var
        Observer: Codeunit "Probe Observer";
    begin
        CallerCell(Observer, 'y', 'insx');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRF Y table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('commit after true|text', 'END');
        Observer.Expect('commit after true|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure CallerMarksY_InnerInsertsXCommitsAndFails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        CallerCell(Observer, 'y', 'insx,commit,err');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRF Y table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('commit after true|text', 'END');
        Observer.Expect('commit after true|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure CallerMarksTemp_InnerInsertsXAndFails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        CallerCell(Observer, 'tmp', 'insx,err');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRF X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('commit after true|text', 'END');
        Observer.Expect('commit after true|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure CallerMarksTemp_InnerFailsNoWrite()
    var
        Observer: Codeunit "Probe Observer";
    begin
        CallerCell(Observer, 'tmp', 'err');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRF X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('commit after true|text', 'END');
        Observer.Expect('commit after true|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure CallerMarksTemp_InnerInsertsXAndEnds()
    var
        Observer: Codeunit "Probe Observer";
    begin
        CallerCell(Observer, 'tmp', 'insx');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRF X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('commit after true|text', 'END');
        Observer.Expect('commit after true|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure CallerMarksTemp_InnerInsertsXCommitsAndFails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        CallerCell(Observer, 'tmp', 'insx,commit,err');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRF X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('commit after true|text', 'END');
        Observer.Expect('commit after true|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure CallerNoMark_InnerMarksXInsertsAndFails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        CallerCell(Observer, 'x-none', 'fxa,insx,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('commit after true|text', 'END');
        Observer.Expect('commit after true|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure CallerNoMark_InnerMarksXFailsNoWrite()
    var
        Observer: Codeunit "Probe Observer";
    begin
        CallerCell(Observer, 'x-none', 'fxa,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('commit after true|text', 'END');
        Observer.Expect('commit after true|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure CallerNoMark_InnerMarksXInsertsAndEnds()
    var
        Observer: Codeunit "Probe Observer";
    begin
        CallerCell(Observer, 'x-none', 'fxa,insx');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRF X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('commit after true|text', 'END');
        Observer.Expect('commit after true|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure CallerNoMark_InnerInsertsXCommitsMarksAndFails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        CallerCell(Observer, 'x-none', 'insx,commit,fxa,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('commit after true|text', 'END');
        Observer.Expect('commit after true|code', 'Dialog');
        Observer.Expect('durable rows', '[inner;] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure CallerNoMark_InnerMarksXInsertsCommitsAndFails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        CallerCell(Observer, 'x-none', 'fxa,insx,commit,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRF X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('commit after true|text', 'END');
        Observer.Expect('commit after true|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    // ---- (2) the inner codeunit marks through several variables (the preview groups of issue 8970) ----

    local procedure InnerCell(var Observer: Codeunit "Probe Observer"; Steps: Text)
    begin
        Initialize(Steps);
        asserterror RunThenEnd();
        Observer.ObserveError('run end');
        Observer.Observe('log', Mode.GetLog());
        asserterror CommitThenEnd();
        Observer.ObserveError('commit');
        Observer.Observe('durable rows', DurableRows());
    end;

    [Test]
    procedure Inner_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'startx,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Inner_StartX_InsertY_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'startx,insy,fya,fyc,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Inner_StartX_MarkYInProcTwice_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'startx,fyp,fyp,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Inner_StartX_MarkYByA_MarkYByC_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'startx,fya,fyc');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRF X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Inner_StartX_TrueXByB_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'startx,txb,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Inner_StartX_TrueXByB_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'startx,txb,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRF X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Inner_StartX_TrueXByB_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'startx,txb');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRF X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Inner_MarkXByA_TrueXByB_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'fxa,txb,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Inner_MarkXByA_TrueXByB_MarkXByC_MarkYByA_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'fxa,txb,fxc,fya,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Inner_MarkXByA_TrueXByB_MarkXByC_MarkYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'fxa,txb,fxc,fya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRF X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Inner_MarkXByA_TrueXByB_MarkXByC_TrueXByA_TrueYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'fxa,txb,fxc,fya,txa,tya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRF X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Inner_MarkXByA_TrueXByB_MarkXByC_TrueXByC_TrueYByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'fxa,txb,fxc,fya,txc,tya');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRF X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Inner_MarkYByA_MarkXByA_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'fya,fxa');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRF Y table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    // ---- (3) a Commit inside the inner codeunit while marks are held ----

    [Test]
    procedure Inner_MarkXMarkY_CommitUnderCommitBehaviorError()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'fxa,fyb,cbcommit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=Commit is prohibited in the current scope. The operation cannot continue. Contact your system administrator.;code=DB:Ok;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Inner_InsertX_CommitUnderCommitBehaviorError()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'insx,cbcommit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=Commit is prohibited in the current scope. The operation cannot continue. Contact your system administrator.;code=DB:Ok;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Inner_MarkXMarkY_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'fxa,fyb,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GRF X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Inner_MarkXInsertX_CommitUnderCommitBehaviorIgnore_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'fxa,insx,cbignore');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GRF X table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'DB:TableInConsistent');
        Observer.Expect('log', '');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Inner_MarkXInsertX_CommitUnderCommitBehaviorIgnore_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'fxa,insx,cbignore,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Inner_MarkXTrueX_InsertX_CommitUnderCommitBehaviorIgnore_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'fxa,txa,insx,cbignore');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=Yes;text=;code=;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[inner;] y=0');
        Observer.Verify();
    end;

    // ---- (4) FieldError and TestField after a mark on a codeunit global ----

    [Test]
    procedure Inner_MarkGlobal_FieldError()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'fxg,fielderr');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=Code must be set in GRF Y Id=''0''.;code=NCLCSRTS:TableErrorStr;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Inner_MarkGlobal_InsertX_FieldError()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'fxg,insx,fielderr');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=Code must be set in GRF Y Id=''0''.;code=NCLCSRTS:TableErrorStr;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Inner_FieldErrorNoMark()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'insx,fielderr');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=Code must be set in GRF Y Id=''0''.;code=NCLCSRTS:TableErrorStr;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Inner_MarkGlobal_InsertY_TestField()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'fxg,insy,testfield');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=Code must have a value in GRF Y: Id=0. It cannot be zero or empty.;code=TestField;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Inner_MarkTemp_InsertX_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        InnerCell(Observer, 'fxtmp,insx,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'run=No;text=inner failed;code=Dialog;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    // ---- (5) the same shapes as plain statements: no asserterror around the run ----

    [Test]
    procedure Plain_StartX_MarkYByA_MarkYByC_Fails()
    var
        Observer: Codeunit "Probe Observer";
        Ok: Boolean;
    begin
        Initialize('startx,insy,fya,fyc,err');
        Ok := Codeunit.Run(Codeunit::"GRF Inner");
        Observer.Observe('run', Format(Ok));
        Observer.ObserveError('last error');
        Observer.Observe('rows after run', Rows());
        asserterror CommitThenEnd();
        Observer.ObserveError('commit');
        Observer.Observe('durable rows', DurableRows());
        Observer.Expect('run', 'No');
        Observer.Expect('last error|text', 'inner failed');
        Observer.Expect('last error|code', 'Dialog');
        Observer.Expect('rows after run', '[] y=0');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Plain_MarkGlobal_InsertX_FieldError()
    var
        Observer: Codeunit "Probe Observer";
        Ok: Boolean;
    begin
        Initialize('fxg,insx,fielderr');
        Ok := Codeunit.Run(Codeunit::"GRF Inner");
        Observer.Observe('run', Format(Ok));
        Observer.ObserveError('last error');
        asserterror CommitThenEnd();
        Observer.ObserveError('commit');
        Observer.Observe('durable rows', DurableRows());
        Observer.Expect('run', 'No');
        Observer.Expect('last error|text', 'Code must be set in GRF Y Id=''0''.');
        Observer.Expect('last error|code', 'NCLCSRTS:TableErrorStr');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;

    [Test]
    procedure Plain_MarkXByA_TrueXByB_MarkXByC_MarkYByA_Fails_ThenCommit()
    var
        Observer: Codeunit "Probe Observer";
        Ok: Boolean;
    begin
        Initialize('fxa,txb,fxc,fya,insx,err');
        Ok := Codeunit.Run(Codeunit::"GRF Inner");
        Observer.Observe('run', Format(Ok));
        Observer.ObserveError('last error');
        asserterror CommitThenEnd();
        Observer.ObserveError('commit');
        Observer.Observe('durable rows', DurableRows());
        Observer.Expect('run', 'No');
        Observer.Expect('last error|text', 'inner failed');
        Observer.Expect('last error|code', 'Dialog');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', '[] y=0');
        Observer.Verify();
    end;
}
