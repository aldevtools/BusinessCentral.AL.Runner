//! A report that sets UseSystemPrinter runs as one without it: Run, RunModal, RunRequestPage, SaveAs, Execute and Print give
//! the same triggers in the same order with it true and false
//! (misc/lt-y13-triage/Y13ReportSystemPrinter.al:289-543, corpus master).

use al_testkit::text;

const SRC: &str = r#"
table 69930 "SY Row" { fields { field(1; Number; Integer) { } } keys { key(PK; Number) { } } }
report 69931 "SY On"
{
    UseSystemPrinter = true; ProcessingOnly = true; UseRequestPage = false;
    dataset
    {
        dataitem(Row; "SY Row")
        {
            trigger OnPreDataItem() begin Log += 'pre;'; end;
            trigger OnAfterGetRecord() begin Log += 'row ' + Format(Number) + ';'; end;
        }
    }
    trigger OnPreReport() begin Log += 'PreReport;'; end;
    var Log: Text;
    procedure GetLog(): Text begin exit(Log); end;
}
report 69932 "SY Off"
{
    UseSystemPrinter = false; ProcessingOnly = true; UseRequestPage = true;
    dataset { dataitem(Row; "SY Row") { trigger OnAfterGetRecord() var L: Codeunit "SY Log"; begin L.Add('row ' + Format(Number)); end; } }
    trigger OnInitReport() var L: Codeunit "SY Log"; begin L.Add('init'); end;
    trigger OnPostReport() var L: Codeunit "SY Log"; begin L.Add('post'); end;
}
codeunit 69934 "SY Log"
{
    SingleInstance = true;
    var Log: Text;
    procedure Add(T: Text) begin Log += T + ';'; end;
    procedure Take(): Text var T: Text; begin T := Log; Log := ''; exit(T); end;
}
codeunit 69933 C
{
    procedure F(): Text
    var R: Report "SY On"; Row: Record "SY Row";
    begin
        Row.Number := 1; Row.Insert(); Row.Number := 2; Row.Insert();
        R.UseRequestPage(false);
        R.RunModal();
        exit(R.GetLog());
    end;
    procedure P(): Text
    var Row: Record "SY Row"; L: Codeunit "SY Log";
    begin
        Row.Number := 1; Row.Insert(); Row.Number := 2; Row.Insert();
        Report.Print(69932, '');
        exit(L.Take());
    end;
}
"#;

#[test]
fn a_report_with_use_system_printer_runs_as_one_without_it() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "F")), "PreReport;pre;row 1;row 2;");
}

#[test]
fn print_of_a_processing_only_report_with_use_system_printer_runs_it_without_a_request_page() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "P")), "init;row 1;row 2;post;");
}
