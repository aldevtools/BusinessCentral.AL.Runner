//! `FPB.PageCaption := text` is the call `FPB.PageCaption(text)` that corpus filterpage/TestFilterPageBuilder.al:600 pins (Base
//! Application's Workflow Event.RunDynamicRequestPage and Exchange Contact Sync assign it; 63 Tests-Workflow tests were refused).

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
codeunit 50480 "T"
{
    Subtype = Test;
    [Test]
    procedure AssignTheCaption()
    var FPB: FilterPageBuilder;
    begin
        FPB.PageCaption := 'Pick your rows';
        if FPB.PageCaption() <> 'Pick your rows' then Error('caption %1', FPB.PageCaption());
    end;
}
"#;

#[test]
fn the_page_caption_of_a_filter_page_builder_can_be_assigned() {
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
        codeunit: 50480,
        name: "AssignTheCaption".into(),
    });
    assert_eq!(o, Outcome::Pass, "{o:?}");
}
