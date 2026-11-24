// Review round 1 of the guarded Codeunit.Run end-of-run check (al-runner issue 8970): a failed run nested in another
// guarded run, by what the outer run does next; and a Commit inside the run before the mark.

table 61420 "GR Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 61421 "GR Other"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 61420 "GR Mode"
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

// Marks the row table and fails: the run nested in the outer one.
codeunit 61421 "GR Fail Marks"
{
    trigger OnRun()
    var
        Row: Record "GR Row";
    begin
        Row.Consistent(false);
        Error('nested failed');
    end;
}

codeunit 61422 "GR Nothing"
{
    trigger OnRun()
    begin
    end;
}

// Runs the steps of "GR Mode", comma separated.
codeunit 61423 "GR Outer"
{
    trigger OnRun()
    var
        Mode: Codeunit "GR Mode";
        Row: Record "GR Row";
        Other: Record "GR Other";
        Step: Text;
    begin
        foreach Step in Mode.Get().Split(',') do
            case Step of
                'nestedfail':
                    if Codeunit.Run(Codeunit::"GR Fail Marks") then;
                'nestedok':
                    if Codeunit.Run(Codeunit::"GR Nothing") then;
                'nestedfailce':
                    NestedFailCommitError();
                'ins':
                    begin
                        Row.Init();
                        Row.Id := 1;
                        Row.Insert();
                    end;
                'commit':
                    Commit();
                'mark':
                    Row.Consistent(false);
                'markother':
                    Other.Consistent(false);
                'err':
                    Error('outer failed');
                'err0':
                    Error('');
            end;
    end;

    [CommitBehavior(CommitBehavior::Error)]
    local procedure NestedFailCommitError()
    begin
        if Codeunit.Run(Codeunit::"GR Fail Marks") then;
    end;
}

codeunit 61424 "GR Tests"
{
    Subtype = Test;

    local procedure Init(Steps: Text)
    var
        Row: Record "GR Row";
        Mode: Codeunit "GR Mode";
    begin
        Row.DeleteAll();
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

    // ---- a failed run nested in another guarded run ----

    [Test]
    procedure NestedRunMarksFails_OuterFails()
    begin
        Init('nestedfailce,err0');
        ExpectFalse(Codeunit.Run(Codeunit::"GR Outer"), 'Dialog', '');
        Commit();
    end;

    [Test]
    procedure NestedRunMarksFails_OuterFailsText()
    begin
        Init('nestedfail,err');
        ExpectFalse(Codeunit.Run(Codeunit::"GR Outer"), 'Dialog', 'outer failed');
        Commit();
    end;

    [Test]
    procedure NestedRunMarksFails_OuterSucceeds()
    begin
        Init('nestedfail');
        if not Codeunit.Run(Codeunit::"GR Outer") then
            Error('outer run failed: %1', GetLastErrorText());
        Commit();
    end;

    [Test]
    procedure NestedRunMarksFails_OuterCommitsThenFails()
    begin
        Init('nestedfail,commit,err0');
        ExpectFalse(Codeunit.Run(Codeunit::"GR Outer"), 'Dialog', '');
    end;

    [Test]
    procedure NestedRunMarksFails_OuterRunsAgainThenFails()
    begin
        Init('nestedfail,nestedok,err0');
        ExpectFalse(Codeunit.Run(Codeunit::"GR Outer"), 'Dialog', '');
    end;

    [Test]
    procedure NestedRunMarksFails_Twice_OuterFails()
    begin
        Init('nestedfail,nestedfail,err0');
        ExpectFalse(Codeunit.Run(Codeunit::"GR Outer"), 'Dialog', '');
    end;

    [Test]
    procedure NestedRunMarksFails_OuterMarksAndFails()
    begin
        Init('nestedfail,markother,err0');
        ExpectFalse(Codeunit.Run(Codeunit::"GR Outer"), 'Dialog', '');
        Commit();
    end;

    // ---- a Commit inside the run before the mark ----

    [Test]
    procedure InnerCommitsThenMarksFails()
    begin
        Init('ins,commit,mark,err');
        ExpectFalse(Codeunit.Run(Codeunit::"GR Outer"), 'Dialog', 'outer failed');
        Commit();
    end;

    [Test]
    procedure InnerCommitsThenMarksSucceeds()
    var
        Ok: Boolean;
    begin
        Init('ins,commit,mark');
        asserterror Ok := Codeunit.Run(Codeunit::"GR Outer");
        if GetLastErrorCode() <> 'DB:TableInConsistent' then
            Error('code %1', GetLastErrorCode());
    end;

    [Test]
    procedure InnerCommitsThenMarksTwoSucceeds()
    var
        Ok: Boolean;
    begin
        Init('ins,commit,mark,markother');
        asserterror Ok := Codeunit.Run(Codeunit::"GR Outer");
        if GetLastErrorCode() <> 'DB:TableInConsistent' then
            Error('code %1', GetLastErrorCode());
    end;

    [Test]
    procedure InnerMarksFails_NoCommit()
    begin
        Init('ins,mark,err');
        ExpectFalse(Codeunit.Run(Codeunit::"GR Outer"), 'Dialog', 'outer failed');
        Commit();
    end;
}
