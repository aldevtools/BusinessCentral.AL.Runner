//! A Delete or DeleteAll of stored Tenant Permission Set rows while Tenant Permission holds rows is refused: BC deletes a set's
//! permissions with it, which this runtime does not follow (crates/al-vm/src/records/platform_rows.rs,
//! `tenant_permission_set_delete_check`; al-runner issue 7854). Without permission rows the delete runs as before.

use super::y48_platform_probes::outcomes;
use al_test_cli::report::Outcome;

const SRC: &str = r#"
codeunit 50000 T
{
  Subtype = Test; TestPermissions = Disabled;
  local procedure AddSet(RoleId: Code[20]) var S: Record "Tenant Permission Set"; begin S."Role ID" := RoleId; S.Name := RoleId; S.Insert(); end;
  local procedure AddPerm(RoleId: Code[20])
  var P: Record "Tenant Permission";
  begin
    P."Role ID" := RoleId; P."Object Type" := P."Object Type"::"Table Data"; P."Object ID" := 18; P.Insert();
  end;
  [Test] procedure DeleteWithPermissions() var S: Record "Tenant Permission Set"; begin
    AddSet('X ONE'); AddPerm('X ONE');
    S.Get(S."App ID", 'X ONE'); S.Delete(); end;
  [Test] procedure DeleteAllWithPermissions() var S: Record "Tenant Permission Set"; begin
    AddSet('X TWO'); AddPerm('X TWO');
    S.SetRange("Role ID", 'X TWO'); S.DeleteAll(); end;
  [Test] procedure DeleteAfterPermissions() var S: Record "Tenant Permission Set"; P: Record "Tenant Permission"; begin
    AddSet('X THREE'); AddPerm('X THREE');
    P.DeleteAll();
    S.Get(S."App ID", 'X THREE'); S.Delete();
    if S.Get(S."App ID", 'X THREE') then Error('still there'); end;
}
"#;

#[test]
fn deleting_a_tenant_permission_set_with_permission_rows_is_refused() {
    let o = outcomes(SRC);
    for name in ["DeleteWithPermissions", "DeleteAllWithPermissions"] {
        let out = &o.iter().find(|(n, _)| n == name).expect("test").1;
        assert!(
            matches!(out, Outcome::NotRun(why) if why.contains("Tenant Permission Set while Tenant Permission holds rows")),
            "{name}: {out:?}"
        );
    }
    let out = &o.iter().find(|(n, _)| n == "DeleteAfterPermissions").expect("test").1;
    assert!(matches!(out, Outcome::Pass), "DeleteAfterPermissions: {out:?}");
}
