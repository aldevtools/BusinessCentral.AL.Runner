// A stand-in for System Application's login code and memos (publisher Microsoft, invented code): "System Initialization
// Impl." raises "System Initialization".OnAfterLogin from Company Triggers OnCompanyOpenCompleted.
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

// Two Booleans, filled on first use (as Environment Information Impl. keeps IsSaaS).
codeunit 153 "Env Memo"
{
    SingleInstance = true;

    var
        Known: Boolean;
        SaaS: Boolean;

    procedure IsSaaS(): Boolean
    begin
        if not Known then begin
            SaaS := false;
            Known := true;
        end;
        exit(SaaS);
    end;
}

// An Integer that both partner apps add to.
codeunit 154 "Sys Count"
{
    SingleInstance = true;

    var
        N: Integer;

    procedure Bump()
    begin
        N += 1;
    end;

    procedure Get(): Integer
    begin
        exit(N);
    end;
}

// A Boolean and a Decimal, filled on first use, at the id of Server Setting Impl.
codeunit 3703 "Setting Memo"
{
    SingleInstance = true;

    var
        Known: Boolean;
        Rate: Decimal;

    procedure Get(): Decimal
    begin
        if not Known then begin
            Rate := 1.5;
            Known := true;
        end;
        exit(Rate);
    end;
}

// The same shape at another id.
codeunit 3704 "Other Setting Memo"
{
    SingleInstance = true;

    var
        Known: Boolean;
        Rate: Decimal;

    procedure Get(): Decimal
    begin
        if not Known then begin
            Rate := 1.5;
            Known := true;
        end;
        exit(Rate);
    end;
}
