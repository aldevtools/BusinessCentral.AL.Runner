//! The test session is a session of its own: its first write of a table asks the `Global Triggers` subscribers for the
//! table's setup again, although the install session asked for the same table (pinned: the setup is asked once per table
//! in the test session for a table the install code wrote, install-upgrade/install-triggers/Y69InstallTriggerSetup.al:104,
//! :131 `d_setup_61180`; a later test of the same session does not ask again, events/platform-triggers/
//! TableTriggerSetupCalls.al:103-125).

use al_sema::workspace::AppSource;
use al_testrun::runtime::CorpusRuntime;
use al_testrun::{CaseId, Outcome};

const APP: &str = r#"
table 50300 "TS Row" { fields { field(1; No; Integer) { } } keys { key(PK; No) { Clustered = true; } } }
codeunit 50300 "TS Asks" { SingleInstance = true;
    var Asks: Integer;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", GetDatabaseTableTriggerSetup, '', false, false)]
    local procedure S(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin if TableId = Database::"TS Row" then Asks += 1; end;
    procedure Count(): Integer begin exit(Asks); end; }
codeunit 50301 "TS Install" { Subtype = Install;
    trigger OnInstallAppPerCompany() var R: Record "TS Row"; begin R.No := 1; R.Insert(); end; }
codeunit 50302 "TS Tests" { Subtype = Test;
    [Test] procedure AskedAgainInTheTestSession() var R: Record "TS Row"; A: Codeunit "TS Asks";
    begin R.No := 2; R.Insert(); if A.Count() <> 1 then Error('asked %1 times', A.Count()); end; }
"#;

#[test]
fn the_test_session_asks_the_setup_of_a_table_the_install_session_asked() {
    let apps = vec![AppSource {
        id: "33333333-3333-3333-3333-333333338945".into(),
        name: "Setup Session".into(),
        dependencies: Vec::new(),
        files: vec![("f.al".into(), APP.into())],
    }];
    let rt = CorpusRuntime::run_for_apps(
        apps,
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
    );
    let got = rt.first_outcome(&CaseId {
        codeunit: 50302,
        name: "AskedAgainInTheTestSession".into(),
    });
    assert_eq!(got, Outcome::Pass);
}
