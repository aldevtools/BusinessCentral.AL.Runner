//! `TenantPermissionSet.DeleteAll()` with one filter, on Role ID, deletes the Tenant Permission rows of each deleted set whose
//! App ID is zero (same App ID and Role ID), under the assumption `tenant-permission-set-delete-cascade`
//! (crates/al-vm/src/tenant_set_cascade.rs; al-runner issue 7854). Every other shape keeps main's result, which deletes only the
//! set rows: Delete, DeleteAll(true), RecordRef.DeleteAll, another filter, a set whose App ID is not zero, a temporary set.

use super::y48_platform_probes::outcomes;
use al_test_cli::report::Outcome;

const SRC: &str = r#"
codeunit 50000 T
{
  Subtype = Test; TestPermissions = Disabled;
  local procedure AddSet(RoleId: Code[20]) var S: Record "Tenant Permission Set"; begin S."Role ID" := RoleId; S.Name := RoleId; S.Insert(); end;
  local procedure AddAppSet() var S: Record "Tenant Permission Set"; P: Record "Tenant Permission";
  begin
    S."App ID" := '{7A1C0E55-3B2D-4E6F-8A90-1B2C3D4E5F60}'; S."Role ID" := 'APP'; S.Insert();
    P."App ID" := S."App ID"; P."Role ID" := 'APP'; P."Object Type" := P."Object Type"::"Table Data"; P."Object ID" := 18; P.Insert();
  end;
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
  [Test] procedure ByDeleteAllTrue() var S: Record "Tenant Permission Set"; begin
    Setup(); S.SetRange("Role ID", 'GONE'); S.DeleteAll(true); Error(Left()); end;
  [Test] procedure ByRecordRefDeleteAll() var S: Record "Tenant Permission Set"; R: RecordRef; begin
    Setup(); S.SetRange("Role ID", 'GONE'); R.GetTable(S); R.DeleteAll(); Error(Left()); end;
  [Test] procedure ByDeleteAllTwoFilters() var S: Record "Tenant Permission Set"; begin
    Setup(); S.SetRange("Role ID", 'GONE'); S.SetRange(Name, 'GONE'); S.DeleteAll(); Error(Left()); end;
  [Test] procedure ByDeleteAllOtherApp() var S: Record "Tenant Permission Set"; P: Record "Tenant Permission"; begin
    AddAppSet(); S.SetRange("Role ID", 'APP'); S.DeleteAll(); P.SetRange("Role ID", 'APP'); Error(Format(P.Count())); end;
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
    // DeleteAll filtered on Role ID: own permissions gone, the set's Rel. row kept, the other set's permission kept.
    assert_eq!(failed_with(&o, "ByDeleteAll"), "0 1 1");
    // The shapes no evidence covers keep main's result: the permissions stay.
    for name in [
        "ByDelete",
        "ByDeleteAllTrue",
        "ByRecordRefDeleteAll",
        "ByDeleteAllTwoFilters",
    ] {
        assert_eq!(failed_with(&o, name), "2 1 1", "{name}");
    }
    assert_eq!(failed_with(&o, "ByDeleteAllOtherApp"), "1");
    // A temporary set's delete leaves the stored permissions alone.
    assert_eq!(failed_with(&o, "TemporarySet"), "2 1 1");
}
