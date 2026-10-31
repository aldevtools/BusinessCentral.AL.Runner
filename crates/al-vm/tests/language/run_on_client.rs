//! `[RunOnClient]` DotNet variables in a test session, which has no client (ADR-027 section 3 row 20). The shapes are
//! the ones corpus writer Y16 pinned on real BC 27.5, 28.5 and 29.0 (corpus master 92fdde19,
//! tests/al-language-onprem-dotnet/lt-bg-runonclient/BgRunOnClient.al:244-468 with the helpers of
//! Y16RunOnClientShapes.al): Microsoft's Camera page hands its `[RunOnClient] [WithEvents]` provider to an implementation
//! codeunit by value and by `var`.

const NOT_SUPPORTED: &str =
    "The operation cannot complete because it uses a feature that is not supported (.NET Framework Interoperability.)";

const DOTNET: &str = r#"
dotnet
{
    assembly("Microsoft.Dynamics.Nav.ClientExtensions")
    {
        type("Microsoft.Dynamics.Nav.Client.Capabilities.CameraProvider"; Cam) { }
        type("Microsoft.Dynamics.Nav.Client.PageNotifier"; Notifier) { }
    }
    assembly("mscorlib")
    {
        type("System.Text.StringBuilder"; Sb) { }
        type("System.IO.Directory"; Dir) { }
    }
}
"#;

