//! A Delete or DeleteAll of a stored Tenant Permission Set row deletes its Tenant Permission rows (same App ID and Role ID),
//! under the assumption `tenant-permission-set-delete-cascade` (crates/al-vm/src/tenant_set_cascade.rs; al-runner issue 7854).
//! Another set's permissions, the Tenant Permission Set Rel. rows and a temporary set keep main's result: nothing else is deleted.

use super::y48_platform_probes::outcomes;
use al_test_cli::report::Outcome;

const SRC: &str = r#"
codeunit 50000 T
{
  Subtype = Test; TestPermissions = Disabled;
  local procedure AddSet(RoleId: Code[20]) var S: Record "Tenant Permission Set"; begin S."Role ID" := RoleId; S.Name := RoleId; S.Insert(); end;
  local procedure AddPerm(RoleId: Code[20]; TableId: Integer)
  var P: Record "Tenant Permission";
  begin
    P."Role ID" := RoleId; P."Object Type" := P."Object Type"::"Table Data"; P."Object ID" := TableId; P.Insert();
  end;
  local procedure AddRel(RoleId: Code[20]; Related: Code[20])
  var L: Record "Tenant Permission Set Rel.";
  begin
    L."Role ID" := RoleId; L."Related Role ID" := Related; L."Related Scope" := L."Related Scope"::Tenant; L.Insert();
  end;
  local procedure Setup()
  begin
    AddSet('GONE'); AddSet('KEPT'); AddPerm('GONE', 18); AddPerm('GONE', 23); AddPerm('KEPT', 18);
    AddRel('GONE', 'KEPT'); AddRel('KEPT', 'GONE');
  end;
  local procedure Left(): Text
  var P: Record "Tenant Permission"; L: Record "Tenant Permission Set Rel.";
  begin
    P.SetRange("Role ID", 'GONE'); L.SetRange("Role ID", 'GONE');
    exit(StrSubstNo('%1 %2 %3', P.Count(), L.Count(), CountKept()));
  end;
  local procedure CountKept(): Integer var P: Record "Tenant Permission"; begin P.SetRange("Role ID", 'KEPT'); exit(P.Count()); end;
  [Test] procedure ByDelete() var S: Record "Tenant Permission Set"; begin
    Setup(); S.Get(S."App ID", 'GONE'); S.Delete(); Error(Left()); end;
  [Test] procedure ByDeleteAll() var S: Record "Tenant Permission Set"; begin
    Setup(); S.SetRange("Role ID", 'GONE'); S.DeleteAll(); Error(Left()); end;
  [Test] procedure TemporarySet() var S: Record "Tenant Permission Set" temporary; begin
    Setup(); S."Role ID" := 'GONE'; S.Insert(); S.Delete(); Error(Left()); end;
}
"#;

fn failed_with(o: &[(String, Outcome)], name: &str) -> String {
    match &o.iter().find(|(n, _)| n == name).expect("test").1 {
        Outcome::Fail(m) => m.clone(),
        other => panic!("{name}: {other:?}"),
    }
}

#[test]
fn deleting_a_tenant_permission_set_deletes_its_permissions_only() {
    let o = outcomes(SRC);
    // Own permissions gone, the set's Rel. row kept, the other set's permission kept.
    assert_eq!(failed_with(&o, "ByDelete"), "0 1 1");
    assert_eq!(failed_with(&o, "ByDeleteAll"), "0 1 1");
    // A temporary set's delete leaves the stored permissions alone.
    assert_eq!(failed_with(&o, "TemporarySet"), "2 1 1");
}
