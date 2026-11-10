//! al-runner issue 7524: a read of a table of a dependency app (the Base Application) under a permission set that
//! does not grant it. Pinned by permissions/permission-sets/AEDependencyTableReads.al (PR 1267; BC 27, 28, 29): with rows
//! in the table the read is denied (DB:ClientReadDenied); on an empty table typed reads raise nothing.
//!
//! A table of a dependency app that is empty in the store is known to be empty only when test data (the restored
//! company, ADR-020) is loaded: then the store holds the database's full content. Without test data BC's database may
//! hold demo rows the runtime does not have, so the read refuses.

use al_sema::workspace::AppKind;
use al_testkit::{AppSource, Built};
use std::sync::Arc;

const BASE: &str = r#"
table 50200 "DR Dep"
{
    fields
    {
        field(1; Code; Code[10]) { }
        field(2; Amount; Decimal) { }
    }
    keys { key(PK; Code) { Clustered = true; } }
}
"#;

const TEST: &str = r#"
dotnet
{
    assembly("Microsoft.Dynamics.Nav.PermissionTestHelper")
    {
        type("Microsoft.Dynamics.Nav.Runtime.PermissionTestHelper"; "PermissionTestHelper") { }
    }
}

table 50201 "DR Own"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

permissionset 50200 "DR Own Only"
{
    Assignable = true;
    Permissions = tabledata "DR Own" = R;
}

codeunit 50200 "DR Probe"
{
    local procedure Lower()
    var
        H: DotNet PermissionTestHelper;
    begin
        H := H.PermissionTestHelper();
        H.AddEffectivePermissionSet('DR OWN ONLY');
    end;

    procedure Seed()
    var
        Rec: Record "DR Dep";
    begin
        Rec.Code := 'A';
        Rec.Insert();
    end;

    procedure EmptyFindFirst(): Text
    var
        Rec: Record "DR Dep";
    begin
        Lower();
        if Rec.FindFirst() then;
        exit('ok');
    end;

    procedure RowsFindFirst(): Text
    var
        Rec: Record "DR Dep";
    begin
        Seed();
        Lower();
        if Rec.FindFirst() then;
        exit('ok');
    end;

    procedure RowsNext(): Text
    var
        Rec: Record "DR Dep";
    begin
        Seed();
        Lower();
        if Rec.Next() = 0 then;
        exit('ok');
    end;

    procedure RowsGetBySystemIdExisting(): Text
    var
        Rec: Record "DR Dep";
        Id: Guid;
    begin
        Seed();
        Rec.FindFirst();
        Id := Rec.SystemId;
        Clear(Rec);
        Lower();
        if Rec.GetBySystemId(Id) then;
        exit('ok');
    end;

    procedure RowsGetBySystemIdMissing(): Text
    var
        Rec: Record "DR Dep";
    begin
        Seed();
        Lower();
        if Rec.GetBySystemId(CreateGuid()) then;
        exit('ok');
    end;

    procedure RowsFilterNoMatch(): Text
    var
        Rec: Record "DR Dep";
    begin
        Seed();
        Lower();
        Rec.SetRange(Code, 'ZZ');
        if Rec.FindFirst() then;
        exit('ok');
    end;

    procedure RowsFindEqualBlankKey(): Text
    var
        Rec: Record "DR Dep";
    begin
        Seed();
        Lower();
        if Rec.Find('=') then;
        exit('ok');
    end;

    procedure RowsRecordRefGetExisting(): Text
    var
        Rec: Record "DR Dep";
        RecRef: RecordRef;
        Id: RecordId;
    begin
        Seed();
        Rec.FindFirst();
        Id := Rec.RecordId();
        Lower();
        RecRef.Open(Database::"DR Dep");
        RecRef.Get(Id);
        exit('ok');
    end;
}
"#;

const DENIED: &str = "Sorry, the current permissions prevented the action. (TableData 50200 DR Dep Read";

fn built(test_data: bool) -> Built {
    let apps = vec![
        AppSource {
            id: "base".into(),
            name: "Base Application".into(),
            dependencies: vec![],
            files: vec![("b.al".into(), BASE.into())],
        },
        AppSource {
            id: "test".into(),
            name: "Test".into(),
            dependencies: vec!["base".into()],
            files: vec![("t.al".into(), TEST.into())],
        },
    ];
    let kinds = vec![AppKind::Dependency, AppKind::Workspace];
    let (ws, l) = al_sema::lower::compile_apps_with(apps, kinds, &Default::default(), None, Default::default());
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    if !test_data {
        return Built::new(l);
    }
    use al_host::testdata::{TableError, TableProvider, TableRequest, TableRows, TestData};
    struct Empty;
    impl TableProvider for Empty {
        fn company(&self) -> &str {
            "C"
        }
        fn source(&self) -> String {
            "empty".into()
        }
        fn read(&self, _: &TableRequest) -> Result<Arc<TableRows>, TableError> {
            Err(TableError::Absent)
        }
    }
    let mut engine = al_vm::Engine::new(l.program.clone(), al_testkit::blocked(&l));
    Arc::get_mut(&mut engine).expect("not shared").test_data = Some(TestData::new(Arc::new(Empty)));
    Built {
        session: al_vm::Session::new(engine),
        lowered: l,
    }
}

fn denied(routine: &str, test_data: bool) {
    let r = built(test_data).run_in("DR Probe", routine);
    assert!(
        matches!(r, Err(al_builtins::native::Signal::Error(..))),
        "{routine}: {r:?}"
    );
    let msg = al_testkit::error_message(r);
    assert!(msg.starts_with(DENIED), "{routine}: {msg}");
}

fn ok(routine: &str, test_data: bool) {
    let r = built(test_data).run_in("DR Probe", routine);
    assert_eq!(al_testkit::text(r), "ok", "{routine}");
}

fn refused(routine: &str, test_data: bool) {
    let r = built(test_data).run_in("DR Probe", routine);
    assert!(
        matches!(r, Err(al_builtins::native::Signal::Unsupported(_))),
        "{routine}: {r:?}"
    );
}

#[test]
fn empty_dependency_table_with_test_data_is_known_empty() {
    ok("EmptyFindFirst", true);
}

#[test]
fn empty_dependency_table_without_test_data_refuses() {
    refused("EmptyFindFirst", false);
}

#[test]
fn rows_deny_each_pinned_shape() {
    denied("RowsFindFirst", false);
    denied("RowsNext", false);
    denied("RowsGetBySystemIdExisting", false);
    denied("RowsFilterNoMatch", false);
    denied("RowsRecordRefGetExisting", false);
}

#[test]
fn rows_pinned_ok_shapes() {
    ok("RowsGetBySystemIdMissing", false);
    ok("RowsFindEqualBlankKey", false);
}
