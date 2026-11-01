//! Gap cycle 5, batch 5a: the platform-table and media shapes corpus writer Y48 pinned (corpus PR 980).
//! AL source -> compile against the System app of the corpus packages -> run as a test codeunit `T`; the values are the ones
//! real BC produced there.
//!
//! Needs the `corpus` submodule, including its `.alpackages` (`System.app`).

use al_deps::package::read_package;
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;
use std::path::Path;

fn system_app_files() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/tests/al-language/.alpackages");
    let bytes = std::fs::read(dir.join("System.app")).expect("corpus submodule not checked out");
    read_package(&bytes).expect("System.app reads").1
}

/// The outcome of each test of codeunit `T` in `main`, an app on the System app, run as BC 27.5.
fn outcomes(main: &str) -> Vec<(String, Outcome)> {
    outcomes_app("main", main)
}

/// [`outcomes`] with the id of the app under test chosen (a platform table lists apps by Guid).
fn outcomes_app(app_id: &str, main: &str) -> Vec<(String, Outcome)> {
    use al_sema::workspace::{AppKind, AppSource};
    let app = |id: &str, name: &str, files: Vec<(String, String)>| AppSource {
        id: id.into(),
        name: name.into(),
        dependencies: vec![],
        files,
    };
    let apps = vec![
        app(app_id, "main", vec![("main.al".into(), main.into())]),
        app(al_sema::workspace::PLATFORM_APP_ID, "System", system_app_files()),
    ];
    let symbols = ["BC27PLUS".to_string()].into();
    let (_, l) = al_sema::lower::compile_apps_with(
        apps,
        vec![AppKind::Workspace, AppKind::Dependency],
        &symbols,
        None,
        Default::default(),
    );
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let mut program = (*l.program).clone();
    for a in program.apps.iter_mut().filter(|a| a.version.is_empty()) {
        a.version = "1.0.0.0".into();
    }
    let engine = Engine::with_target(
        std::sync::Arc::new(program),
        blocked,
        Some(al_version::Target::for_bc(
            al_version::BcVersion::parse("27.5.0.0").unwrap(),
        )),
    );
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == "T")
        .expect("test codeunit T");
    run_codeunit(&engine, &l, oi)
}

fn passes(src: &str) {
    passes_app("main", src);
}

fn passes_app(app_id: &str, src: &str) {
    let o = outcomes_app(app_id, src);
    assert!(!o.is_empty());
    let bad: Vec<_> = o.into_iter().filter(|(_, r)| *r != Outcome::Pass).collect();
    assert!(bad.is_empty(), "{bad:?}");
}

// G5-02: Feature Key Get matches the ID case-insensitively and shows the listed casing; SetRange stays case-sensitive; Insert
// (with and without triggers, of an unknown or a listed ID) and Delete of a listed row raise "The Feature Key table is
// read-only." code DB:Ok and change nothing (record/lt-y48-feature-key-writes/Y48FeatureKeyWrites.al:104 FK_Get_OtherCasing,
// :126 FK_Insert_UnknownId, :151 FK_Insert_UnknownId_RunTrigger, :167 FK_Insert_ListedId, :188 FK_Delete_ListedRow).
#[test]
fn feature_key_get_ignores_case_and_writes_raise_read_only() {
    passes(
        r#"codeunit 50001 "Step" { var S: Text; K: Text[50]; R: Text;
  procedure SetStep(NS: Text; NK: Text[50]) begin S := NS; K := NK; R := ''; end;
  procedure Res(): Text begin exit(R); end;
  trigger OnRun() var F: Record "Feature Key"; begin
    case S of
      'insert', 'insert true': begin F.Init(); F.ID := K; F.Enabled := F.Enabled::"All Users"; R := Format(F.Insert(S = 'insert true')); end;
      'delete': begin F.Get(K); R := Format(F.Delete()); end;
    end; end; }
codeunit 50000 T { Subtype = Test;
  local procedure Run(S: Text; K: Text[50]): Text var C: Codeunit "Step"; begin
    C.SetStep(S, K); ClearLastError(); if C.Run() then exit('ok:' + C.Res()); exit('error:' + GetLastErrorText() + '|' + GetLastErrorCode()); end;
  local procedure Check(Got: Text; Want: Text) begin if Got <> Want then Error('got %1, want %2', Got, Want); end;
  [Test] procedure GetOtherCasing() var F: Record "Feature Key"; begin
    if not F.Get('DisableSOAPwebservicesonMicrosoftUIpages') then Error('not found');
    Check(F.ID, 'DisableSoapWebServicesOnMicrosoftUIPages');
    Clear(F); if not F.Get('disablesoapwebservicesonmicrosoftuipages') then Error('lower not found');
    Clear(F); F.SetRange(ID, 'DisableSOAPwebservicesonMicrosoftUIpages'); Check(Format(F.Count()), '0'); end;
  [Test] procedure Writes() var F: Record "Feature Key"; Before: Integer; begin
    Before := F.Count();
    Check(Run('insert', 'LTProbeUnknownKey'), 'error:The Feature Key table is read-only.|DB:Ok');
    Check(Run('insert true', 'LTProbeUnknownKey2'), 'error:The Feature Key table is read-only.|DB:Ok');
    Check(Run('insert', 'DisableSoapWebServicesOnMicrosoftUIPages'), 'error:The Feature Key table is read-only.|DB:Ok');
    Check(Run('insert', 'DisableSOAPwebservicesonMicrosoftUIpages'), 'error:The Feature Key table is read-only.|DB:Ok');
    Check(Run('delete', 'DisableSoapWebServicesOnMicrosoftUIPages'), 'error:The Feature Key table is read-only.|DB:Ok');
    Check(Format(F.Count() - Before), '0');
    if F.Get('LTProbeUnknownKey') then Error('inserted');
    if not F.Get('DisableSoapWebServicesOnMicrosoftUIPages') then Error('deleted'); end; }"#,
    );
}

