// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#![cfg(test)]
//! The login of the test session (`login.rs`, al-runner issue 7343), mirroring corpus master
//! events/platform-triggers/LgnLoginChecklistTests.al (corpus PR 1231). The dependency app stands in for System
//! Application's "System Initialization" (150) and "System Initialization Impl." (151), "Guided Experience" (1990) and
//! Base Application's "Checklist Events" (1997), in the shape of their code: the Company Triggers OnCompanyOpenCompleted
//! subscriber raises OnAfterLogin, whose subscriber raises OnRegisterAssistedSetup for a Web client.

use super::*;

const DEP: &str = r#"
table 1990 "Guided Experience Item" { fields { field(1; "Object ID"; Integer) { } field(2; Completed; Boolean) { } } keys { key(PK; "Object ID") { Clustered = true; } } }
table 1991 "Checklist Setup" { fields { field(1; K; Code[10]) { } field(2; Done; Boolean) { } } keys { key(PK; K) { Clustered = true; } } }
codeunit 150 "System Initialization"
{
    [IntegrationEvent(false, false)] internal procedure OnAfterLogin() begin end;
}
codeunit 151 "System Initialization Impl."
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpenCompleted', '', false, false)]
    local procedure InitCompany()
    var SystemInitialization: Codeunit "System Initialization";
    begin
        SystemInitialization.OnAfterLogin();
    end;
}
codeunit 1990 "Guided Experience"
{
    procedure InsertAssistedSetup(ObjectID: Integer)
    var G: Record "Guided Experience Item";
    begin
        G."Object ID" := ObjectID; G.Insert();
    end;
    procedure AssistedSetupExistsAndIsNotComplete(ObjectID: Integer): Boolean
    var G: Record "Guided Experience Item";
    begin
        if not G.Get(ObjectID) then exit(false);
        exit(not G.Completed);
    end;
    procedure Register() begin OnRegisterAssistedSetup(); end;
    [IntegrationEvent(false, false)] local procedure OnRegisterAssistedSetup() begin end;
}
codeunit 1997 "Checklist Events"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure OnAfterLogIn()
    var
        S: Record "Checklist Setup";
        G: Codeunit "Guided Experience";
    begin
        if not (Session.CurrentClientType() in [ClientType::Web, ClientType::Windows, ClientType::Desktop]) then
            exit;
        if S.Get('') then
            exit;
        G.Register();
        S.Done := true; S.Insert();
    end;
}
"#;

const MAIN: &str = r#"
codeunit 77320 "LGN Login State"
{
    SingleInstance = true;
    var Entries: List of [Text];
    procedure Add(Entry: Text) begin Entries.Add(Entry); end;
    procedure AsText(): Text var E: Text; R: Text; begin foreach E in Entries do R += E + ';'; exit(R); end;
}
codeunit 77321 "LGN Login Subs"
{
    local procedure Note(EventName: Text)
    var State: Codeunit "LGN Login State";
    begin
        State.Add(EventName + '|' + Format(CurrentClientType()) + '|' + Format(GuiAllowed()));
    end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure AfterLogin() begin Note('OnAfterLogin'); end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterAssistedSetup', '', false, false)]
    local procedure RegisterAssistedSetup()
    var G: Codeunit "Guided Experience";
    begin
        Note('OnRegisterAssistedSetup');
        G.InsertAssistedSetup(77322);
    end;
}
codeunit 77323 "LGN Login Checklist Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var State: Codeunit "LGN Login State";

    // LgnLoginChecklistTests.al:123-150
    [Test]
    procedure FirstTest()
    var G: Codeunit "Guided Experience";
    begin
        if State.AsText() <> 'OnRegisterAssistedSetup|Web|Yes;OnAfterLogin|Web|Yes;' then
            Error('entries %1', State.AsText());
        if not G.AssistedSetupExistsAndIsNotComplete(77322) then
            Error('no row');
    end;

    // LgnLoginChecklistTests.al:153-166
    [Test]
    procedure SecondTest()
    var G: Codeunit "Guided Experience";
    begin
        if State.AsText() <> 'OnRegisterAssistedSetup|Web|Yes;OnAfterLogin|Web|Yes;' then
            Error('entries %1', State.AsText());
        if not G.AssistedSetupExistsAndIsNotComplete(77322) then
            Error('no row');
    end;
}
"#;

