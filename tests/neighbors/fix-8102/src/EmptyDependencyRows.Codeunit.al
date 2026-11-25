codeunit 50100 "Empty Dependency Rows"
{
    Subtype = Test;

    // Microsoft's placeholder apps (a source archive with no .al file) are installed apps with no objects.
    [Test]
    procedure MicrosoftPlaceholderManufacturingIsInstalled()
    begin
        RequireInstalled('4d6c453d-e61d-4872-b35d-6c731c0fd93e', 'Manufacturing');
    end;

    [Test]
    procedure MicrosoftPlaceholderServiceManagementIsInstalled()
    begin
        RequireInstalled('2fd9cbd8-c94a-46d6-8274-ea6ccb244f12', 'Service Management');
    end;

    // Another publisher's source archive with no .al file: not this rule, main's answer (skipped).
    [Test]
    procedure OtherPublisherArchiveWithoutAlFileIsInstalled()
    begin
        RequireInstalled('6d1f3c20-8102-4c3d-9e8f-810200000002', 'Neighbors 8102 Empty Other');
    end;

    // The Application aggregate keeps its own rule.
    [Test]
    procedure ApplicationAggregateIsInstalled()
    begin
        RequireInstalled('c1335042-3002-4257-bf8a-75c898ccb1b8', 'Application');
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
