//! al-runner issue 8051: Field."App Package ID" and "App Runtime Package ID" of a field a table extension adds are the ids of
//! the extension's app, not the table's: tests/al-language/metadata/platform-tables/PlatformMetadataColumnValues.al:360-376
//! and :388-405 pin it.
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
pub(crate) fn run(apps: &[(&str, AppKind, &str)]) -> Vec<(String, Outcome, Vec<Assumption>)> {
    run_isolated(apps, al_testrun::isolation::TestIsolation::Codeunit)
}

/// [`run`] under the test isolation `isolation`.
pub(crate) fn run_isolated(
    apps: &[(&str, AppKind, &str)],
    isolation: al_testrun::isolation::TestIsolation,
) -> Vec<(String, Outcome, Vec<Assumption>)> {
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
        isolation,
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
}

/// A table's own fields: a read of Field rows that holds no extension field's row marks nothing.
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
    assert!(o[0].2.is_empty(), "{:?}", o[0].2);
}

/// A filter on the package that leaves the extension field's row out (PlatformMetadataColumnValues.al:388-405) marks nothing.
#[test]
fn issue_8051_a_package_filter_that_drops_the_extension_field_is_unmarked() {
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
    assert!(o[0].2.is_empty(), "{:?}", o[0].2);
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
/// (PlatformMetadataColumnValues.al:569-605; :516-532 for an extension field of the table's own app on Base
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
    assert!(o[0].2.is_empty(), "{:?}", o[0].2);
}

const PLAIN_TABLE: &str = r#"
table 50100 "Plain Line"
{
    fields { field(1; "No."; Code[20]) { } field(2; Description; Text[30]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}"#;

/// A key lookup (Get, or a FindFirst or FindSet with TableNo and No. set to one value each) of a declared field of a table no
/// extension of its own app extends gives null ids, whichever app owns the table; a range of No. and a system field give the
/// table's ids (PlatformMetadataColumnValues.al:569-605 and :618-646; :516-532 for System Application's tables).
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
    assert!(o[0].2.is_empty(), "{:?}", o[0].2);
}

/// A system field of a table of the app under test gives the table's ids on a Get (:618-646). A Get of its
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
    assert!(by("Declared").2.is_empty(), "{:?}", by("Declared").2);
    assert_eq!(by("SystemId").1, Outcome::Pass, "{:?}", by("SystemId").1);
    assert!(by("SystemId").2.is_empty(), "{:?}", by("SystemId").2);
}

/// A key read with a filter on App Package ID too (TableNo, No. and the package filter in group 0; PlatformMetadataColumnValues.al
/// :844-868): on a table whose key lookup gives null ids, FindFirst and FindSet test the filter against the null
/// ids, while IsEmpty and Count test it against the table's ids.
#[test]
fn issue_8051_key_lookup_with_a_package_filter_finds_by_the_null_ids() {
    let src = format!(
        "{PLAIN_TABLE}{}",
        r#"
codeunit 50120 T
{
    Subtype = Test;

    local procedure Reads(var F: Record Field): Text
    begin
        exit(Format(F.FindFirst()) + ' ' + Format(F.FindSet()) + ' ' + Format(F.IsEmpty()) + ' ' + Format(F.Count()));
    end;

    [Test]
    procedure PackageFilters()
    var
        F: Record Field;
        Tbl: Record AllObj;
        Empty: Guid;
    begin
        Tbl.Get(Tbl."Object Type"::Table, 50100);
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 1);
        F.SetRange("App Package ID", Tbl."App Package ID");
        if Reads(F) <> 'No No No 1' then
            Error('= table %1', Reads(F));
        F.SetFilter("App Package ID", '<>%1', Tbl."App Package ID");
        if Reads(F) <> 'Yes Yes Yes 0' then
            Error('<> table %1', Reads(F));
        if not IsNullGuid(F."App Package ID") then
            Error('found ids');
        F.SetRange("App Package ID", Empty);
        if Reads(F) <> 'Yes Yes Yes 0' then
            Error('= empty %1', Reads(F));
        if F.Next() <> 0 then
            Error('next');
    end;
}"#
    );
    let o = run(&[(OWNER_ID, AppKind::Workspace, &src)]);
    assert_eq!(o.len(), 1);
    assert_eq!(o[0].1, Outcome::Pass, "{:?}", o[0].1);
    assert!(o[0].2.is_empty(), "{:?}", o[0].2);
}