// G5-02: Intelligent Cloud is an ordinary table: empty, Insert stores a row Get then finds, Modify of Enabled sticks, a second
// Insert answers false without an error, Delete empties it, another key is insertable (al-language-onprem/platform/
// lt-y48-intelligent-cloud/Y48IntelligentCloudWrites.al: IC_InsertModifyGet, IC_Standard).
#[test]
fn intelligent_cloud_is_an_ordinary_table() {
    passes(
        r#"codeunit 50000 T { Subtype = Test;
  [Test] procedure Writes() var IC: Record "Intelligent Cloud"; IC2: Record "Intelligent Cloud"; begin
    if not IC2.IsEmpty() then Error('not empty');
    if not IC.Get() then if not IC.Insert() then Error('insert');
    if not IC2.Get() then Error('get'); if IC2.Enabled then Error('enabled'); if IC2.Count() <> 1 then Error('count');
    IC.Enabled := true; IC.Modify(); Clear(IC2); IC2.Get(); if not IC2.Enabled then Error('modify');
    if IC.Insert() then Error('duplicate');
    IC.Delete(); if not IC2.IsEmpty() then Error('delete');
    IC."Primary Key" := 'LTY48'; IC.Insert(); if IC2.Count() <> 1 then Error('other key'); end; }"#,
    );
}

// G5-03: "Security Filter" (12) of an Expanded Permission row of a permissionset object is blank, plain and with format 9, and
// copies into a Tenant Permission as blank (permissions/lt-y48-security-filter/Y48ExpandedSecurityFilter.al:120
// EP_SecurityFilter_MetadataSet). That file is on corpus master (corpus PR 980) but not yet in the corpus pin, so the
// generated pinned_columns.rs does not list column 12 and the read is refused. The corpus bump that brings the file in
// makes this test fail: then assert a pass again.
#[test]
fn expanded_permission_security_filter_of_a_metadata_set_is_blank() {
    refused_with(
        "column 12 of platform table 'Expanded Permission'",
        r#"table 50001 "SF Table" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
permissionset 50002 "LTPSMETA" { Assignable = true; Permissions = tabledata "SF Table" = R; }
codeunit 50000 T { Subtype = Test;
  [Test] procedure Blank() var EP: Record "Expanded Permission"; TP: Record "Tenant Permission"; begin
    EP.SetRange("Role ID", 'LTPSMETA'); EP.FindFirst();
    if Format(EP."Security Filter") <> '' then Error('format'); if Format(EP."Security Filter", 0, 9) <> '' then Error('format 9');
    TP."Security Filter" := EP."Security Filter"; if Format(TP."Security Filter", 0, 9) <> '' then Error('copied'); end; }"#,
    );
}

fn refused_with(why: &str, src: &str) {
    let o = outcomes(src);
    assert!(!o.is_empty());
    assert!(
        o.iter()
            .all(|(_, r)| matches!(r, Outcome::NotRun(m) if m.contains(why))),
        "{o:?}"
    );
}

