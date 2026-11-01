//! Setting a control that is bound to a page variable does not start the new row of a page with a source table: nothing is
//! inserted when the next action runs (Microsoft's Planning Worksheet, DelayedInsert, `CurrentWkshBatchName.SetValue` and then
//! `CalculateRegenerativePlan.Invoke()` run a report, which refuses while a write is pending: Tests-SCM-Planning
//! OrderTrackingForCalcRegenPlanSalesUsingForecastLFLItem).

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50210 "VC Row" { fields { field(1; Id; Code[10]) { } } keys { key(PK; Id) { Clustered = true; } } }
page 50211 "VC Page"
{
    PageType = Worksheet; SourceTable = "VC Row"; DelayedInsert = true;
    layout { area(Content) { field(BatchCtl; Batch) { trigger OnValidate() begin CurrPage.SaveRecord(); end; } repeater(R) { field(IdCtl; Rec.Id) { } } } }
    actions { area(Processing) { action(Check) { trigger OnAction() var R: Record "VC Row"; begin if not R.IsEmpty() then Error('a row was inserted'); end; } } }
    var Batch: Code[10];
}
codeunit 50212 "T"
{
    Subtype = Test;
    [Test]
    procedure SettingAVariableControlInsertsNothing()
    var P: TestPage "VC Page";
    begin
        Commit();
        P.OpenEdit();
        P.BatchCtl.SetValue('B1');
        P.Check.Invoke();
        P.Close();
    end;
}
"#;

#[test]
fn setting_a_variable_control_inserts_no_new_row() {
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
        codeunit: 50212,
        name: "SettingAVariableControlInsertsNothing".into(),
    });
    assert_eq!(o, Outcome::Pass);
}
