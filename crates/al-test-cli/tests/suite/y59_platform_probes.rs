//! Gap cycle 6, batch 6a: the platform-table shapes corpus writer Y59 pinned (corpus PR 1003, corpus master 9cf26907), and the
//! Tenant Profile Setting rows of lt-y35-stored-tables/Y35StoredTables.al. AL source -> compile against the System app of the
//! corpus packages -> run as a test codeunit `T`; the values are the ones real BC produced there.
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

/// The outcome of each test of codeunit `T` in `main`, an app with id `app_id` on the System app, run as BC 27.5.
fn outcomes_for(app_id: &str, main: &str) -> Vec<(String, Outcome)> {
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

const APP: &str = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";

fn passes(src: &str) {
    let o = outcomes_for(APP, src);
    assert!(!o.is_empty());
    let bad: Vec<_> = o.into_iter().filter(|(_, r)| *r != Outcome::Pass).collect();
    assert!(bad.is_empty(), "{bad:?}");
}

// G6-01a: "Object Name" (6) of Metadata Permission is the object's caption, or its name without a Caption; blank until
// CalcFields; filled by SetAutoCalcFields; filterable. "Role Name" (3) is the set's Caption, "Assignable" (13) Yes and "AL Object
// Name" (15) the object name (permissions/lt-y59-metadata-permission/Y59MetadataPermissionNames.al:146, :173, :195, :257, :280).
#[test]
fn metadata_permission_object_name_and_the_other_flowfield_columns() {
    passes(
        r#"table 50100 "Cap Table" { Caption = 'Cap Table Caption'; fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
table 50101 "NoCap Table" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
page 50100 "Cap Page" { Caption = 'Cap Page Caption'; SourceTable = "Cap Table"; layout { area(Content) { field(Id; Rec.Id) { } } } }
codeunit 50100 "Perm Cu" { }
report 50100 "Perm Report" { Caption = 'Perm Report Caption'; ProcessingOnly = true; }
query 50100 "Perm Query" { Caption = 'Perm Query Caption'; elements { dataitem(C; "Cap Table") { column(Id; Id) { } } } }
xmlport 50100 "Perm XmlPort" { schema { textelement(Root) { } } }
permissionset 50100 "PS Names" { Assignable = true; Caption = 'PS Names Caption';
  Permissions = tabledata "Cap Table" = RIMD, table "Cap Table" = X, tabledata "NoCap Table" = R, page "Cap Page" = X, codeunit "Perm Cu" = X,
    report "Perm Report" = X, query "Perm Query" = X, xmlport "Perm XmlPort" = X; }
codeunit 50000 "T" { Subtype = Test;
  local procedure Own(var M: Record "Metadata Permission") var Mi: ModuleInfo; begin
    NavApp.GetCurrentModuleInfo(Mi); M.SetRange("App ID", Mi.Id()); M.SetRange("Role ID", 'PS NAMES'); end;
  local procedure Key(var M: Record "Metadata Permission"): Text begin exit(Format(M."Object Type") + ' ' + Format(M."Object ID")); end;
  [Test] procedure AfterCalcFields() var M: Record "Metadata Permission"; R: Text; begin
    Own(M); if M.Count() <> 8 then Error('count %1', M.Count());
    M.FindSet(); repeat
      if M."Object Name" <> '' then Error('before calcfields %1', M."Object Name");
      M.CalcFields("Object Name", "Role Name", Assignable, "AL Object Name");
      R += Key(M) + '=' + M."Object Name" + '|' + M."Role Name" + '|' + Format(M.Assignable) + '|' + M."AL Object Name" + '|' + Format(M.Type) + ';';
    until M.Next() = 0;
    if R <> 'Table Data 50100=Cap Table Caption|PS Names Caption|Yes|Cap Table|Include;Table Data 50101=NoCap Table|PS Names Caption|Yes|NoCap Table|Include;' +
      'Table 50100=Cap Table Caption|PS Names Caption|Yes|Cap Table|Include;Report 50100=Perm Report Caption|PS Names Caption|Yes|Perm Report|Include;' +
      'Codeunit 50100=Perm Cu|PS Names Caption|Yes|Perm Cu|Include;XMLport 50100=Perm XmlPort|PS Names Caption|Yes|Perm XmlPort|Include;' +
      'Page 50100=Cap Page Caption|PS Names Caption|Yes|Cap Page|Include;Query 50100=Perm Query Caption|PS Names Caption|Yes|Perm Query|Include;' then Error(R); end;
  [Test] procedure AutoCalcAndFilter() var M: Record "Metadata Permission"; begin
    Own(M); M.SetAutoCalcFields("Object Name"); M.SetRange("Object Type", M."Object Type"::Page); M.FindFirst();
    if M."Object Name" <> 'Cap Page Caption' then Error('auto %1', M."Object Name");
    M.SetRange("Object Type"); M.SetFilter("Object Name", '@cap table*'); if M.Count() <> 2 then Error('filter %1', M.Count());
    M.SetFilter("Object Name", '*Caption'); if M.Count() <> 5 then Error('caption %1', M.Count()); end; }"#,
    );
}

// G6-02: Page Metadata "SourceTableTemporary" (25) is Yes for a page that declares it true and for a page on a TableType =
// Temporary table; No for explicit false, no property and no SourceTable. "App ID" (29) is the app's id
// (platform/lt-y59-page-metadata/Y59PageMetaSourceTemp.al:192-197, :236-239, :257).
#[test]
fn page_metadata_source_table_temporary_and_app_id() {
    passes(
        r#"table 50100 "Src" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { } } }
table 50101 "TmpSrc" { TableType = Temporary; fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { } } }
page 50101 "PTrue" { PageType = List; SourceTable = "Src"; SourceTableTemporary = true; layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } } } } }
page 50102 "PFalse" { PageType = List; SourceTable = "Src"; SourceTableTemporary = false; layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } } } } }
page 50103 "PNone" { PageType = List; SourceTable = "Src"; layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } } } } }
page 50104 "PTmp" { PageType = List; SourceTable = "TmpSrc"; layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } } } } }
page 50105 "PCard" { PageType = Card; SourceTable = "Src"; SourceTableTemporary = true; layout { area(Content) { field(Code; Rec.Code) { } } } }
page 50106 "PNoSrc" { PageType = Card; layout { area(Content) { field(I; I) { } } } var I: Text; }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure ByDeclaration() var P: Record "Page Metadata"; Mi: ModuleInfo; R: Text; begin
    NavApp.GetCurrentModuleInfo(Mi); P.SetRange(ID, 50101, 50106); P.FindSet();
    repeat R += Format(P.ID) + '=' + Format(P.SourceTableTemporary) + Format(P."App ID" = Mi.Id()) + ';'; until P.Next() = 0;
    if R <> '50101=YesYes;50102=NoYes;50103=NoYes;50104=YesYes;50105=YesYes;50106=NoYes;' then Error(R);
    P.SetRange(SourceTableTemporary, true); if P.Count() <> 3 then Error('true %1', P.Count()); end;
  [Test] procedure RecordRefOpen() var P: Record "Page Metadata"; S: Record "Src"; RR: RecordRef; begin
    P.Get(50101); RR.Open(P.SourceTable, P.SourceTableTemporary); if not RR.IsTemporary() then Error('temp');
    RR.Init(); RR.Field(1).Value := 'A'; RR.Insert(); RR.Close(); if not S.IsEmpty() then Error('in db'); end; }"#,
    );
}

