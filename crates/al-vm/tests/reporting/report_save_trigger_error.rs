//! An error a report trigger raises inside Report.SaveAs (`reports/save_trigger_error.rs`; corpus pull request 1579,
//! reports/layouts-saveas/SaveAsTriggerErrors.al, issue 8486): the measured shape answers under the assumption
//! saveas-trigger-error-reaches-caller, and a save under a layout selection row, which the cells do not have, keeps main's
//! refusal.

use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_builtins::value::Value;

const OBJECTS: &str = r#"
table 51090 "TE Row"
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Id) { } }
}

codeunit 51091 "TE Fire"
{
    procedure Fire() begin Error('TE stop'); end;
}

report 51092 "TE Word"
{
    UsageCategory = None;
    DefaultRenderingLayout = W;
    dataset { dataitem(T; "TE Row") { column(Id; Id) { } } }
    rendering { layout(W) { Type = Word; LayoutFile = 'TeWord.docx'; } }
    trigger OnPreReport() var F: Codeunit "TE Fire"; begin F.Fire(); end;
}

report 51094 "TE Inner"
{
    UsageCategory = None;
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset { dataitem(T; "TE Row") { trigger OnPostDataItem() begin Error('TE inner'); end; } }
}

// The measured Word shape, whose OnPreReport runs another report that fails in its OnPostDataItem (not measured).
report 51095 "TE Word Nested"
{
    UsageCategory = None;
    DefaultRenderingLayout = W;
    dataset { dataitem(T; "TE Row") { column(Id; Id) { } } }
    rendering { layout(W) { Type = Word; LayoutFile = 'TeWordNested.docx'; } }
    trigger OnPreReport() begin Report.Run(Report::"TE Inner", false); end;
}

table 51096 "TE Crm Entity" { TableType = CRM;
    fields { field(1; EntityId; Guid) { } field(2; Name; Text[100]) { } }
    keys { key(PK; EntityId) { } } }

// The measured Word shape, whose OnPreReport writes a row of a CRM table before its error.
report 51097 "TE Word Crm"
{
    UsageCategory = None;
    DefaultRenderingLayout = W;
    dataset { dataitem(T; "TE Row") { column(Id; Id) { } } }
    rendering { layout(W) { Type = Word; LayoutFile = 'TeWordCrm.docx'; } }
    trigger OnPreReport()
    var E: Record "TE Crm Entity";
    begin
        E.Init(); E.EntityId := CreateGuid(); E.Name := 'x'; E.Insert();
        Error('TE stop');
    end;
}

codeunit 51093 "TE Tests"
{
    local procedure Saved(): Text
    var R: Record "TE Row"; Tmp: Record "TE Row" temporary; Out: OutStream;
    begin
        R.DeleteAll();
        R.Id := 1;
        R.Insert();
        ClearLastError();
        Tmp.Data.CreateOutStream(Out);
        if Report.SaveAs(Report::"TE Word", '', ReportFormat::Pdf, Out) then
            exit('Yes');
        exit('No|' + GetLastErrorCode() + '|' + GetLastErrorText());
    end;

    procedure NoSelectionRow(): Text begin exit(Saved()); end;

    local procedure SavedOf(ReportId: Integer): Text
    var R: Record "TE Row"; Tmp: Record "TE Row" temporary; Out: OutStream;
    begin
        R.DeleteAll();
        R.Id := 1;
        R.Insert();
        ClearLastError();
        Tmp.Data.CreateOutStream(Out);
        if Report.SaveAs(ReportId, '', ReportFormat::Pdf, Out) then
            exit('Yes');
        exit('No|' + GetLastErrorCode() + '|' + GetLastErrorText());
    end;

    procedure NestedReportError(): Text begin exit(SavedOf(Report::"TE Word Nested")); end;

    procedure CrmWriteThenError(): Text
    begin
        if HasTableConnection(TableConnectionType::CRM, 'TE') then UnregisterTableConnection(TableConnectionType::CRM, 'TE');
        RegisterTableConnection(TableConnectionType::CRM, 'TE', '@@test@@');
        SetDefaultTableConnection(TableConnectionType::CRM, 'TE');
        exit(SavedOf(Report::"TE Word Crm"));
    end;

    procedure WordBuiltinRow(): Text
    var S: Record "Report Layout Selection";
    begin
        S."Report ID" := Report::"TE Word"; S."Company Name" := CompanyName(); S.Type := S.Type::"Word (built-in)"; S.Insert();
        exit(Saved());
    end;
}
"#;

fn run(proc: &str) -> Result<Value, Signal> {
    super::report_management_stand_in::built(OBJECTS).run_in("TE Tests", proc)
}

#[test]
fn a_trigger_error_in_the_measured_shape_answers_false_with_the_error() {
    assumptions::clear_hits();
    assert_eq!(al_testkit::text(run("NoSelectionRow")), "No|Dialog|TE stop");
    assert!(assumptions::take_hits().contains(&Assumption::SaveasTriggerErrorReachesCaller));
}

/// The cells have no layout selection row: under the company's Word (built-in) row the save keeps main's refusal.
#[test]
fn a_trigger_error_under_a_layout_selection_row_keeps_the_refusal() {
    match run("WordBuiltinRow") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("an error in a SaveAs of report"), "{why}"),
        other => panic!("expected the refusal, got {other:?}"),
    }
}

/// The error of a report another report's trigger runs keeps its own origin: here the nested report's OnPostDataItem, which
/// the cells do not measure, not the outer report's OnPreReport (issue 8486, review of the merge train). Main's refusal stays.
#[test]
fn an_error_of_a_nested_reports_trigger_keeps_the_refusal() {
    match run("NestedReportError") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("an error in a SaveAs of report"), "{why}"),
        other => panic!("expected the refusal, got {other:?}"),
    }
}

/// A row written to a CRM table, which lives in its connection's store, counts as a write of the run: main's refusal stays.
#[test]
fn a_crm_write_before_the_trigger_error_keeps_the_refusal() {
    match run("CrmWriteThenError") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("an error in a SaveAs of report"), "{why}"),
        other => panic!("expected the refusal, got {other:?}"),
    }
}
