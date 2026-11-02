//! al-runner issue 6716: a permission set that names a table BC 29 ships twice under one id, the Base Application's
//! `ObsoleteState = Moved` copy and its Business Foundation target. The moved copy is not in the runtime app
//! (platform/lt-aw-fresh-tenant/AwTests.al:426-442), and a permission row is keyed on (object type, id), so the name
//! names one table and the set composes in full.

use al_sema::workspace::AppKind;
use al_testkit::{AppSource, Built};

const FOUNDATION: &str = r#"
table 231 "Reason Code"
{
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}
"#;

const BASE: &str = r#"
dotnet
{
    assembly("Microsoft.Dynamics.Nav.PermissionTestHelper")
    {
        type("Microsoft.Dynamics.Nav.Runtime.PermissionTestHelper"; "PermissionTestHelper") { }
    }
}

table 231 "Reason Code"
{
    ObsoleteState = Moved;
    MovedTo = 'f3552374-a1f2-4356-848e-196002525837';
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}

table 50100 "PM Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

permissionset 50100 "PM Moved"
{
    Assignable = true;
    Permissions = tabledata "Reason Code" = R,
                  tabledata "PM Row" = R;
}

"#;

const TEST: &str = r#"
codeunit 50100 "PM Probe"
{
    procedure InsertUnderMovedSet(): Text
    var
        H: DotNet PermissionTestHelper;
        Row: Record "PM Row";
    begin
        H := H.PermissionTestHelper();
        H.AddEffectivePermissionSet('PM MOVED');
        Row.Id := 1;
        if Row.Insert() then;
        exit('inserted');
    end;
}
"#;

fn apps() -> Vec<AppSource> {
    vec![
        AppSource {
            id: "foundation".into(),
            name: "Business Foundation".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), FOUNDATION.into())],
        },
        AppSource {
            id: "base".into(),
            name: "Base Application".into(),
            dependencies: vec!["foundation".into()],
            files: vec![("b.al".into(), BASE.into())],
        },
        AppSource {
            id: "test".into(),
            name: "Test".into(),
            dependencies: vec!["base".into()],
            files: vec![("t.al".into(), TEST.into())],
        },
    ]
}

/// The set composes in full, so the Insert it does not grant raises BC's permission error (TestMockBasics.al:72-73)
/// instead of being refused as "not fully composed".
#[test]
fn a_set_naming_a_moved_table_and_its_target_composes() {
    // Microsoft's apps are dependencies: the AL compiler vetted them, so the shared id is not reported.
    let kinds = vec![AppKind::Dependency, AppKind::Dependency, AppKind::Workspace];
    let (ws, l) = al_sema::lower::compile_apps_with(apps(), kinds, &Default::default(), None, Default::default());
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let mut b = Built::new(l);
    let r = b.run_in("PM Probe", "InsertUnderMovedSet");
    let msg = al_testkit::error_message(r);
    assert!(
        msg.starts_with("Sorry, the current permissions prevented the action. (TableData 50100 PM Row Insert"),
        "{msg}"
    );
}