// G6-01b: Metadata Permission Set Rel. has a row per include (also one a permission set extension adds) and per exclude, in key
// order, Related Role ID upper case; none for a set without includes; a non-assignable set's rows are listed; Get with lower-case
// Role IDs finds a row; writes raise the read-only error (permissions/lt-y59-metadata-permission/Y59PermissionSetRel.al:192-197,
// :221-229, :297-304).
#[test]
fn metadata_permission_set_rel_lists_includes_and_excludes() {
    passes(
        r#"permissionset 50101 "RelA" { Assignable = true; }
permissionset 50102 "RelB" { Assignable = true; IncludedPermissionSets = "RelA"; }
permissionset 50103 "RelC" { Assignable = true; IncludedPermissionSets = "RelA", "RelB"; ExcludedPermissionSets = "RelD"; }
permissionset 50104 "RelD" { Assignable = true; }
permissionset 50106 "RelF" { Assignable = false; IncludedPermissionSets = "RelA"; }
permissionsetextension 50100 "RelB Ext" extends "RelB" { IncludedPermissionSets = "RelD"; }
codeunit 50000 "T" { Subtype = Test;
  local procedure Own(): Guid var Mi: ModuleInfo; begin NavApp.GetCurrentModuleInfo(Mi); exit(Mi.Id()); end;
  local procedure Rows(Role: Code[30]): Text var R: Record "Metadata Permission Set Rel."; T: Text; begin
    R.SetRange("App ID", Own()); R.SetRange("Role ID", Role);
    if R.FindSet() then repeat T += Format(R.Type) + ':' + R."Related Role ID" + Format(R."Related App ID" = Own()) + ';'; until R.Next() = 0;
    exit(Format(R.Count()) + '|' + T); end;
  [Test] procedure Lists() var T: Text; begin
    T := Rows('RELC') + Rows('RELB') + Rows('RELA') + Rows('RELD') + Rows('RELF');
    if T <> '3|Include:RELAYes;Include:RELBYes;Exclude:RELDYes;2|Include:RELAYes;Include:RELDYes;0|0|1|Include:RELAYes;' then Error(T); end;
  [Test] procedure GetAndFilter() var R: Record "Metadata Permission Set Rel."; begin
    if not R.Get(Own(), 'relc', Own(), 'reld') then Error('get'); if R.Type <> R.Type::Exclude then Error('type');
    if R.Get(Own(), 'RELA', Own(), 'RELB') then Error('missing');
    R.Reset(); R.SetRange("Related Role ID", 'RELA'); if R.Count() <> 3 then Error('related %1', R.Count()); end;
  [Test] procedure Writes() var R: Record "Metadata Permission Set Rel."; begin
    R."App ID" := Own(); R."Role ID" := 'RELX'; R."Related Role ID" := 'RELA'; asserterror R.Insert();
    if GetLastErrorText() <> 'The Metadata Permission Set Rel. table is read-only.' then Error(GetLastErrorText());
    if GetLastErrorCode() <> 'DB:Ok' then Error(GetLastErrorCode());
    R.Get(Own(), 'RELB', Own(), 'RELA'); asserterror R.Delete();
    if GetLastErrorText() <> 'The Metadata Permission Set Rel. table is read-only.' then Error(GetLastErrorText()); end; }"#,
    );
}

