// Neighbors of the guarded Codeunit.Run end-of-run check for Consistent(false) marks (al-runner issue 8970): each fact the
// check reads, flipped one at a time, each new refusal, and each reader of a mark set by a temporary record.

table 61400 "GC Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Code; Code[10]) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

table 61401 "GC Other"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 61402 "GC Captioned"
{
    Caption = 'GC Caption Text';
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 61403 "GC Ext Captioned"
{
    Caption = 'GC Own Caption';
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

tableextension 61400 "GC Ext Caption" extends "GC Ext Captioned"
{
    Caption = 'GC Extension Caption';
}

codeunit 61400 "GC Mode"
{
    SingleInstance = true;

    var
        Steps: Text;

    procedure Set(NewSteps: Text)
    begin
        Steps := NewSteps;
    end;

    procedure Get(): Text
    begin
        exit(Steps);
    end;
}

// Runs the steps of "GC Mode", comma separated.
codeunit 61401 "GC Inner"
{
    trigger OnRun()
    begin
        DoSteps();
    end;

    procedure DoSteps()
    var
        Mode: Codeunit "GC Mode";
        Row: Record "GC Row";
        Row2: Record "GC Row";
        Other: Record "GC Other";
        Captioned: Record "GC Captioned";
        ExtCaptioned: Record "GC Ext Captioned";
        TempRow: Record "GC Row" temporary;
        I: Integer;
        Step: Text;
    begin
        foreach Step in Mode.Get().Split(',') do
            case Step of
                'mark':
                    Row.Consistent(false);
                'release':
                    Row2.Consistent(true);
                'markother':
                    Other.Consistent(false);
                'markcap':
                    Captioned.Consistent(false);
                'markext':
                    ExtCaptioned.Consistent(false);
                'marktemp':
                    TempRow.Consistent(false);
                'ins':
                    begin
                        Row.Init();
                        Row.Id := 1;
                        Row.Insert();
                    end;
                'commit':
                    Commit();
                'err':
                    Error('inner failed');
                'err0':
                    Error('');
                'get':
                    Other.Get(999);
                'testfield':
                    begin
                        Row.Init();
                        Row.TestField(Code);
                    end;
                'div':
                    begin
                        I := 0;
                        I := 1 div I;
                    end;
                'nested':
                    if Codeunit.Run(Codeunit::"GC Nothing") then;
                'nestedrec':
                    if Codeunit.Run(Codeunit::"GC Other Rec", Other) then;
                'nestedfail':
                    if Codeunit.Run(Codeunit::"GC Fail Marks") then;
                'nestedget':
                    if Codeunit.Run(Codeunit::"GC Get Fails") then;
            end;
    end;
}

codeunit 61402 "GC Nothing"
{
    trigger OnRun()
    begin
    end;
}

codeunit 61403 "GC Other Rec"
{
    TableNo = "GC Other";

    trigger OnRun()
    var
        Inner: Codeunit "GC Inner";
    begin
        Inner.DoSteps();
    end;
}

codeunit 61404 "GC Row Rec"
{
    TableNo = "GC Row";

    trigger OnRun()
    var
        Inner: Codeunit "GC Inner";
    begin
        Inner.DoSteps();
    end;
}

// Marks and fails, whatever the mode: the run nested in another run.
codeunit 61405 "GC Fail Marks"
{
    trigger OnRun()
    var
        Row: Record "GC Row";
    begin
        Row.Consistent(false);
        Error('nested failed');
    end;
}

// An inner Get that fails, whatever the mode: the run nested in another run.
codeunit 61407 "GC Get Fails"
{
    trigger OnRun()
    var
        Other: Record "GC Other";
    begin
        Other.Get(999);
    end;
}

codeunit 61406 "GC Tests"
{
    Subtype = Test;

    local procedure Init(Steps: Text)
    var
        Row: Record "GC Row";
        Other: Record "GC Other";
        Mode: Codeunit "GC Mode";
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Commit();
        ClearLastError();
        Mode.Set(Steps);
    end;

    local procedure ExpectFalse(Ok: Boolean; Code: Text; Text: Text)
    begin
        if Ok then
            Error('the run returned true');
        if GetLastErrorCode() <> Code then
            Error('code %1', GetLastErrorCode());
        if GetLastErrorText() <> Text then
            Error('text %1', GetLastErrorText());
    end;

    local procedure ExpectInconsistent(Table: Text)
    begin
        if GetLastErrorCode() <> 'DB:TableInConsistent' then
            Error('code %1', GetLastErrorCode());
        if StrPos(GetLastErrorText(), 'inconsistencies in the ' + Table + ' table.') = 0 then
            Error('text %1', GetLastErrorText());
    end;

    // ---- no mark before the run, the inner codeunit marks and fails ----

    [Test]
    procedure InnerMarksFails_Error()
    begin
        Init('mark,ins,err');
        ExpectFalse(Codeunit.Run(Codeunit::"GC Inner"), 'Dialog', 'inner failed');
        Commit();
    end;

    [Test]
    procedure InnerMarksFails_ErrorEmpty()
    begin
        Init('mark,err0');
        ExpectFalse(Codeunit.Run(Codeunit::"GC Inner"), 'Dialog', '');
        Commit();
    end;

    [Test]
    procedure InnerMarksFails_RecordNotFound()
    begin
        Init('mark,get');
        ExpectFalse(Codeunit.Run(Codeunit::"GC Inner"), 'DB:RecordNotFound', 'The GC Other does not exist. Identification fields and values: Id=''999''');
        Commit();
    end;

    [Test]
    procedure InnerMarksFails_TestField()
    begin
        Init('mark,testfield');
        ExpectFalse(Codeunit.Run(Codeunit::"GC Inner"), 'TestField', 'Code must have a value in GC Row: Id=0. It cannot be zero or empty.');
        Commit();
    end;

    [Test]
    procedure InnerMarksFails_DivideByZero()
    var
        Ok: Boolean;
    begin
        Init('mark,div');
        Ok := Codeunit.Run(Codeunit::"GC Inner");
    end;

    [Test]
    procedure InnerMarksCommits()
    var
        Ok: Boolean;
    begin
        Init('mark,ins,commit');
        Ok := Codeunit.Run(Codeunit::"GC Inner");
    end;

    [Test]
    procedure InnerMarksReleasedByAnotherFails()
    var
        Ok: Boolean;
    begin
        Init('mark,release,err');
        Ok := Codeunit.Run(Codeunit::"GC Inner");
    end;

    [Test]
    procedure InnerMarksTwoTablesFails()
    begin
        Init('mark,markother,err');
        ExpectFalse(Codeunit.Run(Codeunit::"GC Inner"), 'Dialog', 'inner failed');
        Commit();
    end;

    [Test]
    procedure InnerMarksTemporaryFails()
    begin
        Init('marktemp,err');
        ExpectFalse(Codeunit.Run(Codeunit::"GC Inner"), 'Dialog', 'inner failed');
        Commit();
    end;

    [Test]
    procedure InnerMarksFails_RecordArgOtherTable()
    var
        Other: Record "GC Other";
    begin
        Init('mark,err');
        ExpectFalse(Codeunit.Run(Codeunit::"GC Other Rec", Other), 'Dialog', 'inner failed');
        Commit();
    end;

    [Test]
    procedure InnerMarksFails_InstanceRecordArgOtherTable()
    var
        Other: Record "GC Other";
        Runner: Codeunit "GC Other Rec";
    begin
        Init('mark,err');
        ExpectFalse(Runner.Run(Other), 'Dialog', 'inner failed');
        Commit();
    end;

    [Test]
    procedure InnerMarksFails_RecordArgMarkedTable()
    var
        Row: Record "GC Row";
        Ok: Boolean;
    begin
        Init('mark,err');
        Ok := Codeunit.Run(Codeunit::"GC Row Rec", Row);
    end;

    [Test]
    procedure InnerMarksFails_TemporaryRecordArg()
    var
        Other: Record "GC Other" temporary;
        Ok: Boolean;
    begin
        Init('mark,err');
        Ok := Codeunit.Run(Codeunit::"GC Other Rec", Other);
    end;

    [Test]
    procedure NestedRunMarksFails()
    begin
        Init('nestedfail');
        if not Codeunit.Run(Codeunit::"GC Inner") then
            Error('outer run failed: %1', GetLastErrorText());
        Commit();
    end;

    // ---- no mark before the run, the inner codeunit marks and succeeds ----

    [Test]
    procedure InnerMarksSucceeds_Pinned()
    var
        Ok: Boolean;
    begin
        Init('mark');
        asserterror Ok := Codeunit.Run(Codeunit::"GC Inner");
        ExpectInconsistent('GC Row');
    end;

    [Test]
    procedure InnerMarksTwoTablesSucceeds()
    var
        Ok: Boolean;
    begin
        Init('markother,mark');
        asserterror Ok := Codeunit.Run(Codeunit::"GC Inner");
        ExpectInconsistent('GC Other');
    end;

    [Test]
    procedure InnerReleasedByAnotherSucceeds()
    var
        Ok: Boolean;
    begin
        Init('mark,release');
        asserterror Ok := Codeunit.Run(Codeunit::"GC Inner");
        ExpectInconsistent('GC Row');
    end;

    [Test]
    procedure InnerMarksCaptionedSucceeds()
    var
        Ok: Boolean;
    begin
        Init('markcap');
        asserterror Ok := Codeunit.Run(Codeunit::"GC Inner");
        ExpectInconsistent('GC Caption Text');
    end;

    [Test]
    procedure InnerMarksExtensionCaptionSucceeds()
    var
        Ok: Boolean;
    begin
        Init('markext');
        asserterror Ok := Codeunit.Run(Codeunit::"GC Inner");
    end;

    [Test]
    procedure InnerMarksSucceeds_RecordArgOtherTable()
    var
        Other: Record "GC Other";
        Ok: Boolean;
    begin
        Init('mark');
        asserterror Ok := Codeunit.Run(Codeunit::"GC Other Rec", Other);
        ExpectInconsistent('GC Row');
    end;

    [Test]
    procedure NestedRunMarksTwoTablesSucceeds()
    var
        Ok: Boolean;
    begin
        Init('mark,markother,nested');
        Ok := Codeunit.Run(Codeunit::"GC Inner");
    end;

    // ---- the caller marks before the run ----

    [Test]
    procedure CallerMarks_Pinned()
    var
        Row: Record "GC Row";
        Ok: Boolean;
    begin
        Init('');
        Row.Consistent(false);
        asserterror Ok := Codeunit.Run(Codeunit::"GC Nothing");
        ExpectInconsistent('GC Row');
    end;

    [Test]
    procedure CallerMarks_RecordArgOtherTable()
    var
        Row: Record "GC Row";
        Other: Record "GC Other";
        Ok: Boolean;
    begin
        Init('err');
        Row.Consistent(false);
        asserterror Ok := Codeunit.Run(Codeunit::"GC Other Rec", Other);
        ExpectInconsistent('GC Row');
    end;

    [Test]
    procedure CallerMarks_InnerGetFails()
    var
        Row: Record "GC Row";
        Ok: Boolean;
    begin
        Init('get');
        Row.Consistent(false);
        asserterror Ok := Codeunit.Run(Codeunit::"GC Inner");
        ExpectInconsistent('GC Row');
    end;

    [Test]
    procedure CallerMarks_InnerMarksOtherAndFails()
    var
        Row: Record "GC Row";
        Ok: Boolean;
    begin
        Init('markother,err');
        Row.Consistent(false);
        asserterror Ok := Codeunit.Run(Codeunit::"GC Inner");
        ExpectInconsistent('GC Row');
    end;

    [Test]
    procedure CallerReleased_InnerGetFails()
    var
        Row: Record "GC Row";
        Row2: Record "GC Row";
        Ok: Boolean;
    begin
        Init('get');
        Row.Consistent(false);
        Row2.Consistent(true);
        asserterror Ok := Codeunit.Run(Codeunit::"GC Inner");
    end;

    [Test]
    procedure CallerMarks_InnerGetFails_RecordArg()
    var
        Row: Record "GC Row";
        Other: Record "GC Other";
        Ok: Boolean;
    begin
        Init('get');
        Row.Consistent(false);
        asserterror Ok := Codeunit.Run(Codeunit::"GC Other Rec", Other);
    end;

    [Test]
    procedure CallerMarks_InnerMarksOtherAndGetFails()
    var
        Row: Record "GC Row";
        Ok: Boolean;
    begin
        Init('markother,get');
        Row.Consistent(false);
        asserterror Ok := Codeunit.Run(Codeunit::"GC Inner");
    end;

    [Test]
    procedure CallerMarks_InnerMarksOtherAndDivides()
    var
        Row: Record "GC Row";
        Ok: Boolean;
    begin
        Init('markother,div');
        Row.Consistent(false);
        asserterror Ok := Codeunit.Run(Codeunit::"GC Inner");
    end;

    [Test]
    procedure CallerMarks_InnerMarksTwoOthersAndFails()
    var
        Row: Record "GC Row";
        Ok: Boolean;
    begin
        Init('markother,markcap,err');
        Row.Consistent(false);
        asserterror Ok := Codeunit.Run(Codeunit::"GC Inner");
    end;

    [Test]
    procedure CallerMarks_InnerMarksOtherAndFails_RecordArg()
    var
        Row: Record "GC Row";
        Other: Record "GC Other";
        Ok: Boolean;
    begin
        Init('markcap,err');
        Row.Consistent(false);
        asserterror Ok := Codeunit.Run(Codeunit::"GC Other Rec", Other);
    end;

    [Test]
    procedure CallerReleased_InnerMarksOtherAndFails()
    var
        Row: Record "GC Row";
        Row2: Record "GC Row";
        Ok: Boolean;
    begin
        Init('markother,err');
        Row.Consistent(false);
        Row2.Consistent(true);
        asserterror Ok := Codeunit.Run(Codeunit::"GC Inner");
    end;

    [Test]
    procedure NestedCallerMarks_InnerGetFails()
    var
        Ok: Boolean;
    begin
        Init('mark,nestedget');
        Ok := Codeunit.Run(Codeunit::"GC Inner");
    end;

    [Test]
    procedure CallerWritesNoMark()
    var
        Row: Record "GC Row";
        Ok: Boolean;
    begin
        Init('err');
        Row.Id := 5;
        Row.Insert();
        asserterror Ok := Codeunit.Run(Codeunit::"GC Inner");
        if GetLastErrorCode() <> 'DB:Ok' then
            Error('code %1', GetLastErrorCode());
    end;

    [Test]
    procedure CallerMarksCaptioned()
    var
        Captioned: Record "GC Captioned";
        Ok: Boolean;
    begin
        Init('');
        Captioned.Consistent(false);
        asserterror Ok := Codeunit.Run(Codeunit::"GC Nothing");
        ExpectInconsistent('GC Caption Text');
    end;

    [Test]
    procedure CallerMarksTemporary_InnerFails()
    var
        TempRow: Record "GC Row" temporary;
        Ok: Boolean;
    begin
        Init('err');
        TempRow.Consistent(false);
        asserterror Ok := Codeunit.Run(Codeunit::"GC Inner");
        ExpectInconsistent('GC Row');
    end;

    [Test]
    procedure NestedCallerMarks_RecordArg()
    var
        Ok: Boolean;
    begin
        Init('mark,nestedrec');
        Ok := Codeunit.Run(Codeunit::"GC Inner");
    end;

    [Test]
    procedure NestedCallerMarks_Pinned()
    var
        Ok: Boolean;
    begin
        Init('mark,nested');
        Ok := Codeunit.Run(Codeunit::"GC Inner");
    end;

    // ---- Commit and the test end ----

    [Test]
    procedure Commit_ReleasedByAnother()
    var
        Marker: Record "GC Row";
        Releaser: Record "GC Row";
    begin
        Init('');
        Marker.Consistent(false);
        Releaser.Consistent(true);
        asserterror Commit();
        ExpectInconsistent('GC Row');
    end;

    [Test]
    procedure Commit_ReleasedByTemporary()
    var
        Marker: Record "GC Row";
        Releaser: Record "GC Row" temporary;
    begin
        Init('');
        Marker.Consistent(false);
        Releaser.Consistent(true);
        Commit();
    end;

    [Test]
    procedure Commit_ReleasedThenTrueOnFirst()
    var
        Marker: Record "GC Row";
        Releaser: Record "GC Row";
    begin
        Init('');
        Marker.Consistent(false);
        Releaser.Consistent(true);
        Marker.Consistent(true);
        Commit();
    end;

    [Test]
    procedure Commit_ReleasedThenFalseOnFirst()
    var
        Marker: Record "GC Row";
        Releaser: Record "GC Row";
    begin
        Init('');
        Marker.Consistent(false);
        Releaser.Consistent(true);
        Marker.Consistent(false);
        Commit();
    end;

    [Test]
    procedure Commit_TemporaryMarks()
    var
        TempRow: Record "GC Row" temporary;
    begin
        Init('');
        TempRow.Consistent(false);
        asserterror Commit();
        ExpectInconsistent('GC Row');
    end;

    [Test]
    procedure Commit_TemporaryMarksAndLifts()
    var
        TempRow: Record "GC Row" temporary;
    begin
        Init('');
        TempRow.Consistent(false);
        TempRow.Consistent(true);
        Commit();
    end;

    [Test]
    procedure Commit_Captioned()
    var
        Captioned: Record "GC Captioned";
    begin
        Init('');
        Captioned.Consistent(false);
        asserterror Commit();
        ExpectInconsistent('GC Caption Text');
    end;

    [Test]
    procedure TestEnd_TemporaryMarks()
    var
        TempRow: Record "GC Row" temporary;
    begin
        Init('');
        TempRow.Consistent(false);
    end;

    [Test]
    procedure UnguardedRun_TemporaryMarks()
    var
        TempRow: Record "GC Row" temporary;
    begin
        Init('');
        TempRow.Consistent(false);
        Codeunit.Run(Codeunit::"GC Nothing");
    end;

    // Last: the refusal leaves a write behind, which the runner does not trust for later tests.
    [Test]
    procedure CallerWritesThenMarks()
    var
        Row: Record "GC Row";
        Ok: Boolean;
    begin
        Init('err');
        Row.Id := 5;
        Row.Insert();
        Row.Consistent(false);
        asserterror Ok := Codeunit.Run(Codeunit::"GC Inner");
    end;
}
