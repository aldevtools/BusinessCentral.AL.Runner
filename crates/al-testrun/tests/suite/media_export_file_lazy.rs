//! Media.ExportFile and MediaSet.ExportFile are refused when they run, not before: Base Application's Item Picture page
//! has them in an action's trigger, and the 82 Tests-SCM tests that only open an Item Card never reach it (the corpus skips
//! both calls: streams/lt-stream-read-write-media/P102Media.al line 5, so what they write is not pinned).

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50420 "MEF Row" { fields { field(1; Id; Integer) { } field(2; Pic; MediaSet) { } field(3; One; Media) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 50421 "H"
{
    procedure Export(Run: Boolean)
    var Row: Record "MEF Row";
    begin
        if Run then begin
            Row.One.ExportFile('x.png');
            Row.Pic.ExportFile('y');
        end;
    end;
}
codeunit 50420 "T"
{
    Subtype = Test;
    [Test]
    procedure NeverReachedRuns()
    var H: Codeunit H;
    begin
        H.Export(false);
    end;
    [Test]
    procedure ReachedIsRefused()
    var H: Codeunit H;
    begin
        H.Export(true);
    end;
}
"#;

#[test]
fn an_unreached_export_file_does_not_stop_the_test() {
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
        codeunit: 50420,
        name: "NeverReachedRuns".into(),
    });
    assert_eq!(o, Outcome::Pass, "{o:?}");
    let o = rt.first_outcome(&CaseId {
        codeunit: 50420,
        name: "ReachedIsRefused".into(),
    });
    assert!(matches!(o, Outcome::NotRun(_)), "{o:?}");
}
