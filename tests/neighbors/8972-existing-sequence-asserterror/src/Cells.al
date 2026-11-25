// Neighbors of the asserterror cells of corpus PR 1573 (SqExistingRestart.al): an existing number sequence restarted or
// deleted, then an asserterror. Each codeunit: OnRun makes the sequence (Current 2), A writes and commits, B steps, C reads.

codeunit 52800 "N6 RE5"
{
    Subtype = Test;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6RE5') then begin NumberSequence.Insert('P6RE5', 1, 1); NumberSequence.Next('P6RE5'); NumberSequence.Next('P6RE5'); end;
    end;

    [Test]
    procedure A_RE5WritesCommits()
    begin
        Rows.Put('RE5-INIT'); Commit();
    end;

    [Test]
    
    procedure B_RE5Step()
    begin
        NumberSequence.Restart('P6RE5', 1002); Rows.Put('RE5-B1'); asserterror Error('inner');
    end;

    [Test]
    procedure C_RE5Reads()
    begin
        if not NumberSequence.Exists('P6RE5', true) then
            Error('OBS missing row=%1', Rows.Has('RE5-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6RE5', true), Rows.Has('RE5-B1'));
    end;
}

codeunit 52801 "N6 RE4"
{
    Subtype = Test;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6RE4') then begin NumberSequence.Insert('P6RE4', 1, 1); NumberSequence.Next('P6RE4'); NumberSequence.Next('P6RE4'); end;
    end;

    [Test]
    procedure A_RE4WritesCommits()
    begin
        Rows.Put('RE4-INIT'); Commit();
    end;

    [Test]
    
    procedure B_RE4Step()
    begin
        NumberSequence.Restart('P6RE4', 1002); asserterror Error('inner');
    end;

    [Test]
    procedure C_RE4Reads()
    begin
        if not NumberSequence.Exists('P6RE4', true) then
            Error('OBS missing row=%1', Rows.Has('RE4-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6RE4', true), Rows.Has('RE4-B1'));
    end;
}

codeunit 52802 "N6 RE"
{
    Subtype = Test;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6RE') then begin NumberSequence.Insert('P6RE', 1, 1); NumberSequence.Next('P6RE'); NumberSequence.Next('P6RE'); end;
    end;

    [Test]
    procedure A_REWritesCommits()
    begin
        Rows.Put('RE-INIT'); Commit();
    end;

    [Test]
    
    procedure B_REStep()
    begin
        NumberSequence.Restart('P6RE', 1002); Rows.Put('RE-B1'); asserterror Error('inner'); Error('B deliberate');
    end;

    [Test]
    procedure C_REReads()
    begin
        if not NumberSequence.Exists('P6RE', true) then
            Error('OBS missing row=%1', Rows.Has('RE-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6RE', true), Rows.Has('RE-B1'));
    end;
}

codeunit 52803 "N6 RE2"
{
    Subtype = Test;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6RE2') then begin NumberSequence.Insert('P6RE2', 1, 1); NumberSequence.Next('P6RE2'); NumberSequence.Next('P6RE2'); end;
    end;

    [Test]
    procedure A_RE2WritesCommits()
    begin
        Rows.Put('RE2-INIT'); Commit();
    end;

    [Test]
    
    procedure B_RE2Step()
    begin
        NumberSequence.Restart('P6RE2', 1002); asserterror begin Rows.Put('RE2-B1'); Error('inner'); end; Error('B deliberate');
    end;

    [Test]
    procedure C_RE2Reads()
    begin
        if not NumberSequence.Exists('P6RE2', true) then
            Error('OBS missing row=%1', Rows.Has('RE2-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6RE2', true), Rows.Has('RE2-B1'));
    end;
}

codeunit 52804 "N6 DE"
{
    Subtype = Test;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6DE') then begin NumberSequence.Insert('P6DE', 1, 1); NumberSequence.Next('P6DE'); NumberSequence.Next('P6DE'); end;
    end;

    [Test]
    procedure A_DEWritesCommits()
    begin
        Rows.Put('DE-INIT'); Commit();
    end;

    [Test]
    
    procedure B_DEStep()
    begin
        NumberSequence.Delete('P6DE'); Rows.Put('DE-B1'); asserterror Error('inner'); Error('B deliberate');
    end;

    [Test]
    procedure C_DEReads()
    begin
        if not NumberSequence.Exists('P6DE', true) then
            Error('OBS missing row=%1', Rows.Has('DE-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6DE', true), Rows.Has('DE-B1'));
    end;
}

