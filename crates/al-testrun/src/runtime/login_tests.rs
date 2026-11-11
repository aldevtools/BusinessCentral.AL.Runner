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
table 1996 "Checklist Setup" { fields { field(1; K; Code[10]) { } field(2; Done; Boolean) { } } keys { key(PK; K) { Clustered = true; } } }
codeunit 150 "System Initialization"
{
    procedure IsInProgress(): Boolean var Impl: Codeunit "System Initialization Impl."; begin exit(Impl.IsInProgress()); end;
    [IntegrationEvent(false, false, true)] internal procedure OnAfterLogin() begin end;
}
codeunit 151 "System Initialization Impl."
{
    SingleInstance = true;
    var InitializationInProgress: Boolean;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpenCompleted', '', false, false)]
    local procedure InitCompany()
    var SystemInitialization: Codeunit "System Initialization";
    begin
        InitializationInProgress := true;
        SystemInitialization.OnAfterLogin();
        InitializationInProgress := false;
    end;
    procedure IsInProgress(): Boolean begin exit(InitializationInProgress); end;
}
codeunit 1995 "Checklist"
{
    procedure ShouldInitializeChecklist(Skip: Boolean): Boolean var S: Record "Checklist Setup"; begin exit(not S.Get('')); end;
    procedure MarkChecklistSetupAsDone() var S: Record "Checklist Setup"; begin if S.Get('') then exit; S.Done := true; S.Insert(); end;
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
    procedure Register() begin OnRegisterAssistedSetup(); OnRegisterManualSetup(); OnRegisterGuidedExperienceItem(); end;
    [IntegrationEvent(false, false)] local procedure OnRegisterAssistedSetup() begin end;
    [IntegrationEvent(false, false)] local procedure OnRegisterManualSetup() begin end;
    [IntegrationEvent(false, false)] local procedure OnRegisterGuidedExperienceItem() begin end;
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

// A test codeunit that inserts the row itself before the page reads it runs without the login: here the dependency's
// login code stops with an error, so a codeunit that raised the login would not run. The app's own login subscriber
// registered the row before the first test (LgnLoginChecklistTests.al:137-138), so the test finds it and does not
// insert it again.
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
        "    procedure Register() begin OnRegisterAssistedSetup(); OnRegisterManualSetup(); OnRegisterGuidedExperienceItem(); end;\n",
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
    procedure Register() begin OnRegisterAssistedSetup(); OnRegisterManualSetup(); OnRegisterGuidedExperienceItem(); end;
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

// A test that reads and completes the row only in the subscriber of an event it raises. Its own calls do not touch the
// table and its reach writes it, so the login is not raised; the app's own login subscriber ran in the test session, so
// the row is there (LgnLoginChecklistTests.al:137-138). Until al-runner issue 7717 this read was refused.
#[test]
fn a_test_that_reads_the_row_only_in_an_event_subscriber_sees_it_without_the_login() {
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
    assert_eq!(
        super::tests::outcomes_over_dependency(main, &dep_with_fan_out()),
        vec![("TheSubscriberSeesTheRow".to_string(), Outcome::Pass)]
    );
}

// al-runner issue 7717: the cases where a wrong "no login" decision still gave a wrong result, from the variant apps of
// the second review of the fix for issue 7605. BC raises the login before the first test, so the login's rows are there
// (LgnLoginChecklistTests.al:137-138).

/// The outcomes and the assumption ids of test codeunit `codeunit` of `main` over the dependency `dep`.
fn outcomes_and_hits(main: &str, dep: &str, codeunit: &str) -> super::tests::OutcomesAndHits {
    super::tests::outcomes_and_hits_of_apps(&[("main", &["dep"], main), ("dep", &[], dep)], codeunit)
}

/// An app under test whose `OnRegisterAssistedSetup` subscriber registers object 51200, and `tests`.
fn registration_and(tests: &str) -> String {
    format!(
        r#"
codeunit 51200 "Own Insert Registration"
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterAssistedSetup', '', false, false)]
    local procedure HandleOnRegisterAssistedSetup()
    var G: Codeunit "Guided Experience";
    begin
        G.InsertAssistedSetup(51200);
    end;
}}
{tests}"#
    )
}

// Gap 1: tests of the codeunit insert into the table the login adds rows to, so the codeunit runs without the login.
// The app's login subscriber ran in the test session, so its row (object 51200) is there, as on BC
// (LgnLoginChecklistTests.al:137-138): the count is 3 (the login's row and two of the codeunit's own) and the insert of the login's key fails, as they do on BC. On main
// the first test failed, and the count and the insert of the login's key passed.
#[test]
fn a_test_codeunit_that_inserts_into_the_login_table_itself_sees_the_logins_row() {
    let main = registration_and(
        r#"
codeunit 51201 "Own Insert Tests"
{
    Subtype = Test;
    [Test]
    procedure ReadsTheRowOfTheLogin()
    var G: Codeunit "Guided Experience";
    begin
        if not G.AssistedSetupExistsAndIsNotComplete(51200) then
            Error('The assisted setup row does not exist.');
    end;

    [Test]
    procedure InsertsARowOfItsOwn()
    var G: Codeunit "Guided Experience";
    begin
        G.InsertAssistedSetup(51201);
        if not G.AssistedSetupExistsAndIsNotComplete(51201) then
            Error('The own row does not exist.');
    end;

    [Test]
    procedure CountAfterOwnInsertIsOne()
    var R: Record "Guided Experience Item";
    begin
        R."Object ID" := 51202;
        R.Insert();
        if R.Count() <> 1 then
            Error('Count is %1', R.Count());
    end;

    [Test]
    procedure InsertsTheKeyOfTheLoginRow()
    var G: Codeunit "Guided Experience";
    begin
        G.InsertAssistedSetup(51200);
    end;
}
"#,
    );
    let (o, hits) = outcomes_and_hits(&main, DEP, "Own Insert Tests");
    assert_eq!(o.len(), 4, "{o:?}");
    assert_eq!(o[0], ("ReadsTheRowOfTheLogin".to_string(), Outcome::Pass));
    assert_eq!(o[1], ("InsertsARowOfItsOwn".to_string(), Outcome::Pass));
    assert_eq!(
        o[2],
        (
            "CountAfterOwnInsertIsOne".to_string(),
            Outcome::Fail("Count is 3".into())
        )
    );
    assert!(
        matches!(&o[3], (n, Outcome::Fail(m)) if n == "InsertsTheKeyOfTheLoginRow" && m.contains("already exists")),
        "{o:?}"
    );
    // Each test touches the table the app's login subscriber wrote alone: `login-own-registration-reads-alone`.
    assert!(
        hits.iter()
            .all(|(_, h)| h == &vec!["login-own-registration-reads-alone"]),
        "{hits:?}"
    );
}

