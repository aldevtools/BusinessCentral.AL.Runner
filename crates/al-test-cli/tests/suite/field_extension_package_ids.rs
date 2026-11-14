//! al-runner issue 8051: Field."App Package ID" and "App Runtime Package ID" of a field a table extension adds are the ids of
//! the extension's app, not the table's. Corpus PR 1402 (merged, not pinned yet) measures it:
//! tests/al-language/metadata/platform-tables/PlatformMetadataColumnValues.al:360-376 and :388-405. Until it is pinned the rows
//! run through the assumption `field-row-extension-package-ids`.
//!
//! Needs `System.app` of the pinned build in the artifact cache (`al_testfiles`).

use al_builtins::assumptions::Assumption;
use al_deps::package::read_package;
use al_sema::workspace::{AppKind, AppSource};
use al_test_cli::report::Outcome;
use al_vm::Engine;

const OWNER_ID: &str = "85e17fa2-1200-47c9-8026-2913229126b9";
const MIDDLE_ID: &str = "5b0c3f6e-1a2b-4c3d-8e9f-a1b2c3d4e5b2";
const TEST_ID: &str = "2a2e7610-64e6-4556-9178-a59feed7ad64";

/// Runs codeunit `T` of the last of `apps` (id, kind, source), each app depending on every app before it, over the
/// platform's System app; each test with the assumptions it went through.
fn run(apps: &[(&str, AppKind, &str)]) -> Vec<(String, Outcome, Vec<Assumption>)> {
    let system = read_package(&std::fs::read(al_testfiles::system_app()).unwrap())
        .expect("System.app reads")
        .1;
    let mut sources = Vec::new();
    let mut kinds = Vec::new();
    let mut ids: Vec<String> = Vec::new();
    for (id, kind, src) in apps {
        sources.push(AppSource {
            id: id.to_string(),
            name: id.to_string(),
            dependencies: ids.clone(),
            files: vec![(format!("{id}.al"), src.to_string())],
        });
        kinds.push(*kind);
        ids.push(id.to_string());
    }
    sources.push(AppSource {
        id: al_sema::workspace::PLATFORM_APP_ID.into(),
        name: "System".into(),
        dependencies: Vec::new(),
        files: system,
    });
    kinds.push(AppKind::Dependency);
    let symbols = ["BC27PLUS".to_string()].into();
    let (_, l) = al_sema::lower::compile_apps_with(sources, kinds, &symbols, None, Default::default());
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    let mut program = (*l.program).clone();
    for a in program.apps.iter_mut().filter(|a| a.version.is_empty()) {
        a.version = "1.0.0.0".into();
    }
    let engine = Engine::with_target(
        std::sync::Arc::new(program),
        blocked,
        Some(al_version::Target::for_bc(
            al_version::BcVersion::parse("29.0.0.0").unwrap(),
        )),
    );
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == "T")
        .expect("test codeunit T");
    let mut hits = Vec::new();
    let o = al_testrun::runtime::run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        al_testrun::isolation::TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
    );
    o.into_iter()
        .map(|(name, outcome)| {
            let h = hits
                .iter()
                .filter(|(n, _)| *n == name)
                .flat_map(|(_, h)| h.iter().copied())
                .collect();
            (name, outcome, h)
        })
        .collect()
}

const OWNER: &str = r#"
table 50100 "Repro Line"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

codeunit 50101 "Repro Field Lister"
{
    procedure ForeignFieldCount(TableNo: Integer): Integer
    var
        TableField: Record Field;
        OwnModuleInfo: ModuleInfo;
    begin
        NavApp.GetCurrentModuleInfo(OwnModuleInfo);
        TableField.SetRange(TableNo, TableNo);
        TableField.SetFilter("No.", '<%1', 2000000000);
        TableField.SetRange(Class, TableField.Class::Normal);
        TableField.SetRange(Enabled, true);
        TableField.SetFilter(ObsoleteState, '<>%1', TableField.ObsoleteState::Removed);
        TableField.SetFilter("App Package ID", '<>%1', OwnModuleInfo.PackageId());
        exit(TableField.Count());
    end;
}"#;

