// A stand-in for the System Application's codeunit the probe's OnOpenPage calls: false, as on the OnPrem container the
// probe was measured on.
codeunit 457 "Environment Information"
{
    procedure IsSaaSInfrastructure(): Boolean
    begin
        exit(false);
    end;
}