// Gap 1 across two tests (review probe "PB Cross Test"): the first test keeps what it read of the login's row in a
// SingleInstance codeunit, the second reads what was kept. The row is there, so all three pass, as on BC.
#[test]
fn a_test_that_keeps_what_the_logins_row_says_for_a_later_test_passes() {
    let main = registration_and(
        r#"
codeunit 51203 "Own Insert State"
{
    SingleInstance = true;
    var Seen: Boolean;
    procedure SetSeen(V: Boolean) begin Seen := V; end;
    procedure GetSeen(): Boolean begin exit(Seen); end;
}
codeunit 51204 "Own Insert Cross Test"
{
    Subtype = Test;
    [Test]
    procedure KeepsWhatTheRowSays()
    var G: Codeunit "Guided Experience"; S: Codeunit "Own Insert State";
    begin
        S.SetSeen(G.AssistedSetupExistsAndIsNotComplete(51200));
    end;

    [Test]
    procedure ReadsWhatTheEarlierTestKept()
    var S: Codeunit "Own Insert State";
    begin
        if not S.GetSeen() then
            Error('The row was not there in the earlier test.');
    end;

    [Test]
    procedure RegistersItsOwnRow()
    var G: Codeunit "Guided Experience";
    begin
        G.InsertAssistedSetup(51209);
    end;
}
"#,
    );
    let (o, _) = outcomes_and_hits(&main, DEP, "Own Insert Cross Test");
    assert_all_pass(&o);
}

