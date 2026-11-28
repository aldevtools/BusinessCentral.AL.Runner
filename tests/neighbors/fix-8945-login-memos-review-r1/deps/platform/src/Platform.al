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

codeunit 130011 "LR Backup Stand-in"
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

// Loads on the insert event of the partner table (61030); the setup has no row.
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

    procedure Reset()
    begin
        ChangeLogSetupRead := false;
        MonitorSensitiveFieldSetupRead := false;
    end;

    procedure IsRead(): Boolean
    begin
        exit(ChangeLogSetupRead and MonitorSensitiveFieldSetupRead);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseInsert', '', false, false)]
    local procedure DatabaseInsert(RecRef: RecordRef)
    begin
        if RecRef.Number = 61030 then
            Load();
    end;
}

codeunit 6153 "API Webhook Notification Mgt."
{
    SingleInstance = true;

    var
        CachedApiSubscriptionEnabled: Boolean;
        UseCachedApiSubscriptionEnabled: Boolean;

    procedure Ask()
    begin
        if UseCachedApiSubscriptionEnabled then
            exit;
        CachedApiSubscriptionEnabled := true;
        UseCachedApiSubscriptionEnabled := true;
    end;

    procedure Asked(): Boolean
    begin
        exit(CachedApiSubscriptionEnabled and UseCachedApiSubscriptionEnabled);
    end;

    // Asked in the setup ask of the partner's inner table (61031), which the partner writes inside the insert event.
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
        if TableId = 61031 then
            Ask();
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

    procedure FillCompany()
    begin
        if not ApplicationAreaCache.ContainsKey('Company:') then
            ApplicationAreaCache.Add('Company:', '#Suite');
    end;

    procedure Areas(): Text
    var
        T: Text;
    begin
        if ApplicationAreaCache.Get('User:', T) then;
        exit(T);
    end;
}
