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

// al-runner issue 7574: the test codeunit reads the row only through the page it opens as a TestPage, in the page's
// OnQueryClosePage and in an action. The page has no source table, so the page variable counts as a write of a table
// that is not known, and the reach of the test then holds the subscribers of the `Global Triggers` database events. In
// Microsoft's apps those write the tables the login subscriber writes; `DATABASE_SUBSCRIBER` stands in for them.
const WIZARD: &str = r#"
page 51140 "Repro Plain Wizard"
{
    PageType = NavigatePage;
    layout { area(Content) { group(Step1) { InstructionalText = 'Step 1'; } } }
    actions
    {
        area(Processing)
        {
            action(ActionCheck)
            {
                trigger OnAction()
                var G: Codeunit "Guided Experience";
                begin
                    if not G.AssistedSetupExistsAndIsNotComplete(51140) then
                        Error('The assisted setup row of the wizard does not exist.');
                end;
            }
        }
    }
    trigger OnQueryClosePage(CloseAction: Action): Boolean
    var G: Codeunit "Guided Experience";
    begin
        if CloseAction = Action::OK then
            if G.AssistedSetupExistsAndIsNotComplete(51140) then
                exit(Confirm('Setup is not complete. Exit anyway?', false));
    end;
}
codeunit 51140 "Repro Wizard Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterAssistedSetup', '', false, false)]
    local procedure HandleOnRegisterAssistedSetup()
    var G: Codeunit "Guided Experience";
    begin
        G.InsertAssistedSetup(51140);
    end;
}
codeunit 51141 "Repro Plain Wizard Test"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    [HandlerFunctions('ConfirmHandler')]
    procedure ClosingTheWizardAsksToConfirm()
    var Wizard: TestPage "Repro Plain Wizard";
    begin
        Wizard.OpenEdit();
        Wizard.Close();
    end;

    [Test]
    procedure AnActionOfTheWizardSeesTheRow()
    var Wizard: TestPage "Repro Plain Wizard";
    begin
        Wizard.OpenView();
        Wizard.ActionCheck.Invoke();
    end;

    [ConfirmHandler]
    procedure ConfirmHandler(Question: Text; var Reply: Boolean)
    begin
        Reply := true;
    end;
}
"#;

const DATABASE_SUBSCRIBER: &str = r#"
codeunit 1992 "Guided Experience Log"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseInsert', '', false, false)]
    local procedure OnInsert(RecRef: RecordRef)
    var G: Record "Guided Experience Item";
    begin
        if G.Get(-1) then
            G.Modify();
    end;
}
"#;

#[test]
fn a_test_that_reads_the_row_only_through_the_page_it_opens_sees_it() {
    let dep = format!("{DEP}{DATABASE_SUBSCRIBER}");
    assert_eq!(
        super::tests::outcomes_over_dependency(WIZARD, &dep),
        vec![
            ("ClosingTheWizardAsksToConfirm".to_string(), Outcome::Pass),
            ("AnActionOfTheWizardSeesTheRow".to_string(), Outcome::Pass)
        ]
    );
}

// A test codeunit that inserts the row itself before the page reads it has it without the login, and runs as before:
// here the dependency's login code stops with an error, so a codeunit that raised the login would not run.
#[test]
fn a_test_that_registers_the_row_itself_before_the_page_reads_it_runs_without_the_login() {
    let dep = format!(
        r#"{DEP}{DATABASE_SUBSCRIBER}
codeunit 1991 "Guided Experience Events"
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpenCompleted', '', false, false)]
    local procedure StopsHere()
    begin
        Error('The login code cannot run here.');
    end;
}}
"#
    );
    let main = WIZARD.replace(
        "    var Wizard: TestPage \"Repro Plain Wizard\";\n    begin\n",
        "    var Wizard: TestPage \"Repro Plain Wizard\"; G: Codeunit \"Guided Experience\";\n    begin\n        if not G.AssistedSetupExistsAndIsNotComplete(51140) then G.InsertAssistedSetup(51140);\n",
    );
    assert_ne!(main, WIZARD);
    assert_eq!(
        super::tests::outcomes_over_dependency(&main, &dep),
        vec![
            ("ClosingTheWizardAsksToConfirm".to_string(), Outcome::Pass),
            ("AnActionOfTheWizardSeesTheRow".to_string(), Outcome::Pass)
        ]
    );
}