// A login subscriber that does not run to its end: what BC does then is not pinned, so the codeunit is not run, as on
// the Raise path.
#[test]
fn a_login_subscriber_that_fails_refuses_the_codeunit() {
    let main = r#"
codeunit 51205 "Failing Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterAssistedSetup', '', false, false)]
    local procedure HandleOnRegisterAssistedSetup()
    var G: Codeunit "Guided Experience";
    begin
        G.InsertAssistedSetup(51205);
        Error('The registration stops here.');
    end;
}
codeunit 51206 "Failing Registration Test"
{
    Subtype = Test;
    [Test]
    procedure InsertsARowOfItsOwn()
    var G: Codeunit "Guided Experience";
    begin
        G.InsertAssistedSetup(51206);
    end;
}
"#;
    let (o, _) = outcomes_and_hits(main, DEP, "Failing Registration Test");
    match o.as_slice() {
        [(_, Outcome::NotRun(why))] => assert!(
            why.contains("login subscriber") && why.contains("The registration stops here."),
            "{why}"
        ),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// Gap 2: an app under test whose `OnAfterLogin` subscriber adds a row to "H Login Row", tables that read it other than
/// through a record method, and one test codeunit per read whose own calls never touch it (the login is not raised).
const READS_PAST_THE_GUARD: &str = r#"
table 51250 "H Login Row"
{
    fields { field(1; Code; Code[20]) { } field(2; Name; Text[50]) { } field(3; Amount; Decimal) { } }
    keys { key(PK; Code) { Clustered = true; } }
}
table 51256 "H Blob" { fields { field(1; K; Integer) { } field(2; Data; Blob) { } } keys { key(PK; K) { Clustered = true; } } }
xmlport 51257 "H Rows Text"
{
    Direction = Export;
    Format = VariableText;
    schema
    {
        textelement(Root)
        {
            tableelement(Row; "H Login Row")
            {
                fieldelement(Code; Row.Code) { }
            }
        }
    }
}
codeunit 51250 "H Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure HandleOnAfterLogin()
    var Row: Record "H Login Row";
    begin
        if Row.Get('LOGIN') then
            exit;
        Row.Code := 'LOGIN';
        Row.Name := 'from login';
        Row.Amount := 5;
        Row.Insert();
    end;
}
table 51251 "H Status"
{
    fields
    {
        field(1; PK; Code[10]) { }
        field(2; "Login Rows"; Integer) { FieldClass = FlowField; CalcFormula = count("H Login Row"); Editable = false; }
        field(3; "Has Login Row"; Boolean) { FieldClass = FlowField; CalcFormula = exist("H Login Row" where(Code = const('LOGIN'))); Editable = false; }
        field(4; "Login Name"; Text[50]) { FieldClass = FlowField; CalcFormula = lookup("H Login Row".Name where(Code = const('LOGIN'))); Editable = false; }
    }
    keys { key(PK; PK) { Clustered = true; } }
}
table 51252 "H Doc"
{
    fields
    {
        field(1; PK; Code[10]) { }
        field(2; "Row Code"; Code[20]) { TableRelation = "H Login Row"; }
    }
    keys { key(PK; PK) { Clustered = true; } }
}
page 51252 "H Doc Card"
{
    PageType = Card;
    SourceTable = "H Doc";
    layout
    {
        area(Content)
        {
            group(General)
            {
                field(PK; Rec.PK) { ApplicationArea = All; }
                field("Row Code"; Rec."Row Code") { ApplicationArea = All; }
            }
        }
    }
}
query 51253 "H Rows"
{
    QueryType = Normal;
    elements
    {
        dataitem(Row; "H Login Row")
        {
            column(Code; Code) { }
        }
    }
}
// The trigger also modifies the login's rows, on a path the test never takes: the test's reach then writes the table,
// so that is no reason to raise the login.
table 51254 "H Trig"
{
    fields { field(1; PK; Code[10]) { } field(2; Cnt; Integer) { } field(3; Stamp; Boolean) { } }
    keys { key(PK; PK) { Clustered = true; } }
    trigger OnInsert()
    var Rows: Query "H Rows"; Row: Record "H Login Row";
    begin
        Rows.Open();
        while Rows.Read() do
            Cnt += 1;
        Rows.Close();
        if Stamp then
            Row.ModifyAll(Name, 'stamped');
    end;
}
codeunit 51260 "H Read 1"
{
    Subtype = Test;
    [Test]
    procedure CountFlowField()
    var S: Record "H Status";
    begin
        S.PK := 'A1'; S.Insert();
        S.CalcFields("Login Rows");
        if S."Login Rows" <> 1 then Error('Login Rows is %1', S."Login Rows");
    end;
}
codeunit 51261 "H Read 2"
{
    Subtype = Test;
    [Test]
    procedure ExistFlowField()
    var S: Record "H Status";
    begin
        S.PK := 'A2'; S.Insert();
        S.CalcFields("Has Login Row");
        if not S."Has Login Row" then Error('Has Login Row is No');
    end;
}
codeunit 51262 "H Read 3"
{
    Subtype = Test;
    [Test]
    procedure LookupFlowField()
    var S: Record "H Status";
    begin
        S.PK := 'A3'; S.Insert();
        S.CalcFields("Login Name");
        if S."Login Name" <> 'from login' then Error('Login Name is %1', S."Login Name");
    end;
}
codeunit 51263 "H Read 4"
{
    Subtype = Test;
    [Test]
    procedure FilterOnFlowFieldFindsNothing()
    var S: Record "H Status";
    begin
        S.PK := 'A5'; S.Insert();
        S.SetRange("Has Login Row", true);
        if not S.IsEmpty() then Error('A status row has the login row.');
    end;
}
codeunit 51264 "H Read 5"
{
    Subtype = Test;
    [Test]
    procedure TableRelationCheckOfAPageField()
    var Doc: TestPage "H Doc Card";
    begin
        Doc.OpenNew();
        Doc.PK.SetValue('B1');
        Doc."Row Code".SetValue('LOGIN');
        Doc."Row Code".AssertEquals('LOGIN');
        Doc.Close();
    end;
}
codeunit 51265 "H Read 6"
{
    Subtype = Test;
    [Test]
    procedure QueryInATableTrigger()
    var T: Record "H Trig";
    begin
        T.PK := 'C1';
        T.Insert(true);
        if T.Cnt <> 1 then Error('Cnt is %1', T.Cnt);
    end;
}
codeunit 51266 "H Read 7"
{
    Subtype = Test;
    [Test]
    procedure RecordRefNextWithoutFind()
    var Ref: RecordRef;
    begin
        Ref.Open(51250);
        if Ref.Next() = 0 then Error('No row.');
    end;
}
codeunit 51267 "H Read 8"
{
    Subtype = Test;
    [Test]
    procedure RecordRefGetBySystemId()
    var Ref: RecordRef;
    begin
        Ref.Open(51250);
        if Ref.GetBySystemId(CreateGuid()) then Error('A row with a new SystemId.');
    end;
}
codeunit 51268 "H Read 9"
{
    Subtype = Test;
    [Test]
    procedure FieldRefCalcSum()
    var Ref: RecordRef; Fld: FieldRef; D: Decimal;
    begin
        Ref.Open(51250);
        Fld := Ref.Field(3);
        Fld.CalcSum();
        D := Fld.Value();
        if D <> 5 then Error('The sum is %1.', D);
    end;
}
codeunit 51269 "H Read 10"
{
    Subtype = Test;
    [Test]
    procedure XmlportExport()
    var B: Record "H Blob" temporary; OutS: OutStream; InS: InStream; Txt: Text;
    begin
        B.Data.CreateOutStream(OutS);
        Xmlport.Export(Xmlport::"H Rows Text", OutS);
        B.Data.CreateInStream(InS);
        InS.ReadText(Txt);
        if not Txt.Contains('LOGIN') then Error('The export has no row: %1', Txt);
    end;
}
"#;

// Gap 2: each read finds the login's row (on main: 8 wrong failures and 1 wrong pass of these 10). The filter on the
// FlowField finds the status row, so that test fails, as it does on BC.
#[test]
fn reads_of_the_login_table_that_are_not_record_methods_see_the_logins_row() {
    for n in 1..=10 {
        let (o, _) = outcomes_and_hits(READS_PAST_THE_GUARD, DEP, &format!("H Read {n}"));
        match n {
            4 => assert_eq!(
                o,
                vec![(
                    "FilterOnFlowFieldFindsNothing".to_string(),
                    Outcome::Fail("A status row has the login row.".into())
                )]
            ),
            _ => assert_all_pass(&o),
        }
    }
}

