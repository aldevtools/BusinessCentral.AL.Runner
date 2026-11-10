//! The object a permission set line names is the one its own app sees: the app and its dependencies, as the AL compiler resolves
//! it. Corpus `permissions/lt-y94-page-open-abp/Y94AbpActionRef.al` Y94_Ref_GrantsB (BC 27, 28, 29): `Mock.Set('D365 BASIC')`
//! composes in full although the corpus app declares a table of the same name as one a Base Application set names
//! (`_fixtures/ALTRelationNameCollisionTables.al:28` table 60991 "Config. Package Table" against Base Application table 8613).

use super::permission_writes::{BASE, run_main_on};
use al_testrun::Outcome;
use al_testrun::runtime::RunOptions;

const DEP: &str = r#"
table 51590 "NS Dup Name" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
permissionset 51590 "NS Dep Set" { Assignable = true; Permissions = tabledata "NS Dup Name" = R; }
"#;

const MAIN: &str = r#"
table 51591 "NS Dup Name" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
table 51592 "NS Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }

codeunit 51593 "NS Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure TheDependencySetComposesInFull()
    var
        Row: Record "NS Row";
        Mock: Codeunit "Permissions Mock";
    begin
        Mock.Start();
        Mock.Set('NS Dep Set');
        Row.Id := 1;
        asserterror Row.Insert();
        Mock.Stop();
        if GetLastErrorCode() <> 'DB:ClientInsertDenied' then
            Error('code: %1 %2', GetLastErrorCode(), GetLastErrorText());
    end;
}
"#;

#[test]
fn a_set_line_names_the_object_its_own_app_sees() {
    let rt = run_main_on(MAIN, &format!("{BASE}{DEP}"), &RunOptions::default());
    let o = rt
        .results()
        .iter()
        .find(|r| r.test == "TheDependencySetComposesInFull")
        .expect("ran")
        .outcome
        .clone();
    assert_eq!(o, Outcome::Pass);
}
