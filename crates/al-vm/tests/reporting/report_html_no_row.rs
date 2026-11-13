//! A rendering save whose run wrote no dataset row (`report_placeholder::no_row`; corpus pull request 1371,
//! reports/html-email-body/RxLayoutSelectionTests.al:852-1001, and 1362, reports/layouts-html-text/HtRenderedTextTests.al
//! Html_SaveAs_NoDatasetRow), and the saves ended by CurrReport.Quit (HtRenderedTextTests.al Html_SaveAs_Quit*). A Word layout
//! report outside the measured cells refuses; an RDLC report outside them writes its placeholder as before.

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

report 51045 "RN Legacy"
{
    UsageCategory = None;
    DefaultLayout = Word;
    WordLayout = 'RnLegacy.docx';
    dataset { dataitem(T; "RT Row") { column(Id; Id) { } } }
}

report 51046 "RN Word Post"
{
    UsageCategory = None;
    DefaultRenderingLayout = W;
    dataset { dataitem(T; "RT Row") { column(Id; Id) { } } }
    rendering { layout(W) { Type = Word; LayoutFile = 'RnWordPost.docx'; } }
    trigger OnPostReport() begin end;
}

report 51047 "RN Word Pre"
{
    UsageCategory = None;
    DefaultRenderingLayout = W;
    dataset { dataitem(T; "RT Row") { column(Id; Id) { } } }
    rendering { layout(W) { Type = Word; LayoutFile = 'RnWordPre.docx'; } }
    trigger OnPreReport() var Q: Codeunit "RN Quit At"; begin Q.Set('pre ran'); end;
}

report 51048 "RN Legacy Pre"
{
    UsageCategory = None;
    DefaultLayout = Word;
    WordLayout = 'RnLegacyPre.docx';
    dataset { dataitem(T; "RT Row") { column(Id; Id) { } } }
    trigger OnPreReport() begin end;
}

report 51049 "RN Rdlc Pre"
{
    UsageCategory = None;
    DefaultRenderingLayout = R;
    dataset { dataitem(T; "RT Row") { column(Id; Id) { } } }
    rendering { layout(R) { Type = RDLC; LayoutFile = 'RnRdlcPre.rdl'; } }
    trigger OnPreReport() begin end;
}

