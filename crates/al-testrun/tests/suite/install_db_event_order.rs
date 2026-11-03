//! al-runner issue 6852: two apps under test with no dependency path between them. App A's subscriber turns the database
//! insert (modify, delete, rename) event on for A's own table, and A's install code writes it. B's subscriber to the event
//! runs only if BC installed B first (ADR-046 amendment 6: InstallOrderSubscriberTests.al:66-79, :104-117), and that order
//! is not known. When B's body does nothing for A's table (empty, or guarded by the table number so that it is false for A's
//! table), the outcome is the same in both orders, so A's install code runs. Anything else still stops it.

use al_sema::workspace::AppSource;
use al_testrun::runtime::CorpusRuntime;
use al_testrun::{CaseId, Outcome};

const A_ID: &str = "11111111-1111-1111-1111-111111116852";
const B_ID: &str = "22222222-2222-2222-2222-222222226852";

const A: &str = r#"
table 50100 "OA Tag"
{
    fields { field(1; Tag; Code[20]) { } }
    keys { key(PK; Tag) { Clustered = true; } }
}

codeunit 50100 "OA Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Tag: Record "OA Tag";
    begin
        Tag.Tag := 'INSTALLED';
        if Tag.Insert() then;
    end;
}

codeunit 50101 "OA Tests"
{
    Subtype = Test;

    [Test]
    procedure InstallRanToItsEnd()
    var
        Tag: Record "OA Tag";
    begin
        Tag.Get('INSTALLED');
    end;
}

codeunit 50102 "OA Flag Own Table"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", GetDatabaseTableTriggerSetup, '', false, false)]
    local procedure GetDatabaseTableTriggerSetup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean)
    begin
        if TableId = Database::"OA Tag" then
            OnDatabaseInsert := true;
    end;
}
"#;

/// B's subscriber to the insert event, with `body` as its statements.
fn b(body: &str) -> String {
    format!(
        r#"
table 50200 "OB Log Source"
{{
    fields {{ field(1; No; Integer) {{ }} }}
    keys {{ key(PK; No) {{ Clustered = true; }} }}
}}

table 50201 "OB Other"
{{
    fields {{ field(1; No; Integer) {{ }} }}
    keys {{ key(PK; No) {{ Clustered = true; }} }}
}}

codeunit 50200 "OB Subscriber"
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", GetDatabaseTableTriggerSetup, '', false, false)]
    local procedure GetDatabaseTableTriggerSetup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean)
    begin
        if TableId in [Database::"OB Log Source", Database::"OB Other"] then
            OnDatabaseInsert := true;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", OnDatabaseInsert, '', false, false)]
    local procedure OnDatabaseInsertEvent(RecRef: RecordRef)
    begin
        {body}
    end;

    procedure LogInsert(RecRef: RecordRef)
    begin
    end;
}}
"#
    )
}

fn app(id: &str, src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: format!("Order {}", &id[..1]),
        dependencies: Vec::new(),
        files: vec![("f.al".into(), src.into())],
    }
}

fn outcome(b_body: &str) -> Outcome {
    let apps = vec![app(A_ID, A), app(B_ID, &b(b_body))];
    let kinds = vec![al_sema::AppKind::Workspace; apps.len()];
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &[], &Default::default(), None, Default::default());
    rt.first_outcome(&CaseId {
        codeunit: 50101,
        name: "InstallRanToItsEnd".into(),
    })
}

fn refused(body: &str) -> String {
    match outcome(body) {
        Outcome::NotRun(why) => why,
        other => panic!("expected not run, got {other:?}"),
    }
}

/// The reporter's first shape: an empty body.
#[test]
fn an_empty_insert_subscriber_does_not_stop_the_install() {
    assert_eq!(outcome(""), Outcome::Pass);
}

/// The reporter's second shape: a guard on the table number around a call that writes B's own table.
#[test]
fn a_guard_on_the_table_number_that_misses_the_table_does_not_stop_the_install() {
    let body = r#"if RecRef.Number() in [Database::"OB Log Source", Database::"OB Other"] then
            LogInsert(RecRef);"#;
    assert_eq!(outcome(body), Outcome::Pass);
}

#[test]
fn an_equality_guard_that_misses_the_table_does_not_stop_the_install() {
    assert_eq!(
        outcome(r#"if RecRef.Number() = Database::"OB Log Source" then LogInsert(RecRef);"#),
        Outcome::Pass
    );
}

#[test]
fn a_case_on_the_table_number_that_misses_the_table_does_not_stop_the_install() {
    let body = r#"case RecRef.Number() of
            Database::"OB Log Source":
                LogInsert(RecRef);
        end;"#;
    assert_eq!(outcome(body), Outcome::Pass);
}

/// Guards joined with `and` / `or`, and an early `exit`.
#[test]
fn guards_joined_with_and_or_exit_do_not_stop_the_install() {
    let body = r#"if RecRef.Number() <> Database::"OB Log Source" then
            exit;
        LogInsert(RecRef);"#;
    assert_eq!(outcome(body), Outcome::Pass);
}

/// A body that does something for every table changes what B's own tables see if B installed first.
#[test]
fn an_unguarded_call_still_stops_the_install() {
    let why = refused("LogInsert(RecRef);");
    assert!(why.contains("install order"), "{why}");
}

/// The guard names A's table (50100): the body runs for it, so the order matters.
#[test]
fn a_guard_that_names_the_table_still_stops_the_install() {
    let why = refused("if RecRef.Number() in [50100, 50999] then LogInsert(RecRef);");
    assert!(why.contains("install order"), "{why}");
}

/// A guard on something other than the table number cannot be decided.
#[test]
fn a_guard_on_a_record_value_still_stops_the_install() {
    let why = refused(r#"if RecRef.Name() = 'OB Log Source' then LogInsert(RecRef);"#);
    assert!(why.contains("install order"), "{why}");
}

/// The modify and delete events: A's install modifies and deletes its row, B's subscribers are empty or guarded.
#[test]
fn modify_and_delete_subscribers_that_do_nothing_for_the_table_do_not_stop_the_install() {
    let a = A
        .replace("OnDatabaseInsert := true;", "begin OnDatabaseInsert := true; OnDatabaseModify := true; OnDatabaseDelete := true; end;")
        .replace(
            "if Tag.Insert() then;",
            "if Tag.Insert() then;\n        Tag.Modify();\n        Tag.Delete();\n        Tag.Tag := 'INSTALLED';\n        if Tag.Insert() then;",
        );
    let b = b("").replace(
        "    procedure LogInsert",
        r#"    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", OnDatabaseModify, '', false, false)]
    local procedure OnDatabaseModifyEvent(RecRef: RecordRef)
    begin
        if RecRef.Number() = Database::"OB Log Source" then
            LogInsert(RecRef);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", OnDatabaseDelete, '', false, false)]
    local procedure OnDatabaseDeleteEvent(RecRef: RecordRef)
    begin
    end;

    procedure LogInsert"#,
    );
    let apps = vec![app(A_ID, &a), app(B_ID, &b)];
    let kinds = vec![al_sema::AppKind::Workspace; apps.len()];
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &[], &Default::default(), None, Default::default());
    let o = rt.first_outcome(&CaseId {
        codeunit: 50101,
        name: "InstallRanToItsEnd".into(),
    });
    assert_eq!(o, Outcome::Pass);
}
