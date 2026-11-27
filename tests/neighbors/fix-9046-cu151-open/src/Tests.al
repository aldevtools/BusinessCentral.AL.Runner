codeunit 52652 "CT Open Tests"
{
    Subtype = Test;

    [Test]
    procedure CompanyOpenCompletedOfTheInstallSession()
    var
        Row: Record "CT Open TE";
    begin
        // BC answers Yes there (corpus PR 1592, install-upgrade/IcInstallCanCreateTaskTests.al; assumption install-session-can-create-task).
        Row.Get('OPEN');
        if Row.Value <> 'Yes' then
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
    procedure TestSessionCompanyOpenSubscriberSeesYes()
    var
        Seen: Codeunit "CT Open Seen";
    begin
        if Seen.Open() <> 'Yes' then
            Error('OnCompanyOpen of the test session: %1', Seen.Open());
    end;

    [Test]
    procedure TestSessionCompanyOpenCompletedSubscriberSeesYes()
    var
        Seen: Codeunit "CT Open Seen";
    begin
        if Seen.OpenCompleted() <> 'Yes' then
            Error('OnCompanyOpenCompleted of the test session: %1', Seen.OpenCompleted());
    end;

    [Test]
    procedure TestSessionCanCreateTaskAfterTheInstall()
    begin
        if TaskScheduler.CanCreateTask() then
            Error('the test session can create a task');
    end;
}
