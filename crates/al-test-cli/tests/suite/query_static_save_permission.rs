//! A static `Query.SaveAsCsv(Number, OutStream)` and `Query.SaveAsXml(Number, OutStream)` under a restricted set check the
//! query of that number, not the query of the last instance call in the session (al-runner issue 7612). The check is the one
//! `Query.Open` takes (Y24PermExecute.al:62-63, Query Execute denied).

use super::y48_platform_probes::{outcomes, outcomes_edited};
use al_test_cli::report::Outcome;

const SRC: &str = r#"
dotnet { assembly("Microsoft.Dynamics.Nav.PermissionTestHelper") {
  type("Microsoft.Dynamics.Nav.Runtime.PermissionTestHelper"; "PermissionTestHelper") { } } }
table 50100 "Q Row" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { Clustered = true; } } }
query 50100 "Granted Q" { elements { dataitem(R; "Q Row") { column(Id; Id) { } } } }
query 50101 "Denied Q" { elements { dataitem(R; "Q Row") { column(Id; Id) { } } } }
permissionset 50100 "NARROW" { Assignable = true; Permissions = tabledata "Q Row" = R, query "Granted Q" = X, codeunit T = X; }
codeunit 50000 T { Subtype = Test; TestPermissions = Disabled;
  local procedure Restrict() var H: DotNet PermissionTestHelper; begin
    H := H.PermissionTestHelper(); H.AddEffectivePermissionSet('NARROW'); end;
  [Test] procedure CsvOfDeniedFirst() var Row: Record "Q Row" temporary; OutS: OutStream; begin
    Restrict(); Row.Data.CreateOutStream(OutS);
    asserterror Query.SaveAsCsv(Query::"Denied Q", OutS); Error(GetLastErrorCode()); end;
  [Test] procedure CsvOfDeniedAfterGranted() var Row: Record "Q Row" temporary; Q: Query "Granted Q"; OutS: OutStream; begin
    Restrict(); Q.Open(); Q.Close(); Row.Data.CreateOutStream(OutS);
    asserterror Query.SaveAsCsv(Query::"Denied Q", OutS); Error(GetLastErrorCode()); end;
  [Test] procedure XmlOfDeniedAfterGranted() var Row: Record "Q Row" temporary; Q: Query "Granted Q"; OutS: OutStream; begin
    Restrict(); Q.Open(); Q.Close(); Row.Data.CreateOutStream(OutS);
    asserterror Query.SaveAsXml(Query::"Denied Q", OutS); Error(GetLastErrorCode()); end;
  [Test] procedure CsvOfGranted() var Row: Record "Q Row" temporary; OutS: OutStream; begin
    Restrict(); Row.Data.CreateOutStream(OutS);
    if Query.SaveAsCsv(Query::"Granted Q", OutS) then Error('saved'); Error('not saved'); end;
}
"#;

#[test]
fn static_save_as_checks_the_query_of_its_number() {
    let o = outcomes(SRC);
    let get = |name: &str| {
        &o.iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("{name} in {o:?}"))
            .1
    };
    for name in ["CsvOfDeniedFirst", "CsvOfDeniedAfterGranted", "XmlOfDeniedAfterGranted"] {
        assert_eq!(*get(name), Outcome::Fail("DB:ExecuteObjDenied".into()), "{name}");
    }
    assert_eq!(*get("CsvOfGranted"), Outcome::Fail("saved".into()));
}

const SHELL_SRC: &str = r#"
table 50100 "Q Row" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { Clustered = true; } } }
query 50102 "Shell Q" { elements { dataitem(R; "Q Row") { column(Id; Id) { } } } }
codeunit 50000 T { Subtype = Test;
  [Test] procedure CsvOfShell() var Row: Record "Q Row" temporary; OutS: OutStream; begin
    Row.Data.CreateOutStream(OutS); if Query.SaveAsCsv(50102, OutS) then Error('saved'); Error('not saved'); end;
  [Test] procedure XmlOfShell() var Row: Record "Q Row" temporary; OutS: OutStream; begin
    Row.Data.CreateOutStream(OutS); if Query.SaveAsXml(50102, OutS) then Error('saved'); Error('not saved'); end;
  [Test] procedure CsvOfUnknown() var Row: Record "Q Row" temporary; OutS: OutStream; begin
    Row.Data.CreateOutStream(OutS); if Query.SaveAsCsv(50199, OutS) then Error('saved'); Error('not saved'); end;
}
"#;

/// A query of a dependency app that was not compiled is refused (BC finds it); an unknown id keeps main's No
/// (xmlports/import/XpTQuerySave.al:SaveAsCsv_UnknownQueryId) unless the run skipped a dependency app.
#[test]
fn static_save_as_of_an_uncompiled_or_skipped_query_refuses() {
    let shell = |p: &mut al_ir::Program| {
        let i = p.objects.iter().position(|o| o.name == "Shell Q").expect("Shell Q");
        p.objects[i].uncompiled = true;
    };
    let o = outcomes_edited(SHELL_SRC, shell, Vec::new());
    let get = |o: &[(String, Outcome)], name: &str| o.iter().find(|(n, _)| n == name).expect(name).1.clone();
    for name in ["CsvOfShell", "XmlOfShell"] {
        match get(&o, name) {
            Outcome::NotRun(why) => assert!(why.contains("was not compiled"), "{name}: {why}"),
            other => panic!("{name}: {other:?}"),
        }
    }
    assert_eq!(get(&o, "CsvOfUnknown"), Outcome::Fail("not saved".into()));
    let o = outcomes_edited(SHELL_SRC, |_| {}, vec!["`Dep` by Publisher".into()]);
    match get(&o, "CsvOfUnknown") {
        Outcome::NotRun(why) => assert!(why.contains("skipped the dependency"), "{why}"),
        other => panic!("{other:?}"),
    }
    assert_eq!(get(&o, "CsvOfShell"), Outcome::Fail("saved".into()));
}
