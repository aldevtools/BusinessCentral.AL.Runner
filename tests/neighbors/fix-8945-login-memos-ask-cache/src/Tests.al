// Neighbors of the setup-ask cache gate (assumption partner-login-setup-ask-cache): caches changed in the setup ask of a
// partner login write, with an install codeunit, so the install session asked first (Y69InstallTriggerSetup.al:134).
codeunit 60982 "LS Own Cache"
{
    SingleInstance = true;
    var
        Asks: Integer;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
        if TableId = Database::"LSP Tab" then
            Asks += 1;
    end;

    procedure Count(): Integer
    begin
        exit(Asks);
    end;
}

codeunit 60983 "LS Foreign Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    var
        F: Codeunit "LSP Foreign Cache";
    begin
        if TableId = Database::"LSP Tab" then
            F.Bump();
    end;
}

codeunit 60981 "LS Ask Tests"
{
    Subtype = Test;

    [Test]
    procedure PartnerCacheAfterOwnWrite()
    var
        P: Codeunit "LSP Setup Cache";
    begin
        if P.Last() = 'NEVER' then
            Error('partner cache');
    end;

    [Test]
    procedure OwnCounterAfterOwnWrite()
    var
        O: Codeunit "LS Own Cache";
    begin
        if O.Count() < 0 then
            Error('own counter');
    end;

    [Test]
    procedure PartnerCacheHoldsTheAsk()
    var
        P: Codeunit "LSP Setup Cache";
    begin
        if P.Last() <> 'ASKED' then
            Error('partner cache: %1', P.Last());
    end;

    [Test]
    procedure OwnCounterCountsTheAsk()
    var
        O: Codeunit "LS Own Cache";
    begin
        if O.Count() <> 1 then
            Error('own counter: %1', O.Count());
    end;

    [Test]
    procedure TwoLoginWritesTwoAsks()
    var
        C: Codeunit "LSP Count Cache";
    begin
        if C.Count() <> 2 then
            Error('count cache: %1', C.Count());
    end;

    [Test]
    procedure SideCacheFilledBySetupSubscriberOfItsApp()
    var
        S: Codeunit "LSP Side Cache";
    begin
        if S.Count() <> 1 then
            Error('side cache: %1', S.Count());
    end;

    [Test]
    procedure EventCacheChangedByOtherEventSubscriber()
    var
        E: Codeunit "LSP Event Cache";
    begin
        if E.Count() < 0 then
            Error('event cache');
    end;

    [Test]
    procedure MixedCacheChangedOutsideTheAskToo()
    var
        M: Codeunit "LSP Mixed Cache";
    begin
        if M.Count() < 0 then
            Error('mixed cache');
    end;

    [Test]
    procedure LaterCacheChangedAfterTheFill()
    var
        L: Codeunit "LSP Later Cache";
    begin
        if L.Count() < 0 then
            Error('later cache');
    end;

    [Test]
    procedure NestedCacheAskInsideInsertRaise()
    var
        N: Codeunit "LSP Nested Cache";
    begin
        if N.Seen() then;
    end;

    [Test]
    procedure ForeignCacheChangedByOtherAppSubscriber()
    var
        F: Codeunit "LSP Foreign Cache";
    begin
        if F.Count() < 0 then
            Error('foreign cache');
    end;

    [Test]
    procedure Control()
    begin
    end;
}

codeunit 60984 "LS Ask Tests Two"
{
    Subtype = Test;

    [Test]
    procedure NextCodeunitReadsPartnerCache()
    var
        P: Codeunit "LSP Setup Cache";
    begin
        if P.Last() = 'NEVER' then
            Error('partner cache');
    end;
}

codeunit 60980 "LS Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    begin
    end;
}
