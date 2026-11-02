//! The Expanded Permission rows of tenant permission sets (crates/al-vm/src/platform/tenant_compose.rs). Each test names the
//! corpus test of permissions/lt-y67-perm-compose/Y67PermCompose.al that pins it.

use super::y48_platform_probes::outcomes;
use al_test_cli::report::Outcome;

const HELPERS: &str = r#"
  local procedure AddSet(RoleId: Code[20]) var S: Record "Tenant Permission Set"; begin S."Role ID" := RoleId; S.Name := RoleId; S.Insert(); end;
  local procedure AddPerm(RoleId: Code[20]; TableId: Integer; R: Option; I: Option; M: Option; PermType: Option)
  var P: Record "Tenant Permission";
  begin
    P."Role ID" := RoleId; P."Object Type" := P."Object Type"::"Table Data"; P."Object ID" := TableId;
    P."Read Permission" := R; P."Insert Permission" := I; P."Modify Permission" := M; P.Type := PermType; P.Insert();
  end;
  local procedure AddRel(RoleId: Code[20]; Related: Code[20]; RelType: Option)
  var L: Record "Tenant Permission Set Rel.";
  begin
    L."Role ID" := RoleId; L."Related Role ID" := Related; L.Type := RelType; L."Related Scope" := L."Related Scope"::Tenant; L.Insert();
  end;
  local procedure Rows(RoleId: Code[20]): Text
  var E: Record "Expanded Permission"; T: Text;
  begin
    E.SetRange("Role ID", RoleId);
    if E.FindSet() then
      repeat
        T += Format(E."Object ID") + ':R=' + Format(E."Read Permission") + ',I=' + Format(E."Insert Permission") + ',M=' + Format(E."Modify Permission") + ',D=' + Format(E."Delete Permission") + ',' + Format(E.Scope) + ' ';
      until E.Next() = 0;
    if T = '' then exit('no rows');
    exit(T);
  end;
"#;

fn run(body: &str) -> Vec<(String, Outcome)> {
    outcomes(&format!(
        "codeunit 50000 T {{ Subtype = Test; TestPermissions = Disabled;\n{HELPERS}\n{body}\n}}"
    ))
}

fn failed_with(o: &[(String, Outcome)], name: &str) -> String {
    match &o.iter().find(|(n, _)| n == name).expect("test").1 {
        Outcome::Fail(m) => m.clone(),
        other => panic!("{name}: {other:?}"),
    }
}