// G5-01: the Published Application row of the Application app: one row, Package ID and Runtime Package ID not null and
// different, Published As Global, Tenant ID blank, Tenant Visible; its Package ID is no other app's and no object's, so a
// filter on another app's Package ID plus Tenant Visible finds that app alone (al-language-onprem/platform/lt-y48-published-app/
// Y48PublishedApplication.al:154 PA_ApplicationRow_PackageIds; the shape of Extension Operation Impl. and the Objects page).
#[test]
fn published_application_lists_the_application_row() {
    passes_app(
        "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
        r#"codeunit 50000 T { Subtype = Test;
  [Test] procedure ApplicationRow() var A: Record "Published Application"; B: Record "Published Application"; O: Record AllObjWithCaption; MI: ModuleInfo; begin
    A.SetRange(ID, 'c1335042-3002-4257-bf8a-75c898ccb1b8'); A.FindFirst(); if A.Count() <> 1 then Error('count');
    if IsNullGuid(A."Package ID") or IsNullGuid(A."Runtime Package ID") then Error('null');
    if A."Package ID" = A."Runtime Package ID" then Error('same ids');
    if A."Published As" <> A."Published As"::Global then Error('published as');
    if A."Tenant ID" <> '' then Error('tenant id');
    B.SetRange("Package ID", A."Package ID"); B.SetRange("Tenant Visible", true); B.FindFirst();
    if (B.ID <> A.ID) or (B.Count() <> 1) then Error('application by package id');
    NavApp.GetCurrentModuleInfo(MI); if MI.PackageId = A."Package ID" then Error('own package id');
    B.SetRange("Package ID", MI.PackageId); B.FindFirst(); if (B.ID <> MI.Id) or (B.Count() <> 1) then Error('own by package id');
    O.SetRange("App Package ID", A."Package ID"); if not O.IsEmpty() then Error('objects'); end; }"#,
    );
}

// G5-01 (runtime note): FieldNo of an unpinned column reads no value, so it is not refused (the shape of Base Application's
// Data Classification Eval. Data: `DummyPublishedApplication.FieldNo(Blob)`); a read of the column still is.
#[test]
fn field_no_of_an_unpinned_column_is_not_a_read() {
    let o = outcomes(
        r#"codeunit 50000 T { Subtype = Test;
  [Test] procedure Number() var A: Record "Published Application"; begin if A.FieldNo(Blob) <> 22 then Error('%1', A.FieldNo(Blob)); end;
  [Test] procedure Read() var A: Record "Published Application"; begin A.CalcFields(Blob); end; }"#,
    );
    assert_eq!(o[0], ("Number".to_string(), Outcome::Pass), "{o:?}");
    assert!(
        matches!(&o[1].1, Outcome::NotRun(why) if why.contains("column 22")),
        "{o:?}"
    );
}

// G5-04: the Tenant Media row of a two-argument import of content that is no picture has Mime Type application/octet-stream,
// in a Media field and in a MediaSet (media/lt-y48-tenant-media/Y48TenantMediaRows.al:333 TM_DeleteRowOfHeldMedia, :484
// `item 1 tm row`); a three-argument import keeps its type (:399).
#[test]
fn tenant_media_mime_type_of_a_two_argument_import_is_octet_stream() {
    passes(
        r#"table 50001 "M Row" { fields { field(1; Id; Integer) { } field(2; Pic; Media) { } field(3; Pics; MediaSet) { } field(4; B; Blob) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 50000 T { Subtype = Test;
  local procedure Mime(Id: Guid): Text var TM: Record "Tenant Media"; begin TM.Get(Id); exit(TM.Description + '|' + TM."Mime Type"); end;
  local procedure Stream(var B: Record "M Row"; var InS: InStream) var O: OutStream; begin Clear(B.B); B.B.CreateOutStream(O); O.WriteText('content'); B.B.CreateInStream(InS); end;
  [Test] procedure Mimes() var R: Record "M Row"; B: Record "M Row"; InS: InStream; begin
    R.Id := 1; R.Insert();
    Stream(B, InS); R.Pic.ImportStream(InS, 'd'); if Mime(R.Pic.MediaId()) <> 'd|application/octet-stream' then Error(Mime(R.Pic.MediaId()));
    Stream(B, InS); R.Pics.ImportStream(InS, 'a'); if Mime(R.Pics.Item(1)) <> 'a|application/octet-stream' then Error(Mime(R.Pics.Item(1)));
    Stream(B, InS); R.Pic.ImportStream(InS, 'test.txt', 'text/plain'); if Mime(R.Pic.MediaId()) <> 'test.txt|text/plain' then Error(Mime(R.Pic.MediaId())); end; }"#,
    );
}