/// The reporter's repro (issue 8051, two apps under test): the field the test app's table extension adds carries the test
/// app's package ids after Get, FindFirst on the key and a scan; a Get of the table's own field still gives null ids
/// (PlatformMetadataColumnValues.al:287-311); a read filtered on the owner's package id keeps only the extension field.
#[test]
fn issue_8051_extension_field_carries_its_apps_package_ids() {
    let test = r#"
tableextension 50110 "Repro Line Ext" extends "Repro Line"
{
    fields { field(50110; "Ext Station"; Text[20]) { } }
}

codeunit 50120 T
{
    Subtype = Test;

    [Test]
    procedure FieldTableListsTheExtensionField()
    var
        TableField: Record Field;
    begin
        TableField.SetRange(TableNo, Database::"Repro Line");
        TableField.SetFilter("No.", '<%1', 2000000000);
        if TableField.Count() <> 3 then
            Error('count %1', TableField.Count());
    end;

    [Test]
    procedure ExtensionFieldHasAnotherAppPackageId()
    var
        OwnField: Record Field;
        ExtField: Record Field;
    begin
        OwnField.Get(Database::"Repro Line", 1);
        ExtField.Get(Database::"Repro Line", 50110);
        if ExtField."App Package ID" = OwnField."App Package ID" then
            Error('same %1', OwnField."App Package ID");
    end;

    [Test]
    procedure PackageIdFilterKeepsOnlyTheExtensionField()
    var
        FieldLister: Codeunit "Repro Field Lister";
    begin
        if FieldLister.ForeignFieldCount(Database::"Repro Line") <> 1 then
            Error('count %1', FieldLister.ForeignFieldCount(Database::"Repro Line"));
    end;

    [Test]
    procedure ExtensionFieldIdsAreTheExtensionsAfterGetFindAndScan()
    var
        F: Record Field;
        Ext: Record AllObj;
        Tbl: Record AllObj;
        Own: ModuleInfo;
    begin
        NavApp.GetCurrentModuleInfo(Own);
        Ext.Get(Ext."Object Type"::TableExtension, 50110);
        Tbl.Get(Tbl."Object Type"::Table, 50100);
        F.Get(50100, 50110);
        if (F."App Package ID" <> Ext."App Package ID") or (F."App Runtime Package ID" <> Ext."App Runtime Package ID") then
            Error('get');
        if F."App Package ID" <> Own.PackageId() then
            Error('get own');
        Clear(F);
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 50110);
        F.FindFirst();
        if (F."App Package ID" <> Ext."App Package ID") or (F."App Runtime Package ID" <> Ext."App Runtime Package ID") then
            Error('findfirst');
        Clear(F);
        F.SetRange(TableNo, 50100);
        F.FindSet();
        while F."No." <> 50110 do
            F.Next();
        if F."App Package ID" = Tbl."App Package ID" then
            Error('scan');
        F.Get(50100, 1);
        if not IsNullGuid(F."App Package ID") then
            Error('own field get');
    end;
}"#;
    let o = run(&[
        (OWNER_ID, AppKind::Workspace, OWNER),
        (TEST_ID, AppKind::Workspace, test),
    ]);
    assert_eq!(o.len(), 4);
    let bad: Vec<_> = o.iter().filter(|(_, r, _)| *r != Outcome::Pass).collect();
    assert!(bad.is_empty(), "{bad:?}");
    // The control reads no package column, but its rows include the extension field's.
    let unflagged: Vec<_> = o
        .iter()
        .filter(|(_, _, h)| !h.contains(&Assumption::FieldRowExtensionPackageIds))
        .map(|(n, ..)| n)
        .collect();
    assert!(unflagged.is_empty(), "not flagged: {unflagged:?}");
}

