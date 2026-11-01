//! FieldRef.CalcField on a Blob field is accepted (Temp Blob Impl.FromFieldRef in the System Application calls it on every Blob it
//! reads; assumption fieldref-calcfield-of-a-blob). On another stored field it stays refused.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50550 "FCB Row" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } field(3; Nm; Text[20]) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 50550 "T"
{
    Subtype = Test;
    [Test]
    procedure CalcFieldOfABlob()
    var Row: Record "FCB Row"; RR: RecordRef; FR: FieldRef; OutS: OutStream;
    begin
        Row.Id := 1; Row.Data.CreateOutStream(OutS); OutS.WriteText('abc'); Row.Insert();
        RR.GetTable(Row);
        FR := RR.Field(2);
        FR.CalcField();
    end;
    [Test]
    procedure CalcFieldOfATextField()
    var Row: Record "FCB Row"; RR: RecordRef; FR: FieldRef;
    begin
        Row.Id := 2; Row.Insert();
        RR.GetTable(Row);
        FR := RR.Field(3);
        FR.CalcField();
    end;
}
"#;

#[test]
fn calc_field_of_a_blob_runs_and_of_a_text_field_is_refused() {
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
        codeunit: 50550,
        name: "CalcFieldOfABlob".into(),
    });
    assert_eq!(o, Outcome::Pass, "{o:?}");
    let o = rt.first_outcome(&CaseId {
        codeunit: 50550,
        name: "CalcFieldOfATextField".into(),
    });
    assert!(matches!(o, Outcome::NotRun(_)), "{o:?}");
}
