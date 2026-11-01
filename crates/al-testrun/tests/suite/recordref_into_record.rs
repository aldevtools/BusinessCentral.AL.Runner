//! A RecordRef held by a Variant goes into a Record parameter of its own table as a copy of the row it is on (Base
//! Application's Approvals Mgmt.InformUserOnStatusChange passes its Variant to `ShowSalesApprovalStatus(SalesHeader: Record ..)`;
//! 73 Tests-Workflow tests; assumption recordref-into-record).

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50540 "RRR Row" { fields { field(1; Id; Integer) { } field(2; Nm; Text[20]) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 50540 "T"
{
    Subtype = Test;
    local procedure Show(Row: Record "RRR Row"): Text begin exit(Row.Nm); end;
    [Test]
    procedure VariantRecordRefIntoRecordParameter()
    var Row: Record "RRR Row"; RR: RecordRef; V: Variant;
    begin
        Row.Id := 1; Row.Nm := 'one'; Row.Insert();
        RR.GetTable(Row);
        RR.FindFirst();
        V := RR;
        if Show(V) <> 'one' then Error('got %1', Show(V));
    end;
}
"#;

#[test]
fn a_record_ref_in_a_variant_goes_into_a_record_parameter() {
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
        codeunit: 50540,
        name: "VariantRecordRefIntoRecordParameter".into(),
    });
    assert_eq!(o, Outcome::Pass, "{o:?}");
}
