// A stand-in for System Application's login code and its login memos (publisher Microsoft, invented code):
// "System Initialization Impl." fills the memo through the facade, sets a memo global the measured procedures do not
// read (Family), inserts a row of its own, then raises "System Initialization".OnAfterLogin from Company Triggers
// OnCompanyOpenCompleted, then calls code the runner refuses (Debugger), as the corpus run refuses System Application's
// InitCompany at Assisted Company Setup.CompanyActive (IsEmpty on an unseeded G/L Entry, ADR-047). The memos keep a
// Boolean filled on first use.
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

table 150 "LMR MS Login Row"
{
    fields
    {
        field(1; "No."; Integer) { }
    }
    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

codeunit 151 "System Initialization Impl."
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpenCompleted', '', false, false)]
    local procedure InitCompany()
    var
        EnvInfo: Codeunit "Environment Information";
        EnvInfoImpl: Codeunit "Environment Information Impl.";
        SysInit: Codeunit "System Initialization";
        MsRow: Record "LMR MS Login Row";
    begin
        if EnvInfo.IsSaaS() then;
        EnvInfoImpl.SetFamily('FC');
        MsRow."No." := 1;
        MsRow.Insert();
        SysInit.Raise();
        Halt();
    end;

    local procedure Halt()
    begin
        Debugger.Deactivate();
    end;
}

codeunit 457 "Environment Information"
{
    SingleInstance = true;

    var
        Impl: Codeunit "Environment Information Impl.";

    procedure IsSaaS(): Boolean
    begin
        exit(Impl.IsSaaS());
    end;

    procedure IsSandbox(): Boolean
    begin
        exit(Impl.IsSandbox());
    end;

    procedure IsOnPrem(): Boolean
    begin
        exit(not Impl.IsSaaS());
    end;

    procedure GetFamily(): Text
    begin
        exit(Impl.GetFamily());
    end;
}

codeunit 3702 "Environment Information Impl."
{
    SingleInstance = true;

    var
        Known: Boolean;
        SaaS: Boolean;
        Family: Text;

    procedure IsSaaS(): Boolean
    begin
        if not Known then begin
            SaaS := false;
            Known := true;
        end;
        exit(SaaS);
    end;

    procedure IsSandbox(): Boolean
    begin
        exit(IsSaaS());
    end;

    procedure SetFamily(Value: Text)
    begin
        Family := Value;
    end;

    procedure GetFamily(): Text
    begin
        exit(Family);
    end;
}

codeunit 6723 "Server Setting"
{
    SingleInstance = true;

    var
        Impl: Codeunit "Server Setting Impl.";

    procedure GetEnableMembershipEntitlement(): Boolean
    begin
        exit(Impl.GetEnableMembershipEntitlement());
    end;
}

codeunit 3703 "Server Setting Impl."
{
    SingleInstance = true;

    var
        Known: Boolean;
        Entitled: Boolean;

    procedure GetEnableMembershipEntitlement(): Boolean
    begin
        if not Known then begin
            Entitled := false;
            Known := true;
        end;
        exit(Entitled);
    end;
}
