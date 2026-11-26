// Review round 1 of the Graph client members (corpus PR 1612, Y93GraphNoDirectory.al): a GraphQuery the constructor did
// not build, and the foreach body walk over a UserInfo whose source PR 1612 does not measure.
dotnet
{
    assembly("Microsoft.Dynamics.Nav.AzureADGraphClient")
    {
        type("Microsoft.Dynamics.Nav.AzureADGraphClient.GraphQuery"; "GC GraphQuery") { }
    }
    assembly("Microsoft.Dynamics.Nav.LicensingService.Model")
    {
        type("Microsoft.Dynamics.Nav.LicensingService.Model.UserInfo"; "GC UserInfo") { }
        type("Microsoft.Dynamics.Nav.LicensingService.Model.GroupInfo"; "GC GroupInfo") { }
    }
}

// A login subscriber whose foreach body loops over a UserInfo built by its constructor (Y58MockGraphQuery.al:51 builds
// UserInfo that way). The constructor and the item members keep main's count.
codeunit 50400 "GC Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure OnAfterLogin()
    var
        U: DotNet "GC UserInfo";
        I: DotNet "GC GroupInfo";
    begin
        if not IsNull(U) then begin
            U := U.UserInfo();
            foreach I in U.Groups() do
                if I.DisplayName() = '' then;
        end;
    end;
}

codeunit 50401 "GC Disabled Tests"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure LoginSubscriber_ConstructedUserInfo()
    begin
    end;
}
