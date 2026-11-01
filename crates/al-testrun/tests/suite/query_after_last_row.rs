//! A query read to its last row takes a new filter, and its next Read raises until it is opened again
//! (errors/lt-ms-refusals-131/P131QuerySetRangeAfterRead.al:154-170). Opened again it runs with the new filter (Base
//! Application's Workflow Step Instance.SetTableRelationValues sets filters on its query, opens it and reads it to the end,
//! once per table relation; query/lt-query-read-to-end/QueryReadToEnd.al:88-200). A query that still has rows to read takes the filter and its next
//! Read raises until it is opened again (query/lt-query-setrange-after-read/QuerySetRangeAfterRead.al:52-77).

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50470 "QLR Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
query 50470 "QLR Query" { elements { dataitem(R; "QLR Row") { column(Id; Id) { } } } }
codeunit 50470 "T"
{
    Subtype = Test;
    [Test]
    procedure FilterAgainAfterTheLastRow()
    var Row: Record "QLR Row"; Q: Query "QLR Query"; N: Integer;
    begin
        Row.Id := 1; Row.Insert(); Row.Id := 2; Row.Insert();
        Q.SetRange(Id, 1);
        Q.Open();
        while Q.Read() do N += 1;
        Q.SetRange(Id, 2);
        Q.Open();
        while Q.Read() do N += 10;
        if N <> 11 then Error('N = %1', N);
    end;
    [Test]
    procedure FilterWithRowsLeftThenReadRaises()
    var Row: Record "QLR Row"; Q: Query "QLR Query";
    begin
        Row.Id := 3; Row.Insert(); Row.Id := 4; Row.Insert();
        Q.Open();
        if Q.Read() then;
        Q.SetRange(Id, 2);
        asserterror if Q.Read() then;
        if GetLastErrorCode() <> 'Query' then Error('code %1', GetLastErrorCode());
    end;
    [Test]
    procedure FilterAfterTheLastRowThenReadRaises()
    var Row: Record "QLR Row"; Q: Query "QLR Query";
    begin
        // errors/lt-ms-refusals-131/P131QuerySetRangeAfterRead.al:154-170.
        Row.Id := 5; Row.Insert(); Row.Id := 6; Row.Insert();
        Q.Open();
        while Q.Read() do;
        if Q.Read() then Error('read after the end');
        Q.SetRange(Id, 5);
        asserterror if Q.Read() then;
        if GetLastErrorCode() <> 'Query' then Error('code %1', GetLastErrorCode());
    end;
}
"#;

#[test]
fn a_query_read_to_the_end_takes_a_new_filter() {
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
        codeunit: 50470,
        name: "FilterAgainAfterTheLastRow".into(),
    });
    assert_eq!(o, Outcome::Pass, "{o:?}");
    let o = rt.first_outcome(&CaseId {
        codeunit: 50470,
        name: "FilterWithRowsLeftThenReadRaises".into(),
    });
    assert_eq!(o, Outcome::Pass, "{o:?}");
    let o = rt.first_outcome(&CaseId {
        codeunit: 50470,
        name: "FilterAfterTheLastRowThenReadRaises".into(),
    });
    assert_eq!(o, Outcome::Pass, "{o:?}");
}
