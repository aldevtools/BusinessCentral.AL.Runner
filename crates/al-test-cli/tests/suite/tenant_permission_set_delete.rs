//! A Delete or DeleteAll of a stored Tenant Permission Set row with App ID zero deletes its Tenant Permission rows and the
//! Tenant Permission Set Rel. rows from and to it, under the assumption `tenant-permission-set-delete-cascade`
//! (crates/al-vm/src/tenant_set_cascade.rs; corpus probe TenantSetDeleteCascade.al). A DeleteAll of a set whose App ID is not
//! zero deletes nothing else; a temporary set keeps main's result. The shapes the probe did not measure refuse: a delete event on
//! Tenant Permission, a relation of another kind, Delete of a set whose App ID is not zero.

use super::y48_platform_probes::outcomes;
use al_test_cli::report::Outcome;

const SETUP: &str = r#"
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
  var P: Record "Tenant Permission"; L: Record "Tenant Permission Set Rel."; K: Record "Tenant Permission"; M: Record "Tenant Permission Set Rel.";
  begin
    P.SetRange("Role ID", 'GONE'); L.SetRange("Role ID", 'GONE'); K.SetRange("Role ID", 'KEPT'); M.SetRange("Role ID", 'KEPT');
    exit(StrSubstNo('%1 %2 %3 %4', P.Count(), L.Count(), K.Count(), M.Count()));
  end;
"#;

/// One test codeunit with `SETUP` and `tests`, then `more` objects.
fn src(tests: &str, more: &str) -> String {
    format!("codeunit 50000 T\n{{\n  Subtype = Test; TestPermissions = Disabled;\n{SETUP}\n{tests}\n}}\n{more}")
}

const MEASURED: &str = r#"
  [Test] procedure ByDelete() var S: Record "Tenant Permission Set"; begin
    Setup(); S.Get(S."App ID", 'GONE'); S.Delete(); Error(Left()); end;
  [Test] procedure ByDeleteTrue() var S: Record "Tenant Permission Set"; begin
    Setup(); S.Get(S."App ID", 'GONE'); S.Delete(true); Error(Left()); end;
  [Test] procedure ByDeleteAll() var S: Record "Tenant Permission Set"; begin
    Setup(); S.SetRange("Role ID", 'GONE'); S.DeleteAll(); Error(Left()); end;
  [Test] procedure ByDeleteAllTrue() var S: Record "Tenant Permission Set"; begin
    Setup(); S.SetRange("Role ID", 'GONE'); S.DeleteAll(true); Error(Left()); end;
  [Test] procedure ByRecordRefDelete() var S: Record "Tenant Permission Set"; R: RecordRef; begin
    Setup(); S.Get(S."App ID", 'GONE'); R.GetTable(S); R.Delete(); Error(Left()); end;
  [Test] procedure ByRecordRefDeleteAll() var S: Record "Tenant Permission Set"; R: RecordRef; begin
    Setup(); S.SetRange("Role ID", 'GONE'); R.GetTable(S); R.DeleteAll(); Error(Left()); end;
  [Test] procedure ByDeleteAllOtherApp() var S: Record "Tenant Permission Set"; P: Record "Tenant Permission"; begin
    AddAppSet(); S.SetRange("Role ID", 'APP'); S.DeleteAll(); P.SetRange("Role ID", 'APP'); Error(Format(P.Count())); end;
  [Test] procedure TemporarySet() var S: Record "Tenant Permission Set" temporary; begin
    Setup(); S."Role ID" := 'GONE'; S.Insert(); S.Delete(); Error(Left()); end;
"#;

/// Each refused shape alone: a refusal ends the rest of its codeunit.
const REFUSED: [(&str, &str, &str); 3] = [
    (
        "OtherAppDelete",
        r#"  [Test] procedure OtherAppDelete() var S: Record "Tenant Permission Set"; begin
    AddAppSet(); S.Get('{7A1C0E55-3B2D-4E6F-8A90-1B2C3D4E5F60}', 'APP'); S.Delete(); end;"#,
        "",
    ),
    (
        "SystemScopeRel",
        r#"  [Test] procedure SystemScopeRel() var S: Record "Tenant Permission Set"; L: Record "Tenant Permission Set Rel."; begin
    Setup(); L."Role ID" := 'OTHER'; L."Related Role ID" := 'GONE'; L."Related Scope" := L."Related Scope"::System; L.Insert();
    S.SetRange("Role ID", 'GONE'); S.DeleteAll(); end;"#,
        "",
    ),
    (
        "PermDeleteSubscriber",
        r#"  [Test] procedure PermDeleteSubscriber() var S: Record "Tenant Permission Set"; Sub: Codeunit PermSub; begin
    Setup(); BindSubscription(Sub); S.SetRange("Role ID", 'GONE'); S.DeleteAll(); end;"#,
        r#"codeunit 50004 PermSub
{
  EventSubscriberInstance = Manual;
  [EventSubscriber(ObjectType::Table, Database::"Tenant Permission", 'OnAfterDeleteEvent', '', false, false)]
  local procedure OnAfterDeletePerm(var Rec: Record "Tenant Permission"; RunTrigger: Boolean) begin end;
}"#,
    ),
];

fn outcome<'a>(o: &'a [(String, Outcome)], name: &str) -> &'a Outcome {
    &o.iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("{name} in {o:?}"))
        .1
}

fn failed_with(o: &[(String, Outcome)], name: &str) -> String {
    match outcome(o, name) {
        Outcome::Fail(m) => m.clone(),
        other => panic!("{name}: {other:?}"),
    }
}

#[test]
fn deleting_a_tenant_permission_set_deletes_its_permissions_and_relations() {
    let o = outcomes(&src(MEASURED, ""));
    // Measured by TenantSetDeleteCascade.al on every delete shape: the set's permissions and both relations go, the other
    // set's permission stays.
    for name in [
        "ByDelete",
        "ByDeleteTrue",
        "ByDeleteAll",
        "ByDeleteAllTrue",
        "ByRecordRefDelete",
        "ByRecordRefDeleteAll",
    ] {
        assert_eq!(failed_with(&o, name), "0 0 1 0", "{name}");
    }
    // A DeleteAll of a set whose App ID is not zero leaves its permission (TSDC_DeleteAllOtherApp).
    assert_eq!(failed_with(&o, "ByDeleteAllOtherApp"), "1");
    // A temporary set's delete leaves the stored rows alone (main's result).
    assert_eq!(failed_with(&o, "TemporarySet"), "2 1 1 1");
}

#[test]
fn shapes_the_probe_did_not_measure_refuse() {
    for (name, test, more) in REFUSED {
        let o = outcomes(&src(test, more));
        match outcome(&o, name) {
            Outcome::NotRun(why) => assert!(why.contains("Tenant Permission Set"), "{name}: {why}"),
            other => panic!("{name}: {other:?}"),
        }
    }
}
