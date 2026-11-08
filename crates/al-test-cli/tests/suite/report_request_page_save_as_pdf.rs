//! `TestRequestPage.SaveAsPdf(FileName)` in a request page handler of a report with an RDLC layout runs the report with
//! the page's values and writes a placeholder PDF to the file (owner decision 2026-11-08, ADR-040 amendment). A report
//! with an Excel layout keeps the refusal.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 69990 "RQP Row"
{
    fields { field(1; "No."; Code[20]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
codeunit 69991 "RQP Log"
{
    SingleInstance = true;
    var L: Text;
    procedure Add(T: Text) begin L += T + ';'; end;
    procedure Take(): Text var T: Text; begin T := L; L := ''; exit(T); end;
}
report 69992 "RQP Rdlc"
{
    DefaultLayout = RDLC;
    RDLCLayout = 'RqpRdlc.rdl';
    dataset
    {
        dataitem(D; "RQP Row")
        {
            column(No; "No.") { }
            trigger OnAfterGetRecord() begin Log.Add('row ' + D."No."); end;
        }
    }
    requestpage
    {
        layout { area(Content) { field(FlagCtl; Flag) { ApplicationArea = All; } } }
    }
    trigger OnPreReport() begin Log.Add('pre flag ' + Format(Flag)); end;
    var
        Log: Codeunit "RQP Log";
        Flag: Boolean;
}
report 69993 "RQP Excel"
{
    DefaultRenderingLayout = Xl;
    dataset { dataitem(D; "RQP Row") { column(No; "No.") { } } }
    requestpage { layout { area(Content) { } } }
    rendering { layout(Xl) { Type = Excel; LayoutFile = 'RqpExcel.xlsx'; } }
}
codeunit 69994 "RQP Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var
        Log: Codeunit "RQP Log";

    [Test]
    [HandlerFunctions('RpPdf')]
    procedure SaveAsPdfWritesAFile()
    var
        R: Record "RQP Row";
        F: File;
        Got: Text;
    begin
        R.DeleteAll();
        R."No." := 'A'; R.Insert();
        Commit();
        Log.Take();
        Report.Run(Report::"RQP Rdlc", true);
        Got := Log.Take();
        if Got <> 'pre flag Yes;row A;' then
            Error('log %1', Got);
        if not Exists('out/rqp.pdf') then
            Error('no file');
        F.Open('out/rqp.pdf');
        if F.Len() = 0 then
            Error('empty');
    end;

    [Test]
    [HandlerFunctions('RpPdfExcel')]
    procedure ExcelLayoutStaysRefused()
    begin
        Report.Run(Report::"RQP Excel", true);
    end;

    [RequestPageHandler]
    procedure RpPdf(var Page: TestRequestPage "RQP Rdlc")
    begin
        Page.FlagCtl.SetValue(true);
        Page.SaveAsPdf('out/rqp.pdf');
    end;

    [RequestPageHandler]
    procedure RpPdfExcel(var Page: TestRequestPage "RQP Excel")
    begin
        Page.SaveAsPdf('out/rqx.pdf');
    end;
}
"#;

#[test]
fn request_page_save_as_pdf_writes_a_placeholder_file() {
    let o = outcomes_in(SRC, "RQP Tests");
    let got = |t: &str| o.iter().find(|(n, _)| n == t).map(|(_, o)| o.clone());
    assert_eq!(got("SaveAsPdfWritesAFile"), Some(Outcome::Pass), "{o:?}");
    match got("ExcelLayoutStaysRefused") {
        Some(Outcome::NotRun(why)) => assert!(why.contains("SaveAsPdf"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