/// Not measured: a key read with a filter on App Runtime Package ID, or with the package filter in another filter group. Refused,
/// as before; Count keeps main's answer.
#[test]
fn issue_8051_key_lookup_with_an_unmeasured_package_filter_is_refused() {
    let src = format!(
        "{PLAIN_TABLE}{}",
        r#"
codeunit 50120 T
{
    Subtype = Test;

    [Test]
    procedure RuntimeFilter()
    var
        F: Record Field;
        Tbl: Record AllObj;
    begin
        Tbl.Get(Tbl."Object Type"::Table, 50100);
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 1);
        F.SetRange("App Runtime Package ID", Tbl."App Runtime Package ID");
        if F.FindFirst() then;
    end;

    [Test]
    procedure OtherGroup()
    var
        F: Record Field;
        Tbl: Record AllObj;
    begin
        Tbl.Get(Tbl."Object Type"::Table, 50100);
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 1);
        F.FilterGroup(2);
        F.SetRange("App Package ID", Tbl."App Package ID");
        F.FilterGroup(0);
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
        F.SetRange("App Runtime Package ID", Tbl."App Runtime Package ID");
        if F.Count() <> 1 then
            Error('count %1', F.Count());
    end;
}"#
    );
    let o = run(&[(OWNER_ID, AppKind::Workspace, &src)]);
    let by = |n: &str| o.iter().find(|(x, ..)| x == n).unwrap().1.clone();
    for n in ["RuntimeFilter", "OtherGroup"] {
        assert!(
            matches!(by(n), Outcome::NotRun(ref why) if why.contains("App Package ID")),
            "{n}: {:?}",
            by(n)
        );
    }
    assert_eq!(by("Count"), Outcome::Pass);
}

/// Every other read of field 1 whose filters set TableNo and No. to one value each answers as a Get does (null here: a dependency
/// table no extension of its own app extends): FindLast, Find('-'), Find('+'), Find('='), a descending FindFirst, Next from a
/// blank record, a third filter on Class, both filters in FilterGroup 2, TableNo in group 2 and No. in 0, RecordRef.FindFirst and
/// RecordRef.Find('=') (PlatformMetadataColumnValues.al:753-772).
#[test]
fn issue_8051_other_key_read_forms_answer_as_get() {
    let test = r#"
codeunit 50120 T
{
    Subtype = Test;

    local procedure Null(var F: Record Field; What: Text)
    begin
        if not IsNullGuid(F."App Package ID") or not IsNullGuid(F."App Runtime Package ID") then
            Error(What);
    end;

    [Test]
    procedure Forms()
    var
        F: Record Field;
        RecRef: RecordRef;
        FldRef: FieldRef;
    begin
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 1);
        F.FindLast();
        Null(F, 'findlast');
        F.Find('-');
        Null(F, 'find-');
        F.Find('+');
        Null(F, 'find+');
        Clear(F);
        F.TableNo := 50100;
        F."No." := 1;
        F.Find('=');
        Null(F, 'find=');
        Clear(F);
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 1);
        F.SetAscending("No.", false);
        F.FindFirst();
        Null(F, 'descending');
        Clear(F);
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 1);
        F.Next();
        Null(F, 'next from blank');
        Clear(F);
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 1);
        F.SetRange(Class, F.Class::Normal);
        F.FindFirst();
        Null(F, 'class');
        Clear(F);
        F.FilterGroup(2);
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 1);
        F.FilterGroup(0);
        F.FindFirst();
        Null(F, 'group 2');
        Clear(F);
        F.FilterGroup(2);
        F.SetRange(TableNo, 50100);
        F.FilterGroup(0);
        F.SetRange("No.", 1);
        F.FindFirst();
        Null(F, 'groups 2 and 0');
        RecRef.Open(Database::Field);
        FldRef := RecRef.Field(1);
        FldRef.SetRange(50100);
        FldRef := RecRef.Field(2);
        FldRef.SetRange(1);
        RecRef.FindFirst();
        RecRef.SetTable(F);
        Null(F, 'recordref findfirst');
        RecRef.Close();
        RecRef.Open(Database::Field);
        FldRef := RecRef.Field(1);
        FldRef.Value := 50100;
        FldRef := RecRef.Field(2);
        FldRef.Value := 1;
        RecRef.Find('=');
        RecRef.SetTable(F);
        Null(F, 'recordref find=');
    end;
}"#;
    let o = run(&[
        (OWNER_ID, AppKind::Dependency, PLAIN_TABLE),
        (TEST_ID, AppKind::Workspace, test),
    ]);
    assert_eq!(o.len(), 1);
    assert_eq!(o[0].1, Outcome::Pass, "{:?}", o[0].1);
    assert!(o[0].2.is_empty(), "{:?}", o[0].2);
}

