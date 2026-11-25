// A stand-in for Microsoft's login code and caches (publisher Microsoft, invented code with the measured global names):
// "System Initialization Impl." raises "System Initialization".OnAfterLogin from Company Triggers OnCompanyOpenCompleted;
// a setup subscriber turns on the insert event of every table, as Backup Management of Microsoft's test libraries does.
codeunit 150 "System Initialization"
{
    procedure Raise()
    begin
        OnAfterLogin();
    end;

    [IntegrationEvent(false, false, true)]
    procedure OnAfterLogin()
    begin
    end;
}

codeunit 151 "System Initialization Impl."
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpenCompleted', '', false, false)]
    local procedure InitCompany()
    var
        S: Codeunit "System Initialization";
    begin
        S.Raise();
    end;
}

// GlobalTriggerManagement passes the Global Triggers events on as its OnAfter events.
codeunit 49 GlobalTriggerManagement
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
        OnAfterGetDatabaseTableTriggerSetup(TableId, OnDatabaseInsert, OnDatabaseModify, OnDatabaseDelete, OnDatabaseRename);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseInsert', '', false, false)]
    local procedure DatabaseInsert(RecRef: RecordRef)
    begin
        OnAfterOnDatabaseInsert(RecRef);
    end;

    [IntegrationEvent(false, false)]
    procedure OnAfterGetDatabaseTableTriggerSetup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnAfterOnDatabaseInsert(RecRef: RecordRef)
    begin
    end;
}

codeunit 130011 "LM Backup Stand-in"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
        OnDatabaseInsert := true;
    end;
}

table 405 "Change Log Setup"
{
    fields
    {
        field(1; "Primary Key"; Code[10]) { }
        field(2; "Change Log Activated"; Boolean) { }
    }
    keys
    {
        key(PK; "Primary Key") { Clustered = true; }
    }
}

// Loads on the insert event; the setup has no row.
codeunit 423 "Change Log Management"
{
    SingleInstance = true;

    var
        ChangeLogSetup: Record "Change Log Setup";
        ChangeLogSetupRead: Boolean;
        MonitorSensitiveFieldSetupRead: Boolean;

    procedure Load()
    begin
        if ChangeLogSetupRead then
            exit;
        if ChangeLogSetup.Get() then;
        ChangeLogSetupRead := true;
        MonitorSensitiveFieldSetupRead := true;
    end;

    procedure IsRead(): Boolean
    begin
        exit(ChangeLogSetupRead and MonitorSensitiveFieldSetupRead);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseInsert', '', false, false)]
    local procedure DatabaseInsert(RecRef: RecordRef)
    begin
        // Only the partner table's insert (60930): the test runner's own writes do not reach the cache.
        if RecRef.Number = 60930 then
            Load();
    end;
}

codeunit 6153 "API Webhook Notification Mgt."
{
    SingleInstance = true;

    var
        CachedApiSubscriptionEnabled: Boolean;
        UseCachedApiSubscriptionEnabled: Boolean;

    procedure Enabled(): Boolean
    begin
        exit(CachedApiSubscriptionEnabled and UseCachedApiSubscriptionEnabled);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseInsert', '', false, false)]
    local procedure DatabaseInsert(RecRef: RecordRef)
    begin
        if RecRef.Number <> 60930 then
            exit;
        if UseCachedApiSubscriptionEnabled then
            exit;
        CachedApiSubscriptionEnabled := true;
        UseCachedApiSubscriptionEnabled := true;
    end;
}

codeunit 3702 "Environment Information Impl."
{
    SingleInstance = true;

    var
        IsSaasInitialized: Boolean;
        IsSaaSConfig: Boolean;
        IsSandboxConfig: Boolean;
        IsSandboxInitialized: Boolean;

    procedure IsSaaS(): Boolean
    begin
        if not IsSaasInitialized then begin
            IsSaasInitialized := true;
            IsSaaSConfig := true;
        end;
        exit(IsSaaSConfig);
    end;

    procedure IsSandbox(): Boolean
    begin
        if not IsSandboxInitialized then begin
            IsSandboxInitialized := true;
            IsSandboxConfig := true;
        end;
        exit(IsSandboxConfig);
    end;
}

codeunit 9170 "Conf./Personalization Mgt."
{
    SingleInstance = true;

    var
        CurrentAllProfile: Text;
        CurrentAllProfileFetched: Boolean;
        ProfileFound: Boolean;

    procedure Fetch()
    begin
        if CurrentAllProfileFetched then
            exit;
        CurrentAllProfile := 'BUSINESS MANAGER';
        CurrentAllProfileFetched := true;
        ProfileFound := true;
    end;

    procedure Profile(): Text
    begin
        exit(CurrentAllProfile);
    end;
}

codeunit 9177 "Application Area Cache"
{
    SingleInstance = true;

    var
        ApplicationAreaCache: Dictionary of [Text, Text];

    procedure Fill()
    begin
        if not ApplicationAreaCache.ContainsKey('User:') then
            ApplicationAreaCache.Add('User:', '#Basic');
    end;

    procedure Areas(): Text
    var
        T: Text;
    begin
        if ApplicationAreaCache.Get('User:', T) then;
        exit(T);
    end;
}
