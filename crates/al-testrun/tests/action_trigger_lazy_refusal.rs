//! A page action's OnAction that cannot be lowered is refused when the action runs, not before: Base Application's Item Card
//! pages hold actions (Xmlport.Run, Media.ExportFile) that the Tests-SCM tests opening the card never invoke (81 tests). A
//! test that invokes the action is not run (reported with the reason), as before.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
page 50430 "LAZ Page" { PageType = Card;
    layout { area(Content) { field(F; Flag) { } } }
    actions { area(Processing) { action(Boom) { trigger OnAction() begin CodeCoverageLoad(); end; } } }
    var Flag: Boolean; }
codeunit 50430 "T"
{
    Subtype = Test;
    [Test]
    procedure OpeningThePageRuns()
    var P: TestPage "LAZ Page";
    begin
        P.OpenView();
        P.Close();
    end;
    [Test]
    procedure InvokingTheActionIsRefused()
    var P: TestPage "LAZ Page";
    begin
        P.OpenView();
        P.Boom.Invoke();
    end;
}
"#;

#[test]
fn an_unlowerable_action_trigger_is_refused_only_when_invoked() {
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), SRC.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    );
    let o = rt.first_outcome(&CaseId {
        codeunit: 50430,
        name: "OpeningThePageRuns".into(),
    });
    assert_eq!(o, Outcome::Pass, "{o:?}");
    let o = rt.first_outcome(&CaseId {
        codeunit: 50430,
        name: "InvokingTheActionIsRefused".into(),
    });
    assert!(matches!(o, Outcome::NotRun(_)), "{o:?}");
}

// al-runner issue 6014: a refusal in a routine an action's OnAction calls (directly or further down, on the page or on a
// FactBox part) is raised when the action runs too. A test that reaches the routine through its own code is still refused
// before the run.
const REACH: &str = r#"
table 50440 "LAZ Item" { fields { field(1; "No."; Code[20]) { } field(2; Description; Text[100]) { } }
    keys { key(PK; "No.") { Clustered = true; } } }
codeunit 50441 "LAZ Mgt"
{
    procedure Shrink()
    begin
        Inner();
    end;
    procedure Inner()
    begin
        CodeCoverageLoad();
    end;
}
page 50442 "LAZ Part" { PageType = CardPart; SourceTable = "LAZ Item";
    layout { area(Content) { field(Description; Rec.Description) { } } }
    actions { area(Processing) {
        action(PartShrink) { trigger OnAction() var M: Codeunit "LAZ Mgt"; begin M.Shrink(); end; }
        action(PartDirect) { trigger OnAction() var M: Codeunit "LAZ Mgt"; begin M.Inner(); end; } } } }
page 50443 "LAZ Card" { PageType = Card; SourceTable = "LAZ Item";
    layout {
        area(Content) { field("No."; Rec."No.") { } field(Description; Rec.Description) { } }
        area(FactBoxes) { part(PicturePart; "LAZ Part") { SubPageLink = "No." = field("No."); } } }
    actions { area(Processing) {
        action(CardShrink) { trigger OnAction() var M: Codeunit "LAZ Mgt"; begin M.Shrink(); end; } } } }
codeunit 50444 "LAZ Reach Tests"
{
    Subtype = Test;
    [Test]
    procedure OpenCardWithRefusedActions()
    var R: Record "LAZ Item"; P: TestPage "LAZ Card";
    begin
        R.DeleteAll();
        R."No." := 'A1';
        R.Insert();
        P.OpenEdit();
        P.GoToRecord(R);
        P.Description.SetValue('Edited');
        if P.Description.Value() <> 'Edited' then
            Error('shown %1', P.Description.Value());
        P.Close();
        R.Get('A1');
        if R.Description <> 'Edited' then
            Error('stored %1', R.Description);
    end;
    [Test]
    procedure InvokeCardAction()
    var P: TestPage "LAZ Card";
    begin
        P.OpenView();
        P.CardShrink.Invoke();
    end;
    [Test]
    procedure InvokePartAction()
    var P: TestPage "LAZ Card";
    begin
        P.OpenView();
        P.PicturePart.PartShrink.Invoke();
    end;
    [Test]
    procedure InvokePartDirectAction()
    var P: TestPage "LAZ Card";
    begin
        P.OpenView();
        P.PicturePart.PartDirect.Invoke();
    end;
    [Test]
    procedure OwnCodeReachesTheRoutine()
    var M: Codeunit "LAZ Mgt"; P: TestPage "LAZ Card";
    begin
        P.OpenView();
        P.Close();
        if false then
            M.Shrink();
    end;
}
"#;