/// Gap 3: an app under test whose `OnAfterLogin` subscriber fills "K Run Row" through `Codeunit.Run` and "K Child"
/// through the OnInsert trigger of "K Parent".
const SUBSCRIBER_RUNS_AND_TRIGGERS: &str = r#"
table 51300 "K Run Row" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
table 51301 "K Parent"
{
    fields { field(1; Code; Code[20]) { } }
    keys { key(PK; Code) { Clustered = true; } }
    trigger OnInsert()
    var Child: Record "K Child";
    begin
        Child.Code := Rec.Code;
        Child.Insert();
    end;
}
table 51302 "K Child" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
codeunit 51300 "K Seed"
{
    trigger OnRun()
    var Row: Record "K Run Row";
    begin
        if Row.Get('LOGIN') then
            exit;
        Row.Code := 'LOGIN';
        Row.Insert();
    end;
}
codeunit 51301 "K Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure HandleOnAfterLogin()
    var Parent: Record "K Parent";
    begin
        Codeunit.Run(Codeunit::"K Seed");
        if not Parent.Get('LOGIN') then begin
            Parent.Code := 'LOGIN';
            Parent.Insert(true);
        end;
    end;
}
codeunit 51310 "K Run Row Test"
{
    Subtype = Test;
    [Test]
    procedure ReadsTheRowTheLoginLeftThroughCodeunitRun()
    var Row: Record "K Run Row";
    begin
        Row.Get('LOGIN');
    end;
}
codeunit 51311 "K Child Test"
{
    Subtype = Test;
    [Test]
    procedure ReadsTheRowTheLoginLeftThroughOnInsert()
    var Child: Record "K Child";
    begin
        Child.Get('LOGIN');
    end;
}
"#;

// Gap 3: what the login subscriber writes through the codeunit it runs and through a table trigger is what it leaves, so
// a test codeunit that reads it raises the login and finds the row (on main both failed with "does not exist").
#[test]
fn a_test_sees_what_the_login_subscriber_wrote_through_codeunit_run_and_a_table_trigger() {
    for (codeunit, test) in [
        ("K Run Row Test", "ReadsTheRowTheLoginLeftThroughCodeunitRun"),
        ("K Child Test", "ReadsTheRowTheLoginLeftThroughOnInsert"),
    ] {
        let (o, _) = outcomes_and_hits(SUBSCRIBER_RUNS_AND_TRIGGERS, DEP, codeunit);
        assert_eq!(o, vec![(test.to_string(), Outcome::Pass)]);
    }
}

// Round three of the review of issue 7717: the cases where the round-two design (the login subscribers alone in a copy
// of the session, operations that address their rows refused) still gave a wrong result, from the review's probe apps.
// The app's login subscribers run in the test session itself before the first test (LgnLoginChecklistTests.al:131-147:
// both events, in the test session, before the first test; their row is there in both tests), so what they leave is
// there in every form: rows, AutoIncrement counters, the session language, what an event they raise leaves.

/// The outcomes of test codeunit `codeunit` of `main` over the dependency `dep`, under `isolation`, in a session without
/// a client when `no_client` (`--no-client`).
fn outcomes_in(
    main: &str,
    dep: &str,
    codeunit: &str,
    isolation: TestIsolation,
    no_client: bool,
) -> Vec<(String, Outcome)> {
    let (ws, l) = al_sema::lower::compile_apps(
        [("main", vec!["dep".to_string()], main), ("dep", vec![], dep)]
            .into_iter()
            .map(|(id, dependencies, src)| al_sema::workspace::AppSource {
                id: id.into(),
                name: id.into(),
                dependencies,
                files: vec![(format!("{id}.al"), src.into())],
            })
            .collect(),
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let oi = l.program.objects.iter().position(|o| o.name == codeunit).unwrap();
    let mut scratch = Scratch {
        flags: super::session_flags::SessionFlags::of(&RunOptions {
            no_client,
            ..Default::default()
        }),
        ..Default::default()
    };
    run_codeunit_core(
        &engine,
        &l,
        oi,
        None,
        isolation,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut Vec::new(),
        &mut scratch,
        None,
        false,
    )
}

/// Asserts that every test of `o` passed.
fn assert_all_pass(o: &[(String, Outcome)]) {
    assert!(!o.is_empty());
    for (n, r) in o {
        assert_eq!(*r, Outcome::Pass, "{n}: {o:?}");
    }
}

/// An app under test whose `OnAfterLogin` subscriber sets the session language to Turkish and adds a row, a row with a
/// Blob and an amount, a row of a table with an AutoIncrement key, and raises an event of its own app whose subscriber
/// adds a row too (review round two, probe apps p1, p3 and p4).
const LOGIN_LEAVES: &str = r#"
table 51700 "Q Row"
{
    fields { field(1; Code; Code[20]) { } field(2; Name; Text[50]) { } field(3; Pic; Blob) { } field(4; Amount; Decimal) { } }
    keys { key(PK; Code) { Clustered = true; } }
}
table 51701 "Q Auto"
{
    fields { field(1; "Entry No."; Integer) { AutoIncrement = true; } field(2; Name; Text[50]) { } }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
table 51702 "Q Event Row" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
codeunit 51700 "Q Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure HandleOnAfterLogin()
    var
        Row: Record "Q Row";
        Auto: Record "Q Auto";
        OutS: OutStream;
    begin
        GlobalLanguage(1055);
        Row.Code := 'LOGIN';
        Row.Name := 'from login';
        Row.Amount := 5;
        Row.Pic.CreateOutStream(OutS);
        OutS.WriteText('picture');
        Row.Insert();
        Auto.Name := 'from login';
        Auto.Insert();
        OnQLogin();
    end;

    [IntegrationEvent(false, false)]
    local procedure OnQLogin() begin end;
}
codeunit 51701 "Q Event Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Q Login", 'OnQLogin', '', false, false)]
    local procedure HandleOnQLogin()
    var Row: Record "Q Event Row";
    begin
        Row.Code := 'LOGIN';
        Row.Insert();
    end;
}
// The tests insert into the login's tables themselves, so the static rule does not raise the login (p3, p4 "RA").
codeunit 51710 "Q Own Inserts"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // p3: the language the login set is the session's, and Code conversion follows it.
    [Test]
    procedure L1_TheLanguageTheLoginSet()
    var C: Code[10];
    begin
        C := 'i';
        if GlobalLanguage() <> 1055 then
            Error('GlobalLanguage is %1', GlobalLanguage());
        if C <> 'İ' then
            Error('Code of i is %1', C);
    end;

    // p4 A3: the login's row took entry 1.
    [Test]
    procedure A3_AutoIncrementAfterTheLoginRow()
    var Auto: Record "Q Auto";
    begin
        Auto.Name := 'own';
        Auto.Insert();
        if Auto."Entry No." <> 2 then
            Error('Entry No. is %1', Auto."Entry No.");
    end;

    // p4 A4: the Blob of the login's row, by a key set by hand.
    [Test]
    procedure A4_CalcFieldsOfABlobByAKeySetByHand()
    var Row: Record "Q Row";
    begin
        Row.Code := 'LOGIN';
        Row.CalcFields(Pic);
        if not Row.Pic.HasValue() then
            Error('The Blob of the login row is empty.');
    end;

    // p4 B4: the insert of the login's key fails.
    [Test]
    procedure B4_InsertOfTheLoginKey()
    var Row: Record "Q Row";
    begin
        Row.Code := 'LOGIN';
        if Row.Insert() then
            Error('The insert of the login key worked.');
    end;

    // p4 B5: an OR filter group finds the own row by its code and the login's row by its name.
    [Test]
    procedure B5_OrGroupThatLetsTheLoginRowIn()
    var Row: Record "Q Row";
    begin
        Row.Code := 'OWN';
        Row.Name := 'own';
        Row.Insert();
        Row.Reset();
        Row.FilterGroup(-1);
        Row.SetRange(Code, 'OWN');
        Row.SetRange(Name, 'from login');
        if Row.Count() <> 2 then
            Error('Count is %1', Row.Count());
    end;
}
// No test touches the login's tables through its own calls: p4 "RE" (writes through a RecordRef) and p1 E1.
codeunit 51711 "Q Ref Writes"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure E1_RecordRefModifyOfTheLoginRow()
    var Ref: RecordRef;
    begin
        Ref.Open(TableNo());
        Ref.Field(1).Value := 'LOGIN';
        Ref.Get(Ref.RecordId());
        Ref.Field(2).Value := 'changed';
        Ref.Modify();
    end;

    [Test]
    procedure E2_RecordRefInsertOfTheLoginKey()
    var Ref: RecordRef;
    begin
        Ref.Open(TableNo());
        Ref.Field(1).Value := 'LOGIN';
        if Ref.Insert() then
            Error('The insert of the login key worked.');
    end;

    [Test]
    procedure E3_RecordRefDeleteOfTheLoginRow()
    var Ref: RecordRef;
    begin
        Ref.Open(TableNo());
        Ref.Field(1).Value := 'LOGIN';
        if not Ref.Delete() then
            Error('The login row could not be deleted.');
    end;

    local procedure TableNo(): Integer
    var N: Integer;
    begin
        Evaluate(N, '51700');
        exit(N);
    end;
}
codeunit 51712 "Q Own Event"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure E1_ReadsTheRowAnOwnEventSubscriberLeft()
    var Row: Record "Q Event Row";
    begin
        Row.Get('LOGIN');
    end;
}
codeunit 51713 "Q Untouched"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure TouchesNothingOfTheLogin()
    begin
        if 1 + 1 <> 2 then
            Error('arithmetic');
    end;
}
"#;

