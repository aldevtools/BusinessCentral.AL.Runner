//! A test app given as a package file (`.Source.zip` or `.app`) reads its own packaged resources with
//! `NavApp.GetResource`, as an app folder does.

use super::dependencies::zip;
use super::*;

/// The test codeunit: session/TestNavAppGetResource.al:26-69 (top-level file, nested path, GetResourceAsText, the
/// missing-name error) and session/Y18NavAppResourceEncodings.al (a name with a space).
const TESTS: &str = r#"codeunit 50330 "Own Res Tests"
{
    Subtype = Test;

    [Test]
    procedure TopLevel()
    var
        S: InStream;
        T: Text;
    begin
        NavApp.GetResource('greeting.txt', S, TextEncoding::UTF8);
        S.ReadText(T);
        if T <> 'Hello from packaged resource' then Error('top: %1', T);
    end;

    [Test]
    procedure Nested()
    begin
        if NavApp.GetResourceAsText('sub/nested file.txt') <> 'nested-resource-content' then
            Error('nested: %1', NavApp.GetResourceAsText('sub/nested file.txt'));
    end;

    [Test]
    procedure Missing()
    var
        S: InStream;
    begin
        asserterror NavApp.GetResource('missing.txt', S);
        if StrPos(GetLastErrorText(), 'A resource matching ''missing.txt'' could not be found in app') <> 1 then
            Error('missing: %1', GetLastErrorText());
    end;
}"#;

fn passes_all(app: &Path) {
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download"]);
    let t = text(&o);
    assert_eq!(code(&o), 0, "{t}");
    assert!(t.contains("Tests: 3 total, 3 passed"), "{t}");
}

#[test]
fn a_source_archive_target_reads_the_resources_of_its_resource_folders() {
    let d = tempfile::tempdir().unwrap();
    let app = d.path().join("Own Res Tests.Source.zip");
    std::fs::write(
        &app,
        zip(&[
            (
                "app.json",
                r#"{"id":"22222222-0000-0000-0000-000000000031","name":"Own Res Tests","publisher":"Test","version":"1.0.0.0","dependencies":[],"resourceFolders":["Resources"]}"#,
            ),
            ("src/T.Codeunit.al", TESTS),
            ("Resources/greeting.txt", "Hello from packaged resource"),
            ("Resources/sub/nested file.txt", "nested-resource-content"),
        ]),
    )
    .unwrap();
    passes_all(&app);
}

#[test]
fn a_compiled_app_target_reads_its_resources_with_their_names_decoded() {
    let d = tempfile::tempdir().unwrap();
    let id = "22222222-0000-0000-0000-000000000032";
    let manifest = format!(
        r#"<Package xmlns="http://schemas.microsoft.com/navx/2015/manifest">
  <App Id="{id}" Name="Own Res App Tests" Publisher="Test" Version="1.0.0.0" Runtime="17.0" Target="Cloud" ShowMyCode="True" />
  <Dependencies />
</Package>"#
    );
    let app = d.path().join("Test_Own Res App Tests_1.0.0.0.app");
    std::fs::write(
        &app,
        zip(&[
            ("NavxManifest.xml", &manifest),
            ("src/T.Codeunit.al", TESTS),
            ("resources/greeting.txt", "Hello from packaged resource"),
            ("resources/sub/nested%20file.txt", "nested-resource-content"),
        ]),
    )
    .unwrap();
    passes_all(&app);
}