const SHAPES: &str = r#"
page 50100 Host
{
    PageType = Card;

    var
        Impl: Codeunit Impl;
        [RunOnClient]
        [WithEvents]
        CameraProvider: DotNet Cam;
        [RunOnClient]
        DirectoryHelper: DotNet Dir;
        PictureCount: Integer;

    [TryFunction]
    procedure TryIsAvailable(var Available: Boolean)
    begin
        Available := Impl.IsAvailable(CameraProvider);
    end;

    [TryFunction]
    procedure TryOpen(var Available: Boolean)
    begin
        Impl.OnOpen(CameraProvider, Available);
    end;

    [TryFunction]
    procedure TryForceCreate(var Step: Text)
    begin
        Impl.ForceCreate(CameraProvider, Step);
    end;

    [TryFunction]
    procedure TryDirectoryExists(var Step: Text)
    begin
        Step := 'exists';
        Step := 'exists=' + Format(DirectoryHelper.Exists('C:\'));
    end;

    [TryFunction]
    procedure TryDirectoryIsEmpty(var Step: Text)
    begin
        Step := 'getfiles';
        Step := 'empty=' + Format(DirectoryHelper.GetFiles('C:\').Length = 0);
    end;

    procedure ProviderIsNull(): Boolean
    begin
        exit(IsNull(CameraProvider));
    end;

    procedure Pictures(): Integer
    begin
        exit(PictureCount);
    end;

    trigger CameraProvider::PictureAvailable(PictureName: Text; PictureFilePath: Text)
    begin
        PictureCount += 1;
    end;
}

codeunit 50101 Impl
{
    procedure IsAvailable(CameraProvider: DotNet Cam): Boolean
    begin
        exit(CameraProvider.IsAvailable());
    end;

    procedure OnOpen(var CameraProvider: DotNet Cam; var Available: Boolean)
    begin
        Available := IsAvailable(CameraProvider);
        if not Available then
            exit;
        CameraProvider := CameraProvider.Create();
    end;

    procedure ForceCreate(var CameraProvider: DotNet Cam; var Step: Text)
    begin
        Step := 'create';
        CameraProvider := CameraProvider.Create();
        Step := 'created isnull=' + Format(IsNull(CameraProvider));
    end;

    [TryFunction]
    procedure TryServerIsAvailable(var Available: Boolean)
    var
        ServerCamera: DotNet Cam;
    begin
        Available := ServerCamera.IsAvailable();
    end;

    [TryFunction]
    procedure TryServerCreate(var Step: Text)
    var
        ServerCamera: DotNet Cam;
    begin
        Step := 'create';
        ServerCamera := ServerCamera.Create();
        Step := 'created isnull=' + Format(IsNull(ServerCamera));
    end;
}

codeunit 50102 Probe
{
    local procedure Err(): Text
    begin
        exit('error [' + GetLastErrorText() + '] code=' + GetLastErrorCode());
    end;

    procedure ByValue(): Text
    var
        Host: Page Host;
        Available: Boolean;
    begin
        Available := true;
        if not Host.TryIsAvailable(Available) then
            exit(Err());
        exit('ret=' + Format(Available) + ' isnull=' + Format(Host.ProviderIsNull()) + ' pictures=' + Format(Host.Pictures()));
    end;

    procedure OnOpenByVar(): Text
    var
        Host: Page Host;
        Available: Boolean;
    begin
        Available := true;
        if not Host.TryOpen(Available) then
            exit(Err());
        exit('available=' + Format(Available) + ' isnull=' + Format(Host.ProviderIsNull()));
    end;

    procedure CreateThroughVar(): Text
    var
        Host: Page Host;
        Step: Text;
    begin
        Step := 'untouched';
        if Host.TryForceCreate(Step) then
            exit('ok ' + Step);
        exit('step=' + Step + ' ' + Err() + ' isnull=' + Format(Host.ProviderIsNull()));
    end;

    procedure ServerSide(): Text
    var
        Impl: Codeunit Impl;
        Available: Boolean;
        Step: Text;
        R: Text;
    begin
        Available := true;
        if Impl.TryServerIsAvailable(Available) then
            R := 'ret=' + Format(Available)
        else
            R := Err();
        Step := 'untouched';
        if Impl.TryServerCreate(Step) then
            exit(R + ' ok [' + Step + ']');
        exit(R + ' step=' + Step + ' ' + Err());
    end;

    [TryFunction]
    local procedure TryNotifierGuard(var Result: Text; var WasNull: Boolean)
    var
        [RunOnClient]
        LocalNotifier: DotNet Notifier;
    begin
        Result := 'start';
        if LocalNotifier.IsAvailable() then begin
            LocalNotifier := LocalNotifier.Create();
            LocalNotifier.NotifyPageReady();
            Result := 'notified';
        end else
            Result := 'not available';
        WasNull := IsNull(LocalNotifier);
    end;

    procedure NotifierGuard(): Text
    var
        Result: Text;
        WasNull: Boolean;
    begin
        Result := 'untouched';
        if TryNotifierGuard(Result, WasNull) then
            exit('ok [' + Result + '] isnull=' + Format(WasNull));
        exit('step=' + Result + ' ' + Err());
    end;

    procedure Directory(): Text
    var
        Host: Page Host;
        Step: Text;
        R: Text;
    begin
        Step := 'untouched';
        if Host.TryDirectoryExists(Step) then
            R := 'ok ' + Step
        else
            R := 'step=' + Step + ' ' + Err();
        Step := 'untouched';
        if Host.TryDirectoryIsEmpty(Step) then
            exit(R + ' | ok ' + Step);
        exit(R + ' | step=' + Step + ' ' + Err());
    end;

    [TryFunction]
    local procedure TryServerToClient(var Step: Text)
    var
        ServerBuilder: DotNet Sb;
        [RunOnClient]
        ClientBuilder: DotNet Sb;
    begin
        Step := 'construct';
        ServerBuilder := ServerBuilder.StringBuilder();
        ServerBuilder.Append('srv');
        Step := 'assign';
        ClientBuilder := ServerBuilder;
        Step := 'assigned isnull=' + Format(IsNull(ClientBuilder));
    end;

    [TryFunction]
    local procedure TryClientToServer(var Step: Text)
    var
        ServerBuilder: DotNet Sb;
        [RunOnClient]
        ClientBuilder: DotNet Sb;
    begin
        Step := 'assign';
        ServerBuilder := ClientBuilder;
        Step := 'assigned isnull=' + Format(IsNull(ServerBuilder));
    end;

    procedure ServerToClient(): Text
    var
        Step: Text;
    begin
        Step := 'untouched';
        if TryServerToClient(Step) then
            exit('ok ' + Step);
        exit('step=' + Step + ' ' + Err());
    end;

    procedure ClientToServer(): Text
    var
        Step: Text;
    begin
        Step := 'untouched';
        if TryClientToServer(Step) then
            exit('ok ' + Step);
        exit('step=' + Step + ' ' + Err());
    end;

    [TryFunction]
    local procedure TryClearAndVariant(var Step: Text)
    var
        [RunOnClient]
        ClientBuilder: DotNet Sb;
        V: Variant;
    begin
        Step := 'clear';
        Clear(ClientBuilder);
        Step := 'cleared isnull=' + Format(IsNull(ClientBuilder));
        V := ClientBuilder;
        Step += ' variant isdotnet=' + Format(V.IsDotNet());
    end;

    procedure ClearAndVariant(): Text
    var
        Step: Text;
    begin
        Step := 'untouched';
        if TryClearAndVariant(Step) then
            exit('ok ' + Step);
        exit('step=' + Step + ' ' + Err());
    end;
}
"#;

fn run(routine: &str) -> String {
    let src = format!("{DOTNET}{SHAPES}");
    al_testkit::text(al_testkit::run_in(&src, "Probe", routine))
}

// BgRunOnClient.al Y16_RunOnClient_CameraPageShape_ImplByValue: the static IsAvailable() through a by-value parameter answers
// No, the provider stays null and its event trigger never runs.
#[test]
fn the_camera_page_hands_its_provider_by_value() {
    assert_eq!(run("ByValue"), "ret=No isnull=Yes pictures=0");
}

// BgRunOnClient.al Y16_RunOnClient_CameraPageShape_OnOpenByVar.
#[test]
fn the_camera_page_hands_its_provider_by_var_and_stops_when_not_available() {
    assert_eq!(run("OnOpenByVar"), "available=No isnull=Yes");
}

// BgRunOnClient.al Y16_RunOnClient_CreateThroughVarParameter: the `var` parameter is the [RunOnClient] variable, so
// Create() on it fails as on the variable itself.
#[test]
fn create_through_a_var_parameter_of_a_run_on_client_variable_fails() {
    assert_eq!(
        run("CreateThroughVar"),
        format!(
            "step=create error [{NOT_SUPPORTED}] code=DotNetFromAlNotSupported:DotNetFromAlNotSupported isnull=Yes"
        )
    );
}

// BgRunOnClient.al Y16_ServerTier_CameraProvider_IsAvailableAndCreate: without [RunOnClient] the calls run on the service
// tier: IsAvailable() answers No and Create() gives null with no error.
#[test]
fn a_camera_provider_without_run_on_client_is_not_available_and_creates_null() {
    assert_eq!(run("ServerSide"), "ret=No ok [created isnull=Yes]");
}

// BgRunOnClient.al Y16_RunOnClient_RoleCenterPageNotifierGuard.
#[test]
fn the_page_notifier_guard_skips_the_client_code() {
    assert_eq!(run("NotifierGuard"), "ok [not available] isnull=Yes");
}

// BgRunOnClient.al Y16_RunOnClient_FileManagementDirectory: File Management's client System.IO.Directory.
#[test]
fn a_static_call_on_a_client_directory_fails() {
    let e = format!("error [{NOT_SUPPORTED}] code=DotNetFromAlNotSupported:DotNetFromAlNotSupported");
    assert_eq!(run("Directory"), format!("step=exists {e} | step=getfiles {e}"));
}

// BgRunOnClient.al Y16_RunOnClient_AssignBetweenClientAndServerVariables `client to server`: the null of a [RunOnClient]
// variable goes to a service-tier variable. The other direction needs a service-tier object, so it is tested with a fake .NET
// host (al-vm/tests/dotnet.rs `a_service_tier_object_cannot_be_assigned_to_a_run_on_client_variable`).
#[test]
fn the_null_of_a_run_on_client_variable_goes_to_a_service_tier_variable() {
    assert_eq!(run("ClientToServer"), "ok assigned isnull=Yes");
}

// BgRunOnClient.al Y16_RunOnClient_ClearAndVariant.
#[test]
fn clear_and_a_variant_of_a_run_on_client_variable() {
    assert_eq!(run("ClearAndVariant"), "ok cleared isnull=Yes variant isdotnet=Yes");
}

#[test]
fn an_unpinned_member_of_a_run_on_client_variable_stays_refused() {
    let src = format!(
        "{DOTNET} codeunit 50103 U {{ var [RunOnClient] B: DotNet Sb; procedure F(): Integer begin exit(B.Length); end; }}"
    );
    let r = al_testkit::run_in(&src, "U", "F");
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}

// al-runner issue 6489: a page whose [RunOnClient] [WithEvents] CameraProvider goes by value to a codeunit's IsAvailable,
// as System Application's Camera page does; a test that asks the page whether the camera is available runs.
#[test]
fn issue_6489_the_camera_of_a_page_is_not_available() {
    let src = format!(
        "{DOTNET} codeunit 50600 CamImpl {{ procedure IsAvailable(CameraProvider: DotNet Cam): Boolean begin exit(CameraProvider.IsAvailable()); end; }} \
         page 50601 CamPage {{ PageType = Card; var [RunOnClient] [WithEvents] CameraProvider: DotNet Cam; \
         procedure IsAvailable(): Boolean var Impl: Codeunit CamImpl; begin exit(Impl.IsAvailable(CameraProvider)); end; }} \
         codeunit 50602 T {{ procedure F(): Boolean var Camera: Page CamPage; begin exit(Camera.IsAvailable()); end; }}"
    );
    assert!(!al_testkit::boolean(al_testkit::run_in(&src, "T", "F")));
}