/// A table extension of the table's own app that declares no field, only a `modify`, makes a key lookup give the table's ids
/// (PlatformMetadataColumnValues.al:784-815).
#[test]
fn issue_8051_a_modify_only_same_app_extension_gives_the_tables_ids() {
    let src = r#"
table 50100 "Modify Line"
{
    fields { field(1; "No."; Code[20]) { } field(2; Description; Text[30]) { Caption = 'Description'; } }
    keys { key(PK; "No.") { Clustered = true; } }
}

tableextension 50110 "Modify Line Ext" extends "Modify Line"
{
    fields { modify(Description) { Caption = 'Extended Description'; } }
}

codeunit 50120 T
{
    Subtype = Test;

    [Test]
    procedure ModifyOnly()
    var
        F: Record Field;
        Tbl: Record AllObj;
    begin
        Tbl.Get(Tbl."Object Type"::Table, 50100);
        F.Get(50100, 1);
        if F."App Package ID" <> Tbl."App Package ID" then
            Error('field 1 %1', F."App Package ID");
        F.Get(50100, 2);
        if F."App Runtime Package ID" <> Tbl."App Runtime Package ID" then
            Error('field 2 %1', F."App Runtime Package ID");
    end;
}"#;
    let o = run(&[(OWNER_ID, AppKind::Workspace, src)]);
    assert_eq!(o.len(), 1);
    assert_eq!(o[0].1, Outcome::Pass, "{:?}", o[0].1);
    assert!(o[0].2.is_empty(), "{:?}", o[0].2);
}

/// Every kind of declared field, and a field of a TableType = Temporary table, gives null ids on a Get of a dependency table no
/// extension of its own app extends (PlatformMetadataColumnValues.al:784-815).
#[test]
fn issue_8051_get_of_every_field_kind_and_a_temporary_table_is_null() {
    let tables = r#"
table 50100 "Kinds"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Pending; Integer) { ObsoleteState = Pending; ObsoleteReason = 'p'; }
        field(3; Removed; Integer) { ObsoleteState = Removed; ObsoleteReason = 'p'; }
        field(4; Flow; Integer) { FieldClass = FlowField; CalcFormula = count("Kinds"); }
        field(5; "Flow Filter"; Integer) { FieldClass = FlowFilter; }
        field(6; Disabled; Integer) { Enabled = false; }
        field(7; Picture; Blob) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