codeunit 51044 "RN Tests"
{
    // The answer, whether the stream holds anything, and the error the save left.
    procedure Used(ReportId: Integer; Fmt: ReportFormat): Text
    begin
        Rows(0);
        ClearLastError();
        exit(Saved(ReportId, Fmt) + '|' + GetLastErrorCode());
    end;

    [TryFunction]
    local procedure TryStatement(ReportId: Integer; Fmt: ReportFormat; var Tmp: Record "RT Row" temporary)
    var Out: OutStream;
    begin
        Tmp.Data.CreateOutStream(Out, TextEncoding::UTF8);
        Report.SaveAs(ReportId, '', Fmt, Out);
    end;

    // Whether the statement form completed, whether the stream holds anything, and the error it raised.
    procedure Statement(ReportId: Integer; Fmt: ReportFormat): Text
    var Tmp: Record "RT Row" temporary; Done: Boolean;
    begin
        Rows(0);
        ClearLastError();
        Done := TryStatement(ReportId, Fmt, Tmp);
        exit(Format(Done) + '|' + Format(Tmp.Data.HasValue()) + '|' + GetLastErrorCode() + '|' + GetLastErrorText());
    end;

    procedure WordPdf(): Text begin exit(Used(Report::"RT Word", ReportFormat::Pdf)); end;
    procedure WordWord(): Text begin exit(Used(Report::"RT Word", ReportFormat::Word)); end;
    procedure WordHtml(): Text begin exit(Used(Report::"RT Word", ReportFormat::Html)); end;
    procedure WordPdfStatement(): Text begin exit(Statement(Report::"RT Word", ReportFormat::Pdf)); end;
    procedure WordWordStatement(): Text begin exit(Statement(Report::"RT Word", ReportFormat::Word)); end;
    procedure WordHtmlStatement(): Text begin exit(Statement(Report::"RT Word", ReportFormat::Html)); end;
    procedure WordPrePdf(): Text
    var Q: Codeunit "RN Quit At";
    begin
        Q.Set('');
        exit(Used(Report::"RN Word Pre", ReportFormat::Pdf) + '|' + Format(Q.Is('pre ran')));
    end;
    procedure WordPostPdf(): Text begin exit(Used(Report::"RN Word Post", ReportFormat::Pdf)); end;
    procedure WordPostHtml(): Text begin exit(Used(Report::"RN Word Post", ReportFormat::Html)); end;
    procedure LegacyHtml(): Text begin exit(Used(Report::"RN Legacy", ReportFormat::Html)); end;
    procedure LegacyPdf(): Text begin exit(Used(Report::"RN Legacy", ReportFormat::Pdf)); end;
    procedure LegacyPdfStatement(): Text begin exit(Statement(Report::"RN Legacy", ReportFormat::Pdf)); end;
    procedure LegacyWord(): Text begin exit(Used(Report::"RN Legacy", ReportFormat::Word)); end;
    procedure LegacyPreHtml(): Text begin exit(Used(Report::"RN Legacy Pre", ReportFormat::Html)); end;
    procedure LegacyPrePdf(): Text begin exit(Used(Report::"RN Legacy Pre", ReportFormat::Pdf)); end;
    procedure RdlcPdf(): Text begin exit(Used(Report::"RN Rdlc", ReportFormat::Pdf)); end;
    procedure RdlcWord(): Text begin exit(Used(Report::"RN Rdlc", ReportFormat::Word)); end;
    procedure RdlcPdfStatement(): Text begin exit(Statement(Report::"RN Rdlc", ReportFormat::Pdf)); end;
    procedure RdlcExcel(): Text begin exit(Used(Report::"RN Rdlc", ReportFormat::Excel)); end;
    procedure RdlcPrePdf(): Text begin exit(Used(Report::"RN Rdlc Pre", ReportFormat::Pdf)); end;
    procedure TwoItemsPdf(): Text begin exit(Used(Report::"RN Two", ReportFormat::Pdf)); end;

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
    procedure NoRowLegacy(): Text begin Rows(0); exit(Saved(Report::"RN Legacy", ReportFormat::Html)); end;
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
    assert!(assumptions::take_hits().contains(&Assumption::ReportSaveNoDatasetRow));
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

const EMPTY: &str = "The report couldn\u{2019}t be generated, because it was empty. Adjust your filters and try again.";

/// A Word layout `DefaultRenderingLayout` names: Pdf, Word and Html answer false and leave ReportResultEmpty; the statement
/// form raises it (RxLayoutSelectionTests.al:852, :873, :915). OnPreReport has run (`used log` `OnPreReport;OnPreRendering;`).
#[test]
fn a_word_layout_save_with_no_dataset_row_answers_or_raises_the_empty_report_error() {
    for p in ["WordPdf", "WordWord", "WordHtml"] {
        assumptions::clear_hits();
        assert_eq!(text(p), "No|No|ReportResultEmpty", "{p}");
        assert!(
            assumptions::take_hits().contains(&Assumption::ReportSaveNoDatasetRow),
            "{p}"
        );
    }
    for p in ["WordPdfStatement", "WordWordStatement", "WordHtmlStatement"] {
        assert_eq!(text(p), format!("No|No|ReportResultEmpty|{EMPTY}"), "{p}");
    }
    assert_eq!(text("WordPrePdf"), "No|No|ReportResultEmpty|Yes");
}

/// BC does not run OnPostReport then (RxLayoutSelectionTests.al:852 `used log`); this runtime has run it, so such a report
/// refuses. Main answered false for its Html save.
#[test]
fn a_word_layout_report_with_onpostreport_and_no_dataset_row_refuses() {
    for p in ["WordPostPdf", "WordPostHtml"] {
        refused(p, "declares OnPostReport");
    }
}

/// `DefaultLayout = Word` with `WordLayout`, no report trigger: Html and Pdf as above (RxLayoutSelectionTests.al:979, :1001).
/// Word is not measured. With a report trigger the Html save keeps main's answer (false, no error left) and Pdf refuses.
#[test]
fn a_legacy_declared_report_with_no_dataset_row() {
    assert_eq!(text("LegacyHtml"), "No|No|ReportResultEmpty");
    assert_eq!(text("LegacyPdf"), "No|No|ReportResultEmpty");
    assert_eq!(text("LegacyPdfStatement"), format!("No|No|ReportResultEmpty|{EMPTY}"));
    assert_eq!(text("NoRowLegacy"), "No|No");
    assert_eq!(text("LegacyPreHtml"), "No|No|");
    for p in ["LegacyWord", "LegacyPrePdf"] {
        refused(p, "no dataset row");
    }
}

/// An RDLC layout `DefaultRenderingLayout` names, no report trigger: Pdf and Word answer true and write nothing, in both
/// forms (RxLayoutSelectionTests.al:937, :958). Outside the cell the placeholder is written, as on main.
#[test]
fn an_rdlc_save_with_no_dataset_row_answers_true_and_writes_nothing() {
    for p in ["RdlcPdf", "RdlcWord"] {
        assumptions::clear_hits();
        assert_eq!(text(p), "Yes|No|", "{p}");
        assert!(
            assumptions::take_hits().contains(&Assumption::ReportSaveNoDatasetRow),
            "{p}"
        );
    }
    assert_eq!(text("RdlcPdfStatement"), "Yes|No||");
    assert_eq!(text("RdlcExcel"), "Yes|Yes|");
    assert_eq!(text("RdlcPrePdf"), "Yes|Yes|");
}

#[test]
fn the_shapes_with_no_dataset_row_the_probe_did_not_measure_refuse() {
    for p in ["NoRowTwoItems", "TwoItemsPdf", "NoRowFile"] {
        refused(p, "no dataset row");
    }
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
