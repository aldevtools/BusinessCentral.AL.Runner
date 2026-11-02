//! `Report.Execute(Parameters)` with a Date option: the `yyyy-MM-dd` text `RunRequestPage` writes lands in the variable the
//! control shows (report/lt-report-requestpage/RptRequestPage.al:878-893 RunRequestPage_ThenExecute_AppliesValues:
//! `<Field name="DateVal">2021-06-05</Field>` gives `date=2021-06-05` in OnPreReport). Other texts are not pinned.

use al_builtins::native::Signal;

const SRC: &str = r#"
table 91570 "RPD Row" { fields { field(1; No; Integer) { } } keys { key(PK; No) { } } }
report 91571 "RPD Report"
{
    ProcessingOnly = true;
    dataset { dataitem(R; "RPD Row") { } }
    requestpage
    {
        layout { area(Content) { field(DateControl; D) { } } }
    }
    trigger OnPreReport() begin AtPre := Format(D, 0, 9); end;
    var
        D: Date;
        AtPre: Text;
    procedure Result(): Text begin exit(AtPre); end;
}
codeunit 91572 C
{
    local procedure Run(Value: Text): Text
    var
        A: Report "RPD Report";
    begin
        A.Execute('<?xml version="1.0" standalone="yes"?><ReportParameters name="RPD Report" id="91571"><Options>' + Value + '</Options></ReportParameters>');
        exit(A.Result());
    end;
    procedure Iso(): Text begin exit(Run('<Field name="D">2021-06-05</Field>')); end;
    procedure OtherForm(): Text begin exit(Run('<Field name="D">06/05/2021</Field>')); end;
}
"#;

#[test]
fn an_iso_date_option_lands_in_the_variable() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "Iso")), "2021-06-05");
}

#[test]
fn a_date_option_in_another_form_is_refused() {
    for routine in ["OtherForm"] {
        match al_testkit::run_in(SRC, "C", routine) {
            Err(Signal::Unsupported(s)) => assert!(s.contains("parameter D"), "{routine}: {s}"),
            other => panic!("{routine}: {other:?}"),
        }
    }
}