// The round-two wrong results (p3 language, p4 A3 and A4, RecordRef writes, p1 E1) give BC's result.
#[test]
fn what_the_apps_own_login_subscribers_leave_is_there_in_every_form() {
    for codeunit in ["Q Own Inserts", "Q Ref Writes", "Q Own Event", "Q Untouched"] {
        assert_all_pass(&outcomes_in(
            LOGIN_LEAVES,
            DEP,
            codeunit,
            TestIsolation::Codeunit,
            false,
        ));
    }
}

/// Whether `o` is a refusal because what the login does in this session is not known.
fn refused_unmodelled(o: &Outcome, what: &str) -> bool {
    matches!(o, Outcome::NotRun(why) if why.contains("login") && why.contains(what))
}

// A session without a client (`--no-client`): which login events BC raises there is not pinned. The app's login
// subscribers run, watched: when they set the session language, the codeunit is refused; a test that touches a table
// they wrote, through any path, is not run; one that touches none passes.
#[test]
fn without_a_client_what_the_login_would_leave_is_refused() {
    let o = outcomes_in(LOGIN_LEAVES, DEP, "Q Untouched", TestIsolation::Codeunit, true);
    assert!(
        refused_unmodelled(&o[0].1, "session without a client")
            && matches!(&o[0].1, Outcome::NotRun(w) if w.contains("language")),
        "{o:?}"
    );
    let rows_only = LOGIN_LEAVES.replace("        GlobalLanguage(1055);\n", "");
    // The table of a RecordRef is not known statically: the login is not raised, and the watch refuses each write.
    let o = outcomes_in(&rows_only, DEP, "Q Ref Writes", TestIsolation::Codeunit, true);
    for (n, r) in &o {
        assert!(refused_unmodelled(r, "session without a client"), "{n}: {o:?}");
    }
    // A test that inserts into the login's tables is not run; one that only reads them raises the login, which refuses
    // there (Microsoft's login code asks CurrentClientType). The test of the language touches no table: it runs (the login
    // of this variant leaves the language alone).
    for codeunit in ["Q Own Inserts", "Q Own Event"] {
        let o = outcomes_in(&rows_only, DEP, codeunit, TestIsolation::Codeunit, true);
        for (n, r) in o.iter().filter(|(n, _)| n != "L1_TheLanguageTheLoginSet") {
            assert!(
                matches!(r, Outcome::NotRun(w) if w.contains("session without a client")),
                "{n}: {o:?}"
            );
        }
    }
    assert_all_pass(&outcomes_in(
        &rows_only,
        DEP,
        "Q Untouched",
        TestIsolation::Codeunit,
        true,
    ));
}

