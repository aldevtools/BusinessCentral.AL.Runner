//! SetValue on a request page control bound to a field of a report's record variable (`field(Nm; Buf.Name)`) writes the field,
//! as Base Application's "Implement Standard Cost Change" request page (`ItemJnlLine."Journal Template Name"`) needs. The write
//! through an expression control is pinned by corpus misc/lt-probes-needed-7/PnGTempControl.al and pages/W1Controls.al:
//! TempRecordControl_SetValue; a page with no source table that has actions stays refused for the refresh after an action,
//! which a request page does not have.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50510 "RPE Row" { fields { field(1; Id; Integer) { } field(2; Nm; Text[30]) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 50510 "RPE Log" { SingleInstance = true; var Seen: Text; procedure Put(T: Text) begin Seen := T; end; procedure Get(): Text begin exit(Seen); end; }
report 50510 "RPE Report" { ProcessingOnly = true;
    requestpage { layout { area(Content) { field(Nm; Buf.Nm) { } } } }
    trigger OnPreReport() var L: Codeunit "RPE Log"; begin L.Put(Buf.Nm); end;
    var Buf: Record "RPE Row" temporary; }
codeunit 50511 "T"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('Fill')]
    procedure SetTheField()
    var L: Codeunit "RPE Log";
    begin
        Report.Run(Report::"RPE Report");
        if L.Get() <> 'hello' then Error('seen %1', L.Get());
    end;
    [RequestPageHandler]
    procedure Fill(var P: TestRequestPage "RPE Report") begin P.Nm.SetValue('hello'); P.OK().Invoke(); end;
}
"#;

#[test]
fn a_request_page_control_bound_to_a_record_field_takes_a_value() {
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
        codeunit: 50511,
        name: "SetTheField".into(),
    });
    assert_eq!(o, Outcome::Pass, "{o:?}");
}
