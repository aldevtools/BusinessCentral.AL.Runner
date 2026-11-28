// Neighbors of al-runner issue 8945: what a later test reads in the caches and memos the partner login subscribers
// filled (stand-in Microsoft caches with the measured global names), and
// whether the test session asks the Global Triggers setup again for a table the install session asked.
table 60950 "LM Ask Row"
{
    fields { field(1; No; Integer) { } }
    keys { key(PK; No) { Clustered = true; } }
}

codeunit 60951 "LM Asks"
{
    SingleInstance = true;

    var
        Asks: Integer;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
        if TableId = Database::"LM Ask Row" then
            Asks += 1;
    end;

    procedure Count(): Integer
    begin
        exit(Asks);
    end;
}

codeunit 60952 "LM Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "LM Ask Row";
    begin
        R.No := 1;
        if R.Insert() then;
    end;
}

codeunit 60953 "LM Login Tests"
{
    Subtype = Test;

    [Test]
    procedure ReadsChangeLogLoadedOnInsert()
    var
        C: Codeunit "Change Log Management";
    begin
        if not C.IsRead() then
            Error('change log not read');
    end;

    [Test]
    procedure ReadsApiCacheLoadedOnInsert()
    var
        W: Codeunit "API Webhook Notification Mgt.";
    begin
        if not W.Enabled() then
            Error('api cache');
    end;

    [Test]
    procedure ReadsProfileFetchedDirectly()
    var
        P: Codeunit "Conf./Personalization Mgt.";
    begin
        if P.Profile() <> 'BUSINESS MANAGER' then
            Error('profile');
    end;

    [Test]
    procedure ReadsAreasFilledDirectly()
    var
        A: Codeunit "Application Area Cache";
    begin
        if A.Areas() <> '#Basic' then
            Error('areas');
    end;

    [Test]
    procedure ReadsEnvInfoSplitOverTwoSubscribers()
    var
        E: Codeunit "Environment Information Impl.";
    begin
        if not (E.IsSaaS() and E.IsSandbox()) then
            Error('env');
    end;

    [Test]
    procedure TestSessionAsksSetupAgain()
    var
        R: Record "LM Ask Row";
        A: Codeunit "LM Asks";
    begin
        R.No := 2;
        R.Insert();
        if A.Count() <> 1 then
            Error('asked %1 times', A.Count());
    end;
}