// RequiredTestIsolation = Disabled: what the login writes would meet what an earlier codeunit left, which is not pinned
// (the same reason a codeunit whose session raised the login is not run there, `run.rs`). The subscribers run, watched:
// a test that touches a table they wrote is not run, one that touches none passes.
#[test]
fn under_disabled_isolation_what_the_login_would_leave_is_refused() {
    let rows_only = LOGIN_LEAVES.replace("        GlobalLanguage(1055);\n", "");
    let o = outcomes_in(&rows_only, DEP, "Q Ref Writes", TestIsolation::Disabled, false);
    for (n, r) in &o {
        assert!(refused_unmodelled(r, "Disabled"), "{n}: {o:?}");
    }
    assert_all_pass(&outcomes_in(
        &rows_only,
        DEP,
        "Q Untouched",
        TestIsolation::Disabled,
        false,
    ));
}

// Round four (review of issue 7717, round three). The app's subscribers run alone only where that gives BC's state.

/// HIGH 1 (probe p5): an `OnAfterLogin` subscriber that notes what Microsoft's login code set before it (the checklist
/// set up, the login in progress), and two test codeunits: one only reads (the login is raised), one inserts its own row
/// first. On BC both sessions get the same login, so both see the same note.
const READS_MICROSOFTS_LOGIN: &str = r#"
table 51800 "R Note" { fields { field(1; Code; Code[20]) { } field(2; Txt; Text[250]) { } } keys { key(PK; Code) { Clustered = true; } } }
codeunit 51800 "R Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure AfterLogin()
    var Note: Record "R Note"; Checklist: Codeunit Checklist; SysInit: Codeunit "System Initialization";
    begin
        Note.Code := 'AFTER';
        Note.Txt := 'ShouldInit=' + Format(Checklist.ShouldInitializeChecklist(false)) + ' InProgress=' + Format(SysInit.IsInProgress());
        Note.Insert();
    end;
}
codeunit 51810 "RA Reads Only"
{
    Subtype = Test;
    [Test]
    procedure A1_WhatTheSubscriberSaw()
    var Note: Record "R Note";
    begin
        Note.Get('AFTER');
        if Note.Txt <> 'ShouldInit=No InProgress=Yes' then Error('SAW %1', Note.Txt);
    end;
}
codeunit 51811 "RB Inserts"
{
    Subtype = Test;
    [Test]
    procedure B1_WhatTheSubscriberSaw()
    var Note: Record "R Note";
    begin
        Note.Code := 'OWN'; Note.Insert();
        Note.Get('AFTER');
        if Note.Txt <> 'ShouldInit=No InProgress=Yes' then Error('SAW %1', Note.Txt);
    end;
}
"#;

// A subscriber that reads what Microsoft's login code sets before it gets the login raised, so both codeunits see what
// they see on BC.
#[test]
fn a_subscriber_that_reads_what_microsofts_login_sets_gets_the_login_raised() {
    for codeunit in ["RA Reads Only", "RB Inserts"] {
        assert_all_pass(&outcomes_in(
            READS_MICROSOFTS_LOGIN,
            DEP,
            codeunit,
            TestIsolation::Codeunit,
            false,
        ));
    }
}

/// HIGH 2 (probe p6c): the app's install code marks the checklist as set up, so Microsoft's login code does not raise
/// OnRegisterAssistedSetup; the subscribers only log, they read nothing of Microsoft's.
const CHECKLIST_DONE_AT_INSTALL: &str = r#"
table 51880 "T Log" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
codeunit 51880 "T Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure AfterLogin() var L: Record "T Log"; begin L.Code := 'AFTER'; L.Insert(); end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterAssistedSetup', '', false, false)]
    local procedure Register() var L: Record "T Log"; begin L.Code := 'REG'; L.Insert(); end;
}
codeunit 51881 "T Install"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() var Checklist: Codeunit Checklist; begin Checklist.MarkChecklistSetupAsDone(); end;
}
codeunit 51890 "TA Reads Only"
{
    Subtype = Test;
    [Test]
    procedure A1_Log() var L: Record "T Log"; begin if L.Get('REG') then Error('REG is there'); L.Get('AFTER'); end;
}
codeunit 51891 "TB Inserts"
{
    Subtype = Test;
    [Test]
    procedure B1_Log() var L: Record "T Log"; begin L.Code := 'OWN'; L.Insert(); if L.Get('REG') then Error('REG is there'); L.Get('AFTER'); end;
}
"#;

// With the checklist set up, OnRegisterAssistedSetup is not raised: Microsoft's code decides, in both codeunits.
#[test]
fn with_the_checklist_set_up_the_registration_event_is_not_raised() {
    for codeunit in ["TA Reads Only", "TB Inserts"] {
        assert_all_pass(&outcomes_in(
            CHECKLIST_DONE_AT_INSTALL,
            DEP,
            codeunit,
            TestIsolation::Codeunit,
            false,
        ));
    }
}

/// MEDIUM 1 (probes p8, p9): an `OnAfterLogin` subscriber that inserts a row and then raises an error. OnAfterLogin is an
/// isolated event: the error is rolled back with the insert and the session goes on
/// (codeunits/TestIsolatedEventSubscriberError.al).
const FAILING_AFTER_LOGIN: &str = r#"
table 51895 "U Row" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
codeunit 51895 "U Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure AfterLogin() var Row: Record "U Row"; begin Row.Code := 'BEFORE ERROR'; Row.Insert(); Error('U Login fails'); end;
}
codeunit 51896 "UA Reads Only"
{
    Subtype = Test;
    [Test]
    procedure A1_Count() var Row: Record "U Row"; begin if Row.Count() <> 0 then Error('Count=%1', Row.Count()); end;
}
codeunit 51897 "UB Inserts"
{
    Subtype = Test;
    [Test]
    procedure B1_Count() var Row: Record "U Row"; begin Row.Code := 'OWN'; Row.Insert(); if Row.Count() <> 1 then Error('Count=%1', Row.Count()); end;
}
codeunit 51898 "UC Unrelated"
{
    Subtype = Test;
    [Test]
    procedure C1_Text() begin if UpperCase('a') <> 'A' then Error('text'); end;
}
"#;

