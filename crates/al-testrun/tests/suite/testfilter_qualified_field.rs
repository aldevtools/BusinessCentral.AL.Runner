//! `TP.Filter.SetFilter(TP.Filter."Field", text)` names the field through the filter itself, as Microsoft's Workflow Overview Tests
//! do (`CustomerCard.WorkflowStatus.FILTER.SetFilter(CustomerCard.WorkflowStatus.FILTER."Workflow Code", ...)`; 64 tests
//! were refused as "is not assignable"). The bare name form is pinned by corpus handlers/TestPageActionRunObjectCodeunitRec_Tests.al:95.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50440 "TFQ Row" { fields { field(1; Id; Integer) { } field(2; Grp; Code[10]) { } } keys { key(PK; Id) { Clustered = true; } } }
page 50440 "TFQ List" { PageType = List; SourceTable = "TFQ Row";
    layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } field(Grp; Rec.Grp) { } } } } }
codeunit 50440 "T"
{
    Subtype = Test;
    [Test]
    procedure QualifiedFilterField()
    var L: TestPage "TFQ List"; Row: Record "TFQ Row";
    begin
        Row.Id := 1; Row.Grp := 'A'; Row.Insert();
        Row.Id := 2; Row.Grp := 'B'; Row.Insert();
        L.OpenView();
        L.Filter.SetFilter(L.Filter.Grp, 'B');
        L.First();
        if L.Id.AsInteger() <> 2 then Error('first row is %1', L.Id.Value);
    end;
}
"#;

#[test]
fn a_filter_field_named_through_the_filter_is_accepted() {
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
        codeunit: 50440,
        name: "QualifiedFilterField".into(),
    });
    assert_eq!(o, Outcome::Pass, "{o:?}");
}
