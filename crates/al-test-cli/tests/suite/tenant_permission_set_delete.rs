//! A Delete or DeleteAll of a stored Tenant Permission Set row with App ID zero that Tenant Permission or Tenant Permission Set
//! Rel. rows name is refused (crates/al-vm/src/tenant_set_delete.rs): BC deletes those rows with the set (corpus probe
//! TenantSetDeleteCascade.al), which the runner does not do yet. Every other delete of the table keeps main's result.

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
  var S: Record "Tenant Permission Set"; P: Record "Tenant Permission"; L: Record "Tenant Permission Set Rel.";
  begin
    exit(StrSubstNo('%1 %2 %3', S.Count(), P.Count(), L.Count()));
  end;
"#;

/// One test codeunit with `SETUP` and `tests`.
fn src(tests: &str) -> String {
    format!("codeunit 50000 T\n{{\n  Subtype = Test; TestPermissions = Disabled;\n{SETUP}\n{tests}\n}}\n")
}

/// Main's result: the set row goes, every other row stays.
const KEPT: &str = r#"
  [Test] procedure NoRowsNameIt() var S: Record "Tenant Permission Set"; begin
    Setup(); AddSet('EMPTY'); S.Get(S."App ID", 'EMPTY'); S.Delete(); Error(Left()); end;
  [Test] procedure DeleteAllOtherApp() var S: Record "Tenant Permission Set"; begin
    AddAppSet(); S.SetRange("Role ID", 'APP'); S.DeleteAll(); Error(Left()); end;
  [Test] procedure DeleteOtherApp() var S: Record "Tenant Permission Set"; begin
    AddAppSet(); S.Get('{7A1C0E55-3B2D-4E6F-8A90-1B2C3D4E5F60}', 'APP'); S.Delete(); Error(Left()); end;
  [Test] procedure TemporarySet() var S: Record "Tenant Permission Set" temporary; begin
    Setup(); S."Role ID" := 'GONE'; S.Insert(); S.Delete(); Error(Left()); end;
  [Test] procedure MissingSet() var S: Record "Tenant Permission Set"; Ok: Boolean; begin
    AddPerm('NOSET', 18); S."Role ID" := 'NOSET'; Ok := S.Delete(); Error('%1 %2', Ok, Left()); end;
  [Test] procedure RenamedInBuffer() var S: Record "Tenant Permission Set"; begin
    Setup(); S.Get(S."App ID", 'KEPT'); S."Role ID" := 'NOSET'; AddPerm('NOSET', 18); if S.Delete() then; Error(Left()); end;
"#;

/// Each refused shape alone: a refusal ends the rest of its codeunit.
const REFUSED: [(&str, &str); 7] = [
    (
        "ByDelete",
        r#"[Test] procedure ByDelete() var S: Record "Tenant Permission Set"; begin Setup(); S.Get(S."App ID", 'GONE'); S.Delete(); end;"#,
    ),
    (
        "ByDeleteTrue",
        r#"[Test] procedure ByDeleteTrue() var S: Record "Tenant Permission Set"; begin Setup(); S.Get(S."App ID", 'GONE'); S.Delete(true); end;"#,
    ),
    (
        "ByDeleteAll",
        r#"[Test] procedure ByDeleteAll() var S: Record "Tenant Permission Set"; begin Setup(); S.SetRange("Role ID", 'GONE'); S.DeleteAll(); end;"#,
    ),
    (
        "ByDeleteAllTrue",
        r#"[Test] procedure ByDeleteAllTrue() var S: Record "Tenant Permission Set"; begin Setup(); S.SetRange("Role ID", 'GONE'); S.DeleteAll(true); end;"#,
    ),
    (
        "ByRecordRefDelete",
        r#"[Test] procedure ByRecordRefDelete() var S: Record "Tenant Permission Set"; R: RecordRef; begin Setup(); S.Get(S."App ID", 'GONE'); R.GetTable(S); R.Delete(); end;"#,
    ),
    (
        "ByRecordRefDeleteAll",
        r#"[Test] procedure ByRecordRefDeleteAll() var S: Record "Tenant Permission Set"; R: RecordRef; begin Setup(); S.SetRange("Role ID", 'GONE'); R.GetTable(S); R.DeleteAll(); end;"#,
    ),
    (
        "RelationOnly",
        r#"[Test] procedure RelationOnly() var S: Record "Tenant Permission Set"; begin AddSet('A'); AddSet('B'); AddRel('B', 'A'); S.Get(S."App ID", 'A'); S.Delete(); end;"#,
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
fn other_tenant_permission_set_deletes_keep_mains_result() {
    let o = outcomes(&src(KEPT));
    assert_eq!(failed_with(&o, "NoRowsNameIt"), "2 3 2");
    assert_eq!(failed_with(&o, "DeleteAllOtherApp"), "0 1 0");
    assert_eq!(failed_with(&o, "DeleteOtherApp"), "0 1 0");
    assert_eq!(failed_with(&o, "TemporarySet"), "2 3 2");
    assert_eq!(failed_with(&o, "MissingSet"), "No 0 1 0");
    assert_eq!(failed_with(&o, "RenamedInBuffer"), "2 4 2");
}

#[test]
fn deleting_a_set_that_permission_rows_name_refuses() {
    for (name, test) in REFUSED {
        let o = outcomes(&src(test));
        match outcome(&o, name) {
            Outcome::NotRun(why) => assert!(why.contains("TenantSetDeleteCascade.al"), "{name}: {why}"),
            other => panic!("{name}: {other:?}"),
        }
    }
}