codeunit 52805 "N6 RE3"
{
    Subtype = Test;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6RE3') then begin NumberSequence.Insert('P6RE3', 1, 1); NumberSequence.Next('P6RE3'); NumberSequence.Next('P6RE3'); end;
    end;

    [Test]
    procedure A_RE3WritesCommits()
    begin
        Rows.Put('RE3-INIT'); Commit();
    end;

    [Test]
    
    procedure B_RE3Step()
    begin
        NumberSequence.Restart('P6RE3', 1002); asserterror Error('inner'); Error('B deliberate');
    end;

    [Test]
    procedure C_RE3Reads()
    begin
        if not NumberSequence.Exists('P6RE3', true) then
            Error('OBS missing row=%1', Rows.Has('RE3-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6RE3', true), Rows.Has('RE3-B1'));
    end;
}

codeunit 52806 "N6 NextAfterStep"
{
    Subtype = Test;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6NextAfterStep') then begin NumberSequence.Insert('P6NextAfterStep', 1, 1); NumberSequence.Next('P6NextAfterStep'); NumberSequence.Next('P6NextAfterStep'); end;
    end;

    [Test]
    procedure A_NextAfterStepWritesCommits()
    begin
        Rows.Put('NextAfterStep-INIT'); Commit();
    end;

    [Test]
    
    procedure B_NextAfterStepStep()
    begin
        NumberSequence.Restart('P6NextAfterStep', 1002); Rows.Put('NextAfterStep-B1'); NumberSequence.Next('P6NextAfterStep'); asserterror Error('inner');
    end;

    [Test]
    procedure C_NextAfterStepReads()
    begin
        if not NumberSequence.Exists('P6NextAfterStep', true) then
            Error('OBS missing row=%1', Rows.Has('NextAfterStep-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6NextAfterStep', true), Rows.Has('NextAfterStep-B1'));
    end;
}

codeunit 52807 "N6 WriteBeforeStep"
{
    Subtype = Test;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6WriteBeforeStep') then begin NumberSequence.Insert('P6WriteBeforeStep', 1, 1); NumberSequence.Next('P6WriteBeforeStep'); NumberSequence.Next('P6WriteBeforeStep'); end;
    end;

    [Test]
    procedure A_WriteBeforeStepWritesCommits()
    begin
        Rows.Put('WriteBeforeStep-INIT'); Commit();
    end;

    [Test]
    
    procedure B_WriteBeforeStepStep()
    begin
        Rows.Put('WriteBeforeStep-B1'); NumberSequence.Restart('P6WriteBeforeStep', 1002); asserterror Error('inner');
    end;

    [Test]
    procedure C_WriteBeforeStepReads()
    begin
        if not NumberSequence.Exists('P6WriteBeforeStep', true) then
            Error('OBS missing row=%1', Rows.Has('WriteBeforeStep-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6WriteBeforeStep', true), Rows.Has('WriteBeforeStep-B1'));
    end;
}

codeunit 52808 "N6 CompanyWide"
{
    Subtype = Test;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6CompanyWide', false) then begin NumberSequence.Insert('P6CompanyWide', 1, 1, false); NumberSequence.Next('P6CompanyWide', false); NumberSequence.Next('P6CompanyWide', false); end;
    end;

    [Test]
    procedure A_CompanyWideWritesCommits()
    begin
        Rows.Put('CompanyWide-INIT'); Commit();
    end;

    [Test]
    
    procedure B_CompanyWideStep()
    begin
        NumberSequence.Restart('P6CompanyWide', 1002, false); Rows.Put('CompanyWide-B1'); asserterror Error('inner');
    end;

    [Test]
    procedure C_CompanyWideReads()
    begin
        if not NumberSequence.Exists('P6CompanyWide', false) then
            Error('OBS missing row=%1', Rows.Has('CompanyWide-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6CompanyWide', false), Rows.Has('CompanyWide-B1'));
    end;
}

