//! A request page control bound to a report variable shows the value the caller set before `RunModal` and keeps it on
//! OK: Base Application's "Get Demand To Reserve" takes its DemandType through `SetParameters` and a request page with
//! a bound field, and its OnPreDataItem triggers `Break` by it (corpus
//! reportextensiontrigger/TestPrecompiledReportExtensionDataItems.al:DemandSalesOrders_..., DemandAssemblyComponents_...).

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
enum 50103 "RV Kind" { value(0; All) { Caption = 'All'; } value(1; "Sales Orders") { Caption = 'Sales Orders'; } value(4; "Job Usage") { Caption = 'Project Usage'; } }
codeunit 50100 "RV Log" { SingleInstance = true; var V: Integer; procedure Set(N: Integer) begin V := N; end; procedure Get(): Integer begin exit(V); end; }
report 50101 "RV Rpt"
{
    ProcessingOnly = true;
    requestpage { SaveValues = true; layout { area(Content) { group(BatchFilters) { Editable = false; Visible = ShowBatch; field(FromBatchField; FromBatch) { } } group(Settings) { field(ShowField; ShowBatch) { } field(DemandField; DemandType) { Editable = DemandEditable; } } } } trigger OnOpenPage() begin DemandEditable := FromBatch = FromBatch::All; if not DemandEditable then DemandType := FromBatch; end; }
    var DemandType: Enum "RV Kind"; FromBatch: Enum "RV Kind"; DemandEditable: Boolean; ShowBatch: Boolean;
    trigger OnInitReport() begin Clear(FromBatch); end;
    trigger OnPreReport() var L: Codeunit "RV Log"; begin L.Set(DemandType.AsInteger()); end;
    procedure SetParameters(NewDemand: Enum "RV Kind") begin DemandType := NewDemand; end;
}
codeunit 50102 "T"
{
    Subtype = Test;
    [RequestPageHandler]
    procedure H(var RP: TestRequestPage "RV Rpt") begin RP.OK().Invoke(); end;
    [Test]
    [HandlerFunctions('H')]
    procedure SetBeforeRunModal()
    var R: Report "RV Rpt"; L: Codeunit "RV Log";
    begin
        R.SetParameters("RV Kind"::"Sales Orders");
        R.RunModal();
        if L.Get() <> 1 then Error('the report ran with %1', L.Get());
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure SetAfterAnEarlierConfirmedRun()
    var R: Report "RV Rpt"; R2: Report "RV Rpt"; L: Codeunit "RV Log";
    begin
        R.RunModal();
        R2.SetParameters("RV Kind"::"Job Usage");
        R2.RunModal();
        if L.Get() <> 4 then Error('the second run used %1', L.Get());
    end;
}
"#;

#[test]
fn a_value_set_before_runmodal_survives_the_request_page() {
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
        codeunit: 50102,
        name: "SetBeforeRunModal".into(),
    });
    assert_eq!(o, Outcome::Pass);
    let o = rt.first_outcome(&CaseId {
        codeunit: 50102,
        name: "SetAfterAnEarlierConfirmedRun".into(),
    });
    assert_eq!(o, Outcome::Pass);
}