// The standard wizard (al-runner issue 7605, and the review of issue 7574 before it): its Finish action calls
// `Guided Experience.CompleteAssistedSetup`, which modifies the row the login registered (System Application: FindSet,
// then ModifyAll(Completed, true)). A Modify cannot create the row, so the test still has it only from the login:
// LgnLoginChecklistTests.al:137-138 pins that it is there before the first test. The page and the first two tests are
// the reporter's repro of issue 7605 (both pass on BC 29.0), which never invokes Finish; the third test invokes it.
const FINISH_WIZARD: &str = r#"
page 51150 "Repro Finish Wizard"
{
    PageType = NavigatePage;
    layout { area(Content) { group(Step1) { InstructionalText = 'Step 1'; } } }
    actions
    {
        area(Processing)
        {
            action(ActionFinish)
            {
                InFooterBar = true;
                trigger OnAction()
                var G: Codeunit "Guided Experience";
                begin
                    G.CompleteAssistedSetup(51150);
                    CurrPage.Close();
                end;
            }
            action(ActionCheck)
            {
                InFooterBar = true;
                trigger OnAction()
                var G: Codeunit "Guided Experience";
                begin
                    if not G.AssistedSetupExistsAndIsNotComplete(51150) then
                        Error('The assisted setup row of the wizard does not exist.');
                end;
            }
        }
    }
    trigger OnQueryClosePage(CloseAction: Action): Boolean
    var G: Codeunit "Guided Experience";
    begin
        if CloseAction = Action::OK then
            if G.AssistedSetupExistsAndIsNotComplete(51150) then
                exit(Confirm('Setup is not complete. Exit anyway?', false));
    end;
}
codeunit 51150 "Repro Finish Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterAssistedSetup', '', false, false)]
    local procedure HandleOnRegisterAssistedSetup()
    var G: Codeunit "Guided Experience";
    begin
        G.InsertAssistedSetup(51150);
    end;
}
codeunit 51151 "Repro Finish Wizard Test"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    [HandlerFunctions('ConfirmHandler')]
    procedure ClosingTheWizardAsksToConfirm()
    var Wizard: TestPage "Repro Finish Wizard";
    begin
        Wizard.OpenEdit();
        Wizard.Close();
    end;

    [Test]
    procedure AnActionOfTheWizardSeesTheRow()
    var Wizard: TestPage "Repro Finish Wizard";
    begin
        Wizard.OpenView();
        Wizard.ActionCheck.Invoke();
    end;

    [Test]
    procedure FinishCompletesTheRow()
    var Wizard: TestPage "Repro Finish Wizard"; G: Codeunit "Guided Experience";
    begin
        Wizard.OpenEdit();
        Wizard.ActionFinish.Invoke();
        if not G.Exists(51150) then
            Error('The assisted setup row does not exist after Finish.');
        if G.AssistedSetupExistsAndIsNotComplete(51150) then
            Error('The assisted setup is still not complete.');
    end;

    [ConfirmHandler]
    procedure ConfirmHandler(Question: Text; var Reply: Boolean)
    begin
        Reply := true;
    end;
}
"#;

/// `DEP` with `CompleteAssistedSetup` and `Exists` on "Guided Experience", in the shape of System Application's code.
fn dep_with_complete() -> String {
    let dep = DEP.replace(
        "    procedure Register() begin OnRegisterAssistedSetup(); end;\n",
        r#"    procedure CompleteAssistedSetup(ObjectID: Integer)
    var G: Record "Guided Experience Item";
    begin
        G.SetRange("Object ID", ObjectID);
        if G.FindSet() then
            G.ModifyAll(Completed, true);
    end;
    procedure Exists(ObjectID: Integer): Boolean
    var G: Record "Guided Experience Item";
    begin
        exit(G.Get(ObjectID));
    end;
    procedure Register() begin OnRegisterAssistedSetup(); end;
"#,
    );
    assert_ne!(dep, DEP);
    dep
}

#[test]
fn a_wizard_whose_finish_action_completes_the_row_sees_the_row_the_login_registered() {
    let dep = format!("{}{DATABASE_SUBSCRIBER}", dep_with_complete());
    assert_eq!(
        super::tests::outcomes_over_dependency(FINISH_WIZARD, &dep),
        vec![
            ("ClosingTheWizardAsksToConfirm".to_string(), Outcome::Pass),
            ("AnActionOfTheWizardSeesTheRow".to_string(), Outcome::Pass),
            ("FinishCompletesTheRow".to_string(), Outcome::Pass)
        ]
    );
}

// The review of the fix for issue 7605: variants of the reporter's wizard. Each adds code the tests never run; the row
// is there before the first test whatever the test does (LgnLoginChecklistTests.al:137-138), so all of them pass on BC.

/// `dep_with_complete` plus a Base Application table whose event subscriber registers an assisted setup, as Microsoft's
/// apps do somewhere behind the events of most of their tables, and the `Global Triggers` subscriber.
fn dep_with_fan_out() -> String {
    format!(
        r#"{}{DATABASE_SUBSCRIBER}
table 18 Customer {{ fields {{ field(1; "No."; Code[20]) {{ }} }} keys {{ key(PK; "No.") {{ Clustered = true; }} }} }}
codeunit 1993 "Customer Checklist"
{{
    [EventSubscriber(ObjectType::Table, Database::Customer, 'OnAfterInsertEvent', '', false, false)]
    local procedure OnAfterInsertCustomer(var Rec: Record Customer)
    var G: Codeunit "Guided Experience";
    begin
        G.InsertAssistedSetup(18);
    end;
}}
"#,
        dep_with_complete()
    )
}

