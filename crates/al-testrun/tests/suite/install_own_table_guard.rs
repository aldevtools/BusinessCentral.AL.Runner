//! Al-runner issue 6924: two apps under test with no dependency path between them. While app A installs, app B's
//! subscriber runs only if BC installed B first, and that order is not known (corpus install-upgrade/install-triggers/
//! Y79InstallUnrelated.al:6-12: a subscriber of an unrelated app runs during an install exactly when its app was
//! installed first). B's subscriber here exits unless a row of B's own table "OB Reg" exists. When that table is empty
//! and no install code of any app under test writes it, the subscriber exits before it does anything in the order where
//! it runs, so the result is the same in both orders and A's install runs to its end. When B's install code writes the
//! table, the order decides, and the install stays refused.

use al_sema::workspace::AppSource;
use al_testrun::runtime::CorpusRuntime;
use al_testrun::{CaseId, Outcome};

const A_ID: &str = "11111111-1111-1111-1111-111111116924";
const B_ID: &str = "22222222-2222-2222-2222-222222226924";

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
"#;

/// App B: its registration table, a setup subscriber with `body`, a database insert subscriber with `insert_body`, and
/// `more` (further objects, such as an install codeunit).
fn b(body: &str, insert_body: &str, more: &str) -> String {
    format!(
        r#"
table 50200 "OB Reg"
{{
    fields {{ field(1; "Table ID"; Integer) {{ }} field(2; Active; Boolean) {{ }} }}
    keys {{ key(PK; "Table ID") {{ Clustered = true; }} }}
}}

codeunit 50200 "OB Subscriber"
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", GetDatabaseTableTriggerSetup, '', false, false)]
    local procedure GetDatabaseTableTriggerSetup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    var
        Reg: Record "OB Reg";
    begin
        {body}
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", OnDatabaseInsert, '', false, false)]
    local procedure OnDatabaseInsertEvent(RecRef: RecordRef)
    var
        Reg: Record "OB Reg";
    begin
        {insert_body}
    end;

    procedure Note()
    begin
    end;

    procedure Register(TableId: Integer)
    var
        Reg: Record "OB Reg";
    begin
        Reg."Table ID" := TableId;
        Reg.Insert();
    end;
}}
{more}
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

fn outcome_with(body: &str, insert_body: &str, more: &str) -> Outcome {
    outcome_of(A, body, insert_body, more)
}

fn outcome_of(a: &str, body: &str, insert_body: &str, more: &str) -> Outcome {
    let apps = vec![app(A_ID, a), app(B_ID, &b(body, insert_body, more))];
    let kinds = vec![al_sema::AppKind::Workspace; apps.len()];
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &[], &Default::default(), None, Default::default());
    rt.first_outcome(&CaseId {
        codeunit: 50101,
        name: "InstallRanToItsEnd".into(),
    })
}

fn outcome(body: &str) -> Outcome {
    outcome_with(body, "", "")
}

fn refused(body: &str, insert_body: &str, more: &str) -> String {
    match outcome_with(body, insert_body, more) {
        Outcome::NotRun(why) => why,
        other => panic!("expected not run, got {other:?}"),
    }
}

/// The reporter's shape (triage row 1a).
const GET_GUARD: &str = r#"if not Reg.Get(TableId) then
            exit;
        OnDatabaseInsert := true;
        OnDatabaseModify := true;
        OnDatabaseDelete := true;
        OnDatabaseRename := true;"#;

/// B's install code registers A's table: if B installed first, A's inserts are logged.
const B_INSTALL_REGISTERS: &str = r#"
codeunit 50201 "OB Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Reg: Record "OB Reg";
    begin
        Reg."Table ID" := 50100;
        if Reg.Insert() then;
    end;
}
"#;

#[test]
fn a_flag_set_only_after_a_row_of_the_own_empty_table_does_not_stop_the_install() {
    assert_eq!(outcome(GET_GUARD), Outcome::Pass);
}

#[test]
fn an_is_empty_guard_after_set_range_on_the_own_empty_table_does_not_stop_the_install() {
    let body = r#"Reg.SetRange("Table ID", TableId);
        Reg.SetRange(Active, true);
        if Reg.IsEmpty() then
            exit;
        OnDatabaseInsert := true;"#;
    assert_eq!(outcome(body), Outcome::Pass);
}

/// B's code that writes the table runs outside install (a setup page, a test): it does not change the install.
#[test]
fn code_outside_install_that_writes_the_table_does_not_stop_the_install() {
    let more = r#"
codeunit 50202 "OB Setup"
{
    procedure Enable()
    var
        S: Codeunit "OB Subscriber";
    begin
        S.Register(50100);
    end;
}
"#;
    assert_eq!(outcome_with(GET_GUARD, "", more), Outcome::Pass);
}