#[test]
fn the_test_session_raises_the_login_before_the_first_test() {
    let o = super::tests::outcomes_over_dependency(MAIN, DEP);
    assert_eq!(
        o,
        vec![
            ("FirstTest".to_string(), Outcome::Pass),
            ("SecondTest".to_string(), Outcome::Pass)
        ]
    );
}

// The issue's shape: only the row tells, no state of the subscriber's codeunit.
#[test]
fn a_test_that_reads_the_row_a_login_subscriber_registered_sees_it() {
    let main = r#"
codeunit 51020 "Repro Row Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterAssistedSetup', '', false, false)]
    local procedure HandleOnRegisterAssistedSetup()
    var G: Codeunit "Guided Experience";
    begin
        G.InsertAssistedSetup(51020);
    end;
}
codeunit 51021 "Repro Row Test"
{
    Subtype = Test;
    [Test]
    procedure RowExists()
    var G: Codeunit "Guided Experience";
    begin
        if not G.AssistedSetupExistsAndIsNotComplete(51020) then
            Error('The assisted setup row of the wizard does not exist.');
    end;
}
"#;
    assert_eq!(
        super::tests::outcomes_over_dependency(main, DEP),
        vec![("RowExists".to_string(), Outcome::Pass)]
    );
}

// A test codeunit that reads only what the subscribers of events raised inside the login subscriber's calls write runs
// without the login (corpus Y64InstallSubscribers.al reads a table a telemetry logger subscriber of another app writes):
// what counts is what the login subscriber writes through its own calls, not the event fan-out of its reach, which in
// Microsoft's apps covers most of Base Application. Here the dependency's login code stops with an error, so a codeunit
// that raised the login would not run.
#[test]
fn a_test_that_reads_only_the_event_fan_out_of_the_login_subscriber_runs_without_the_login() {
    let dep = DEP.replace(
        "        G.\"Object ID\" := ObjectID; G.Insert();\n",
        "        G.\"Object ID\" := ObjectID; G.Insert();\n        OnAfterInsertAssistedSetup();\n",
    ) + r#"
codeunit 1991 "Guided Experience Events"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpenCompleted', '', false, false)]
    local procedure StopsHere()
    begin
        Error('The login code cannot run here.');
    end;
}
"#;
    let dep = dep.replace(
        "    [IntegrationEvent(false, false)] local procedure OnRegisterAssistedSetup() begin end;",
        "    [IntegrationEvent(false, false)] local procedure OnRegisterAssistedSetup() begin end;\n    [IntegrationEvent(false, false)] local procedure OnAfterInsertAssistedSetup() begin end;",
    );
    let main = r#"
table 51030 "Fan Out Log" { fields { field(1; K; Integer) { } } keys { key(PK; K) { Clustered = true; } } }
codeunit 51030 "Fan Out Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterAssistedSetup', '', false, false)]
    local procedure HandleOnRegisterAssistedSetup()
    var G: Codeunit "Guided Experience";
    begin
        G.InsertAssistedSetup(51030);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnAfterInsertAssistedSetup', '', false, false)]
    local procedure Logs()
    var L: Record "Fan Out Log";
    begin
        L.K := 1;
        if L.Insert() then;
    end;
}
codeunit 51031 "Fan Out Test"
{
    Subtype = Test;
    [Test]
    procedure ReadsTheLog()
    var L: Record "Fan Out Log";
    begin
        if L.Count() > 1 then
            Error('more than one log row');
    end;
}
"#;
    assert_eq!(
        super::tests::outcomes_over_dependency(main, &dep),
        vec![("ReadsTheLog".to_string(), Outcome::Pass)]
    );
}
