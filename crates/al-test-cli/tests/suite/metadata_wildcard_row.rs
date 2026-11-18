//! The Metadata Permission row of a permission set whose `Permissions` is exactly `page * = X` (events/CoPermissionLeads.al:
//! 461-478, crates/al-vm/src/platform/permissions.rs `metadata_wildcard_row`). Every other wildcard keeps main's refusal (al-runner
//! issue 8032 asks for `table * = X`, which no corpus case measures yet).

use super::y48_platform_probes::outcomes;
use al_test_cli::report::Outcome;

const SETS: &str = r#"
permissionset 50100 "W Page X" { Assignable = true; Permissions = page * = X; }
permissionset 50101 "W Table X" { Assignable = true; Permissions = table * = X; }
permissionset 50102 "W Page RX" { Assignable = true; Permissions = page * = RX; }
permissionset 50103 "W Page Mixed" { Assignable = true; Permissions = page * = X, codeunit T = X; }
permissionset 50104 "W Codeunit X" { Assignable = true; Permissions = codeunit * = X; }
permissionset 50105 "W Not Assignable" { Assignable = false; Permissions = page * = X; }
permissionset 50106 "W No Assignable" { Permissions = page * = X; }
permissionset 50107 "W Includes" { Assignable = true; IncludedPermissionSets = "W Page X"; Permissions = page * = X; }
permissionset 50108 "W Excludes" { Assignable = true; ExcludedPermissionSets = "W Page X"; Permissions = page * = X; }
permissionset 50109 "W Extended" { Assignable = true; Permissions = page * = X; }
permissionsetextension 50110 "W Ext" extends "W Extended" { Permissions = codeunit T = X; }
permissionset 50111 "W Caption Upper" { Assignable = true; Caption = 'Wild caption'; Permissions = PAGE * = X; }
"#;

fn src() -> String {
    let test = |name: &str, role: &str| {
        format!(
            r#"  [Test] procedure {name}() var M: Record "Metadata Permission"; R: Text; begin
    M.SetRange("Role ID", '{role}'); if M.FindSet() then repeat
      R += Format(M."Object Type") + ' ' + Format(M."Object ID") + ' X=' + Format(M."Execute Permission") + ';';
    until M.Next() = 0; Error(R); end;
"#
        )
    };
    format!(
        "{SETS}\ncodeunit 50000 T {{ Subtype = Test;\n{}{}{}{}{}{}{}{}{}{}{}}}\n",
        test("PageX", "W PAGE X"),
        test("TableX", "W TABLE X"),
        test("PageRX", "W PAGE RX"),
        test("PageMixed", "W PAGE MIXED"),
        test("CodeunitX", "W CODEUNIT X"),
        test("NotAssignable", "W NOT ASSIGNABLE"),
        test("NoAssignable", "W NO ASSIGNABLE"),
        test("Includes", "W INCLUDES"),
        test("Excludes", "W EXCLUDES"),
        test("Extended", "W EXTENDED"),
        test("CaptionUpper", "W CAPTION UPPER"),
    )
}

#[test]
fn only_the_measured_page_wildcard_gives_a_row() {
    let o = outcomes(&src());
    let get = |name: &str| {
        &o.iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("{name} in {o:?}"))
            .1
    };
    assert_eq!(*get("PageX"), Outcome::Fail("Page 0 X=Yes;".into()));
    // A Caption on the set and an upper-case `PAGE` change nothing: the Role Name is the existing rule, AL is case-insensitive.
    assert_eq!(*get("CaptionUpper"), Outcome::Fail("Page 0 X=Yes;".into()));
    for name in [
        "TableX",
        "PageRX",
        "PageMixed",
        "CodeunitX",
        "NotAssignable",
        "NoAssignable",
        "Includes",
        "Excludes",
        "Extended",
    ] {
        match get(name) {
            Outcome::NotRun(why) => assert!(why.contains("grants a whole object type"), "{name}: {why}"),
            other => panic!("{name}: {other:?}"),
        }
    }
}