table 50101 "Temp Kinds"
{
    TableType = Temporary;
    fields { field(1; "No."; Code[20]) { } field(2; Description; Text[30]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}"#;
    let test = r#"
codeunit 50120 T
{
    Subtype = Test;

    [Test]
    procedure Kinds()
    var
        F: Record Field;
        No: Integer;
    begin
        for No := 1 to 7 do begin
            F.Get(50100, No);
            if not IsNullGuid(F."App Package ID") then
                Error('field %1', No);
        end;
        F.Get(50101, 2);
        if not IsNullGuid(F."App Package ID") then
            Error('temporary');
    end;
}"#;
    let o = run(&[
        (OWNER_ID, AppKind::Dependency, tables),
        (TEST_ID, AppKind::Workspace, test),
    ]);
    assert_eq!(o.len(), 1);
    assert_eq!(o[0].1, Outcome::Pass, "{:?}", o[0].1);
}

/// A key read with a package filter is measured as a FindFirst or FindSet only (PlatformMetadataColumnValues.al:827-842,
/// ObservePackageFilters). Next from a blank record is refused, as on main.
#[test]
fn issue_8051_key_lookup_with_a_package_filter_and_next_from_blank_is_refused() {
    let src = format!(
        "{PLAIN_TABLE}{}",
        r#"
codeunit 50120 T
{
    Subtype = Test;

    [Test]
    procedure NextFromBlank()
    var
        F: Record Field;
        Tbl: Record AllObj;
    begin
        Tbl.Get(Tbl."Object Type"::Table, 50100);
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 1);
        F.SetRange("App Package ID", Tbl."App Package ID");
        if F.Next() <> 0 then;
    end;
}"#
    );
    let o = run(&[(OWNER_ID, AppKind::Workspace, &src)]);
    assert_eq!(o.len(), 1);
    assert!(
        matches!(o[0].1, Outcome::NotRun(ref why) if why.contains("App Package ID")),
        "{:?}",
        o[0].1
    );
}

/// On a table that a modify-only extension of its own app extends, a key FindFirst with a package filter tests the filter
/// against the table's ids (PlatformMetadataColumnValues.al:784-815 and :844-868).
#[test]
fn issue_8051_package_filtered_key_read_on_a_modify_only_same_app_extension_tests_the_tables_ids() {
    let src = r#"
table 50100 "Modify Line"
{
    fields { field(1; "No."; Code[20]) { } field(2; Description; Text[30]) { Caption = 'Description'; } }
    keys { key(PK; "No.") { Clustered = true; } }
}

tableextension 50110 "Modify Line Ext" extends "Modify Line"
{
    fields { modify(Description) { Caption = 'Extended Description'; } }
}

codeunit 50120 T
{
    Subtype = Test;

    [Test]
    procedure PackageFilter()
    var
        F: Record Field;
        Tbl: Record AllObj;
    begin
        Tbl.Get(Tbl."Object Type"::Table, 50100);
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 1);
        F.SetRange("App Package ID", Tbl."App Package ID");
        if not F.FindFirst() then
            Error('= table');
    end;
}"#;
    let o = run(&[(OWNER_ID, AppKind::Workspace, src)]);
    assert_eq!(o.len(), 1);
    assert_eq!(o[0].1, Outcome::Pass, "{:?}", o[0].1);
    assert!(o[0].2.is_empty(), "{:?}", o[0].2);
}

/// The outcome of test `n` among `o`.
fn outcome<'a>(o: &'a [(String, Outcome, Vec<Assumption>)], n: &str) -> &'a (String, Outcome, Vec<Assumption>) {
    o.iter()
        .find(|(x, ..)| x == n)
        .unwrap_or_else(|| panic!("no test {n}: {o:?}"))
}

/// Asserts that each test of `names` is refused with a reason that contains `needle`.
fn assert_refused(o: &[(String, Outcome, Vec<Assumption>)], names: &[&str], needle: &str) {
    for n in names {
        let r = &outcome(o, n).1;
        assert!(matches!(r, Outcome::NotRun(why) if why.contains(needle)), "{n}: {r:?}");
    }
}

