// Neighbors of the Graph client members corpus PR 1612 measures (Y93GraphNoDirectory.al), on a tier without Entra ID.
dotnet
{
    assembly("Microsoft.Dynamics.Nav.AzureADGraphClient")
    {
        type("Microsoft.Dynamics.Nav.AzureADGraphClient.GraphQuery"; "GM GraphQuery") { }
    }
    assembly("Microsoft.Dynamics.Nav.LicensingService.Model")
    {
        type("Microsoft.Dynamics.Nav.LicensingService.Model.UserInfo"; "GM UserInfo") { }
        type("Microsoft.Dynamics.Nav.LicensingService.Model.GroupInfo"; "GM GroupInfo") { }
    }
}

// A login subscriber that asks System Application for the plan role center, as Base Application's
// Conf./Personalization Mgt. GetCurrentProfileNoError does for LmXaLoginCaches.al. Base Application itself does not fit
// the neighbor check's memory limit (3072 MB), so the profile call is not here.
codeunit 50300 "GM Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure OnAfterLogin()
    var
        AzureADPlan: Codeunit "Azure AD Plan";
        RoleCenterId: Integer;
    begin
        if AzureADPlan.TryGetAzureUserPlanRoleCenterId(RoleCenterId, UserSecurityId()) then;
        GroupNames();
    end;

    // As "Azure AD Graph Impl.".IsGroupMember: a String method on an item member's result, inside the foreach body
    // over U.Groups() (the body never runs: GetCurrentUser answers null on this tier).
    local procedure GroupNames()
    var
        G: DotNet "GM GraphQuery";
        U: DotNet "GM UserInfo";
        I: DotNet "GM GroupInfo";
    begin
        G := G.GraphQuery();
        U := G.GetCurrentUser();
        if not IsNull(U) then
            foreach I in U.Groups() do
                if I.DisplayName().ToUpper() = '' then;
    end;
}

codeunit 50301 "GM Disabled Tests"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure LoginSubscriber_TestReadsNothing()
    begin
    end;
}

codeunit 50302 "GM Tests"
{
    Subtype = Test;

    [Test]
    procedure PlanRoleCenter_InTest()
    var
        AzureADPlan: Codeunit "Azure AD Plan";
        RoleCenterId: Integer;
    begin
        if AzureADPlan.TryGetAzureUserPlanRoleCenterId(RoleCenterId, UserSecurityId()) then;
    end;

    [Test]
    procedure GetUserByObjectId_Raises()
    var
        G: DotNet "GM GraphQuery";
        U: DotNet "GM UserInfo";
    begin
        G := G.GraphQuery();
        asserterror U := G.GetUserByObjectId('x');
        if GetLastErrorText() <> 'A call to Microsoft.Dynamics.Nav.AzureADGraphClient.GraphQuery.GetUserByObjectId failed with this message: Object reference not set to an instance of an object.' then
            Error('Unexpected: %1', GetLastErrorText());
    end;

    [Test]
    procedure IsGroupMember_Raises()
    var
        G: DotNet "GM GraphQuery";
    begin
        G := G.GraphQuery();
        asserterror if G.IsGroupMember('a', 'b') then;
        if GetLastErrorText() <> 'A call to Microsoft.Dynamics.Nav.AzureADGraphClient.GraphQuery.IsGroupMember failed with this message: Object reference not set to an instance of an object.' then
            Error('Unexpected: %1', GetLastErrorText());
    end;

    // A UserInfo member the probe does not measure: refused on both binaries.
    [Test]
    procedure UserInfo_DisplayName_Refused()
    var
        G: DotNet "GM GraphQuery";
        U: DotNet "GM UserInfo";
    begin
        G := G.GraphQuery();
        U := G.GetCurrentUser();
        if U.DisplayName() = '' then;
    end;
}
