//! A subscriber that names a platform codeunit by its id instead of its name (al-runner issue 7644). The runtime raises the
//! events of `Filter Helper Triggers`, `UI Helper Triggers` and `Reporting Triggers` to the subscribers that name the
//! codeunit; whether BC treats the id form the same is not pinned, so raising such an event refuses. A subscriber written with
//! the id of an app's own codeunit keeps main's result.

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

fn refused(r: Result<al_builtins::value::Value, Signal>) -> String {
    match r {
        Err(Signal::Unsupported(m)) => m.to_string(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// Stands for the platform codeunit of the System symbols (id and name as there); its event as `MakeIntFilter`.
const FILTER_HELPER: &str = r#"
codeunit 2000000007 "Filter Helper Triggers"
{
    [IntegrationEvent(false, false)] procedure MakeIntFilter(var IntFilterText: Text) begin end;
}
table 50210 FRow
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 50100 C
{
    procedure Word(): Text
    var R: Record FRow;
    begin
        R.SetFilter(Id, 'abc');
        exit(R.GetFilter(Id));
    end;
}
"#;

const BY_ID_FILTER_SUB: &str = r#"
codeunit 50211 FSub
{
    [EventSubscriber(ObjectType::Codeunit, 2000000007, 'MakeIntFilter', '', false, false)]
    local procedure MakeIntFilter(var IntFilterText: Text)
    begin
        if IntFilterText = 'abc' then
            IntFilterText := '5';
    end;
}
"#;

#[test]
fn a_make_int_filter_subscriber_written_with_the_codeunit_id_refuses_the_raise() {
    let m = refused(run_in(&format!("{FILTER_HELPER}{BY_ID_FILTER_SUB}"), "C", "Word"));
    assert!(m.contains("by its id"), "{m}");
}

const UI_HELPER: &str = r#"
codeunit 2000000004 "UI Helper Triggers"
{
    [IntegrationEvent(false, false)] procedure CaptionClassTranslate(Language: Integer; CaptionExpr: Text[1024]; var Translation: Text[1024]) begin end;
}
codeunit 50462 Subs
{
    [EventSubscriber(ObjectType::Codeunit, 2000000004, 'CaptionClassTranslate', '', false, false)]
    local procedure T(Language: Integer; CaptionExpr: Text[1024]; var Translation: Text[1024])
    begin
        Translation := 'changed';
    end;
}
codeunit 50469 T
{
    procedure F(): Text
    begin
        exit(CaptionClassTranslate('3,Foo'));
    end;
}
"#;

#[test]
fn a_caption_class_translate_subscriber_written_with_the_codeunit_id_refuses_the_raise() {
    let m = refused(run_in(UI_HELPER, "T", "F"));
    assert!(m.contains("by its id"), "{m}");
}

const REPORTING: &str = r#"
codeunit 2000000005 "Reporting Triggers"
{
    [IntegrationEvent(false, false)] procedure SubstituteReport(ReportId: Integer; RunMode: Option Normal,ParametersOnly,Execute,Print,SaveAs,RunModal; RequestPageXml: Text; RecordRef: RecordRef; var NewReportId: Integer) begin end;
}
report 50500 R { ProcessingOnly = true; UseRequestPage = false; dataset { dataitem(I; Integer) { } } }
codeunit 50502 S
{
    [EventSubscriber(ObjectType::Codeunit, 2000000005, 'SubstituteReport', '', false, false)]
    local procedure OnSubstitute(ReportId: Integer; RunMode: Option Normal,ParametersOnly,Execute,Print,SaveAs,RunModal; RequestPageXml: Text; RecordRef: RecordRef; var NewReportId: Integer)
    begin
        NewReportId := 50509;
    end;
}
codeunit 50503 U
{
    procedure Id(): Text
    begin
        exit(Format(Report.GetSubstituteReportId(Report::R)));
    end;
}
"#;

#[test]
fn a_substitute_report_subscriber_written_with_the_codeunit_id_refuses_the_raise() {
    let m = refused(run_in(REPORTING, "U", "Id"));
    assert!(m.contains("by its id"), "{m}");
}

/// Main's result: a subscriber written with the id of an app's own publisher runs (the raise reaches it).
#[test]
fn a_subscriber_written_with_the_id_of_an_own_codeunit_keeps_running() {
    let src = r#"
codeunit 50200 Pub
{
    [IntegrationEvent(false, false)] procedure OnWork(var Log: Text) begin end;
    procedure Go(): Text var L: Text; begin OnWork(L); exit(L); end;
}
codeunit 50201 Sub
{
    [EventSubscriber(ObjectType::Codeunit, 50200, 'OnWork', '', false, false)]
    local procedure W(var Log: Text) begin Log += 'ran'; end;
}
codeunit 50100 C
{
    procedure F(): Text var P: Codeunit Pub; begin exit(P.Go()); end;
}
"#;
    assert_eq!(text(run_in(src, "C", "F")), "ran");
}
