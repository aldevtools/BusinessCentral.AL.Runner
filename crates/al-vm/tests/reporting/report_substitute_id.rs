//! `Report.GetSubstituteReportId` (report/lt-report-members-testrequestpage/P91Unsure.al:80 GetSubstituteReportId_Values): without a
//! subscriber to SubstituteReport it answers the id itself, also for 0 and an id no report has. With one it raises SubstituteReport
//! (ReportId, RunMode Normal, RequestPageXml '', RecordRef not open, NewReportId -1) and answers the NewReportId a subscriber set
//! (P91Unsure.al:112-170 Sq_GetSubstituteReportId_*).

use al_testkit::{run_in, text};

const SRC: &str = r#"
report 50500 R { ProcessingOnly = true; UseRequestPage = false; dataset { dataitem(I; Integer) { } } }
codeunit 50501 T
{
    procedure Ids(): Text
    begin
        exit(Format(Report.GetSubstituteReportId(Report::R)) + '|' + Format(Report.GetSubstituteReportId(0)) + '|' +
            Format(Report.GetSubstituteReportId(71999)));
    end;
}
"#;

#[test]
fn get_substitute_report_id_without_a_subscriber_is_the_id() {
    assert_eq!(text(run_in(SRC, "T", "Ids")), "50500|0|71999");
}

const LOGGER: &str = r#"
codeunit 50502 S
{
    SingleInstance = true;
    var Log: Text; Change: Boolean;
    procedure SetChange() begin Change := true; end;
    procedure GetLog(): Text begin exit(Log); end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Reporting Triggers", 'SubstituteReport', '', false, false)]
    local procedure OnSubstitute(ReportId: Integer; RunMode: Option Normal,ParametersOnly,Execute,Print,SaveAs,RunModal; RequestPageXml: Text; RecordRef: RecordRef; var NewReportId: Integer)
    begin
        Log += Format(ReportId) + ',' + Format(RunMode) + ',xml=' + RequestPageXml + ',rec=' + Format(RecordRef.Number()) + ',in=' + Format(NewReportId) + ';';
        if Change then
            NewReportId := 50509;
    end;
}
codeunit 50503 U
{
    procedure Logged(): Text
    var S: Codeunit S;
    begin
        exit(Format(Report.GetSubstituteReportId(Report::R)) + '|' + S.GetLog());
    end;

    procedure Changed(): Text
    var S: Codeunit S;
    begin
        S.SetChange();
        exit(Format(Report.GetSubstituteReportId(Report::R)));
    end;
}
"#;

#[test]
fn get_substitute_report_id_raises_substitute_report() {
    let src = format!("{SRC}{LOGGER}");
    assert_eq!(
        text(run_in(&src, "U", "Logged")),
        "50500|50500,Normal,xml=,rec=0,in=-1;"
    );
    assert_eq!(text(run_in(&src, "U", "Changed")), "50509");
}