// The failing subscriber's insert is rolled back and every codeunit runs, as on the Raise path.
#[test]
fn an_error_in_an_after_login_subscriber_is_rolled_back_and_the_codeunit_runs() {
    for codeunit in ["UA Reads Only", "UB Inserts", "UC Unrelated"] {
        assert_all_pass(&outcomes_in(
            FAILING_AFTER_LOGIN,
            DEP,
            codeunit,
            TestIsolation::Codeunit,
            false,
        ));
    }
}

/// MEDIUM 2 (probe p7): under Disabled isolation, an `OnAfterLogin` subscriber sets a SingleInstance flag through a
/// helper codeunit, and another binds a manual subscriber.
const WATCHED_STATE: &str = r#"
codeunit 51900 "V Flag" { SingleInstance = true; var Flag: Boolean; procedure SetFlag() begin Flag := true; end; procedure GetFlag(): Boolean begin exit(Flag); end; }
codeunit 51901 "V Helper" { procedure Mark() var F: Codeunit "V Flag"; begin F.SetFlag(); end; }
codeunit 51902 "V Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure AfterLogin() var H: Codeunit "V Helper"; begin H.Mark(); end;
}
codeunit 51910 "VA Reads The Flag"
{
    Subtype = Test;
    [Test]
    procedure A1_Flag() var F: Codeunit "V Flag"; begin if not F.GetFlag() then Error('no flag'); end;
}
codeunit 51911 "VB Unrelated"
{
    Subtype = Test;
    [Test]
    procedure B1_Text() begin if UpperCase('a') <> 'A' then Error('text'); end;
}
"#;

/// The same with a subscriber that binds a manual subscriber at login.
const WATCHED_BINDING: &str = r#"
codeunit 51920 "W Manual" { EventSubscriberInstance = Manual; SingleInstance = true; }
codeunit 51921 "W Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure AfterLogin() var M: Codeunit "W Manual"; begin BindSubscription(M); end;
}
codeunit 51930 "WA Test"
{
    Subtype = Test;
    [Test]
    procedure A1_Text() begin if UpperCase('a') <> 'A' then Error('text'); end;
}
"#;

// Under Disabled isolation, a codeunit that reads a global the login subscribers write (also through a helper), and any
// codeunit when they bind a manual subscriber, is not run; another runs.
#[test]
fn under_disabled_isolation_state_the_login_leaves_besides_rows_is_refused() {
    let o = outcomes_in(WATCHED_STATE, DEP, "VA Reads The Flag", TestIsolation::Disabled, false);
    assert!(refused_unmodelled(&o[0].1, "codeunit global"), "{o:?}");
    assert_all_pass(&outcomes_in(
        WATCHED_STATE,
        DEP,
        "VB Unrelated",
        TestIsolation::Disabled,
        false,
    ));
    let o = outcomes_in(WATCHED_BINDING, DEP, "WA Test", TestIsolation::Disabled, false);
    assert!(refused_unmodelled(&o[0].1, "bind a manual event subscriber"), "{o:?}");
}

// Round five (review of issue 7717, round four).

/// HIGH 1 (probes p12, p12b): the app subscribes to the registration events Microsoft's code raises after
/// OnRegisterAssistedSetup at login. Their raise is not pinned, so the login is raised through Microsoft's code.
fn registration_events(with_assisted: bool) -> String {
    let assisted = if with_assisted {
        "    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Guided Experience\", 'OnRegisterAssistedSetup', '', false, false)]\n    local procedure A() begin Log('ASSISTED'); end;\n"
    } else {
        ""
    };
    let expected = if with_assisted {
        "ASSISTED;MANUAL;ITEM;"
    } else {
        "MANUAL;ITEM;"
    };
    format!(
        r#"
table 51940 "X Log" {{ fields {{ field(1; No; Integer) {{ AutoIncrement = true; }} field(2; Txt; Text[30]) {{ }} }} keys {{ key(PK; No) {{ Clustered = true; }} }} }}
codeunit 51940 "X Login"
{{
    local procedure Log(T: Text) var L: Record "X Log"; begin L.Txt := T; L.Insert(); end;
{assisted}    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterManualSetup', '', false, false)]
    local procedure M() begin Log('MANUAL'); end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterGuidedExperienceItem', '', false, false)]
    local procedure I() begin Log('ITEM'); end;
}}
codeunit 51941 "XA Reads Only"
{{
    Subtype = Test;
    [Test]
    procedure A1() var L: Record "X Log"; T: Text; begin if L.FindSet() then repeat T += L.Txt + ';'; until L.Next() = 0; if T <> '{expected}' then Error('LOG %1', T); end;
}}
codeunit 51942 "XB Inserts"
{{
    Subtype = Test;
    [Test]
    procedure B1() var L: Record "X Log"; T: Text; begin L.Txt := 'OWN'; L.Insert(); L.Reset(); L.SetFilter(Txt, '<>OWN'); if L.FindSet() then repeat T += L.Txt + ';'; until L.Next() = 0; if T <> '{expected}' then Error('LOG %1', T); end;
}}
"#
    )
}

