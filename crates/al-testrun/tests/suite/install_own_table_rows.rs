//! Al-runner issue 6971: two apps under test with no dependency path between them. While app A installs, app B's trigger setup
//! subscriber runs only if BC installed B first (corpus install-upgrade/install-triggers/Y79InstallUnrelated.al:6-12), and
//! that order is not known. B's subscriber decides from B's own setup table, which B's own install code fills. In the order
//! where the subscriber runs, the table holds the rows B's install code inserted; when those rows are known from constants
//! and none matches the filters the subscriber sets for A's table, it exits before it sets a flag, so the result is the same
//! in both orders and A's install runs to its end. Where a row matches, or the rows are not known, the install stays
//! refused.

use al_sema::workspace::AppSource;
use al_testrun::runtime::CorpusRuntime;
use al_testrun::{CaseId, Outcome};

const A_ID: &str = "11111111-1111-1111-1111-111111116971";
const B_ID: &str = "22222222-2222-2222-2222-222222226971";

/// App A of al-runner issue 6852: its install inserts into its own table, which raises the setup event for `OA Tag`.
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

/// The reporter's subscriber.
const SUBSCRIBER: &str = r#"
        FieldSetup.SetCurrentKey("Table ID");
        FieldSetup.SetRange("Table ID", TableId);
        if FieldSetup.IsEmpty() then
            exit;
        OnDatabaseInsert := true;
        OnDatabaseModify := true;
        OnDatabaseDelete := true;
        OnDatabaseRename := true;"#;

/// The reporter's install code: one row, for a table that is not A's (Customer there; no Base Application here).
const INSTALL: &str = r#"
        FieldSetup."Table ID" := Database::"OB Field Setup";
        FieldSetup."Field ID" := 2;
        if FieldSetup.Insert() then;"#;

/// App B with the setup subscriber `body` and the per-company install code `install`; `more` holds further objects.
fn b(body: &str, install: &str, more: &str) -> String {
    format!(
        r#"
table 50200 "OB Field Setup"
{{
    fields
    {{
        field(1; "Table ID"; Integer) {{ }}
        field(2; "Field ID"; Integer) {{ }}
    }}
    keys
    {{
        key(PK; "Table ID", "Field ID") {{ Clustered = true; }}
    }}
}}

codeunit 50200 "OB Install"
{{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        FieldSetup: Record "OB Field Setup";
        Helper: Codeunit "OB Helper";
    begin
        {install}
    end;
}}

codeunit 50201 "OB Trigger Setup"
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", GetDatabaseTableTriggerSetup, '', false, false)]
    local procedure SetTriggers(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    var
        FieldSetup: Record "OB Field Setup";
    begin
        {body}
    end;
}}

codeunit 50202 "OB Helper"
{{
    procedure Add(var FieldSetup: Record "OB Field Setup")
    begin
        FieldSetup."Table ID" := 50100;
        FieldSetup.Insert();
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

fn outcome(body: &str, install: &str, more: &str) -> Outcome {
    let apps = vec![app(A_ID, A), app(B_ID, &b(body, install, more))];
    let kinds = vec![al_sema::AppKind::Workspace; apps.len()];
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &[], &Default::default(), None, Default::default());
    rt.first_outcome(&CaseId {
        codeunit: 50101,
        name: "InstallRanToItsEnd".into(),
    })
}

fn refused(body: &str, install: &str, more: &str) -> String {
    match outcome(body, install, more) {
        Outcome::NotRun(why) => {
            assert!(why.contains("install order"), "{why}");
            why
        }
        other => panic!("expected not run, got {other:?}"),
    }
}

/// The reporter's shape: B's install code inserts a row for another table only, so for A's table the subscriber exits.
#[test]
fn a_subscriber_that_finds_no_row_of_its_own_install_rows_does_not_stop_the_install() {
    assert_eq!(outcome(SUBSCRIBER, INSTALL, ""), Outcome::Pass);
}

/// Two rows, neither for A's table, one of them inserted only on a branch.
#[test]
fn rows_inserted_on_a_branch_count_as_possible_rows() {
    let install = r#"
        FieldSetup."Table ID" := Database::"OB Field Setup";
        FieldSetup."Field ID" := 2;
        if FieldSetup.Insert() then;
        if CompanyName() <> '' then begin
            FieldSetup."Table ID" := 50300;
            FieldSetup.Insert();
        end;"#;
    assert_eq!(outcome(SUBSCRIBER, install, ""), Outcome::Pass);
}

/// B's install code inserts a row for A's table: if B installed first, the subscriber sets the flags.
#[test]
fn a_row_for_the_table_raised_keeps_the_install_refused() {
    let install = r#"
        FieldSetup."Table ID" := 50100;
        FieldSetup."Field ID" := 1;
        if FieldSetup.Insert() then;"#;
    refused(SUBSCRIBER, install, "");
}

/// The table id of the row is not a constant: the row may be for A's table.
#[test]
fn a_row_whose_key_is_not_a_constant_keeps_the_install_refused() {
    let install = r#"
        FieldSetup."Table ID" := CompanyName().Length() + 50000;
        if FieldSetup.Insert() then;"#;
    refused(SUBSCRIBER, install, "");
}

/// A Modify can change the row's values: the rows are not known.
#[test]
fn a_modify_of_the_table_keeps_the_install_refused() {
    let install = r#"
        FieldSetup."Table ID" := Database::"OB Field Setup";
        if FieldSetup.Insert() then;
        FieldSetup."Table ID" := 50100;
        FieldSetup.Modify();"#;
    refused(SUBSCRIBER, install, "");
}

/// The insert runs in a loop: the rows are not known.
#[test]
fn an_insert_in_a_loop_keeps_the_install_refused() {
    let install = r#"
        repeat
            FieldSetup."Table ID" := FieldSetup."Table ID" + 1;
            if FieldSetup.Insert() then;
        until FieldSetup."Table ID" > 50200;"#;
    refused(SUBSCRIBER, install, "");
}

/// The record goes to another procedure, which inserts it: the rows are not known.
#[test]
fn an_insert_through_a_parameter_keeps_the_install_refused() {
    let install = r#"
        FieldSetup."Table ID" := Database::"OB Field Setup";
        Helper.Add(FieldSetup);"#;
    refused(SUBSCRIBER, install, "");
}

/// A filter whose value the table id does not decide: the subscriber's answer is not known.
#[test]
fn a_filter_on_a_value_not_known_keeps_the_install_refused() {
    let body = r#"
        FieldSetup.SetRange("Table ID", TableId);
        FieldSetup.SetRange("Field ID", CompanyName().Length());
        if FieldSetup.IsEmpty() then
            exit;
        OnDatabaseInsert := true;"#;
    let install = r#"
        FieldSetup."Table ID" := 50100;
        FieldSetup."Field ID" := 2;
        if FieldSetup.Insert() then;"#;
    refused(body, install, "");
}

/// A filter on another field that excludes the one row of A's table.
#[test]
fn a_second_filter_that_excludes_the_row_does_not_stop_the_install() {
    let body = r#"
        FieldSetup.SetRange("Table ID", TableId);
        FieldSetup.SetRange("Field ID", 5);
        if FieldSetup.IsEmpty() then
            exit;
        OnDatabaseInsert := true;"#;
    let install = r#"
        FieldSetup."Table ID" := 50100;
        FieldSetup."Field ID" := 2;
        if FieldSetup.Insert() then;"#;
    assert_eq!(outcome(body, install, ""), Outcome::Pass);
}
