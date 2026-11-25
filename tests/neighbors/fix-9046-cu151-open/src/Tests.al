table 52650 "CT Open Test Log"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[250]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52651 "CT Open Test Session"
{
    // The test session's own Company Triggers subscriber: no install session, main's answer.
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpenCompleted', '', false, false)]
    local procedure OpenCompleted()
    var
        Row: Record "CT Open Test Log";
    begin
        if Session.GetExecutionContext() = ExecutionContext::Install then
            exit;
        Row.Code := 'TESTOPEN';
        Row.Value := Format(TaskScheduler.CanCreateTask());
        if not Row.Insert() then
            Row.Modify();
    end;
}

codeunit 52652 "CT Open Tests"
{
    Subtype = Test;

    [Test]
    procedure CompanyOpenCompletedOfTheInstallSession()
    var
        Row: Record "CT Open TE";
    begin
        Row.Get('OPEN');
        if Row.Value <> 'Yes' then
            Error('company open in the install session: %1', Row.Value);
    end;

    [Test]
    procedure InstallCodeAfterTheRefusedSubscriber()
    var
        Row: Record "CT Open TE";
    begin
        Row.Get('INSTALL');
        if Row.Value <> 'ran' then
            Error('install code: %1', Row.Value);
    end;

    [Test]
    procedure TestSessionCanCreateTaskAfterTheInstall()
    begin
        if TaskScheduler.CanCreateTask() then
            Error('the test session can create a task');
    end;
}
