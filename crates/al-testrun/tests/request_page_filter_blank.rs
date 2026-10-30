//! TestRequestPage data item filters set to an empty text or a text of spaces after a filter
//! (pages/lt-ao-testpage/AoTests.al:698-742, :888-928): an empty text clears the filter (GetFilter empty, the report runs
//! unfiltered); a text of spaces leaves GetFilter at the old filter while the report runs unfiltered, which this runtime
//! does not model, so it is refused.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50211 "BF Row" { fields { field(1; Id; Integer) { } field(2; F; Code[10]) { } } keys { key(PK; Id) { } } }
codeunit 50210 "BF Log" { SingleInstance = true; var V: Text; procedure Add(S: Text) begin V += S + ';'; end; procedure Take(): Text var R: Text; begin R := V; V := ''; exit(R); end; }
report 50212 "BF Rpt"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "BF Row") { RequestFilterFields = F;
        trigger OnPreDataItem() var L: Codeunit "BF Log"; begin L.Add('pre filter=' + Row.GetFilter(F) + ' count=' + Format(Row.Count())); end; } }
}
codeunit 50213 "T"
{
    Subtype = Test;
    var Reset: Text;
    [RequestPageHandler]
    procedure H(var RP: TestRequestPage "BF Rpt")
    var L: Codeunit "BF Log";
    begin
        RP.Row.SetFilter(F, 'A');
        RP.Row.SetFilter(F, Reset);
        L.Add('rp=[' + RP.Row.GetFilter(F) + ']');
        RP.OK().Invoke();
    end;
    local procedure Seed()
    var R: Record "BF Row";
    begin
        R.DeleteAll();
        R.Id := 1; R.F := ''; R.Insert(); R.Id := 2; R.F := 'A'; R.Insert(); R.Id := 3; R.F := 'B'; R.Insert();
        Commit();
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure Empty()
    var L: Codeunit "BF Log";
    begin
        Seed();
        Reset := '';
        Report.Run(Report::"BF Rpt");
        if L.Take() <> 'rp=[];pre filter= count=3;' then Error('empty');
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure Space()
    begin
        Seed();
        Reset := ' ';
        Report.Run(Report::"BF Rpt");
    end;
}
"#;

fn outcome(name: &str) -> Outcome {
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
    rt.first_outcome(&CaseId {
        codeunit: 50213,
        name: name.into(),
    })
}

#[test]
fn an_empty_text_clears_the_request_page_filter() {
    assert_eq!(outcome("Empty"), Outcome::Pass);
}

#[test]
fn a_text_of_spaces_on_a_request_page_filter_is_refused() {
    match outcome("Space") {
        Outcome::NotRun(why) => assert!(why.contains("a text of spaces"), "{why}"),
        other => panic!("{other:?}"),
    }
}
