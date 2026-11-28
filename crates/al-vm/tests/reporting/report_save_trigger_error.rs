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
