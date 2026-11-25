codeunit 52690 "CT Task Tests"
{
    Subtype = Test;

    [Test]
    procedure PerCompanyInstallTriggerSeesYes()
    var
        Row: Record "CT Task TA";
    begin
        Row.Get('INSTALL');
        if Row.Value <> 'Yes' then
            Error('per company install trigger: %1', Row.Value);
    end;

    [Test]
    procedure PerDatabaseInstallTriggerSeesYes()
    var
        Row: Record "CT Task TB";
    begin
        Row.Get('DATABASE');
        if Row.Value <> 'Yes' then
            Error('per database install trigger: %1', Row.Value);
    end;

    [Test]
    procedure ExtensionTriggersSubscriberSeesYes()
    var
        Row: Record "CT Task TC";
    begin
        Row.Get('SUBSCRIBER');
        if Row.Value <> 'Yes' then
            Error('install subscriber: %1', Row.Value);
    end;

    [Test]
    procedure SecondPerCompanyInstallTriggerSeesYes()
    var
        Row: Record "CT Task TD";
    begin
        Row.Get('INSTALL');
        if Row.Value <> 'Yes' then
            Error('per company install trigger: %1', Row.Value);
    end;

    [Test]
    procedure CompanyOpenOfTheInstallSessionSeesYes()
    var
        Row: Record "CT Task TA";
    begin
        Row.Get('OPEN');
        if Row.Value <> 'Yes' then
            Error('company open in the install session: %1', Row.Value);
    end;

    [Test]
    procedure TestSessionSeesNo()
    begin
        if TaskScheduler.CanCreateTask() then
            Error('the test session can create a task');
    end;
}
