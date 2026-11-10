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

// al-runner issue 7717: the cases where a wrong "no login" decision still gave a wrong result, from the variant apps of
// the second review of the fix for issue 7605. BC raises the login before the first test, so the login's rows are there
// (LgnLoginChecklistTests.al:137-138).

/// The outcomes and the assumption ids of test codeunit `codeunit` of `main` over the dependency `dep`.
fn outcomes_and_hits(main: &str, dep: &str, codeunit: &str) -> super::tests::OutcomesAndHits {
    super::tests::outcomes_and_hits_of_apps(&[("main", &["dep"], main), ("dep", &[], dep)], codeunit)
}

const OWN_INSERT_ASSUMED: &str = "login-rows-not-needed-by-own-inserter";

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

// Gap 1: one test of the codeunit inserts into the table the login adds rows to, so the codeunit runs without the login
// and the table is not guarded. A read of it marks the assumption: the test that misses the login's row is not run (it
// failed on main), the test that inserts its own row is a flagged pass.
#[test]
fn a_test_whose_codeunit_can_insert_into_the_login_table_is_flagged_and_its_failure_not_run() {
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
    end;
}
"#,
    );
    let (o, hits) = outcomes_and_hits(&main, DEP, "Own Insert Tests");
    match o.as_slice() {
        [(a, Outcome::NotRun(why)), (b, Outcome::Pass)]
            if a == "ReadsTheRowOfTheLogin" && b == "InsertsARowOfItsOwn" =>
        {
            assert!(
                why.contains(OWN_INSERT_ASSUMED)
                    && why.contains("Guided Experience Item")
                    && why.contains("does not exist"),
                "{why}"
            )
        }
        other => panic!("expected a refused failure and a pass, got {other:?}"),
    }
    assert!(hits.iter().all(|(_, h)| h.contains(&OWN_INSERT_ASSUMED)), "{hits:?}");
}

// Gap 1, a wrong pass on main: the count after the test's own insert is 2 on BC. It still passes here, now flagged.
#[test]
fn a_count_after_the_tests_own_insert_into_the_login_table_is_a_flagged_pass() {
    let main = registration_and(
        r#"
codeunit 51202 "Own Insert Count"
{
    Subtype = Test;
    [Test]
    procedure CountAfterOwnInsertIsOne()
    var R: Record "Guided Experience Item";
    begin
        R."Object ID" := 51202;
        R.Insert();
        if R.Count() <> 1 then
            Error('Count is %1', R.Count());
    end;
}
"#,
    );
    let (o, hits) = outcomes_and_hits(&main, DEP, "Own Insert Count");
    assert_eq!(o, vec![("CountAfterOwnInsertIsOne".to_string(), Outcome::Pass)]);
    assert_eq!(
        hits,
        vec![("CountAfterOwnInsertIsOne".to_string(), vec![OWN_INSERT_ASSUMED])]
    );
}

/// Gap 2: an app under test whose `OnAfterLogin` subscriber adds a row to "H Login Row", tables that read it other than
/// through a record method, and one test codeunit per read whose own calls never touch it (no login, the table is
/// guarded).
const READS_PAST_THE_GUARD: &str = r#"
table 51250 "H Login Row"
{
    fields { field(1; Code; Code[20]) { } field(2; Name; Text[50]) { } }
    keys { key(PK; Code) { Clustered = true; } }
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
"#;

// Gap 2: each read is refused, none finds an empty table (on main: 6 wrong failures and 1 wrong pass of these 8).
#[test]
fn reads_of_the_login_table_that_are_not_record_methods_are_refused_without_the_login() {
    for n in 1..=8 {
        let (o, _) = outcomes_and_hits(READS_PAST_THE_GUARD, DEP, &format!("H Read {n}"));
        match o.as_slice() {
            [(_, Outcome::NotRun(why))] if why.contains("the login was not raised") && why.contains("H Login Row") => {}
            other => panic!("H Read {n}: expected a refusal, got {other:?}"),
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
