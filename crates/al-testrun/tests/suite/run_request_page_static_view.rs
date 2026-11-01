//! The static `Report.RunRequestPage(id)` shows the request page with the data items' views in force, as `Report.Run` does
//! (corpus report/W3ReportRequestPageRun.al:280, RunRequestPage_OrderOfReportAndRequestPageTriggers: `handler view=[1..3]`).

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50211 "RRV Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 50210 "RRV Log" { SingleInstance = true; var V: Text; procedure Add(S: Text) begin V += S + ';'; end; procedure Take(): Text var R: Text; begin R := V; V := ''; exit(R); end; }
report 50212 "RRV Rpt"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "RRV Row") { DataItemTableView = where(Id = filter(1..3)); RequestFilterFields = Id; } }
    trigger OnInitReport() var L: Codeunit "RRV Log"; begin L.Add('init'); end;
}
codeunit 50213 "T"
{
    Subtype = Test;
    [RequestPageHandler]
    procedure H(var RP: TestRequestPage "RRV Rpt")
    var L: Codeunit "RRV Log";
    begin
        L.Add('view=[' + RP.Row.GetFilter(Id) + ']');
        RP.OK().Invoke();
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure StaticRunRequestPageSeesTheView()
    var L: Codeunit "RRV Log";
    begin
        Report.RunRequestPage(Report::"RRV Rpt");
        if L.Take() <> 'init;view=[1..3];' then Error('log');
    end;
}
"#;

#[test]
fn the_static_runrequestpage_shows_the_data_item_views() {
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
        codeunit: 50213,
        name: "StaticRunRequestPageSeesTheView".into(),
    });
    assert_eq!(o, Outcome::Pass);
}
