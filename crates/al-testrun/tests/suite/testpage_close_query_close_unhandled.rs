//! Under Microsoft's Test Runner codeunit (the default runner mode), a Confirm with no ConfirmHandler, or an Error, in the
//! OnQueryClosePage of a TestPage left open at the end of the test does not fail the test: the close stops there and
//! OnClosePage does not run. An explicit Close() whose OnQueryClosePage asks a Confirm with no ConfirmHandler raises
//! 'Unhandled UI: Confirm <question>' with the code TestWrapped (corpus CI run of corpus branch al-runtime-rs-staging-y65 on
//! every major it runs, testpageclose/lt-y65-close-page-types/Y65ClosePageTypes.al Y65_Open07/08/10/12/13 and
//! Y65_Close_Card_Ask_NoHandler; al-runner issue 6673; assumption test-end-close-query-close-error until that file is on
//! the pinned corpus).

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50580 "TQU Row" { fields { field(1; "Code"; Code[10]) { } } keys { key(PK; "Code") { Clustered = true; } } }
codeunit 50581 "TQU Log"
{
    SingleInstance = true;
    var Log: Text; Mode: Integer;
    procedure Add(T: Text) begin Log += T + ';'; end;
    procedure Take(): Text var R: Text; begin R := Log; Log := ''; exit(R); end;
    procedure SetMode(M: Integer) begin Mode := M; end;
    procedure GetMode(): Integer begin exit(Mode); end;
}
page 50580 "TQU Card"
{
    PageType = Card;
    SourceTable = "TQU Row";
    UsageCategory = None;
    layout { area(Content) { field("Code"; Rec."Code") { } } }
    trigger OnQueryClosePage(CloseAction: Action): Boolean
    var L: Codeunit "TQU Log"; Answer: Boolean;
    begin
        L.Add('qc ' + Format(CloseAction));
        case L.GetMode() of
            1: begin Answer := Confirm('Close the card?', true); L.Add('confirm=' + Format(Answer)); exit(Answer); end;
            2: begin Answer := Confirm('Close the card?', false); L.Add('confirm=' + Format(Answer)); exit(Answer); end;
            3: Error('Close refused.');
        end;
        exit(true);
    end;
    trigger OnClosePage()
    var L: Codeunit "TQU Log";
    begin
        L.Add('close');
    end;
}
codeunit 50580 "TQU Test"
{
    Subtype = Test;

    [Test]
    procedure A_ConfirmDefaultTrueLeftOpen()
    var L: Codeunit "TQU Log"; P: TestPage "TQU Card";
    begin
        L.Take(); L.SetMode(1);
        P.OpenEdit();
    end;

    [Test]
    procedure B_Read()
    var L: Codeunit "TQU Log";
    begin
        L.SetMode(0);
        if L.Take() <> 'qc OK;' then Error('log');
    end;

    [Test]
    procedure C_ConfirmDefaultFalseLeftOpen()
    var L: Codeunit "TQU Log"; P: TestPage "TQU Card";
    begin
        L.Take(); L.SetMode(2);
        P.OpenView();
    end;

    [Test]
    procedure D_Read()
    var L: Codeunit "TQU Log";
    begin
        L.SetMode(0);
        if L.Take() <> 'qc OK;' then Error('log');
    end;

    [Test]
    procedure E_ErrorLeftOpen()
    var L: Codeunit "TQU Log"; P: TestPage "TQU Card";
    begin
        L.Take(); L.SetMode(3);
        P.OpenEdit();
    end;

    [Test]
    procedure F_Read()
    var L: Codeunit "TQU Log";
    begin
        L.SetMode(0);
        if L.Take() <> 'qc OK;' then Error('log');
    end;

    [Test]
    procedure G_ExplicitCloseConfirmNoHandler()
    var L: Codeunit "TQU Log"; P: TestPage "TQU Card";
    begin
        L.Take(); L.SetMode(1);
        P.OpenEdit();
        asserterror P.Close();
        L.SetMode(0);
        if GetLastErrorText() <> 'Unhandled UI: Confirm Close the card?' then Error('text %1', GetLastErrorText());
        if GetLastErrorCode() <> 'TestWrapped' then Error('code %1', GetLastErrorCode());
        if L.Take() <> 'qc OK;' then Error('log');
    end;
}
"#;

#[test]
fn an_unhandled_confirm_or_an_error_in_onqueryclosepage_at_the_end_of_the_test_does_not_fail_it() {
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
        "A_ConfirmDefaultTrueLeftOpen",
        "B_Read",
        "C_ConfirmDefaultFalseLeftOpen",
        "D_Read",
        "E_ErrorLeftOpen",
        "F_Read",
        "G_ExplicitCloseConfirmNoHandler",
    ] {
        let case = CaseId {
            codeunit: 50580,
            name: name.into(),
        };
        assert_eq!(
            rt.first_outcome(&case),
            Outcome::Pass,
            "{name}: {:?}",
            rt.first_outcome(&case)
        );
    }
}