// G6-04b: Page Data Personalization is a stored table: Insert(false), a duplicate Insert false keeping the old value, Modify, the
// BLOB read back, "User ID" a FlowField on the User table, Date and Time left blank, any User SID
// (platform/lt-y59-page-personalization/Y59PageDataPersonalization.al:122-129, :152-156, :253-255).
#[test]
fn page_data_personalization_is_a_stored_table() {
    passes(
        r#"codeunit 50000 "T" { Subtype = Test;
  local procedure Put(Id: Code[40]; V: Text): Boolean var P: Record "Page Data Personalization"; O: OutStream; B: BigText; begin
    P."User SID" := UserSecurityId(); P."Object Type" := P."Object Type"::Page; P."Object ID" := 9000; P."Personalization ID" := Id; P.ValueName := 'N';
    P.Value.CreateOutStream(O, TextEncoding::UTF8); B.AddText(V); B.Write(O); exit(P.Insert(false)); end;
  local procedure Read(Id: Code[40]): Text var P: Record "Page Data Personalization"; I: InStream; B: BigText; T: Text; begin
    P.SetRange("User SID", UserSecurityId()); P.SetRange("Personalization ID", Id); P.SetAutoCalcFields(Value);
    if not P.FindFirst() then exit('none'); P.Value.CreateInStream(I, TextEncoding::UTF8); B.Read(I); B.GetSubText(T, 1); exit(T); end;
  [Test] procedure InsertAndRead() var P: Record "Page Data Personalization"; begin
    if not Put('A', 'first') then Error('insert'); if Put('A', 'second') then Error('duplicate');
    if Read('A') <> 'first' then Error(Read('A'));
    P.SetRange("Personalization ID", 'A'); P.FindFirst();
    if P."User ID" <> '' then Error('user id before calcfields'); P.CalcFields("User ID"); if P."User ID" <> UserId() then Error('user id %1', P."User ID");
    if (P.Date <> 0D) or (P.Time <> 0T) then Error('date'); end;
  [Test] procedure OtherUser() var P: Record "Page Data Personalization"; begin
    P."User SID" := CreateGuid(); P."Personalization ID" := 'O'; if not P.Insert(false) then Error('insert');
    P.CalcFields("User ID"); if P."User ID" <> '' then Error(P."User ID"); end; }"#,
    );
}

