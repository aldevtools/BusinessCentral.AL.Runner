codeunit 52652 "CT Open Tests"
{
    Subtype = Test;

    [Test]
    procedure CompanyOpenCompletedOfTheInstallSession()
    var
        Row: Record "CT Open TE";
    begin
        // Main's answer, kept until BC's answer there is measured (assumption install-session-can-create-task).
        Row.Get('OPEN');
        if Row.Value <> 'No' then
            Error('company open in the install session: %1', Row.Value);
    end;

    [Test]
    procedure InstallCodeAfterTheCompanyOpenSubscriber()
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
