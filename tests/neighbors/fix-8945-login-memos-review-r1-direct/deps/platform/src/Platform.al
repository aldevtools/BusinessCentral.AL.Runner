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

codeunit 130011 "LD Backup Stand-in"
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

// Loads on the insert event of the partner table (61060); the setup has no row.
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

    procedure MarkMonitor()
    begin
        MonitorSensitiveFieldSetupRead := true;
    end;

    procedure IsRead(): Boolean
    begin
        exit(ChangeLogSetupRead and MonitorSensitiveFieldSetupRead);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseInsert', '', false, false)]
    local procedure DatabaseInsert(RecRef: RecordRef)
    begin
        if RecRef.Number = 61060 then
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
}
