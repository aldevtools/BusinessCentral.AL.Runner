//! `Report.GetSubstituteReportId` (report/lt-report-members-testrequestpage/P91Unsure.al:80 GetSubstituteReportId_Values): without a
//! subscriber to SubstituteReport it answers the id itself, also for 0 and an id no report has. With a subscriber it is refused.

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

const SUBSCRIBER: &str = r#"
codeunit 50502 S
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Reporting Triggers", 'SubstituteReport', '', false, false)]
    local procedure OnSubstitute(ReportId: Integer; RunMode: Option Normal,ParametersOnly,Execute,Print,SaveAs,RunModal; RequestPageXml: Text; RecordRef: RecordRef; var NewReportId: Integer)
    begin
    end;
}
"#;

#[test]
fn get_substitute_report_id_without_a_subscriber_is_the_id() {
    assert_eq!(text(run_in(SRC, "T", "Ids")), "50500|0|71999");
}

#[test]
fn get_substitute_report_id_with_a_subscriber_is_refused() {
    match run_in(&format!("{SRC}{SUBSCRIBER}"), "T", "Ids") {
        Err(al_builtins::native::Signal::Unsupported(why)) => assert!(why.contains("SubstituteReport"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
