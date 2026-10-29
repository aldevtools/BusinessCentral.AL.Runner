//! A report that sets ShowPrintStatus runs as one without it (al-runner issue 6056: a partner app refused 4,245 tests with
//! "report property showprintstatus is not supported yet"). Corpus writer TK pinned Run, RunModal and Execute of a
//! processing-only report with ShowPrintStatus true, false and unset: the same triggers in the same order, no handler
//! (report/lt-tk-showprintstatus/LtTkShowPrintStatus.al).

use al_testkit::text;

const SRC: &str = r#"
table 69970 "SPS Row" { fields { field(1; Number; Integer) { } } keys { key(PK; Number) { } } }
report 69971 "SPS Rpt"
{
    ShowPrintStatus = true; ProcessingOnly = true; UseRequestPage = false;
    dataset
    {
        dataitem(Row; "SPS Row")
        {
            trigger OnPreDataItem() begin Log += 'pre;'; end;
            trigger OnAfterGetRecord() begin Log += 'row ' + Format(Number) + ';'; end;
            trigger OnPostDataItem() begin Log += 'post;'; end;
        }
    }
    trigger OnPreReport() begin Log += 'PreReport;'; end;
    trigger OnPostReport() begin Log += 'PostReport;'; end;
    var Log: Text;
    procedure GetLog(): Text begin exit(Log); end;
}
codeunit 69972 C
{
    procedure F(): Text
    var R: Report "SPS Rpt"; Row: Record "SPS Row";
    begin
        Row.Number := 1; Row.Insert(); Row.Number := 2; Row.Insert();
        R.UseRequestPage(false);
        R.RunModal();
        exit(R.GetLog());
    end;
}
"#;

#[test]
fn a_report_with_show_print_status_runs_as_one_without_it() {
    al_builtins::assumptions::clear_hits();
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "F")),
        "PreReport;pre;row 1;row 2;post;PostReport;"
    );
    assert!(al_builtins::assumptions::take_hits().is_empty());
}