/// A's own setup subscriber turns database inserts on for its table, so A's install raises OnDatabaseInsert.
const A_INSERTS_ON: &str = r#"
codeunit 50102 "OA Setup"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", GetDatabaseTableTriggerSetup, '', false, false)]
    local procedure GetDatabaseTableTriggerSetup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
        if TableId = Database::"OA Tag" then
            OnDatabaseInsert := true;
    end;
}
"#;

/// The same guard in B's database insert subscriber.
#[test]
fn a_database_insert_subscriber_guarded_by_the_own_empty_table_does_not_stop_the_install() {
    let insert = r#"if not Reg.Get(RecRef.Number()) then
            exit;
        Note();"#;
    assert_eq!(outcome_of(&format!("{A}{A_INSERTS_ON}"), "", insert, ""), Outcome::Pass);
}

/// Without the guard the insert subscriber calls code, and the order decides.
#[test]
fn a_database_insert_subscriber_without_the_guard_keeps_the_install_refused() {
    match outcome_of(&format!("{A}{A_INSERTS_ON}"), "", "Note();", "") {
        Outcome::NotRun(why) => assert!(why.contains("install order"), "{why}"),
        other => panic!("expected not run, got {other:?}"),
    }
}

#[test]
fn install_code_that_writes_the_table_keeps_the_install_refused() {
    let why = refused(GET_GUARD, "", B_INSTALL_REGISTERS);
    assert!(why.contains("install order"), "{why}");
}

/// The write is in a procedure B's install code calls.
#[test]
fn install_code_that_calls_a_writer_of_the_table_keeps_the_install_refused() {
    let more = r#"
codeunit 50201 "OB Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        S: Codeunit "OB Subscriber";
    begin
        S.Register(50100);
    end;
}
"#;
    let why = refused(GET_GUARD, "", more);
    assert!(why.contains("install order"), "{why}");
}

/// Install code that writes through a RecordRef may write any table.
#[test]
fn install_code_that_writes_through_a_record_ref_keeps_the_install_refused() {
    let more = r#"
codeunit 50201 "OB Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        RR: RecordRef;
    begin
        RR.Open(50200);
        RR.Field(1).Value := 50100;
        RR.Insert();
    end;
}
"#;
    let why = refused(GET_GUARD, "", more);
    assert!(why.contains("install order"), "{why}");
}

/// `Reg.Get` as a statement raises an error when the row is missing: in the order where it runs the install fails.
#[test]
fn a_get_whose_result_is_not_used_keeps_the_install_refused() {
    let why = refused("Reg.Get(TableId);\n        OnDatabaseInsert := true;", "", "");
    assert!(why.contains("install order"), "{why}");
}

/// A statement with an effect before the guard.
#[test]
fn a_call_before_the_guard_keeps_the_install_refused() {
    let why = refused(&format!("Note();\n        {GET_GUARD}"), "", "");
    assert!(why.contains("install order"), "{why}");
}

/// A flag set before the guard.
#[test]
fn a_flag_set_before_the_guard_keeps_the_install_refused() {
    let why = refused(&format!("OnDatabaseInsert := true;\n        {GET_GUARD}"), "", "");
    assert!(why.contains("install order"), "{why}");
}

const C_ID: &str = "33333333-3333-3333-3333-333333336924";

/// A shared app C: a table and an event. A's install writes C's table; B subscribes to C's table event and to C's event.
const C: &str = r#"
table 50300 "OC Shared"
{
    fields { field(1; No; Integer) { } }
    keys { key(PK; No) { Clustered = true; } }
}

codeunit 50300 "OC Events"
{
    procedure Raise()
    begin
        OnSomething(50100);
    end;

    [IntegrationEvent(false, false)]
    procedure OnSomething(TableId: Integer)
    begin
    end;
}
"#;

const A_ON_C: &str = r#"
codeunit 50100 "OA Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Shared: Record "OC Shared";
        E: Codeunit "OC Events";
    begin
        E.Raise();
        Shared.No := 1;
        Shared.Insert(true);
    end;
}

codeunit 50101 "OA Tests"
{
    Subtype = Test;

    [Test]
    procedure InstallRanToItsEnd()
    var
        Shared: Record "OC Shared";
    begin
        Shared.Get(1);
    end;
}
"#;

fn b_on_c(event_body: &str, table_event_body: &str) -> String {
    format!(
        r#"
table 50200 "OB Reg"
{{
    fields {{ field(1; "Table ID"; Integer) {{ }} }}
    keys {{ key(PK; "Table ID") {{ Clustered = true; }} }}
}}

codeunit 50200 "OB Subscriber"
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"OC Events", OnSomething, '', false, false)]
    local procedure OnSomething(TableId: Integer)
    var
        Reg: Record "OB Reg";
    begin
        {event_body}
    end;

    [EventSubscriber(ObjectType::Table, Database::"OC Shared", OnBeforeInsertEvent, '', false, false)]
    local procedure OnBeforeInsertShared(var Rec: Record "OC Shared"; RunTrigger: Boolean)
    var
        Reg: Record "OB Reg";
    begin
        {table_event_body}
    end;

    procedure Note()
    begin
    end;
}}
"#
    )
}

