//! `Report.RunRequestPage(Id, Parameters)` with a parameters string whose root has no name and id is refused before the page
//! opens: only SaveAs with such a root is pinned (reports/layouts-saveas/AMRdlcOutput.al:153), and the seed of a request page
//! is not (al-runner issue 7590; corpus probe ReportParametersBareRoot.al queued).

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
report 50520 "RSB Report"
{
    ProcessingOnly = true;
    requestpage { layout { area(Content) { field(StartCtl; StartDate) { } } } }
    var StartDate: Date;
}
codeunit 50521 "T"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('Ok')]
    procedure SeedWithoutNameAndId()
    begin
        Report.RunRequestPage(Report::"RSB Report", '<?xml version="1.0" standalone="yes"?><ReportParameters><Options><Field name="StartDate">2026-01-15</Field></Options></ReportParameters>');
    end;
    [RequestPageHandler]
    procedure Ok(var P: TestRequestPage "RSB Report") begin P.OK().Invoke(); end;
}
"#;

#[test]
fn run_request_page_with_a_root_without_name_and_id_is_refused() {
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
        codeunit: 50521,
        name: "SeedWithoutNameAndId".into(),
    });
    assert!(
        matches!(&o, Outcome::NotRun(m) if m.contains("root has no name and id, given to Report.RunRequestPage")),
        "{o:?}"
    );
}
