//! The read Base Application's "API Webhook Notification Mgt." (codeunit 6153) makes while it answers
//! `Global Triggers.GetDatabaseTableTriggerSetup` under a restricted set: `IsEmpty` of "API Webhook Subscription" with no row
//! passes (permissions/SysFlowFieldPerm.al:489-499). Every neighbor keeps main's refusal: a table with a row, another
//! method, another caller, and a codeunit 6153 of an app that is not Base Application.

use super::y48_platform_probes::outcomes_with_hits_app;
use al_test_cli::report::Outcome;

/// An app with an own table, a subscriber `caller` whose setup answer runs `body`, a set that grants the own table and runs
/// both codeunits but does not grant "API Webhook Subscription", and a test that writes the own table under that set. `seed` runs before the set applies.
fn src(caller: &str, body: &str, seed: &str) -> String {
    let id = caller.split_whitespace().nth(1).expect("codeunit id");
    format!(
        r#"
dotnet {{ assembly("Microsoft.Dynamics.Nav.PermissionTestHelper") {{
  type("Microsoft.Dynamics.Nav.Runtime.PermissionTestHelper"; "PermissionTestHelper") {{ }} }} }}
table 50100 "Own Row" {{ fields {{ field(1; Id; Integer) {{ }} }} keys {{ key(PK; Id) {{ Clustered = true; }} }} }}
{caller}
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", GetDatabaseTableTriggerSetup, '', false, false)]
    local procedure GetDatabaseTableTriggerSetup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    var Sub: Record "API Webhook Subscription";
    begin
        {body}
    end;
}}
permissionset 50100 "NARROW" {{ Assignable = true; Permissions = tabledata "Own Row" = RIMD, codeunit T = X, codeunit {id} = X; }}
codeunit 50000 T {{ Subtype = Test; TestPermissions = Disabled;
  [Test] procedure WriteOwn() var H: DotNet PermissionTestHelper; Row: Record "Own Row"; Sub: Record "API Webhook Subscription"; begin
    {seed}
    H := H.PermissionTestHelper(); H.AddEffectivePermissionSet('NARROW');
    Row.Id := 1; Row.Insert(); end;
}}
"#
    )
}

const MGT: &str = r#"codeunit 6153 "API Webhook Notification Mgt.""#;
const OTHER: &str = r#"codeunit 50101 "Other Setup""#;
const IS_EMPTY: &str = "if Sub.IsEmpty() then;";

fn outcome(app_id: &str, src: &str) -> Outcome {
    let o = outcomes_with_hits_app(app_id, src, "T");
    o.into_iter().find(|(n, _, _)| n == "WriteOwn").expect("WriteOwn").1
}

fn refused(o: Outcome, what: &str) {
    match o {
        Outcome::NotRun(why) => assert!(why.contains("system table"), "{what}: {why}"),
        other => panic!("{what}: expected main's refusal, got {other:?}"),
    }
}

#[test]
fn setup_read_of_an_empty_table_by_base_application_passes() {
    let base = al_ir::BASE_APPLICATION_APP_ID;
    assert_eq!(outcome(base, &src(MGT, IS_EMPTY, "")), Outcome::Pass);
}

#[test]
fn setup_read_neighbors_keep_the_refusal() {
    let base = al_ir::BASE_APPLICATION_APP_ID;
    refused(
        outcome(base, &src(MGT, IS_EMPTY, "Sub.Init(); Sub.Insert();")),
        "table with a row",
    );
    refused(outcome(base, &src(MGT, "if Sub.FindFirst() then;", "")), "FindFirst");
    refused(outcome(base, &src(OTHER, IS_EMPTY, "")), "another caller");
    refused(outcome("main", &src(MGT, IS_EMPTY, "")), "codeunit 6153 of another app");
}
