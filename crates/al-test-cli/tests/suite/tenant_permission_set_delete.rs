//! The delete of a stored Tenant Permission Set row with App ID zero that Tenant Permission or Tenant Permission Set Rel. rows
//! name (crates/al-vm/src/tenant_set_delete.rs): BC deletes those rows with the set (corpus probe TenantSetDeleteCascade.al,
//! corpus pull request 1340, through the assumption `tenant-set-delete-cascade`). Shapes the probe does not measure are
//! refused; a delete no row names keeps main's result.

use super::y48_platform_probes::{outcomes, outcomes_with_hits};
use al_builtins::assumptions::Assumption;
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
  [Test] procedure OtherAppPermOnly() var S: Record "Tenant Permission Set"; P: Record "Tenant Permission"; begin
    AddSet('X'); P."App ID" := '{7A1C0E55-3B2D-4E6F-8A90-1B2C3D4E5F60}'; P."Role ID" := 'X'; P."Object ID" := 18; P.Insert();
    S.Get(S."App ID", 'X'); S.Delete(); Error(Left()); end;
  [Test] procedure TruncateNoRows() var S: Record "Tenant Permission Set"; begin
    AddSet('EMPTY'); S.Truncate(); Error(Left()); end;
  [Test] procedure RenamedInBuffer() var S: Record "Tenant Permission Set"; begin
    Setup(); S.Get(S."App ID", 'KEPT'); S."Role ID" := 'NOSET'; AddPerm('NOSET', 18); if S.Delete() then; Error(Left()); end;
"#;

/// The probe's delete forms (TenantSetDeleteCascade.al:111-197) and a set that only a Rel. row of another set names: the set's
/// rows go with it.
const CASCADE: &str = r#"
  [Test] procedure ByDelete() var S: Record "Tenant Permission Set"; begin Setup(); S.Get(S."App ID", 'GONE'); S.Delete(); Error(Left()); end;
  [Test] procedure ByDeleteTrue() var S: Record "Tenant Permission Set"; begin Setup(); S.Get(S."App ID", 'GONE'); S.Delete(true); Error(Left()); end;
  [Test] procedure ByDeleteAll() var S: Record "Tenant Permission Set"; begin Setup(); S.SetRange("Role ID", 'GONE'); S.DeleteAll(); Error(Left()); end;
  [Test] procedure ByDeleteAllTrue() var S: Record "Tenant Permission Set"; begin Setup(); S.SetRange("Role ID", 'GONE'); S.DeleteAll(true); Error(Left()); end;
  [Test] procedure ByRecordRefDelete() var S: Record "Tenant Permission Set"; R: RecordRef; begin Setup(); S.Get(S."App ID", 'GONE'); R.GetTable(S); R.Delete(); Error(Left()); end;
  [Test] procedure ByRecordRefDeleteAll() var S: Record "Tenant Permission Set"; R: RecordRef; begin Setup(); S.SetRange("Role ID", 'GONE'); R.GetTable(S); R.DeleteAll(); Error(Left()); end;
  [Test] procedure RelationOnly() var S: Record "Tenant Permission Set"; begin AddSet('A'); AddSet('B'); AddRel('B', 'A'); S.Get(S."App ID", 'A'); S.Delete(); Error(Left()); end;
"#;

/// A permission set that grants every right on the three tables, for a session that is restricted all the same.
const RESTRICTED: &str = r#"
dotnet { assembly("Microsoft.Dynamics.Nav.PermissionTestHelper") {
  type("Microsoft.Dynamics.Nav.Runtime.PermissionTestHelper"; "PermissionTestHelper") { } } }
permissionset 50100 "TSD WRITE" { Assignable = true;
  Permissions = tabledata "Tenant Permission Set" = RIMD, tabledata "Tenant Permission" = RIMD,
    tabledata "Tenant Permission Set Rel." = RIMD, codeunit T = X; }
"#;