/// A read of one key in a form no probe measured, of a row a Get gives null ids and a scan the table's (a dependency table no
/// extension of its own app extends): a read of the package ids is refused. Measured are the forms of
/// PlatformMetadataColumnValues.al:569-605 and :753-772; here a third filter on another column than Class, No. in filter group 2
/// with TableNo in 0, the Class filter and group 2 with FindLast, Find(), Find('=<') and RecordRef.Get(RecordId) on an open and a
/// never opened ref. A filter on a range of No. keeps the table's ids (:569-605, main's result).
#[test]
fn issue_8051_unmeasured_key_read_forms_of_a_null_row_are_refused() {
    let test = r#"
codeunit 50120 T
{
    Subtype = Test;

    [Test]
    procedure ThirdFilter()
    var
        F: Record Field;
    begin
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 2);
        F.SetFilter(FieldName, '<>%1', 'Nothing');
        F.FindFirst();
        if IsNullGuid(F."App Package ID") then;
    end;

    [Test]
    procedure NoInGroup2()
    var
        F: Record Field;
    begin
        F.SetRange(TableNo, 50100);
        F.FilterGroup(2);
        F.SetRange("No.", 2);
        F.FilterGroup(0);
        F.FindFirst();
        if IsNullGuid(F."App Package ID") then;
    end;

    [Test]
    procedure ClassFindLast()
    var
        F: Record Field;
    begin
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 1);
        F.SetRange(Class, F.Class::Normal);
        F.FindLast();
        if IsNullGuid(F."App Package ID") then;
    end;

    [Test]
    procedure Group2FindLast()
    var
        F: Record Field;
    begin
        F.FilterGroup(2);
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 1);
        F.FilterGroup(0);
        F.FindLast();
        if IsNullGuid(F."App Runtime Package ID") then;
    end;

    [Test]
    procedure FindNoArgument()
    var
        F: Record Field;
    begin
        F.TableNo := 50100;
        F."No." := 1;
        F.Find();
        if IsNullGuid(F."App Package ID") then;
    end;

    [Test]
    procedure FindEqualsOrLess()
    var
        F: Record Field;
    begin
        F.TableNo := 50100;
        F."No." := 1;
        F.Find('=<');
        if IsNullGuid(F."App Package ID") then;
    end;

    [Test]
    procedure RecordRefGetOpen()
    var
        F: Record Field;
        RecRef: RecordRef;
    begin
        F.Get(50100, 1);
        RecRef.Open(Database::Field);
        RecRef.Get(F.RecordId);
        RecRef.SetTable(F);
        if IsNullGuid(F."App Package ID") then;
    end;

    [Test]
    procedure RecordRefGetUnopened()
    var
        F: Record Field;
        RecRef: RecordRef;
    begin
        F.Get(50100, 1);
        RecRef.Get(F.RecordId);
        RecRef.SetTable(F);
        if IsNullGuid(F."App Package ID") then;
    end;

    [Test]
    procedure RangeOfNo()
    var
        F: Record Field;
        Tbl: Record AllObj;
    begin
        Tbl.Get(Tbl."Object Type"::Table, 50100);
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 1, 2);
        F.FindFirst();
        if F."App Package ID" <> Tbl."App Package ID" then
            Error('range');
    end;
}"#;
    let o = run(&[
        (OWNER_ID, AppKind::Dependency, PLAIN_TABLE),
        (TEST_ID, AppKind::Workspace, test),
    ]);
    assert_refused(
        &o,
        &[
            "ThirdFilter",
            "NoInGroup2",
            "ClassFindLast",
            "Group2FindLast",
            "FindNoArgument",
            "FindEqualsOrLess",
            "RecordRefGetOpen",
            "RecordRefGetUnopened",
        ],
        "package ids of a Field row",
    );
    let range = outcome(&o, "RangeOfNo");
    assert_eq!(range.1, Outcome::Pass, "{:?}", range.1);
    assert!(range.2.is_empty(), "{:?}", range.2);
}

