//! `CodeCoverageRefresh()` is accepted and changes nothing (assumption code-coverage-refresh-is-a-no-op; Code Coverage Mgt.Refresh
//! in the System Application, 9 Tests-SCM-Costing tests were refused for it).

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
codeunit 50520 "T"
{
    Subtype = Test;
    [Test]
    procedure Refresh()
    begin
        CodeCoverageRefresh();
    end;
}
"#;

#[test]
fn code_coverage_refresh_runs() {
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
        codeunit: 50520,
        name: "Refresh".into(),
    });
    assert_eq!(o, Outcome::Pass, "{o:?}");
}