// Both codeunits see the rows of all three registration events, with and without an OnRegisterAssistedSetup subscriber.
#[test]
fn subscribers_of_the_later_registration_events_get_the_login_raised() {
    for with_assisted in [true, false] {
        let main = registration_events(with_assisted);
        for codeunit in ["XA Reads Only", "XB Inserts"] {
            let o = outcomes_in(&main, DEP, codeunit, TestIsolation::Codeunit, false);
            assert!(
                o.iter().all(|(_, r)| *r == Outcome::Pass),
                "{with_assisted} {codeunit}: {o:?}"
            );
        }
    }
}

/// HIGH 2 (probe p10): the dependency registers its own wizard (object 1803) at login. The app's
/// OnRegisterAssistedSetup subscriber registers the app's own wizard (keyed on its own object), and its OnAfterLogin
/// subscriber asks whether the dependency's row exists.
const READS_MICROSOFTS_ROW: &str = r#"
table 51950 "Y Note" { fields { field(1; Code; Code[20]) { } field(2; Txt; Text[50]) { } } keys { key(PK; Code) { Clustered = true; } } }
page 51950 "Y Wizard" { PageType = NavigatePage; layout { area(Content) { group(S) { InstructionalText = 'Step 1'; } } } }
codeunit 51950 "Y Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterAssistedSetup', '', false, false)]
    local procedure Register() var G: Codeunit "Guided Experience"; begin G.InsertAssistedSetup(Page::"Y Wizard"); end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure AfterLogin() var G: Codeunit "Guided Experience"; N: Record "Y Note";
    begin
        N.Code := 'AFTER';
        N.Txt := 'MsItem=' + Format(G.AssistedSetupExistsAndIsNotComplete(1803));
        N.Insert();
    end;
}
codeunit 51951 "YA Reads Only"
{
    Subtype = Test;
    [Test]
    procedure A1() var N: Record "Y Note"; begin N.Get('AFTER'); if N.Txt <> 'MsItem=Yes' then Error('SAW %1', N.Txt); end;
}
codeunit 51952 "YB Inserts"
{
    Subtype = Test;
    [Test]
    procedure B1() var N: Record "Y Note"; begin N.Code := 'OWN'; N.Insert(); N.Get('AFTER'); if N.Txt <> 'MsItem=Yes' then Error('SAW %1', N.Txt); end;
}
"#;

/// The dependency's own registration at login.
const DEP_REGISTRATION: &str = r#"
codeunit 1998 "Dep Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterAssistedSetup', '', false, false)]
    local procedure Register() var G: Codeunit "Guided Experience"; begin G.InsertAssistedSetup(1803); end;
}
"#;

// A subscriber that reads a row the dependency's login code writes, from a table it writes itself, gets the login
// raised: both codeunits see the dependency's row.
#[test]
fn a_subscriber_that_reads_the_dependencys_registration_gets_the_login_raised() {
    let dep = format!("{DEP}{DEP_REGISTRATION}");
    for codeunit in ["YA Reads Only", "YB Inserts"] {
        assert_all_pass(&outcomes_in(
            READS_MICROSOFTS_ROW,
            &dep,
            codeunit,
            TestIsolation::Codeunit,
            false,
        ));
    }
}

// A registration keyed on the app's own object stays Own, and a test that touches the table it wrote is a flagged pass
// (`login-own-registration-reads-alone`).
#[test]
fn a_test_that_touches_what_the_own_subscribers_wrote_is_flagged() {
    let main = READS_MICROSOFTS_ROW.replace(
        "G.AssistedSetupExistsAndIsNotComplete(1803)",
        "G.AssistedSetupExistsAndIsNotComplete(Page::\"Y Wizard\")",
    );
    let dep = format!("{DEP}{DEP_REGISTRATION}");
    let (o, hits) = outcomes_and_hits(&main, &dep, "YB Inserts");
    assert_eq!(o, vec![("B1".to_string(), Outcome::Pass)], "{hits:?}");
    assert!(
        hits.iter()
            .any(|(_, h)| h.contains(&"login-own-registration-reads-alone")),
        "{hits:?}"
    );
}

/// MEDIUM 1 (probe p9b): the OnRegisterAssistedSetup subscriber writes, then an OnAfterLogin subscriber fails. On BC the
/// registration commits with the Checklist Events subscriber before the app's OnAfterLogin subscriber runs, so the
/// failing subscriber is rolled back alone.
const REGISTERS_THEN_FAILS: &str = r#"
table 51960 "Z Row" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
codeunit 51960 "Z Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterAssistedSetup', '', false, false)]
    local procedure Register() var R: Record "Z Row"; begin R.Code := 'REG'; R.Insert(); end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure AfterLogin() var R: Record "Z Row"; begin R.Code := 'AFTER'; R.Insert(); Error('Z Login fails'); end;
}
codeunit 51961 "ZA Reads Only"
{
    Subtype = Test;
    [Test]
    procedure A1() var R: Record "Z Row"; begin R.Get('REG'); if R.Get('AFTER') then Error('AFTER is there'); end;
}
codeunit 51962 "ZB Inserts"
{
    Subtype = Test;
    [Test]
    procedure B1() var R: Record "Z Row"; begin R.Code := 'OWN'; R.Insert(); R.Get('REG'); if R.Get('AFTER') then Error('AFTER is there'); end;
}
codeunit 51963 "ZC Unrelated"
{
    Subtype = Test;
    [Test]
    procedure C1() begin if UpperCase('a') <> 'A' then Error('text'); end;
}
"#;

// The registration's row stays, the failing subscriber's row is rolled back, and every codeunit runs.
#[test]
fn a_failing_after_login_subscriber_is_rolled_back_alone_after_a_registration() {
    for codeunit in ["ZA Reads Only", "ZB Inserts", "ZC Unrelated"] {
        assert_all_pass(&outcomes_in(
            REGISTERS_THEN_FAILS,
            DEP,
            codeunit,
            TestIsolation::Codeunit,
            false,
        ));
    }
}
