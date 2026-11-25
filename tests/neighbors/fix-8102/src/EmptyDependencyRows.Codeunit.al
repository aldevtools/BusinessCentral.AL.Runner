codeunit 50100 "Empty Dependency Rows"
{
    Subtype = Test;

    // Microsoft's placeholder apps ship as a source archive with no .al file (Manufacturing, Service Management):
    // an installed app with no objects. Here a stand-in by the same publisher, without Base Application, so the
    // app stays within the neighbor run's memory limit.
    [Test]
    procedure MicrosoftArchiveWithoutAlFileIsInstalled()
    begin
        RequireInstalled('6d1f3c20-8102-4c3d-9e8f-810200000003', 'Neighbors 8102 Placeholder');
    end;

    // Another publisher's source archive with no .al file: not this rule, main's answer (skipped).
    [Test]
    procedure OtherPublisherArchiveWithoutAlFileIsInstalled()
    begin
        RequireInstalled('6d1f3c20-8102-4c3d-9e8f-810200000002', 'Neighbors 8102 Empty Other');
    end;

    local procedure RequireInstalled(IdText: Text; Name: Text)
    var
        Installed: Record "NAV App Installed App";
        Id: Guid;
    begin
        Evaluate(Id, IdText);
        if not Installed.Get(Id) then
            Error('%1 is not installed', Name);
    end;
}
