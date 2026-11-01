//! A report with SaveValues stores the filters of its data item when the request page closes with OK (corpus
//! report/lt-requestpage-savevalues-date-text-setvalue/P147SaveValuesFilter.al:P147_Filter_SetTableViewVsStored). A later run
//! that gives the data item a record (the record of `RunModal`) keeps that record's filters, on the assumption
//! report-view-over-stored-request-filters. Microsoft's "Suggest Fin. Charge Memo Lines" runs once per memo in one codeunit.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50201 "VS Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 50200 "VS Log" { SingleInstance = true; var V: Text; procedure Add(S: Text) begin V += S + ';'; end; procedure Take(): Text var R: Text; begin R := V; V := ''; exit(R); end; }
report 50202 "VS Rpt"
{
    ProcessingOnly = true;
    requestpage { SaveValues = true; }
    dataset { dataitem(Row; "VS Row") { RequestFilterFields = Id; trigger OnAfterGetRecord() var L: Codeunit "VS Log"; begin L.Add(Format(Row.Id)); end; } }
}
codeunit 50203 "T"
{
    Subtype = Test;
    [RequestPageHandler]
    procedure H(var RP: TestRequestPage "VS Rpt") begin RP.OK().Invoke(); end;
    [Test]
    [HandlerFunctions('H')]
    procedure SecondRunKeepsItsRecord()
    var R: Record "VS Row"; L: Codeunit "VS Log";
    begin
        R.Id := 1; R.Insert(); R.Id := 2; R.Insert(); Commit();
        R.SetRange(Id, 1);
        Report.RunModal(Report::"VS Rpt", true, false, R);
        if L.Take() <> '1;' then Error('first run: %1', L.Take());
        R.SetRange(Id, 2); Commit();
        Report.RunModal(Report::"VS Rpt", true, false, R);
        if L.Take() <> '2;' then Error('second run: %1', L.Take());
    end;
}
"#;

#[test]
fn a_second_run_keeps_the_record_it_was_given() {
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
        codeunit: 50203,
        name: "SecondRunKeepsItsRecord".into(),
    });
    assert_eq!(o, Outcome::Pass);
}