codeunit 52809 "N6 ReadOnly"
{
    Subtype = Test;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6ReadOnly') then begin NumberSequence.Insert('P6ReadOnly', 1, 1); NumberSequence.Next('P6ReadOnly'); NumberSequence.Next('P6ReadOnly'); end;
    end;

    [Test]
    procedure A_ReadOnlyWritesCommits()
    begin
        Rows.Put('ReadOnly-INIT'); Commit();
    end;

    [Test]
    
    procedure B_ReadOnlyStep()
    begin
        NumberSequence.Restart('P6ReadOnly', 1002); if Rows.Has('ReadOnly-INIT') then; asserterror Error('inner');
    end;

    [Test]
    procedure C_ReadOnlyReads()
    begin
        if not NumberSequence.Exists('P6ReadOnly', true) then
            Error('OBS missing row=%1', Rows.Has('ReadOnly-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6ReadOnly', true), Rows.Has('ReadOnly-B1'));
    end;
}

codeunit 52810 "N6 AutoRollback"
{
    Subtype = Test;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6AutoRollback') then begin NumberSequence.Insert('P6AutoRollback', 1, 1); NumberSequence.Next('P6AutoRollback'); NumberSequence.Next('P6AutoRollback'); end;
    end;

    [Test]
    procedure A_AutoRollbackWritesCommits()
    begin
        Rows.Put('AutoRollback-INIT'); Commit();
    end;

    [Test]
    [TransactionModel(TransactionModel::AutoRollback)]
    procedure B_AutoRollbackStep()
    begin
        NumberSequence.Restart('P6AutoRollback', 1002); Rows.Put('AutoRollback-B1'); asserterror Error('inner');
    end;

    [Test]
    procedure C_AutoRollbackReads()
    begin
        if not NumberSequence.Exists('P6AutoRollback', true) then
            Error('OBS missing row=%1', Rows.Has('AutoRollback-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6AutoRollback', true), Rows.Has('AutoRollback-B1'));
    end;
}

codeunit 52811 "N6 LabelError"
{
    Subtype = Test;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6LabelError') then begin NumberSequence.Insert('P6LabelError', 1, 1); NumberSequence.Next('P6LabelError'); NumberSequence.Next('P6LabelError'); end;
    end;

    [Test]
    procedure A_LabelErrorWritesCommits()
    begin
        Rows.Put('LabelError-INIT'); Commit();
    end;

    [Test]
    
    procedure B_LabelErrorStep()
    begin
        NumberSequence.Restart('P6LabelError', 1002); Rows.Put('LabelError-B1'); asserterror Error(InnerErr);
    end;

    [Test]
    procedure C_LabelErrorReads()
    begin
        if not NumberSequence.Exists('P6LabelError', true) then
            Error('OBS missing row=%1', Rows.Has('LabelError-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6LabelError', true), Rows.Has('LabelError-B1'));
    end;
}

codeunit 52812 "N6 GuardedRunFails"
{
    Subtype = Test;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6GuardedRunFails') then begin NumberSequence.Insert('P6GuardedRunFails', 1, 1); NumberSequence.Next('P6GuardedRunFails'); NumberSequence.Next('P6GuardedRunFails'); end;
    end;

    [Test]
    procedure A_GuardedRunFailsWritesCommits()
    begin
        Rows.Put('GuardedRunFails-INIT'); Commit();
    end;

    [Test]
    
    procedure B_GuardedRunFailsStep()
    begin
        NumberSequence.Restart('P6GuardedRunFails', 1002); Rows.Put('GuardedRunFails-B1'); if Codeunit.Run(Codeunit::"N6 Fails") then;
    end;

    [Test]
    procedure C_GuardedRunFailsReads()
    begin
        if not NumberSequence.Exists('P6GuardedRunFails', true) then
            Error('OBS missing row=%1', Rows.Has('GuardedRunFails-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6GuardedRunFails', true), Rows.Has('GuardedRunFails-B1'));
    end;
}

