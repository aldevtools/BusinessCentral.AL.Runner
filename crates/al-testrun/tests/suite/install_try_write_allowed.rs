//! al-runner issue 7348: with `--allow-write-in-try-functions` a write inside a TryFunction whose result is used runs in
//! install code too (OnInstallAppPerDatabase, a DataPerCompany = false table): the TryFunction returns true and the row
//! changes, also after `NavApp.GetResource` and `Media.ImportStream` (corpus
//! tests/al-language-trywrite-allowed/TryWriteInstall.al, run by the corpus CI job "BC TryWrites").

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50100 "Repro Install Row"
{
    DataPerCompany = false;
    fields
    {
        field(1; "No."; Integer) { }
        field(2; Description; Text[50]) { }
        field(3; Thumbnail; Media) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

codeunit 50100 "Repro Try Install"
{
    Subtype = Install;

    trigger OnInstallAppPerDatabase()
    var
        Row: Record "Repro Install Row";
    begin
        Row."No." := 1;
        Row.Insert();
        // The result is used, so this is a try function call.
        if TrySetDescription(Row) then;
        Row."No." := 2;
        Row.Description := '';
        Row.Insert();
        if TrySetResourceThumbnail(Row, 'thumb.png') then;
    end;

    [TryFunction]
    local procedure TrySetDescription(var Row: Record "Repro Install Row")
    begin
        Row.Description := 'Set in install';
        Row.Modify();
    end;

    [TryFunction]
    local procedure TrySetResourceThumbnail(var Row: Record "Repro Install Row"; FileName: Text)
    var
        Logo: InStream;
    begin
        NavApp.GetResource(FileName, Logo);
        Row.Thumbnail.ImportStream(Logo, FileName);
        Row.Description := 'media';
        Row.Modify();
    end;
}

codeunit 50150 "Repro Try Install Test"
{
    Subtype = Test;

    [Test]
    procedure InstallModifyInsideTryFunctionRan()
    var
        Row: Record "Repro Install Row";
    begin
        Row.Get(1);
        if Row.Description <> 'Set in install' then
            Error('description %1', Row.Description);
        Row.Get(2);
        if (Row.Description <> 'media') or not Row.Thumbnail.HasValue() then
            Error('media row %1 %2', Row.Description, Row.Thumbnail.HasValue());
    end;
}
"#;

fn manifest() -> Manifest {
    Manifest {
        publisher: "Me".into(),
        version: "1.0.0.0".into(),
        resources: vec![("thumb.png".into(), b"\x89PNG\r\n\x1a\n".to_vec())],
        include_source: false,
        target: String::new(),
        logo: None,
        description: None,
        layout_files: Vec::new(),
        id_ranges: Vec::new(),
        translations: false,
        package_data: false,
        package_data_files: Vec::new(),
        translation_files: Vec::new(),
        translation_source: None,
        runtime: None,
        app_json_keys: Vec::new(),
        symbols: vec![],
    }
}

fn run(allowed: bool) -> Outcome {
    let apps = vec![AppSource {
        id: "11111111-1111-1111-1111-111111117348".into(),
        name: "Repro".into(),
        dependencies: Vec::new(),
        files: vec![("f.al".into(), SRC.into())],
    }];
    let rt = CorpusRuntime::try_run_suites_with(
        apps,
        vec![al_sema::AppKind::Workspace],
        &[manifest()],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions {
            allow_write_in_try_functions: allowed,
            ..Default::default()
        },
    )
    .expect("the run starts");
    rt.first_outcome(&CaseId {
        codeunit: 50150,
        name: "InstallModifyInsideTryFunctionRan".into(),
    })
}

/// TryWriteInstall.al Install_ModifyInTryFunction_Kept and Install_ResourceThumbnailInTryFunction_Kept.
#[test]
fn modify_in_used_try_function_in_install_code_runs_with_writes_allowed() {
    assert_eq!(run(true), Outcome::Pass);
}
