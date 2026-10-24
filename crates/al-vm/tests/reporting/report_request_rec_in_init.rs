//! The `Rec` of a report request page that states `SourceTable`, read in OnInitReport (al-runner issue 5756): it raises
//! ReportRequestPageNotInitialized before the rest of the trigger runs (report/lt-y-request-page/YRequestPageTests.al:551,
//! RS_RequestPageRec_ReadInOnInitReport_Raises; report 88566 in RsRequestPageRec.al:224). Through a procedure it is not
//! pinned and refused.

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69990 "RI Line" { fields { field(1; DocNo; Code[10]) { } field(2; LineNo; Integer) { } } keys { key(PK; DocNo, LineNo) { } } }
codeunit 69991 "RI State"
{
    SingleInstance = true;
    var Log: Text;
    procedure Put(Value: Text) begin Log += Value + ';'; end;
    procedure Get(): Text begin exit(Log); end;
}
report 69992 "RI Init"
{
    UsageCategory = None;
    ProcessingOnly = true;
    requestpage { SourceTable = "RI Line"; }
    trigger OnInitReport()
    var State: Codeunit "RI State";
    begin
        State.Put('before');
        State.Put('[' + Rec.DocNo + '/' + Format(Rec.LineNo) + ']');
        State.Put('after');
    end;
}
report 69993 "RI Init Proc"
{
    UsageCategory = None;
    ProcessingOnly = true;
    requestpage { SourceTable = "RI Line"; }
    trigger OnInitReport()
    var State: Codeunit "RI State";
    begin
        State.Put(RecText());
    end;
    procedure RecText(): Text
    begin
        exit(Rec.DocNo);
    end;
}
codeunit 69994 C
{
    procedure InInit(): Text
    var State: Codeunit "RI State";
    begin
        asserterror Report.RunModal(Report::"RI Init", false, false);
        exit(GetLastErrorText() + '|' + GetLastErrorCode() + ' ' + State.Get());
    end;

    procedure ThroughProcedure()
    begin
        Report.RunModal(Report::"RI Init Proc", false, false);
    end;
}
"#;

#[test]
fn reading_the_request_page_rec_in_oninitreport_raises() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "InInit")),
        "You cannot call the RequestPage from the OnInitReport Trigger in Report ID \"69992\" because the RequestPage is not yet initialized.|ReportRequestPageNotInitialized before;"
    );
}

#[test]
fn a_procedure_reading_the_request_page_rec_in_oninitreport_is_refused() {
    match al_testkit::run_in(SRC, "C", "ThroughProcedure") {
        Err(Signal::Unsupported(w)) => assert!(w.contains("OnInitReport"), "{w}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
