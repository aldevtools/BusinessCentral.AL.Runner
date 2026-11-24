// A stand-in for Microsoft's login code (the raise sites flipped) and caches (publisher Microsoft, invented code with the measured global names):
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

// Loads in the setup ask (the shape of the job without Microsoft's test libraries); the setup has no row.
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

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
        Load();
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

