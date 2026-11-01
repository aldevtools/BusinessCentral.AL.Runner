//! A TestPage local to the test method that is still open when the test ends closes then: OnQueryClosePage runs with the
//! action OK and then OnClosePage, a Confirm in it goes to the test's ConfirmHandler, and a veto (the trigger answers false)
//! does not fail the test; a global TestPage is not closed (testpageclose/Y47CloseAtTestEnd.al, corpus branch
//! al-runtime-rs-staging-y47; al-runner issue 6587; assumption test-end-closes-open-testpages).

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50570 "TEC Card" { fields { field(1; "Code"; Code[10]) { } field(2; Enabled; Boolean) { } } keys { key(PK; "Code") { Clustered = true; } } }
codeunit 50571 "TEC Log"
{
    SingleInstance = true;
    var Log: Text;
    procedure Add(T: Text) begin Log += T + ';'; end;
    procedure Get(): Text begin exit(Log); end;
}
page 50570 "TEC Card"
{
    PageType = Card;
    SourceTable = "TEC Card";
    UsageCategory = None;
    layout { area(Content) { field("Code"; Rec."Code") { } field(Enabled; Rec.Enabled) { } } }
    trigger OnQueryClosePage(CloseAction: Action): Boolean
    var L: Codeunit "TEC Log";
    begin
        L.Add('qcp:' + Format(CloseAction));
        if not Rec.Enabled then
            if not Confirm('The card is not enabled. Are you sure you want to exit?', true) then
                exit(false);
    end;
    trigger OnClosePage()
    var L: Codeunit "TEC Log";
    begin
        L.Add('close');
    end;
}
codeunit 50570 "TEC Test"
{
    Subtype = Test;
    var Reply: Boolean; GlobalPage: TestPage "TEC Card";

    [Test]
    [HandlerFunctions('ConfirmHandler')]
    procedure A_ClosingAtEndOfTestAsksToConfirm()
    var R: Record "TEC Card"; P: TestPage "TEC Card";
    begin
        Reply := true;
        R."Code" := 'A'; R.Insert();
        P.OpenEdit();
        P.GoToRecord(R);
    end;

    [Test]
    procedure B_TheCloseRanAfterTheEarlierTest()
    var L: Codeunit "TEC Log";
    begin
        if L.Get() <> 'qcp:OK;close;' then Error('log %1', L.Get());
    end;

    [Test]
    [HandlerFunctions('ConfirmHandler')]
    procedure C_AVetoAtTheEndDoesNotFailTheTest()
    var R: Record "TEC Card"; P: TestPage "TEC Card";
    begin
        Reply := false;
        R."Code" := 'C'; R.Insert();
        P.OpenView();
        P.GoToRecord(R);
    end;

    [Test]
    procedure D_AGlobalTestPageStaysOpen()
    var R: Record "TEC Card";
    begin
        R."Code" := 'D'; R.Enabled := true; R.Insert();
        GlobalPage.OpenView();
        GlobalPage.GoToRecord(R);
    end;

    [Test]
    procedure E_TheGlobalTestPageIsStillOpen()
    var L: Codeunit "TEC Log";
    begin
        if GlobalPage."Code".Value <> 'D' then Error('code %1', GlobalPage."Code".Value);
        if L.Get() <> 'qcp:OK;close;qcp:OK;' then Error('log %1', L.Get());
        GlobalPage.Close();
    end;

    [ConfirmHandler]
    procedure ConfirmHandler(Question: Text; var R: Boolean)
    begin
        R := Reply;
    end;
}
"#;

#[test]
fn an_open_local_testpage_closes_when_the_test_ends() {
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
    for name in [
        "A_ClosingAtEndOfTestAsksToConfirm",
        "B_TheCloseRanAfterTheEarlierTest",
        "C_AVetoAtTheEndDoesNotFailTheTest",
        "D_AGlobalTestPageStaysOpen",
        "E_TheGlobalTestPageIsStillOpen",
    ] {
        let o = rt.first_outcome(&CaseId {
            codeunit: 50570,
            name: name.into(),
        });
        assert_eq!(o, Outcome::Pass, "{name}: {o:?}");
    }
}
