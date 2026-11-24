//! `@` alone on a Code field as a DataTransfer source filter inside install code: no cell measures it (al-builtins
//! `filter_at_sign`; corpus PR 1495 measures Record.SetFilter, FieldRef.SetFilter and SetView), so the copy refuses, as before
//! that rule answered `@` alone on a Code field.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50598 "DTA Src" { fields { field(1; "Code"; Code[20]) { } field(2; Kind; Code[20]) { } } keys { key(PK; "Code") { Clustered = true; } } }
table 50599 "DTA Dst" { fields { field(1; "Code"; Code[20]) { } } keys { key(PK; "Code") { Clustered = true; } } }
codeunit 50598 "DTA Install"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var S: Record "DTA Src"; D: Record "DTA Dst"; DT: DataTransfer;
    begin
        S."Code" := 'A'; S.Kind := 'K'; S.Insert();
        S."Code" := 'B'; S.Kind := ''; S.Insert();
        DT.SetTables(Database::"DTA Src", Database::"DTA Dst");
        DT.AddSourceFilter(S.FieldNo(Kind), 'FILTER_TEXT');
        DT.AddFieldValue(S.FieldNo("Code"), D.FieldNo("Code"));
        DT.CopyRows();
    end;
}
codeunit 50599 "DTA Test"
{
    Subtype = Test;
    [Test]
    procedure TheCopyRan()
    var D: Record "DTA Dst";
    begin
        if D.Count() <> 1 then Error('%1', D.Count());
    end;
}
"#;

#[test]
fn datatransfer_at_sign_alone_code_filter_refused() {
    for (filter, named) in [("@", "`@` alone"), ("<>@", "`<>@` alone")] {
        let rt = CorpusRuntime::run_suites_with(
            vec![AppSource {
                id: "main".into(),
                name: "main".into(),
                dependencies: vec![],
                files: vec![("main.al".into(), SRC.replace("FILTER_TEXT", filter))],
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
            codeunit: 50599,
            name: "TheCopyRan".into(),
        });
        assert!(
            matches!(&o, Outcome::NotRun(w) if w.contains(&format!("DataTransfer with {named} on a Code field as a filter"))),
            "{filter}: {o:?}"
        );
    }
}
