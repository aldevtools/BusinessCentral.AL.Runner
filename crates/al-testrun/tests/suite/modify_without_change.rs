//! A Modify that changes no field leaves the row version, so a buffer read before it is not stale afterwards, while a Modify that
//! changes a field makes it stale after a Commit (corpus record/TestRecordStaleBufferWrite.al pins the second;
//! record/lt-priority-assumptions/PriorityAssumptions.al:143-244 pins the first, real BC 29).

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50560 "MWC Row" { fields { field(1; Id; Integer) { } field(2; Qty; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 50560 "T"
{
    Subtype = Test;
    [Test]
    procedure UnchangedModifyDoesNotMakeTheBufferStale()
    var Rec: Record "MWC Row"; Stale: Record "MWC Row";
    begin
        Rec.Id := 1; Rec.Qty := 5; Rec.Insert();
        Commit();
        Stale.Get(1);
        Rec.Get(1);
        Rec.Modify();
        Commit();
        Stale.Qty := 7;
        Stale.Modify();
        Rec.Get(1);
        if Rec.Qty <> 7 then Error('qty %1', Rec.Qty);
        Rec.Delete();
        Commit();
    end;
    [Test]
    procedure ChangedModifyMakesTheBufferStale()
    var Rec: Record "MWC Row"; Stale: Record "MWC Row";
    begin
        Rec.Id := 2; Rec.Qty := 5; Rec.Insert();
        Commit();
        Stale.Get(2);
        Rec.Get(2);
        Rec.Qty := 6;
        Rec.Modify();
        Commit();
        Stale.Qty := 7;
        asserterror Stale.Modify();
        if GetLastErrorCode() <> 'DB:RecordChanged' then Error('code %1', GetLastErrorCode());
        Rec.Delete();
        Commit();
    end;
}
"#;

#[test]
fn an_unchanged_modify_keeps_the_row_version() {
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
        "UnchangedModifyDoesNotMakeTheBufferStale",
        "ChangedModifyMakesTheBufferStale",
    ] {
        let o = rt.first_outcome(&CaseId {
            codeunit: 50560,
            name: name.into(),
        });
        assert_eq!(o, Outcome::Pass, "{name}: {o:?}");
    }
}
