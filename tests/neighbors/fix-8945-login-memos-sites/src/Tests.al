// Neighbors of al-runner issue 8945 at other raise sites: Change Log Management loaded in the setup ask, the profile
// fetched inside the insert event, Environment Information Impl. filled by one subscriber (main's measured shape).
codeunit 60980 "LS Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    begin
    end;
}

codeunit 60981 "LS Login Tests"
{
    Subtype = Test;

    [Test]
    procedure ReadsChangeLogLoadedInAsk()
    var
        C: Codeunit "Change Log Management";
    begin
        if not C.IsRead() then
            Error('change log not read');
    end;

    // Review of al-runner PR 9028: the test loads the cache again before it reads it (main passed: no mark on 423).
    [Test]
    procedure UsesChangeLogAfterLoginAsk()
    var
        C: Codeunit "Change Log Management";
    begin
        C.Load();
        if not C.IsRead() then
            Error('change log not read');
    end;

    [Test]
    procedure ReadsProfileFetchedInRaise()
    var
        P: Codeunit "Conf./Personalization Mgt.";
    begin
        if P.Profile() <> 'BUSINESS MANAGER' then
            Error('profile');
    end;

    [Test]
    procedure ReadsEnvInfoFromOneSubscriber()
    var
        E: Codeunit "Environment Information Impl.";
    begin
        if not (E.IsSaaS() and E.IsSandbox()) then
            Error('env');
    end;
}
