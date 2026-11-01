//! A query column over a RecordId field (Base Application's "Workflow Instance" query has Record_ID; assumption
//! query-column-record-id; 142 Tests-Workflow tests were refused for it). Before the change the whole query was refused.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50450 "QRC Row" { fields { field(1; Id; Integer) { } field(2; Rid; RecordId) { } } keys { key(PK; Id) { Clustered = true; } } }
query 50450 "QRC Query" { elements { dataitem(R; "QRC Row") { column(Id; Id) { } column(Rid; Rid) { } } } }
codeunit 50450 "T"
{
    Subtype = Test;
    [Test]
    procedure ReadsTheColumn()
    var Row: Record "QRC Row"; Q: Query "QRC Query"; Count: Integer;
    begin
        Row.Id := 7; Row.Rid := Row.RecordId(); Row.Insert();
        Q.Open();
        while Q.Read() do begin
            Count += 1;
            if Q.Id <> 7 then Error('id %1', Q.Id);
        end;
        if Count <> 1 then Error('rows %1', Count);
    end;
}
"#;

#[test]
fn a_query_with_a_record_id_column_runs() {
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
        codeunit: 50450,
        name: "ReadsTheColumn".into(),
    });
    assert_eq!(o, Outcome::Pass, "{o:?}");
}