// G6-04a/c: Tenant Profile Setting is stored; a setting of an existing profile sets Default Role Center of its All Profile row,
// DeleteAll turns it off; a setting that names no profile adds no All Profile row (al-language-onprem/platform/
// lt-y35-stored-tables/Y35StoredTables.al:610-617, lt-y59-tenant-profile-setting/Y59AllProfileGhostSetting.al:82-85).
#[test]
fn tenant_profile_setting_sets_its_profile_in_all_profile() {
    passes(
        r#"page 50100 "RC" { PageType = RoleCenter; }
profile "Own Profile" { Caption = 'Own Profile'; RoleCenter = "RC"; }
codeunit 50000 "T" { Subtype = Test;
  local procedure Default(): Text var A: Record "All Profile"; begin
    A.SetRange("Default Role Center", true); A.SetRange(Enabled, true); if not A.FindFirst() then exit('none'); exit(A."Profile ID"); end;
  local procedure Put(App: Guid; Id: Code[30]): Boolean var S: Record "Tenant Profile Setting"; begin
    S."App ID" := App; S."Profile ID" := Id; S."Default Role Center" := true; exit(S.Insert()); end;
  [Test] procedure Link() var S: Record "Tenant Profile Setting"; A: Record "All Profile"; Mi: ModuleInfo; begin
    NavApp.GetCurrentModuleInfo(Mi);
    if not S.IsEmpty() then Error('rows without the Base Application');
    if Default() <> 'none' then Error('before');
    if not Put(Mi.Id(), 'OWN PROFILE') then Error('insert'); if Put(Mi.Id(), 'OWN PROFILE') then Error('duplicate');
    if Default() <> 'OWN PROFILE' then Error('after insert %1', Default());
    S.DeleteAll(); if Default() <> 'none' then Error('after deleteall');
    if not Put(CreateGuid(), 'GHOST') then Error('ghost');
    A.SetRange("Profile ID", 'GHOST'); if not A.IsEmpty() then Error('ghost row'); if Default() <> 'none' then Error('ghost default');
    S.SetRange("Profile ID", 'GHOST'); if S.Count() <> 1 then Error('count'); end; }"#,
    );
}

fn not_run(src: &str, test: &str) -> String {
    let o = outcomes_for(APP, src);
    match o.iter().find(|(n, _)| n == test).map(|(_, r)| r) {
        Some(Outcome::NotRun(why)) => why.clone(),
        other => panic!("{test} was expected to be refused, got {other:?}"),
    }
}

// Two shapes the Metadata Permission Set Rel. provider lets run further, which stay refused: the Expanded Permission rows of a
// set that excludes sets (not pinned; a filter on Object Type and Object ID must not rule the refused row out,
// codeunit/lt-x-object-props/XObjectProps.al:341), and the Description of a profile that declares ProfileDescriptionML (per
// session language, :365-400).
#[test]
fn an_excluding_set_and_a_profile_description_ml_stay_refused() {
    let src = r#"table 50100 "PT" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
permissionset 50101 "Base" { Assignable = true; Permissions = tabledata "PT" = RIM; }
permissionset 50102 "Neg" { Assignable = true; Permissions = tabledata "PT" = IM; }
permissionset 50103 "Main" { Assignable = true; IncludedPermissionSets = "Base"; ExcludedPermissionSets = "Neg"; }
page 50100 "RC" { PageType = RoleCenter; }
profile "Desc Profile" { Caption = 'Desc'; ProfileDescriptionML = ENU = 'English', DEU = 'Deutsch'; RoleCenter = "RC"; }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Excluding() var E: Record "Expanded Permission"; Mi: ModuleInfo; begin
    NavApp.GetCurrentModuleInfo(Mi); E.SetRange("App ID", Mi.Id()); E.SetRange("Role ID", 'MAIN');
    E.SetRange("Object Type", E."Object Type"::"Table Data"); E.SetRange("Object ID", 50100); if E.FindFirst() then; end;
  [Test] procedure DescriptionMl() var A: Record "All Profile"; begin
    A.SetRange("Profile ID", 'DESC PROFILE'); A.FindFirst(); if A.Description = '' then; end; }"#;
    assert!(not_run(src, "Excluding").contains("excludes sets"));
    assert!(not_run(src, "DescriptionMl").contains("ProfileDescriptionML"));
}
