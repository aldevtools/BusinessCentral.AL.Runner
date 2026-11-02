//! al-runner issue 6718: two apps under test with no dependency path between them. While app A installs, app B's
//! subscriber to `Global Triggers.GetDatabaseTableTriggerSetup` runs only if BC installed B first (ADR-046 amendment 6:
//! InstallOrderSubscriberTests.al:66-79, :104-117), and that order is not known. When B's subscriber leaves every flag
//! as it gets it for the table asked about, the outcome is the same in both orders, so A's install code runs. A subscriber
//! that sets a flag for the table, or does more than set its flags, still stops it.

use al_sema::workspace::AppSource;
use al_testrun::runtime::CorpusRuntime;
use al_testrun::{CaseId, Outcome};

const A_ID: &str = "11111111-1111-1111-1111-111111116718";
const B_ID: &str = "22222222-2222-2222-2222-222222226718";

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

/// B's subscriber, with `body` as the statements that set the flags.
fn b(body: &str) -> String {
    format!(
        r#"
table 50200 "OB Log Source"
{{
    fields {{ field(1; No; Integer) {{ }} }}
    keys {{ key(PK; No) {{ Clustered = true; }} }}
}}

codeunit 50200 "OB Subscriber"
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", GetDatabaseTableTriggerSetup, '', false, false)]
    local procedure GetDatabaseTableTriggerSetup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean)
    begin
        {body}
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", OnDatabaseInsert, '', false, false)]
    local procedure OnDatabaseInsertEvent(RecRef: RecordRef)
    begin
    end;

    procedure Note()
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

const ONLY_OWN_TABLE: &str = r#"if TableId = Database::"OB Log Source" then begin
            OnDatabaseInsert := true;
            OnDatabaseModify := true;
            OnDatabaseDelete := true;
        end;"#;

/// The reporter's shape: B sets flags only for its own table, so for A's table it changes nothing in either order.
#[test]
fn a_subscriber_that_sets_no_flag_for_the_table_does_not_stop_the_install() {
    assert_eq!(outcome(ONLY_OWN_TABLE), Outcome::Pass);
}

/// The same with a `case` on the table id.
#[test]
fn a_case_on_the_table_id_that_sets_no_flag_for_the_table_does_not_stop_the_install() {
    let body = r#"case TableId of
            Database::"OB Log Source":
                OnDatabaseInsert := true;
        end;"#;
    assert_eq!(outcome(body), Outcome::Pass);
}

fn refused(body: &str) -> String {
    match outcome(body) {
        Outcome::NotRun(why) => why,
        other => panic!("expected not run, got {other:?}"),
    }
}

/// A subscriber that sets a flag for A's table changes what A's install raises if B installed first.
#[test]
fn a_subscriber_that_sets_a_flag_for_the_table_still_stops_the_install() {
    let why = refused("OnDatabaseInsert := true;");
    assert!(why.contains("install order"), "{why}");
}

/// Writing the value a flag already has for every input changes nothing, but a flag written from another one does.
#[test]
fn a_subscriber_that_copies_one_flag_into_another_still_stops_the_install() {
    let why = refused("OnDatabaseModify := OnDatabaseInsert;");
    assert!(why.contains("install order"), "{why}");
}

/// A subscriber that does more than set its flags (here a call) is not known to change nothing.
#[test]
fn a_subscriber_that_calls_code_still_stops_the_install() {
    let why = refused(&format!("Note();\n{ONLY_OWN_TABLE}"));
    assert!(why.contains("install order"), "{why}");
}
