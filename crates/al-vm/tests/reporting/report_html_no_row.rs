//! An Html save of a Word layout report whose run wrote no dataset row, and the saves ended by CurrReport.Quit (corpus probe
//! pull request 1362, reports/layouts-html-text/HtRenderedTextTests.al Html_SaveAs_NoDatasetRow and Html_SaveAs_Quit*,
//! measured on BC 27.5 and 28.5): BC answers false and writes nothing; every shape the probe did not measure refuses.

use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::Built;

const OBJECTS: &str = r#"
codeunit 51040 "RN Quit At"
{
    SingleInstance = true;
    var At: Text;
    procedure Set(T: Text) begin At := T; end;
    procedure Is(T: Text): Boolean begin exit(At = T); end;
}

report 51041 "RN Quit"
{
    UsageCategory = None;
    DefaultRenderingLayout = Q;
    dataset
    {
        dataitem(T; "RT Row")
        {
            column(Id; Id) { }
            trigger OnAfterGetRecord() var Q: Codeunit "RN Quit At"; begin if Q.Is('OnAfterGetRecord') then CurrReport.Quit(); end;
        }
    }
    rendering { layout(Q) { Type = Word; LayoutFile = 'RnQuit.docx'; } }
    trigger OnPreReport() var Q: Codeunit "RN Quit At"; begin if Q.Is('OnPreReport') then CurrReport.Quit(); end;
    trigger OnPostReport() var Q: Codeunit "RN Quit At"; begin if Q.Is('OnPostReport') then CurrReport.Quit(); end;
}

report 51042 "RN Two"
{
    UsageCategory = None;
    DefaultRenderingLayout = W;
    dataset { dataitem(A; "RT Row") { column(Id; Id) { } } dataitem(B; "RT Row") { column(Id2; Id) { } } }
    rendering { layout(W) { Type = Word; LayoutFile = 'RnTwo.docx'; } }
}

report 51043 "RN Rdlc"
{
    UsageCategory = None;
    DefaultRenderingLayout = R;
    dataset { dataitem(T; "RT Row") { column(Id; Id) { } } }
    rendering { layout(R) { Type = RDLC; LayoutFile = 'RnRdlc.rdl'; } }
}

codeunit 51044 "RN Tests"
{
    local procedure Rows(N: Integer)
    var R: Record "RT Row"; I: Integer;
    begin
        R.DeleteAll();
        for I := 1 to N do begin R.Id := I; R.Insert(); end;
    end;

    local procedure Saved(ReportId: Integer; Fmt: ReportFormat): Text
    var Tmp: Record "RT Row" temporary; Out: OutStream; Ok: Boolean;
    begin
        Tmp.Data.CreateOutStream(Out, TextEncoding::UTF8);
        Ok := Report.SaveAs(ReportId, '', Fmt, Out);
        exit(Format(Ok) + '|' + Format(Tmp.Data.HasValue()));
    end;

    local procedure Quit(At: Text): Text
    var Q: Codeunit "RN Quit At";
    begin
        Rows(2);
        Q.Set(At);
        exit(Saved(Report::"RN Quit", ReportFormat::Html));
    end;

    procedure NoRowHtml(): Text begin Rows(0); exit(Saved(Report::"RT Word", ReportFormat::Html)); end;
    procedure OneRowHtml(): Text begin Rows(1); exit(Saved(Report::"RT Word", ReportFormat::Html)); end;
    procedure NoRowHtmlStatement(): Text
    var Tmp: Record "RT Row" temporary; Out: OutStream;
    begin
        Rows(0);
        Tmp.Data.CreateOutStream(Out, TextEncoding::UTF8);
        Report.SaveAs(Report::"RT Word", '', ReportFormat::Html, Out);
        exit(Format(Tmp.Data.HasValue()));
    end;
    procedure NoRowHtmlError(): Text
    begin
        Rows(0);
        ClearLastError();
        Saved(Report::"RT Word", ReportFormat::Html);
        exit(GetLastErrorCode() + '|' + GetLastErrorText());
    end;
    procedure NoRowPdf(): Text begin Rows(0); exit(Saved(Report::"RT Word", ReportFormat::Pdf)); end;
    procedure NoRowTwoItems(): Text begin Rows(0); exit(Saved(Report::"RN Two", ReportFormat::Html)); end;
    procedure NoRowRdlcPdf(): Text begin Rows(0); exit(Saved(Report::"RN Rdlc", ReportFormat::Pdf)); end;
    procedure NoRowFile(): Text
    begin
        Rows(0);
        exit(Format(Report.SaveAsHtml(Report::"RT Word", 'out.html')));
    end;
    procedure NoQuit(): Text begin exit(Quit('')); end;
    procedure QuitInPreReport(): Text begin exit(Quit('OnPreReport')); end;
    procedure QuitInAfterGetRecord(): Text begin exit(Quit('OnAfterGetRecord')); end;
    procedure QuitInPostReport(): Text begin exit(Quit('OnPostReport')); end;
}
"#;

fn run(proc: &str) -> Result<Value, Signal> {
    Built::from_source(&format!("{}{OBJECTS}", crate::rendered_text::FIXTURE)).run_in("RN Tests", proc)
}

fn text(proc: &str) -> String {
    al_testkit::text(run(proc))
}

#[track_caller]
fn refused(proc: &str, want: &str) {
    match run(proc) {
        Err(Signal::Unsupported(why)) => assert!(why.contains(want), "{proc}: {why}"),
        other => panic!("{proc}: expected a refusal naming {want:?}, got {other:?}"),
    }
}

#[test]
fn an_html_save_with_no_dataset_row_answers_false_and_writes_nothing() {
    assumptions::clear_hits();
    assert_eq!(text("NoRowHtml"), "No|No");
    assert!(assumptions::take_hits().contains(&Assumption::ReportHtmlNoDatasetRow));
    // With a row the placeholder is written, as before.
    assert_eq!(text("OneRowHtml"), "Yes|Yes");
}

/// The false answer leaves BC's error behind without raising it (corpus probe pull request 1371, reports/html-email-body/
/// RxEmailBodyTests.al:134-143 WordDefault_NoRow_EmailBody, BC 27.5, 28.5 and 29.0).
#[test]
fn an_html_save_with_no_dataset_row_leaves_the_empty_report_error() {
    assert_eq!(
        text("NoRowHtmlError"),
        "ReportResultEmpty|The report couldn\u{2019}t be generated, because it was empty. Adjust your filters and try again."
    );
}

#[test]
fn the_shapes_with_no_dataset_row_the_probe_did_not_measure_refuse() {
    for p in ["NoRowHtmlStatement", "NoRowPdf", "NoRowTwoItems", "NoRowFile"] {
        refused(p, "no dataset row");
    }
    // A report whose default layout is not Word keeps main's answer.
    assert_eq!(text("NoRowRdlcPdf"), "Yes|Yes");
}

#[test]
fn a_save_ended_by_quit_refuses_and_one_without_quit_writes_the_placeholder() {
    assert_eq!(text("NoQuit"), "Yes|Yes");
    for p in ["QuitInPreReport", "QuitInAfterGetRecord"] {
        refused(p, "CurrReport.Quit during a SaveAs");
    }
    // Quit in OnPostReport: BC answers true and writes the output (HtRenderedTextTests.al:396-409), as main does.
    assert_eq!(text("QuitInPostReport"), "Yes|Yes");
}