fn all_pass() -> Vec<(String, Outcome)> {
    [
        "ClosingTheWizardAsksToConfirm",
        "AnActionOfTheWizardSeesTheRow",
        "FinishCompletesTheRow",
    ]
    .iter()
    .map(|n| (n.to_string(), Outcome::Pass))
    .collect()
}

/// `FINISH_WIZARD` with `old` replaced by `new`.
fn variant(old: &str, new: &str) -> String {
    let main = FINISH_WIZARD.replace(old, new);
    assert_ne!(main, FINISH_WIZARD);
    main
}

// The Finish action also reads a Base Application table: the events of that table are not the test's own calls.
#[test]
fn a_finish_action_that_also_reads_a_base_application_table_keeps_the_login() {
    let main = variant(
        "                var G: Codeunit \"Guided Experience\";\n                begin\n                    G.CompleteAssistedSetup(51150);\n",
        "                var G: Codeunit \"Guided Experience\"; Cust: Record Customer;\n                begin\n                    if Cust.Get('NOPE') then;\n                    G.CompleteAssistedSetup(51150);\n",
    );
    assert_eq!(
        super::tests::outcomes_over_dependency(&main, &dep_with_fan_out()),
        all_pass()
    );
}

// The Finish action also inserts into a Base Application table, with its triggers.
#[test]
fn a_finish_action_that_also_inserts_into_a_base_application_table_keeps_the_login() {
    let main = variant(
        "                var G: Codeunit \"Guided Experience\";\n                begin\n                    G.CompleteAssistedSetup(51150);\n",
        "                var G: Codeunit \"Guided Experience\"; Cust: Record Customer;\n                begin\n                    Cust.\"No.\" := 'C1'; Cust.Insert(true);\n                    G.CompleteAssistedSetup(51150);\n",
    );
    assert_eq!(
        super::tests::outcomes_over_dependency(&main, &dep_with_fan_out()),
        all_pass()
    );
}

// A temporary record of the table the login adds rows to, and a wizard over that table as a temporary source, insert
// nothing into the table.
#[test]
fn a_temporary_record_or_page_source_of_the_login_table_keeps_the_login() {
    let temp_record = variant(
        "    [ConfirmHandler]\n",
        "    local procedure FillBuffer()\n    var Temp: Record \"Guided Experience Item\" temporary;\n    begin\n        Temp.\"Object ID\" := 1; Temp.Insert();\n    end;\n\n    [ConfirmHandler]\n",
    );
    assert_eq!(
        super::tests::outcomes_over_dependency(&temp_record, &dep_with_fan_out()),
        all_pass()
    );
    let temp_source = variant(
        "    PageType = NavigatePage;\n",
        "    PageType = NavigatePage; SourceTable = \"Guided Experience Item\"; SourceTableTemporary = true;\n",
    );
    assert_eq!(
        super::tests::outcomes_over_dependency(&temp_source, &dep_with_fan_out()),
        all_pass()
    );
}

// The guard: a test that reads and completes the row only in the subscriber of an event it raises. Its own calls do not
// touch the table and its reach writes it, so the login is not raised; the read is refused, it does not find an empty table.
#[test]
fn a_read_of_the_login_table_without_the_login_is_refused() {
    let main = r#"
codeunit 51160 "Guard Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterAssistedSetup', '', false, false)]
    local procedure HandleOnRegisterAssistedSetup()
    var G: Codeunit "Guided Experience";
    begin
        G.InsertAssistedSetup(51160);
    end;
}
codeunit 51161 "Guard Events"
{
    procedure Raise() begin OnCheck(); end;
    [IntegrationEvent(false, false)] local procedure OnCheck() begin end;
}
codeunit 51162 "Guard Subscriber"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guard Events", 'OnCheck', '', false, false)]
    local procedure Check()
    var G: Codeunit "Guided Experience";
    begin
        if not G.Exists(51160) then
            Error('The assisted setup row does not exist.');
        G.CompleteAssistedSetup(51160);
    end;
}
codeunit 51163 "Guard Test"
{
    Subtype = Test;
    [Test]
    procedure TheSubscriberSeesTheRow()
    var E: Codeunit "Guard Events";
    begin
        E.Raise();
    end;
}
"#;
    let got = super::tests::outcomes_over_dependency(main, &dep_with_fan_out());
    match got.as_slice() {
        [(name, Outcome::NotRun(why))] if name == "TheSubscriberSeesTheRow" => assert!(
            why.contains("the login was not raised") && why.contains("Guided Experience Item"),
            "{why}"
        ),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