fn reach_outcomes() -> Vec<(&'static str, Outcome)> {
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), REACH.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    );
    [
        "OpenCardWithRefusedActions",
        "InvokeCardAction",
        "InvokePartAction",
        "InvokePartDirectAction",
        "OwnCodeReachesTheRoutine",
    ]
    .into_iter()
    .map(|name| {
        (
            name,
            rt.first_outcome(&CaseId {
                codeunit: 50444,
                name: name.into(),
            }),
        )
    })
    .collect()
}

#[test]
fn a_refusal_called_from_an_action_is_raised_only_when_the_action_runs() {
    let outcomes = reach_outcomes();
    let get = |n: &str| outcomes.iter().find(|(k, _)| *k == n).map(|(_, o)| o.clone()).unwrap();
    // Opening the card never runs an action of the card or of its FactBox part.
    assert_eq!(get("OpenCardWithRefusedActions"), Outcome::Pass, "{outcomes:?}");
    // Each invoked action reaches the refused builtin: not run, with the reason of the routine that holds it.
    for n in ["InvokeCardAction", "InvokePartAction", "InvokePartDirectAction"] {
        match get(n) {
            Outcome::NotRun(why) => assert!(why.contains("CodeCoverageLoad"), "{n}: {why}"),
            o => panic!("{n}: {o:?}"),
        }
    }
    // The test's own code names the routine: refused before the run, even on a path that never runs it.
    assert!(
        matches!(get("OwnCodeReachesTheRoutine"), Outcome::NotRun(_)),
        "{outcomes:?}"
    );
}

// al-runner issue 6031: the action's OnAction calls a procedure of the page (directly or through another one), and that
// procedure reaches the refused routine. Opening the page runs no action, so the procedure is refused when the action runs,
// as the action trigger itself is (issue 6014). A procedure reachable another way (a trigger that runs on open, a field
// trigger, the test's own code) is still refused before the run. The issue's two shapes side by side: the part whose action
// calls the codeunit (`LZP Part Direct`) and the part whose action calls a local procedure (`LZP Part Local`).
const LOCAL: &str = r#"
table 50450 "LZP Item" { fields { field(1; "No."; Code[20]) { } field(2; Description; Text[100]) { } }
    keys { key(PK; "No.") { Clustered = true; } } }
codeunit 50450 "LZP Mgt"
{
    procedure Shrink()
    begin
        CodeCoverageLoad();
    end;
}
page 50451 "LZP Part Direct" { PageType = CardPart; SourceTable = "LZP Item";
    layout { area(Content) { field(Description; Rec.Description) { } } }
    actions { area(Processing) {
        action(Shrink) { trigger OnAction() var M: Codeunit "LZP Mgt"; begin M.Shrink(); end; } } } }
page 50452 "LZP Card Direct" { PageType = Card; SourceTable = "LZP Item";
    layout {
        area(Content) { field("No."; Rec."No.") { } field(Description; Rec.Description) { } }
        area(FactBoxes) { part(PicturePart; "LZP Part Direct") { SubPageLink = "No." = field("No."); } } } }
page 50453 "LZP Part Local" { PageType = CardPart; SourceTable = "LZP Item";
    layout { area(Content) { field(Description; Rec.Description) { } } }
    actions { area(Processing) {
        action(Shrink) { trigger OnAction() begin ShrinkCurrent(); end; } } }
    local procedure ShrinkCurrent()
    var M: Codeunit "LZP Mgt";
    begin
        M.Shrink();
    end; }
page 50454 "LZP Card Local" { PageType = Card; SourceTable = "LZP Item";
    layout {
        area(Content) { field("No."; Rec."No.") { } field(Description; Rec.Description) { } }
        area(FactBoxes) { part(PicturePart; "LZP Part Local") { SubPageLink = "No." = field("No."); } } } }
page 50455 "LZP Card Own Local" { PageType = Card; SourceTable = "LZP Item";
    layout { area(Content) { field("No."; Rec."No.") { } field(Description; Rec.Description) { } } }
    actions { area(Processing) {
        action(Shrink) { trigger OnAction() begin Outer(); end; } } }
    local procedure Outer()
    begin
        Inner();
    end;
    local procedure Inner()
    var M: Codeunit "LZP Mgt";
    begin
        M.Shrink();
    end; }
page 50456 "LZP On Open" { PageType = Card; SourceTable = "LZP Item";
    layout { area(Content) { field(Description; Rec.Description) { } } }
    actions { area(Processing) {
        action(Shrink) { trigger OnAction() begin ShrinkCurrent(); end; } } }
    trigger OnOpenPage()
    begin
        if false then
            ShrinkCurrent();
    end;
    local procedure ShrinkCurrent()
    var M: Codeunit "LZP Mgt";
    begin
        M.Shrink();
    end; }
page 50457 "LZP Field Trigger" { PageType = Card; SourceTable = "LZP Item";
    layout { area(Content) { field(Description; Rec.Description) {
        trigger OnValidate() begin if false then ShrinkCurrent(); end; } } }
    actions { area(Processing) {
        action(Shrink) { trigger OnAction() begin ShrinkCurrent(); end; } } }
    local procedure ShrinkCurrent()
    var M: Codeunit "LZP Mgt";
    begin
        M.Shrink();
    end; }