/// Shapes the probe does not measure, each alone: a refusal ends the rest of its codeunit. Objects besides the test codeunit
/// go first.
const REFUSED: [(&str, &str, &str); 13] = [
    (
        "PermDatabaseTrigger",
        r#"codeunit 50001 GtSub { [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
          local procedure OnSetup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
          begin OnDatabaseDelete := TableId = Database::"Tenant Permission"; end; }"#,
        r#"[Test] procedure PermDatabaseTrigger() var S: Record "Tenant Permission Set"; begin
          Setup(); S.Get(S."App ID", 'GONE'); S.Delete(); end;"#,
    ),
    (
        "RestrictedSession",
        RESTRICTED,
        r#"[Test] procedure RestrictedSession() var S: Record "Tenant Permission Set"; H: DotNet PermissionTestHelper; begin
          Setup(); H := H.PermissionTestHelper(); H.AddEffectivePermissionSet('TSD WRITE'); S.Get(S."App ID", 'GONE'); S.Delete(); end;"#,
    ),
    (
        "OtherObjectType",
        "",
        r#"[Test] procedure OtherObjectType() var S: Record "Tenant Permission Set"; P: Record "Tenant Permission"; begin
          AddSet('GONE'); P."Role ID" := 'GONE'; P."Object Type" := P."Object Type"::Codeunit; P."Object ID" := 1; P.Insert();
          S.Get(S."App ID", 'GONE'); S.Delete(); end;"#,
    ),
    (
        "ExcludeGrant",
        "",
        r#"[Test] procedure ExcludeGrant() var S: Record "Tenant Permission Set"; P: Record "Tenant Permission"; begin
          AddSet('GONE'); P."Role ID" := 'GONE'; P."Object ID" := 18; P.Type := P.Type::Exclude; P.Insert();
          S.Get(S."App ID", 'GONE'); S.Delete(); end;"#,
    ),
    (
        "PermOfOtherApp",
        "",
        r#"[Test] procedure PermOfOtherApp() var S: Record "Tenant Permission Set"; P: Record "Tenant Permission"; begin
          Setup(); P."App ID" := '{7A1C0E55-3B2D-4E6F-8A90-1B2C3D4E5F60}'; P."Role ID" := 'GONE'; P."Object ID" := 27; P.Insert();
          S.Get(S."App ID", 'GONE'); S.Delete(); end;"#,
    ),
    (
        "RelToSystemScope",
        "",
        r#"[Test] procedure RelToSystemScope() var S: Record "Tenant Permission Set"; L: Record "Tenant Permission Set Rel."; begin
          Setup(); L."Role ID" := 'GONE'; L."Related Role ID" := 'D365 BASIC'; L."Related Scope" := L."Related Scope"::System; L.Insert();
          S.Get(S."App ID", 'GONE'); S.Delete(); end;"#,
    ),
    (
        "RelToNoSet",
        "",
        r#"[Test] procedure RelToNoSet() var S: Record "Tenant Permission Set"; begin
          Setup(); AddRel('GONE', 'NOSET'); S.Get(S."App ID", 'GONE'); S.Delete(); end;"#,
    ),
    (
        "RelToItself",
        "",
        r#"[Test] procedure RelToItself() var S: Record "Tenant Permission Set"; begin
          Setup(); AddRel('GONE', 'GONE'); S.Get(S."App ID", 'GONE'); S.Delete(); end;"#,
    ),
    (
        "TwoSetsInOneDeleteAll",
        "",
        r#"[Test] procedure TwoSetsInOneDeleteAll() var S: Record "Tenant Permission Set"; begin
          Setup(); S.DeleteAll(); end;"#,
    ),
    (
        "TruncateSets",
        "",
        r#"[Test] procedure TruncateSets() var S: Record "Tenant Permission Set"; begin
          Setup(); S.Truncate(); end;"#,
    ),
    (
        "PermSubscriber",
        r#"codeunit 50001 PermSub { [EventSubscriber(ObjectType::Table, Database::"Tenant Permission", 'OnAfterDeleteEvent', '', false, false)]
          local procedure OnAfterDelete(var Rec: Record "Tenant Permission"; RunTrigger: Boolean) begin end; }"#,
        r#"[Test] procedure PermSubscriber() var S: Record "Tenant Permission Set"; begin
          Setup(); S.Get(S."App ID", 'GONE'); S.Delete(); end;"#,
    ),
    (
        "SetSubscriber",
        r#"codeunit 50001 SetSub { [EventSubscriber(ObjectType::Table, Database::"Tenant Permission Set", 'OnBeforeDeleteEvent', '', false, false)]
          local procedure OnBeforeDelete(var Rec: Record "Tenant Permission Set"; RunTrigger: Boolean) begin end; }"#,
        r#"[Test] procedure SetSubscriber() var S: Record "Tenant Permission Set"; begin
          Setup(); S.Get(S."App ID", 'GONE'); S.Delete(); end;"#,
    ),
    (
        "RelSubscriber",
        r#"codeunit 50001 RelSub { [EventSubscriber(ObjectType::Table, Database::"Tenant Permission Set Rel.", 'OnAfterDeleteEvent', '', false, false)]
          local procedure OnAfterDelete(var Rec: Record "Tenant Permission Set Rel."; RunTrigger: Boolean) begin end; }"#,
        r#"[Test] procedure RelSubscriber() var S: Record "Tenant Permission Set"; begin
          Setup(); S.Get(S."App ID", 'GONE'); S.DeleteAll(); end;"#,
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
    assert_eq!(failed_with(&o, "TruncateNoRows"), "0 0 0");
    assert_eq!(failed_with(&o, "OtherAppPermOnly"), "0 1 0");
}