codeunit 52813 "N6 HeldSecondAsserterror"
{
    Subtype = Test;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6HeldSecondAsserterror') then begin NumberSequence.Insert('P6HeldSecondAsserterror', 1, 1); NumberSequence.Next('P6HeldSecondAsserterror'); NumberSequence.Next('P6HeldSecondAsserterror'); end;
    end;

    [Test]
    procedure A_HeldSecondAsserterrorWritesCommits()
    begin
        Rows.Put('HeldSecondAsserterror-INIT'); Commit();
    end;

    [Test]
    
    procedure B_HeldSecondAsserterrorStep()
    begin
        NumberSequence.Restart('P6HeldSecondAsserterror', 1002); asserterror Error('inner'); Rows.Put('HeldSecondAsserterror-B1'); asserterror Error('inner');
    end;

    [Test]
    procedure C_HeldSecondAsserterrorReads()
    begin
        if not NumberSequence.Exists('P6HeldSecondAsserterror', true) then
            Error('OBS missing row=%1', Rows.Has('HeldSecondAsserterror-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6HeldSecondAsserterror', true), Rows.Has('HeldSecondAsserterror-B1'));
    end;
}

codeunit 52814 "N6 DeletePasses"
{
    Subtype = Test;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6DeletePasses') then begin NumberSequence.Insert('P6DeletePasses', 1, 1); NumberSequence.Next('P6DeletePasses'); NumberSequence.Next('P6DeletePasses'); end;
    end;

    [Test]
    procedure A_DeletePassesWritesCommits()
    begin
        Rows.Put('DeletePasses-INIT'); Commit();
    end;

    [Test]
    
    procedure B_DeletePassesStep()
    begin
        NumberSequence.Delete('P6DeletePasses'); Rows.Put('DeletePasses-B1'); asserterror Error('inner');
    end;

    [Test]
    procedure C_DeletePassesReads()
    begin
        if not NumberSequence.Exists('P6DeletePasses', true) then
            Error('OBS missing row=%1', Rows.Has('DeletePasses-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6DeletePasses', true), Rows.Has('DeletePasses-B1'));
    end;
}

codeunit 52815 "N6 DeleteNothing"
{
    Subtype = Test;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6DeleteNothing') then begin NumberSequence.Insert('P6DeleteNothing', 1, 1); NumberSequence.Next('P6DeleteNothing'); NumberSequence.Next('P6DeleteNothing'); end;
    end;

    [Test]
    procedure A_DeleteNothingWritesCommits()
    begin
        Rows.Put('DeleteNothing-INIT'); Commit();
    end;

    [Test]
    
    procedure B_DeleteNothingStep()
    begin
        NumberSequence.Delete('P6DeleteNothing'); asserterror Error('inner');
    end;

    [Test]
    procedure C_DeleteNothingReads()
    begin
        if not NumberSequence.Exists('P6DeleteNothing', true) then
            Error('OBS missing row=%1', Rows.Has('DeleteNothing-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6DeleteNothing', true), Rows.Has('DeleteNothing-B1'));
    end;
}

codeunit 52816 "N6 NoEarlierWrite"
{
    Subtype = Test;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6NoEarlierWrite') then begin NumberSequence.Insert('P6NoEarlierWrite', 1, 1); NumberSequence.Next('P6NoEarlierWrite'); NumberSequence.Next('P6NoEarlierWrite'); end;
    end;

    [Test]
    procedure A_NoEarlierWriteWritesCommits()
    begin
        Commit();
    end;

    [Test]
    
    procedure B_NoEarlierWriteStep()
    begin
        NumberSequence.Restart('P6NoEarlierWrite', 1002); Rows.Put('NoEarlierWrite-B1'); asserterror Error('inner');
    end;

    [Test]
    procedure C_NoEarlierWriteReads()
    begin
        if not NumberSequence.Exists('P6NoEarlierWrite', true) then
            Error('OBS missing row=%1', Rows.Has('NoEarlierWrite-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6NoEarlierWrite', true), Rows.Has('NoEarlierWrite-B1'));
    end;
}

