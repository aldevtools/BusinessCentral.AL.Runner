// Restart of an existing sequence, a Record write and an asserterror (the RE5 shape of corpus PR 1573) in test codeunits
// with RequiredTestIsolation = Function and Disabled: the asserterror's rollback refuses the later reads there.
codeunit 52810 "N6 FunctionIsolationRestartAsserterror"
{
    Subtype = Test;
    RequiredTestIsolation = Function;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6FunctionIsolationRestartAsserterror') then begin NumberSequence.Insert('P6FunctionIsolationRestartAsserterror', 1, 1); NumberSequence.Next('P6FunctionIsolationRestartAsserterror'); NumberSequence.Next('P6FunctionIsolationRestartAsserterror'); end;
    end;

    [Test]
    procedure A_FunctionIsolationRestartAsserterrorWritesCommits()
    begin
        Rows.Put('FunctionIsolationRestartAsserterror-INIT'); Commit();
    end;

    [Test]
    
    procedure B_FunctionIsolationRestartAsserterrorStep()
    begin
        NumberSequence.Restart('P6FunctionIsolationRestartAsserterror', 1002); if NumberSequence.Exists('P6FunctionIsolationRestartAsserterror') then if NumberSequence.Current('P6FunctionIsolationRestartAsserterror') = 0 then; Rows.Put('FunctionIsolationRestartAsserterror-B1'); asserterror Error('inner');
    end;

    [Test]
    procedure C_FunctionIsolationRestartAsserterrorReads()
    begin
        if not NumberSequence.Exists('P6FunctionIsolationRestartAsserterror', true) then
            Error('OBS missing row=%1', Rows.Has('FunctionIsolationRestartAsserterror-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6FunctionIsolationRestartAsserterror', true), Rows.Has('FunctionIsolationRestartAsserterror-B1'));
    end;
}

codeunit 52811 "N6 DisabledIsolationRestartAsserterror"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6DisabledIsolationRestartAsserterror') then begin NumberSequence.Insert('P6DisabledIsolationRestartAsserterror', 1, 1); NumberSequence.Next('P6DisabledIsolationRestartAsserterror'); NumberSequence.Next('P6DisabledIsolationRestartAsserterror'); end;
    end;

    [Test]
    procedure A_DisabledIsolationRestartAsserterrorWritesCommits()
    begin
        Rows.Put('DisabledIsolationRestartAsserterror-INIT'); Commit();
    end;

    [Test]
    
    procedure B_DisabledIsolationRestartAsserterrorStep()
    begin
        NumberSequence.Restart('P6DisabledIsolationRestartAsserterror', 1002); if NumberSequence.Exists('P6DisabledIsolationRestartAsserterror') then if NumberSequence.Current('P6DisabledIsolationRestartAsserterror') = 0 then; Rows.Put('DisabledIsolationRestartAsserterror-B1'); asserterror Error('inner');
    end;

    [Test]
    procedure C_DisabledIsolationRestartAsserterrorReads()
    begin
        if not NumberSequence.Exists('P6DisabledIsolationRestartAsserterror', true) then
            Error('OBS missing row=%1', Rows.Has('DisabledIsolationRestartAsserterror-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6DisabledIsolationRestartAsserterror', true), Rows.Has('DisabledIsolationRestartAsserterror-B1'));
    end;
}