/// A table extension of the table's own app of a kind no probe measured: the measured kinds declare a field without an
/// ObsoleteState (PlatformMetadataColumnValues.al:934) or declare no field and only modify a Caption (:1011). Here one whose only
/// field is Removed, one with neither a field nor a modify, and one whose modify sets ToolTip: a Get's package ids are refused.
#[test]
fn issue_8051_unmeasured_same_app_extension_kinds_are_refused() {
    let src = r#"
table 50100 "Removed Line"
{
    fields { field(1; "No."; Code[20]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}

tableextension 50110 "Removed Line Ext" extends "Removed Line"
{
    fields { field(50110; Gone; Integer) { ObsoleteState = Removed; ObsoleteReason = 'r'; } }
}

table 50101 "Empty Line"
{
    fields { field(1; "No."; Code[20]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}

tableextension 50111 "Empty Line Ext" extends "Empty Line"
{
    procedure Nothing()
    begin
    end;
}

table 50102 "Tip Line"
{
    fields { field(1; "No."; Code[20]) { } field(2; Description; Text[30]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}

tableextension 50112 "Tip Line Ext" extends "Tip Line"
{
    fields { modify(Description) { ToolTip = 'A description.'; } }
}

codeunit 50120 T
{
    Subtype = Test;

    local procedure ReadIds(TableNo: Integer)
    var
        F: Record Field;
    begin
        F.Get(TableNo, 1);
        if IsNullGuid(F."App Package ID") then;
    end;

    [Test]
    procedure RemovedOnly()
    begin
        ReadIds(50100);
    end;

    [Test]
    procedure NeitherFieldNorModify()
    begin
        ReadIds(50101);
    end;

    [Test]
    procedure ModifyToolTip()
    begin
        ReadIds(50102);
    end;
}"#;
    let o = run(&[(OWNER_ID, AppKind::Workspace, src)]);
    assert_refused(
        &o,
        &["RemovedOnly", "NeitherFieldNorModify", "ModifyToolTip"],
        "extension of its own app",
    );
}

/// A key read with a package filter outside the measured shape (PlatformMetadataColumnValues.al:827-842, ObservePackageFilters:
/// FindFirst or FindSet, an = or <> filter on App Package ID of the table's id or the empty id, all in group 0) is refused: on a
/// null row a filter of another id; on a table a modify-only extension of its own app extends, FindLast, and a filter of two ids.
#[test]
fn issue_8051_unmeasured_package_filtered_key_reads_are_refused() {
    let null_row = format!(
        "{PLAIN_TABLE}{}",
        r#"
codeunit 50120 T
{
    Subtype = Test;

    [Test]
    procedure OtherId()
    var
        F: Record Field;
    begin
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 1);
        F.SetRange("App Package ID", CreateGuid());
        if F.FindFirst() then;
    end;
}"#
    );
    let o = run(&[(OWNER_ID, AppKind::Workspace, &null_row)]);
    assert_refused(&o, &["OtherId"], "App Package ID");
    let modify_only = r#"
table 50100 "Modify Line"
{
    fields { field(1; "No."; Code[20]) { } field(2; Description; Text[30]) { Caption = 'Description'; } }
    keys { key(PK; "No.") { Clustered = true; } }
}

tableextension 50110 "Modify Line Ext" extends "Modify Line"
{
    fields { modify(Description) { Caption = 'Extended Description'; } }
}

codeunit 50120 T
{
    Subtype = Test;

    [Test]
    procedure FindLastForm()
    var
        F: Record Field;
        Tbl: Record AllObj;
    begin
        Tbl.Get(Tbl."Object Type"::Table, 50100);
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 1);
        F.SetRange("App Package ID", Tbl."App Package ID");
        if F.FindLast() then;
    end;

    [Test]
    procedure TwoIds()
    var
        F: Record Field;
        Tbl: Record AllObj;
        EmptyId: Guid;
    begin
        Tbl.Get(Tbl."Object Type"::Table, 50100);
        F.SetRange(TableNo, 50100);
        F.SetRange("No.", 1);
        F.SetFilter("App Package ID", '%1|%2', Tbl."App Package ID", EmptyId);
        if F.FindFirst() then;
    end;
}"#;
    let o = run(&[(OWNER_ID, AppKind::Workspace, modify_only)]);
    assert_refused(&o, &["FindLastForm", "TwoIds"], "App Package ID");
}
