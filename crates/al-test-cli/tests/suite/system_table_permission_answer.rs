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
permissionset 50101 "CO RIMD IND" { Assignable = true; Permissions = tabledata Company = rimd, codeunit T = X; }
permissionset 50102 "CO R" { Assignable = true; Permissions = tabledata Company = R, codeunit T = X; }
permissionset 50103 "CO RIM" { Assignable = true; Permissions = tabledata Company = RIM, codeunit T = X; }
permissionset 50104 "TM RIM" { Assignable = true; Permissions = tabledata "Tenant Media" = RIM, codeunit T = X; }
codeunit 50000 T { Subtype = Test; TestPermissions = Disabled;
  local procedure Answer(TableId: Integer): Text begin exit(AnswerUnder('NARROW', TableId)); end;
  local procedure AnswerUnder(Role: Code[20]; TableId: Integer): Text var H: DotNet PermissionTestHelper; R: RecordRef; begin
    H := H.PermissionTestHelper(); H.AddEffectivePermissionSet(Role);
    R.Open(TableId); exit(Format(R.ReadPermission()) + '/' + Format(R.WritePermission())); end;
  [Test] procedure Company() begin Error(Answer(Database::Company)); end;
  [Test] procedure TenantMedia() begin Error(Answer(Database::"Tenant Media")); end;
  [Test] procedure CompanyIndirectAll() begin Error(AnswerUnder('CO RIMD IND', Database::Company)); end;
  [Test] procedure CompanyReadOnly() begin Error(AnswerUnder('CO R', Database::Company)); end;
  [Test] procedure CompanyRim() begin Error(AnswerUnder('CO RIM', Database::Company)); end;
  [Test] procedure TenantMediaRim() begin Error(AnswerUnder('TM RIM', Database::"Tenant Media")); end;
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
    // Levels the pins did not measure keep main's refusal.
    for name in ["CompanyIndirectAll", "CompanyReadOnly", "CompanyRim", "TenantMediaRim"] {
        match get(name) {
            Outcome::NotRun(why) => assert!(
                why.contains("on a system table under sets that do not grant it"),
                "{name}: {why}"
            ),
            other => panic!("{name}: {other:?}"),
        }
    }
    match get("OtherSystemTable") {
        Outcome::NotRun(why) => assert!(
            why.contains("on a system table under sets that do not grant it"),
            "{why}"
        ),
        other => panic!("{other:?}"),
    }
}