fn outcome_on_c(event_body: &str, table_event_body: &str) -> Outcome {
    let dep = |id: &str, src: &str| AppSource {
        dependencies: vec![C_ID.into()],
        ..app(id, src)
    };
    let apps = vec![
        app(C_ID, C),
        dep(A_ID, A_ON_C),
        dep(B_ID, &b_on_c(event_body, table_event_body)),
    ];
    let kinds = vec![al_sema::AppKind::Workspace; apps.len()];
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &[], &Default::default(), None, Default::default());
    rt.first_outcome(&CaseId {
        codeunit: 50101,
        name: "InstallRanToItsEnd".into(),
    })
}

/// Triage rows 1c and 1d: an event and a table event of a shared app, raised by A's install, reach B's subscribers. Both
/// stop unless B's own empty table holds a row.
#[test]
fn event_subscribers_guarded_by_the_own_empty_table_do_not_stop_the_install() {
    let guarded = r#"if Rec.IsTemporary() then
            exit;
        if not Reg.Get(Database::"OC Shared") then
            exit;
        Note();"#;
    let event = "if not Reg.Get(TableId) then\n            exit;\n        Note();";
    assert_eq!(outcome_on_c(event, guarded), Outcome::Pass);
}

#[test]
fn an_event_subscriber_without_the_guard_keeps_the_install_refused() {
    match outcome_on_c("Note();", "") {
        Outcome::NotRun(why) => assert!(why.contains("install order"), "{why}"),
        other => panic!("expected not run, got {other:?}"),
    }
}

#[test]
fn a_table_event_subscriber_without_the_guard_keeps_the_install_refused() {
    match outcome_on_c("", "Note();") {
        Outcome::NotRun(why) => assert!(why.contains("install order"), "{why}"),
        other => panic!("expected not run, got {other:?}"),
    }
}

/// Triage row 1e: A's install deletes a row of C's table, which B extends. B's extension changes the delete only through an
/// OnBeforeDelete trigger of its own.
const A_DELETES_ON_C: &str = r#"
codeunit 50100 "OA Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Shared: Record "OC Shared";
    begin
        Shared.No := 1;
        Shared.Insert(false);
        Shared.Delete(true);
        Shared.No := 2;
        Shared.Insert(false);
    end;
}

codeunit 50101 "OA Tests"
{
    Subtype = Test;

    [Test]
    procedure InstallRanToItsEnd()
    var
        Shared: Record "OC Shared";
    begin
        Shared.Get(2);
    end;
}
"#;

fn outcome_ext(ext_body: &str) -> Outcome {
    let dep = |id: &str, src: &str| AppSource {
        dependencies: vec![C_ID.into()],
        ..app(id, src)
    };
    let b = format!(
        r#"
table 50200 "OB Reg"
{{
    fields {{ field(1; "Table ID"; Integer) {{ }} }}
    keys {{ key(PK; "Table ID") {{ Clustered = true; }} }}
}}

tableextension 50200 "OB Shared Ext" extends "OC Shared"
{{
    fields {{ field(50200; "OB Mark"; Code[10]) {{ }} }}
    {ext_body}
}}

codeunit 50201 "OB Helper"
{{
    procedure Note()
    begin
    end;
}}
"#
    );
    let apps = vec![app(C_ID, C), dep(A_ID, A_DELETES_ON_C), dep(B_ID, &b)];
    let kinds = vec![al_sema::AppKind::Workspace; apps.len()];
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &[], &Default::default(), None, Default::default());
    rt.first_outcome(&CaseId {
        codeunit: 50101,
        name: "InstallRanToItsEnd".into(),
    })
}

#[test]
fn a_table_extension_without_a_delete_trigger_does_not_stop_a_delete_in_the_install() {
    assert_eq!(outcome_ext(""), Outcome::Pass);
}

#[test]
fn a_delete_trigger_guarded_by_the_own_empty_table_does_not_stop_a_delete_in_the_install() {
    let body = r#"trigger OnBeforeDelete()
    var
        Reg: Record "OB Reg";
        H: Codeunit "OB Helper";
    begin
        if not Reg.Get(Database::"OC Shared") then
            exit;
        H.Note();
    end;"#;
    assert_eq!(outcome_ext(body), Outcome::Pass);
}

#[test]
fn a_delete_trigger_that_calls_code_keeps_the_install_refused() {
    let body = r#"trigger OnBeforeDelete()
    var
        H: Codeunit "OB Helper";
    begin
        H.Note();
    end;"#;
    match outcome_ext(body) {
        Outcome::NotRun(why) => assert!(why.contains("install order"), "{why}"),
        other => panic!("expected not run, got {other:?}"),
    }
}