page 50458 "LZP Global" { PageType = Card; SourceTable = "LZP Item";
    layout { area(Content) { field(Description; Rec.Description) { } } }
    actions { area(Processing) {
        action(Shrink) { trigger OnAction() begin ShrinkCurrent(); end; } } }
    procedure ShrinkCurrent()
    var M: Codeunit "LZP Mgt";
    begin
        M.Shrink();
    end; }
codeunit 50459 "LZP Tests"
{
    Subtype = Test;
    [Test]
    procedure OpenCardWhosePartActionCallsCodeunit()
    var R: Record "LZP Item"; P: TestPage "LZP Card Direct";
    begin
        R.DeleteAll();
        R."No." := 'A1';
        R.Insert();
        P.OpenEdit();
        P.GoToRecord(R);
        P.Description.SetValue('Edited');
        if P.Description.Value() <> 'Edited' then
            Error('shown %1', P.Description.Value());
        P.Close();
        R.Get('A1');
        if R.Description <> 'Edited' then
            Error('stored %1', R.Description);
    end;
    [Test]
    procedure OpenCardWhosePartActionCallsLocalProcedure()
    var R: Record "LZP Item"; P: TestPage "LZP Card Local";
    begin
        R.DeleteAll();
        R."No." := 'A1';
        R.Insert();
        P.OpenEdit();
        P.GoToRecord(R);
        P.Description.SetValue('Edited');
        if P.Description.Value() <> 'Edited' then
            Error('shown %1', P.Description.Value());
        P.Close();
        R.Get('A1');
        if R.Description <> 'Edited' then
            Error('stored %1', R.Description);
    end;
    [Test]
    procedure OpenCardWhoseOwnActionCallsLocalProcedures()
    var P: TestPage "LZP Card Own Local";
    begin
        P.OpenView();
        P.Close();
    end;
    [Test]
    procedure OpenPageWithGlobalProcedure()
    var P: TestPage "LZP Global";
    begin
        P.OpenView();
        P.Close();
    end;
    [Test]
    procedure InvokePartActionCallingLocalProcedure()
    var P: TestPage "LZP Card Local";
    begin
        P.OpenView();
        P.PicturePart.Shrink.Invoke();
    end;
    [Test]
    procedure InvokeOwnActionCallingLocalProcedures()
    var P: TestPage "LZP Card Own Local";
    begin
        P.OpenView();
        P.Shrink.Invoke();
    end;
    [Test]
    procedure OpenPageWhoseOnOpenCallsTheProcedure()
    var P: TestPage "LZP On Open";
    begin
        P.OpenView();
        P.Close();
    end;
    [Test]
    procedure OpenPageWhoseFieldTriggerCallsTheProcedure()
    var P: TestPage "LZP Field Trigger";
    begin
        P.OpenView();
        P.Close();
    end;
    [Test]
    procedure OwnCodeCallsTheGlobalProcedure()
    var Pg: Page "LZP Global";
    begin
        if false then
            Pg.ShrinkCurrent();
    end;
}
"#;

#[test]
fn a_page_procedure_called_only_from_an_action_is_refused_only_when_the_action_runs() {
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), LOCAL.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    );
    let get = |name: &str| {
        rt.first_outcome(&CaseId {
            codeunit: 50459,
            name: name.into(),
        })
    };
    // Opening a page runs none of its actions, so nothing only an action calls is refused before the run.
    for n in [
        "OpenCardWhosePartActionCallsCodeunit",
        "OpenCardWhosePartActionCallsLocalProcedure",
        "OpenCardWhoseOwnActionCallsLocalProcedures",
        "OpenPageWithGlobalProcedure",
    ] {
        assert_eq!(get(n), Outcome::Pass, "{n}");
    }
    // Invoking the action reaches the refused builtin: not run, with its reason.
    for n in [
        "InvokePartActionCallingLocalProcedure",
        "InvokeOwnActionCallingLocalProcedures",
    ] {
        match get(n) {
            Outcome::NotRun(why) => assert!(why.contains("CodeCoverageLoad"), "{n}: {why}"),
            o => panic!("{n}: {o:?}"),
        }
    }
    // The procedure is reachable another way: a trigger that runs with the page, or the test's own code.
    for n in [
        "OpenPageWhoseOnOpenCallsTheProcedure",
        "OpenPageWhoseFieldTriggerCallsTheProcedure",
        "OwnCodeCallsTheGlobalProcedure",
    ] {
        match get(n) {
            Outcome::NotRun(why) => assert!(why.contains("CodeCoverageLoad"), "{n}: {why}"),
            o => panic!("{n}: {o:?}"),
        }
    }
}