codeunit 52817 "N6 SecondStep"
{
    Subtype = Test;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6SecondStep') then begin NumberSequence.Insert('P6SecondStep', 1, 1); NumberSequence.Next('P6SecondStep'); NumberSequence.Next('P6SecondStep'); end;
    end;

    [Test]
    procedure A_SecondStepWritesCommits()
    begin
        Rows.Put('SecondStep-INIT'); Commit();
    end;

    [Test]
    
    procedure B_SecondStepStep()
    begin
        NumberSequence.Restart('P6SecondStep', 1002); NumberSequence.Restart('P6SecondStep', 1002); Rows.Put('SecondStep-B1'); asserterror Error('inner');
    end;

    [Test]
    procedure C_SecondStepReads()
    begin
        if not NumberSequence.Exists('P6SecondStep', true) then
            Error('OBS missing row=%1', Rows.Has('SecondStep-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6SecondStep', true), Rows.Has('SecondStep-B1'));
    end;
}

codeunit 52818 "N6 CommitInStepBody"
{
    Subtype = Test;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6CommitInStepBody') then begin NumberSequence.Insert('P6CommitInStepBody', 1, 1); NumberSequence.Next('P6CommitInStepBody'); NumberSequence.Next('P6CommitInStepBody'); end;
    end;

    [Test]
    procedure A_CommitInStepBodyWritesCommits()
    begin
        
    end;

    [Test]
    
    procedure B_CommitInStepBodyStep()
    begin
        Rows.Put('CommitInStepBody-B1'); Commit(); NumberSequence.Restart('P6CommitInStepBody', 1002); Rows.Put('CommitInStepBody-B1'); asserterror Error('inner');
    end;

    [Test]
    procedure C_CommitInStepBodyReads()
    begin
        if not NumberSequence.Exists('P6CommitInStepBody', true) then
            Error('OBS missing row=%1', Rows.Has('CommitInStepBody-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6CommitInStepBody', true), Rows.Has('CommitInStepBody-B1'));
    end;
}

codeunit 52819 "N6 ComputedError"
{
    Subtype = Test;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6ComputedError') then begin NumberSequence.Insert('P6ComputedError', 1, 1); NumberSequence.Next('P6ComputedError'); NumberSequence.Next('P6ComputedError'); end;
    end;

    [Test]
    procedure A_ComputedErrorWritesCommits()
    begin
        Rows.Put('ComputedError-INIT'); Commit();
    end;

    [Test]
    
    procedure B_ComputedErrorStep()
    begin
        NumberSequence.Restart('P6ComputedError', 1002); Rows.Put('ComputedError-B1'); asserterror Error(StrSubstNo('inner %1', 1));
    end;

    [Test]
    procedure C_ComputedErrorReads()
    begin
        if not NumberSequence.Exists('P6ComputedError', true) then
            Error('OBS missing row=%1', Rows.Has('ComputedError-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6ComputedError', true), Rows.Has('ComputedError-B1'));
    end;
}

codeunit 52820 "N6 ReadAfterUndo"
{
    Subtype = Test;
    var
        Rows: Codeunit "N6 Rows";
        InnerErr: Label 'inner';

    trigger OnRun()
    begin
        if not NumberSequence.Exists('P6ReadAfterUndo') then begin NumberSequence.Insert('P6ReadAfterUndo', 1, 1); NumberSequence.Next('P6ReadAfterUndo'); NumberSequence.Next('P6ReadAfterUndo'); end;
    end;

    [Test]
    procedure A_ReadAfterUndoWritesCommits()
    begin
        Rows.Put('ReadAfterUndo-INIT'); Commit();
    end;

    [Test]
    
    procedure B_ReadAfterUndoStep()
    begin
        NumberSequence.Restart('P6ReadAfterUndo', 1002); Rows.Put('ReadAfterUndo-B1'); asserterror Error('inner'); if NumberSequence.Current('P6ReadAfterUndo') <> 2 then Error('B read %1', NumberSequence.Current('P6ReadAfterUndo'));
    end;

    [Test]
    procedure C_ReadAfterUndoReads()
    begin
        if not NumberSequence.Exists('P6ReadAfterUndo', true) then
            Error('OBS missing row=%1', Rows.Has('ReadAfterUndo-B1'));
        Error('OBS cur=%1 row=%2', NumberSequence.Current('P6ReadAfterUndo', true), Rows.Has('ReadAfterUndo-B1'));
    end;
}