/// A table's own fields are not flagged: a read of Field rows that holds no extension field's row marks nothing.
#[test]
fn issue_8051_own_fields_are_not_flagged() {
    let test = r#"
tableextension 50110 "Repro Line Ext" extends "Repro Line"
{
    fields { field(50110; "Ext Station"; Text[20]) { } }
}

codeunit 50120 T
{
    Subtype = Test;

    [Test]
    procedure OwnField()
    var
        F: Record Field;
        Tbl: Record AllObj;
    begin
        Tbl.Get(Tbl."Object Type"::Table, 50100);
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 1, 2);
        F.FindFirst();
        if F."App Package ID" <> Tbl."App Package ID" then
            Error('own');
        // The key filters leave the extension field's row out, so a filter on the package does not read it.
        F.SetRange("App Package ID", Tbl."App Package ID");
        if F.Count() <> 2 then
            Error('own count %1', F.Count());
    end;
}"#;
    let o = run(&[
        (OWNER_ID, AppKind::Workspace, OWNER),
        (TEST_ID, AppKind::Workspace, test),
    ]);
    assert_eq!(o.len(), 1);
    assert_eq!(o[0].1, Outcome::Pass);
    assert!(!o[0].2.contains(&Assumption::FieldRowExtensionPackageIds));
}

/// A filter on the package that leaves the extension field's row out rests on its assumed value too: it is flagged.
#[test]
fn issue_8051_a_package_filter_that_drops_the_extension_field_is_flagged() {
    let test = r#"
tableextension 50110 "Repro Line Ext" extends "Repro Line"
{
    fields { field(50110; "Ext Station"; Text[20]) { } }
}

codeunit 50120 T
{
    Subtype = Test;

    [Test]
    procedure OnlyTheTablesPackage()
    var
        F: Record Field;
        Tbl: Record AllObj;
    begin
        Tbl.Get(Tbl."Object Type"::Table, 50100);
        F.SetRange(TableNo, 50100);
        F.SetFilter("No.", '<%1', 2000000000);
        F.SetRange("App Package ID", Tbl."App Package ID");
        if F.Count() <> 2 then
            Error('count %1', F.Count());
        F.FindLast();
        if F."No." <> 2 then
            Error('last %1', F."No.");
    end;
}"#;
    let o = run(&[
        (OWNER_ID, AppKind::Workspace, OWNER),
        (TEST_ID, AppKind::Workspace, test),
    ]);
    assert_eq!(o.len(), 1);
    assert_eq!(o[0].1, Outcome::Pass);
    assert!(o[0].2.contains(&Assumption::FieldRowExtensionPackageIds));
}

/// Not measured: the package ids of a field that a table extension of a dependency app (compiled from its package, as
/// Microsoft's apps are) adds. A read of them is refused; a read of the row's other columns is answered.
#[test]
fn issue_8051_extension_field_of_a_dependency_app_is_refused() {
    let middle = r#"
tableextension 50110 "Middle Ext" extends "Repro Line"
{
    fields { field(50110; "Middle Note"; Text[20]) { } }
}"#;
    let test = r#"
codeunit 50120 T
{
    Subtype = Test;

    [Test]
    procedure PackageOfDependencyExtensionField()
    var
        F: Record Field;
    begin
        F.Get(50100, 50110);
        if IsNullGuid(F."App Package ID") then;
    end;

    [Test]
    procedure NameOfDependencyExtensionField()
    var
        F: Record Field;
    begin
        F.Get(50100, 50110);
        if F.FieldName <> 'Middle Note' then
            Error('name %1', F.FieldName);
    end;
}"#;
    let o = run(&[
        (OWNER_ID, AppKind::Dependency, OWNER),
        (MIDDLE_ID, AppKind::Dependency, middle),
        (TEST_ID, AppKind::Workspace, test),
    ]);
    let outcome = |name: &str| o.iter().find(|(n, ..)| n == name).map(|(_, r, _)| r.clone()).unwrap();
    assert!(
        matches!(outcome("PackageOfDependencyExtensionField"), Outcome::NotRun(ref why) if why.contains("App Package ID")),
        "{:?}",
        outcome("PackageOfDependencyExtensionField")
    );
    assert_eq!(outcome("NameOfDependencyExtensionField"), Outcome::Pass);
}

