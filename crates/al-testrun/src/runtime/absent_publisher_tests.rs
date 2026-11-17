// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#![cfg(test)]
//! A subscriber whose publisher is not in the program: the app under test depends on an app that is not loaded (a
//! `--no-deps` run), so the publisher codeunit is absent and the runtime cannot raise its events. Real BC raises them
//! (record/table-relation/RenameBlankKeyRelation.al:722-736, Base Application's GlobalTriggerManagement logs
//! `OnAfterOnDatabaseRename` in DatabaseRenameTrigger_NonBlank, :2104-2112). A test codeunit that can see the subscriber
//! ran is not run; one that cannot see it runs as before.

use super::app_outcomes_tests::outcomes_and_hits_of_apps;
use super::*;

const MAIN: &str = r#"
codeunit 50100 "Log"
{
    SingleInstance = true;
    var
        Entries: Text;

    procedure Add(Entry: Text)
    begin
        Entries += Entry;
    end;

    procedure Get(): Text
    begin
        exit(Entries);
    end;
}

codeunit 50101 "Subs"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::GlobalTriggerManagement, 'OnAfterOnDatabaseRename', '', false, false)]
    local procedure AfterDbRename(RecRef: RecordRef; xRecRef: RecordRef)
    var
        Log: Codeunit "Log";
    begin
        Log.Add('renamed');
    end;
}

codeunit 50102 "Sees"
{
    Subtype = Test;

    [Test]
    procedure ReadsTheLog()
    var
        Log: Codeunit "Log";
    begin
        if Log.Get() <> '' then
            Error('the log holds %1', Log.Get());
    end;
}

codeunit 50103 "Blind"
{
    Subtype = Test;

    [Test]
    procedure ReadsNothing()
    begin
    end;
}
"#;

/// The publisher, in a dependency app that is loaded.
const DEP: &str = r#"
codeunit 50 GlobalTriggerManagement
{
    [IntegrationEvent(false, false)]
    procedure OnAfterOnDatabaseRename(RecRef: RecordRef; xRecRef: RecordRef)
    begin
    end;
}
"#;

#[test]
fn a_test_that_sees_a_subscriber_of_a_publisher_in_an_unloaded_app_is_not_run() {
    let (done, _) = outcomes_and_hits_of_apps(&[("main", &["sysapp"], MAIN)], "Sees");
    let [(name, Outcome::NotRun(why))] = done.as_slice() else {
        panic!("{done:?}");
    };
    assert_eq!(name, "ReadsTheLog");
    assert!(
        why.contains("codeunit \"Subs\".AfterDbRename")
            && why.contains("GlobalTriggerManagement")
            && why.contains("whose publisher is in no loaded app"),
        "{why}"
    );
}

#[test]
fn a_test_that_cannot_see_the_subscriber_runs_as_before() {
    let (done, _) = outcomes_and_hits_of_apps(&[("main", &["sysapp"], MAIN)], "Blind");
    assert!(matches!(done.as_slice(), [(_, Outcome::Pass)]), "{done:?}");
}

#[test]
fn a_subscriber_of_a_publisher_in_a_loaded_app_does_not_refuse() {
    let (done, _) = outcomes_and_hits_of_apps(&[("dep", &[], DEP), ("main", &["dep"], MAIN)], "Sees");
    assert!(matches!(done.as_slice(), [(_, Outcome::Pass)]), "{done:?}");
}