/// The probe's measured result: `sets=1 perms gone=0 perms kept=1 rel of gone=0 rel of kept=0` (TenantSetDeleteCascade.al:121),
/// here counted over the whole tables; each pass goes through the assumption.
#[test]
fn deleting_a_set_takes_its_permission_rows_with_it() {
    let o = outcomes_with_hits(&src(CASCADE), "T");
    let flagged = |name: &str| {
        let (_, out, hits) = o
            .iter()
            .find(|(n, _, _)| n == name)
            .unwrap_or_else(|| panic!("{name} in {o:?}"));
        assert!(hits.contains(&Assumption::TenantSetDeleteCascade), "{name}: {hits:?}");
        match out {
            Outcome::Fail(m) => m.clone(),
            other => panic!("{name}: {other:?}"),
        }
    };
    for name in [
        "ByDelete",
        "ByDeleteTrue",
        "ByDeleteAll",
        "ByDeleteAllTrue",
        "ByRecordRefDelete",
        "ByRecordRefDeleteAll",
    ] {
        assert_eq!(flagged(name), "1 1 0", "{name}");
    }
    assert_eq!(flagged("RelationOnly"), "1 0 0");
}

/// An error after the delete rolls the cascade back with the set row: a test codeunit of its own, as it commits.
#[test]
fn a_rollback_puts_the_rows_back() {
    let o = outcomes(&src(r#"
  [Test] procedure RolledBack() begin Setup(); Commit(); asserterror DeleteAndFail(); Error(Left()); end;
  local procedure DeleteAndFail() var S: Record "Tenant Permission Set"; begin S.Get(S."App ID", 'GONE'); S.Delete(); Error('x'); end;
"#));
    assert_eq!(failed_with(&o, "RolledBack"), "2 3 2");
}

/// A delete no row names keeps main's result and does not go through the assumption.
#[test]
fn deletes_no_row_names_are_not_flagged() {
    let o = outcomes_with_hits(&src(KEPT), "T");
    for (name, _, hits) in &o {
        assert!(!hits.contains(&Assumption::TenantSetDeleteCascade), "{name}: {hits:?}");
    }
}

#[test]
fn shapes_the_probe_does_not_measure_refuse() {
    for (name, objects, test) in REFUSED {
        let o = outcomes(&format!("{objects}\n{}", src(test)));
        match outcome(&o, name) {
            Outcome::NotRun(why) => {
                assert!(why.contains("TenantSetDeleteCascade.al"), "{name}: {why}");
                assert!(why.contains(reason(name)), "{name}: {why}");
            }
            other => panic!("{name}: {other:?}"),
        }
    }
}

/// The part of the refusal that names the fact `name` breaks.
fn reason(name: &str) -> &'static str {
    match name {
        "RestrictedSession" => "permission sets do not grant everything",
        "OtherObjectType" | "ExcludeGrant" => "not an Include grant of Table Data",
        "PermOfOtherApp" => "another App ID",
        "RelToSystemScope" => "not an Include relation between tenant sets",
        "RelToNoSet" | "RelToItself" => "the set itself or no stored set",
        "TwoSetsInOneDeleteAll" => "more than one such set",
        "TruncateSets" => "by a Truncate",
        "PermSubscriber" | "PermDatabaseTrigger" => "Tenant Permission row raises events",
        "SetSubscriber" => "Tenant Permission Set row raises events",
        "RelSubscriber" => "Tenant Permission Set Rel. row raises events",
        other => panic!("{other}"),
    }
}

/// The probe file of corpus pull request 1340, unchanged, with the corpus's Probe Observer: every test passes through the
/// assumption except the one with a bound Tenant Permission subscriber, whose event arguments the probe does not log.
#[test]
fn the_probe_passes_flagged() {
    const PROBE: &str = include_str!("../fixtures/tenant-set-delete-cascade.al");
    const OBSERVER: &str = include_str!("../../../../corpus/tests/al-language/_fixtures/ProbeObserver.Codeunit.al");
    let o = outcomes_with_hits(&format!("{PROBE}\n{OBSERVER}"), "XP TSDC Tests");
    assert_eq!(o.len(), 8, "{o:?}");
    for (name, out, hits) in &o {
        if name == "TSDC_DeleteAllPermSubscriber" {
            assert!(
                matches!(out, Outcome::NotRun(why) if why.contains("TenantSetDeleteCascade.al")),
                "{name}: {out:?}"
            );
            continue;
        }
        assert_eq!(*out, Outcome::Pass, "{name}");
        // A set with another App ID takes nothing with it (main's result).
        let through = name != "TSDC_DeleteAllOtherApp";
        assert_eq!(
            hits.contains(&Assumption::TenantSetDeleteCascade),
            through,
            "{name}: {hits:?}"
        );
    }
}

/// The reporter's repro of al-runner issue 7854 (comment of 2026-11-13), unchanged.
#[test]
fn reporters_repro_passes_flagged() {
    const REPRO: &str = r#"codeunit 50400 "Tenant Set Delete Tests"
{
    Subtype = Test;

    [Test]
    procedure DeleteSetRemovesItsPermissions()
    var
        TenantPermissionSet: Record "Tenant Permission Set";
        TenantPermission: Record "Tenant Permission";
        ZeroGuid: Guid;
    begin
        TenantPermissionSet."App ID" := ZeroGuid;
        TenantPermissionSet."Role ID" := 'REPRO SET';
        TenantPermissionSet.Name := 'Repro set';
        TenantPermissionSet.Insert();

        TenantPermission."App ID" := ZeroGuid;
        TenantPermission."Role ID" := 'REPRO SET';
        TenantPermission."Object Type" := TenantPermission."Object Type"::"Table Data";
        TenantPermission."Object ID" := 18;
        TenantPermission."Read Permission" := TenantPermission."Read Permission"::Yes;
        TenantPermission.Insert();

        TenantPermissionSet.Delete();

        // BC deletes the set's Tenant Permission rows with it
        TenantPermission.SetRange("App ID", ZeroGuid);
        TenantPermission.SetRange("Role ID", 'REPRO SET');
        if not TenantPermission.IsEmpty() then
            Error('Tenant Permission rows of the deleted set are still there');
    end;
}
"#;
    let o = outcomes_with_hits(REPRO, "Tenant Set Delete Tests");
    assert_eq!(o.len(), 1);
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
    assert!(o[0].2.contains(&Assumption::TenantSetDeleteCascade), "{o:?}");
}