/// Not measured either: a table extension of an app under test on a table of a dependency app other than Base Application.
#[test]
fn issue_8051_extension_on_another_dependency_apps_table_is_refused() {
    let test = r#"
tableextension 50110 "Repro Line Ext" extends "Repro Line"
{
    fields { field(50110; "Ext Station"; Text[20]) { } }
}

codeunit 50120 T
{
    Subtype = Test;

    [Test]
    procedure PackageOfExtensionField()
    var
        F: Record Field;
    begin
        F.Get(50100, 50110);
        if IsNullGuid(F."App Package ID") then;
    end;
}"#;
    let o = run(&[
        (OWNER_ID, AppKind::Dependency, OWNER),
        (TEST_ID, AppKind::Workspace, test),
    ]);
    assert_eq!(o.len(), 1);
    assert!(
        matches!(o[0].1, Outcome::NotRun(ref why) if why.contains("App Package ID")),
        "{:?}",
        o[0].1
    );
}

/// A table extension in the table's own app makes a key lookup of the table's Field rows give the table's ids
/// (PlatformMetadataColumnValues.al:569-605, corpus PR 1407; :516-532 for an extension field of the table's own app on Base
/// Application's 6502). The field's row gets no extension ids
/// (`extension_packages` takes only another app's extension), and a `<>` filter on the table's id finds no row.
#[test]
fn issue_8051_extension_in_the_tables_own_app_gives_the_tables_ids_on_a_key_lookup() {
    let src = r#"
table 50100 "Same Line"
{
    fields { field(1; "No."; Code[20]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}

tableextension 50110 "Same Line Ext" extends "Same Line"
{
    fields { field(50110; "Ext Station"; Text[20]) { } }
}

codeunit 50120 T
{
    Subtype = Test;

    [Test]
    procedure SameApp()
    var
        F: Record Field;
        Tbl: Record AllObj;
    begin
        Tbl.Get(Tbl."Object Type"::Table, 50100);
        F.Get(50100, 50110);
        if F."App Package ID" <> Tbl."App Package ID" then
            Error('get ext %1', F."App Package ID");
        F.Get(50100, 1);
        if F."App Runtime Package ID" <> Tbl."App Runtime Package ID" then
            Error('get own %1', F."App Package ID");
        F.Reset();
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 50110);
        F.SetFilter("App Package ID", '<>%1', Tbl."App Package ID");
        if F.Count() <> 0 then
            Error('filter %1', F.Count());
    end;
}"#;
    let o = run(&[(OWNER_ID, AppKind::Workspace, src)]);
    assert_eq!(o.len(), 1);
    assert_eq!(o[0].1, Outcome::Pass, "{:?}", o[0].1);
    assert!(o[0].2.contains(&Assumption::FieldKeyLookupPackageIds));
}

const PLAIN_TABLE: &str = r#"
table 50100 "Plain Line"
{
    fields { field(1; "No."; Code[20]) { } field(2; Description; Text[30]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}"#;

/// A key lookup (Get, or a FindFirst or FindSet with TableNo and No. set to one value each) of a declared field of a table no
/// extension of its own app extends gives null ids, whichever app owns the table; a range of No. and a system field give the
/// table's ids (PlatformMetadataColumnValues.al:569-605 and :618-646, corpus PR 1407; :516-532 for System Application's tables).
/// Here the table is a dependency app's, where main gave the table's ids: flagged.
#[test]
fn issue_8051_key_lookup_of_a_dependency_table_without_same_app_extension_is_null() {
    let test = r#"
codeunit 50120 T
{
    Subtype = Test;

    [Test]
    procedure KeyLookups()
    var
        F: Record Field;
        Tbl: Record AllObj;
    begin
        Tbl.Get(Tbl."Object Type"::Table, 50100);
        F.Get(50100, 1);
        if not IsNullGuid(F."App Package ID") or not IsNullGuid(F."App Runtime Package ID") then
            Error('get');
        Clear(F);
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 2);
        F.FindFirst();
        if not IsNullGuid(F."App Package ID") then
            Error('key findfirst');
        F.FindSet();
        if not IsNullGuid(F."App Package ID") then
            Error('key findset');
        F.SetRange("No.", 1, 2);
        F.FindFirst();
        if F."App Package ID" <> Tbl."App Package ID" then
            Error('range findfirst');
        F.Get(50100, 2000000000);
        if F."App Package ID" <> Tbl."App Package ID" then
            Error('systemid');
    end;
}"#;
    let o = run(&[
        (OWNER_ID, AppKind::Dependency, PLAIN_TABLE),
        (TEST_ID, AppKind::Workspace, test),
    ]);
    assert_eq!(o.len(), 1);
    assert_eq!(o[0].1, Outcome::Pass, "{:?}", o[0].1);
    assert!(o[0].2.contains(&Assumption::FieldKeyLookupPackageIds));
}

/// A system field of a table of the app under test gives the table's ids on a Get (main gave null): flagged. A Get of its
/// declared field keeps the pinned null (PlatformMetadataColumnValues.al:287-311) and marks nothing.
#[test]
fn issue_8051_get_of_a_system_field_gives_the_tables_ids() {
    let src = format!(
        "{PLAIN_TABLE}{}",
        r#"
codeunit 50120 T
{
    Subtype = Test;

    [Test]
    procedure Declared()
    var
        F: Record Field;
    begin
        F.Get(50100, 1);
        if not IsNullGuid(F."App Package ID") then
            Error('declared');
    end;

    [Test]
    procedure SystemId()
    var
        F: Record Field;
        Tbl: Record AllObj;
    begin
        Tbl.Get(Tbl."Object Type"::Table, 50100);
        F.Get(50100, 2000000000);
        if F."App Package ID" <> Tbl."App Package ID" then
            Error('systemid');
        F.Get(50100, 2000000004);
        if F."App Package ID" <> Tbl."App Package ID" then
            Error('systemmodifiedby');
    end;
}"#
    );
    let o = run(&[(OWNER_ID, AppKind::Workspace, &src)]);
    let by = |n: &str| o.iter().find(|(x, ..)| x == n).unwrap();
    assert_eq!(by("Declared").1, Outcome::Pass, "{:?}", by("Declared").1);
    assert!(!by("Declared").2.contains(&Assumption::FieldKeyLookupPackageIds));
    assert_eq!(by("SystemId").1, Outcome::Pass, "{:?}", by("SystemId").1);
    assert!(by("SystemId").2.contains(&Assumption::FieldKeyLookupPackageIds));
}

/// Not measured: a key lookup that also filters on a package column, of a row the key lookup gives null ids (BC answers its
/// FindFirst with no row while its Count is 1). The FindFirst is refused; Count keeps main's answer.
#[test]
fn issue_8051_key_lookup_with_a_package_filter_is_refused() {
    let src = format!(
        "{PLAIN_TABLE}{}",
        r#"
codeunit 50120 T
{
    Subtype = Test;

    [Test]
    procedure Find()
    var
        F: Record Field;
        Tbl: Record AllObj;
    begin
        Tbl.Get(Tbl."Object Type"::Table, 50100);
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 1);
        F.SetRange("App Package ID", Tbl."App Package ID");
        if F.FindFirst() then;
    end;

    [Test]
    procedure Count()
    var
        F: Record Field;
        Tbl: Record AllObj;
    begin
        Tbl.Get(Tbl."Object Type"::Table, 50100);
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 1);
        F.SetRange("App Package ID", Tbl."App Package ID");
        if F.Count() <> 1 then
            Error('count %1', F.Count());
    end;
}"#
    );
    let o = run(&[(OWNER_ID, AppKind::Workspace, &src)]);
    let by = |n: &str| o.iter().find(|(x, ..)| x == n).unwrap().1.clone();
    assert!(
        matches!(by("Find"), Outcome::NotRun(ref why) if why.contains("package")),
        "{:?}",
        by("Find")
    );
    assert_eq!(by("Count"), Outcome::Pass);
}