/// Y67PermCompose.al:88 Compose_OwnAndIncluded, :110 Compose_ExcludedSet, :139 Compose_ExcludePermissionRow, :161
/// Compose_OverlapAndDiamond, :191 Compose_IncludedSetMissing_And_Cycle (Delete is Yes by its InitValue).
#[test]
fn tenant_sets_compose_includes_and_excludes() {
    let o = run(r#"
  [Test] procedure Included() var P: Record "Tenant Permission"; begin
    AddSet('A BASE'); AddSet('A MAIN');
    AddPerm('A BASE', 18, 1, 1, 0, P.Type::Include); AddPerm('A BASE', 23, 1, 0, 0, P.Type::Include);
    AddPerm('A MAIN', 18, 0, 0, 1, P.Type::Include); AddPerm('A MAIN', 27, 1, 1, 1, P.Type::Include);
    AddRel('A MAIN', 'A BASE', 0);
    Error(Rows('A BASE') + '|' + Rows('A MAIN')); end;
  [Test] procedure Excluded() var P: Record "Tenant Permission"; begin
    AddSet('B BASE'); AddSet('B NEG'); AddSet('B MAIN');
    AddPerm('B BASE', 18, 1, 1, 1, P.Type::Include); AddPerm('B BASE', 23, 1, 1, 0, P.Type::Include);
    AddPerm('B NEG', 18, 0, 1, 0, P.Type::Include); AddPerm('B NEG', 23, 1, 1, 1, P.Type::Include);
    AddRel('B MAIN', 'B BASE', 0); AddRel('B MAIN', 'B NEG', 1);
    Error(Rows('B MAIN')); end;
  [Test] procedure ExcludeRow() var P: Record "Tenant Permission"; begin
    AddSet('C BASE'); AddSet('C MAIN');
    AddPerm('C BASE', 18, 1, 1, 1, P.Type::Include);
    AddPerm('C MAIN', 18, 0, 1, 0, P.Type::Exclude); AddPerm('C MAIN', 27, 1, 0, 0, P.Type::Exclude);
    AddRel('C MAIN', 'C BASE', 0);
    Error(Rows('C BASE') + '|' + Rows('C MAIN')); end;
  [Test] procedure Diamond() var P: Record "Tenant Permission"; begin
    AddSet('D LEAF'); AddSet('D LEFT'); AddSet('D RIGHT'); AddSet('D TOP');
    AddPerm('D LEAF', 18, 1, 0, 0, P.Type::Include); AddPerm('D LEFT', 18, 0, 1, 0, P.Type::Include); AddPerm('D RIGHT', 18, 0, 0, 1, P.Type::Include);
    AddRel('D LEFT', 'D LEAF', 0); AddRel('D RIGHT', 'D LEAF', 0); AddRel('D TOP', 'D LEFT', 0); AddRel('D TOP', 'D RIGHT', 0);
    Error(Rows('D TOP')); end;
  [Test] procedure Cycle() var P: Record "Tenant Permission"; begin
    AddSet('E ONE'); AddSet('E TWO');
    AddPerm('E ONE', 18, 1, 0, 0, P.Type::Include); AddPerm('E TWO', 23, 1, 0, 0, P.Type::Include);
    AddRel('E ONE', 'E MISSING', 0); AddRel('E ONE', 'E TWO', 0); AddRel('E TWO', 'E ONE', 0);
    Error(Rows('E ONE') + '|' + Rows('E TWO')); end;
"#);
    assert_eq!(
        failed_with(&o, "Included"),
        "18:R=Yes,I=Yes,M= ,D=Yes,Tenant 23:R=Yes,I= ,M= ,D=Yes,Tenant |18:R=Yes,I=Yes,M=Yes,D=Yes,Tenant 23:R=Yes,I= ,M= ,D=Yes,Tenant 27:R=Yes,I=Yes,M=Yes,D=Yes,Tenant "
    );
    assert_eq!(failed_with(&o, "Excluded"), "18:R=Yes,I= ,M=Yes,D= ,Tenant ");
    assert_eq!(
        failed_with(&o, "ExcludeRow"),
        "18:R=Yes,I=Yes,M=Yes,D=Yes,Tenant |18:R=Yes,I= ,M=Yes,D= ,Tenant "
    );
    assert_eq!(failed_with(&o, "Diamond"), "18:R=Yes,I=Yes,M=Yes,D=Yes,Tenant ");
    assert_eq!(
        failed_with(&o, "Cycle"),
        "18:R=Yes,I= ,M= ,D=Yes,Tenant 23:R=Yes,I= ,M= ,D=Yes,Tenant |18:R=Yes,I= ,M= ,D=Yes,Tenant 23:R=Yes,I= ,M= ,D=Yes,Tenant "
    );
}

/// Not pinned: an Indirect right, and a set read again after a grant was added (TestMetadataExpandedPermission.al:196-207 pins
/// that BC answers the old composition) are refused.
#[test]
fn unpinned_tenant_shapes_are_refused() {
    let o = run(r#"
  [Test] procedure Indirect() var P: Record "Tenant Permission"; begin
    AddSet('F ONE'); AddPerm('F ONE', 18, 2, 0, 0, P.Type::Include); Rows('F ONE'); end;
  [Test] procedure Again() var P: Record "Tenant Permission"; begin
    AddSet('G ONE'); AddPerm('G ONE', 18, 1, 0, 0, P.Type::Include); Rows('G ONE');
    AddPerm('G ONE', 23, 1, 0, 0, P.Type::Include); Rows('G ONE'); end;
"#);
    for name in ["Indirect", "Again"] {
        let (_, out) = o.iter().find(|(n, _)| n == name).expect("test");
        assert!(
            matches!(out, Outcome::NotRun(why) if why.contains("tenant set")),
            "{name}: {out:?}"
        );
    }
}
