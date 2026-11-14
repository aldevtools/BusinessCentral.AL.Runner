//! `ReadPermission` and `WritePermission` of a system table under a set that does not name it: false for Company and Tenant
//! Media (metadata/Y19PermSystemTables.al:36-40, :58-63, crates/al-vm/src/permissions.rs `permission_query`). Every other system
//! table keeps main's refusal (al-runner issue 7819).

use super::y48_platform_probes::outcomes;
use al_test_cli::report::Outcome;

const SRC: &str = r#"
dotnet { assembly("Microsoft.Dynamics.Nav.PermissionTestHelper") {
  type("Microsoft.Dynamics.Nav.Runtime.PermissionTestHelper"; "PermissionTestHelper") { } } }
table 50100 "Narrow Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
permissionset 50100 "NARROW" { Assignable = true; Permissions = tabledata "Narrow Row" = R, codeunit T = X; }
codeunit 50000 T { Subtype = Test; TestPermissions = Disabled;
  local procedure Answer(TableId: Integer): Text var H: DotNet PermissionTestHelper; R: RecordRef; begin
    H := H.PermissionTestHelper(); H.AddEffectivePermissionSet('NARROW');
    R.Open(TableId); exit(Format(R.ReadPermission()) + '/' + Format(R.WritePermission())); end;
  [Test] procedure Company() begin Error(Answer(Database::Company)); end;
  [Test] procedure TenantMedia() begin Error(Answer(Database::"Tenant Media")); end;
  [Test] procedure OtherSystemTable() begin Error(Answer(Database::"Tenant Permission Set")); end;
}
"#;

#[test]
fn company_and_tenant_media_answer_false_other_system_tables_refuse() {
    let o = outcomes(SRC);
    let get = |name: &str| {
        &o.iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("{name} in {o:?}"))
            .1
    };
    assert_eq!(*get("Company"), Outcome::Fail("No/No".into()));
    assert_eq!(*get("TenantMedia"), Outcome::Fail("No/No".into()));
    match get("OtherSystemTable") {
        Outcome::NotRun(why) => assert!(
            why.contains("on a system table under sets that do not grant it"),
            "{why}"
        ),
        other => panic!("{other:?}"),
    }
}
