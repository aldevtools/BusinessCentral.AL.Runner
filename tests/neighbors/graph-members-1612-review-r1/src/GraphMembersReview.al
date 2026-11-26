// Review round 1 of the Graph client members (corpus PR 1612, Y93GraphNoDirectory.al): a GraphQuery the constructor did
// not build, and the foreach body walk over a UserInfo whose source PR 1612 does not measure.
dotnet
{
    assembly("Microsoft.Dynamics.Nav.AzureADGraphClient")
    {
        type("Microsoft.Dynamics.Nav.AzureADGraphClient.GraphQuery"; "GR GraphQuery") { }
    }
    assembly("Microsoft.Dynamics.Nav.LicensingService.Model")
    {
        type("Microsoft.Dynamics.Nav.LicensingService.Model.UserInfo"; "GR UserInfo") { }
        type("Microsoft.Dynamics.Nav.LicensingService.Model.GroupInfo"; "GR GroupInfo") { }
    }
}

// A login subscriber whose foreach bodies loop over a UserInfo parameter, and a global. Their item members keep main's count.
codeunit 50350 "GR Login"
{
    var
        GlobalUser: DotNet "GR UserInfo";

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure OnAfterLogin()
    var
        U: DotNet "GR UserInfo";
    begin
        FromParameter(U);
        FromGlobal();
    end;

    local procedure FromParameter(U: DotNet "GR UserInfo")
    var
        I: DotNet "GR GroupInfo";
    begin
        if not IsNull(U) then
            foreach I in U.Groups() do
                if I.DisplayName() = '' then;
    end;

    local procedure FromGlobal()
    var
        I: DotNet "GR GroupInfo";
    begin
        if not IsNull(GlobalUser) then
            foreach I in GlobalUser.Groups() do
                if I.DisplayName() = '' then;
    end;

}

codeunit 50351 "GR Disabled Tests"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure LoginSubscriber_UserInfoSources()
    begin
    end;
}

codeunit 50352 "GR Tests"
{
    Subtype = Test;

    // A GraphQuery never constructed: corpus PR 1612 measures neither member on it. Refused on both binaries.
    [Test]
    procedure G_Null_GetUserByObjectId()
    var
        G: DotNet "GR GraphQuery";
        U: DotNet "GR UserInfo";
    begin
        asserterror U := G.GetUserByObjectId('x');
        if GetLastErrorCode() = '' then
            Error('No error code');
    end;

    [Test]
    procedure G_Null_IsGroupMember()
    var
        G: DotNet "GR GraphQuery";
    begin
        asserterror if G.IsGroupMember('a', 'b') then;
        if GetLastErrorCode() = '' then
            Error('No error code');
    end;
}
