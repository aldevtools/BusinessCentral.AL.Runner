//! `ExcelLayoutMultipleDataSheets = true` on a report (al-runner issue 7836): every call that uses such a report refuses, the
//! calls the corpus probe measured (ExcelMultipleDataSheetsReport.al, corpus pull request 1368) included, until a separate
//! pull request admits them. A report without the property keeps running.

use al_builtins::native::Signal;
use al_testkit::Built;

const DATASET: &str = r#"
    dataset
    {
        dataitem(Num; "XMS Num") { DataItemTableView = where(Number = filter(1 .. 2)); column(Number; Number) { } column(Doubled; Number * 2) { } }
        dataitem(Second; "XMS Num") { DataItemTableView = where(Number = const(7)); column(SecondNumber; Number) { } }
    }
"#;

fn src() -> String {
    format!(
        r#"
report 50960 "XMS Plain" {{ UsageCategory = None; UseRequestPage = false; {DATASET} }}
report 50961 "XMS Multi" {{ UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true; {DATASET} }}
report 50962 "XMS Multi Word"
{{
    UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true; DefaultRenderingLayout = W;
    {DATASET}
    rendering {{ layout(W) {{ Type = Word; LayoutFile = 'XmsWord.docx'; }} }}
}}
report 50963 "XMS Multi Excel"
{{
    UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true; DefaultRenderingLayout = X;
    {DATASET}
    rendering {{ layout(X) {{ Type = Excel; LayoutFile = 'XmsExcel.xlsx'; }} }}
}}
report 50967 "XMS Proc" {{ UsageCategory = None; UseRequestPage = false; ProcessingOnly = true; ExcelLayoutMultipleDataSheets = true; {DATASET} }}
report 50968 "XMS Two Layouts"
{{
    UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true; DefaultRenderingLayout = X;
    {DATASET}
    rendering {{ layout(X) {{ Type = Excel; LayoutFile = 'XmsExcel.xlsx'; }} layout(W) {{ Type = Word; LayoutFile = 'XmsWord.docx'; }} }}
}}
report 50969 "XMS Legacy"
{{
    UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true; DefaultLayout = Word; WordLayout = 'XmsWord.docx';
    {DATASET}
}}
report 50970 "XMS Extended" {{ UsageCategory = None; UseRequestPage = false; ExcelLayoutMultipleDataSheets = true; {DATASET} }}
reportextension 50971 "XMS Ext" extends "XMS Extended"
{{
    rendering {{ layout(E) {{ Type = Excel; LayoutFile = 'XmsExt.xlsx'; }} }}
}}
table 50966 "XMS Num" {{ fields {{ field(1; Number; Integer) {{ }} }} keys {{ key(PK; Number) {{ }} }} }}
table 50964 "XMS Blob" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}

codeunit 50965 "XMS Tests"
{{
    local procedure Setup()
    var N: Record "XMS Num";
    begin
        if not N.IsEmpty() then exit;
        N.Number := 1; N.Insert(); N.Number := 2; N.Insert(); N.Number := 7; N.Insert();
        Commit();
    end;

    [TryFunction]
    local procedure TrySave(Id: Integer; Fmt: ReportFormat; var Tmp: Record "XMS Blob" temporary; var Saved: Boolean)
    var Out: OutStream;
    begin
        Tmp.Data.CreateOutStream(Out);
        Saved := Report.SaveAs(Id, '', Fmt, Out);
    end;

    local procedure Dataset(Id: Integer; Name: Text): Text
    var Tmp: Record "XMS Blob" temporary; InS: InStream; Saved: Boolean; Xml: Text;
    begin
        Setup();
        if not TrySave(Id, ReportFormat::Xml, Tmp, Saved) then
            exit('<failed: ' + GetLastErrorText() + '>');
        Tmp.Data.CreateInStream(InS);
        InS.Read(Xml);
        exit(Format(Saved) + '|' + Xml.Replace(Name, '<name>').Replace(Format(Id, 0, 9), '<id>'));
    end;

    procedure SameDataset(): Text
    var Plain: Text;
    begin
        Plain := Dataset(Report::"XMS Plain", 'XMS Plain');
        exit(CopyStr(Plain, 1, 3) + '|' + Format(Plain.Contains('SecondNumber')) + '|'
            + Format(Dataset(Report::"XMS Multi", 'XMS Multi') = Plain) + '|'
            + Format(Dataset(Report::"XMS Multi Word", 'XMS Multi Word') = Plain) + '|'
            + Format(Dataset(Report::"XMS Multi Excel", 'XMS Multi Excel') = Plain));
    end;

    [TryFunction]
    local procedure TryRun(Id: Integer; Modal: Boolean)
    begin
        if Modal then Report.RunModal(Id, false) else Report.Run(Id, false);
    end;

    local procedure RunFacts(Id: Integer; Modal: Boolean): Text
    begin
        Setup();
        ClearLastError();
        if TryRun(Id, Modal) then
            exit('Yes');
        exit('No|' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure RunNoLayout(): Text begin exit(RunFacts(Report::"XMS Multi", false)); end;
    procedure RunModalNoLayout(): Text begin exit(RunFacts(Report::"XMS Multi", true)); end;
    procedure RunExcel(): Text begin exit(RunFacts(Report::"XMS Multi Excel", false)); end;
    procedure RunModalExcel(): Text begin exit(RunFacts(Report::"XMS Multi Excel", true)); end;
    procedure RunWord(): Text begin exit(RunFacts(Report::"XMS Multi Word", false)); end;

    local procedure ExcelFacts(Id: Integer): Text
    var Tmp: Record "XMS Blob" temporary; Ok: Boolean; Saved: Boolean;
    begin
        Setup();
        Ok := TrySave(Id, ReportFormat::Excel, Tmp, Saved);
        exit(Format(Ok) + '|' + Format(Saved) + '|' + Format(Tmp.Data.Length() > 0));
    end;

    procedure SaveExcelLayout(): Text begin exit(ExcelFacts(Report::"XMS Multi Excel")); end;
    procedure SaveExcelNoLayout(): Text begin exit(ExcelFacts(Report::"XMS Multi")); end;
    // Every other call of a report with the property refuses (main's answer for the report), one per entry.
    procedure VarRunWord() var R: Report "XMS Multi Word"; begin Setup(); R.UseRequestPage(false); R.Run(); end;
    procedure VarRunNoLayout() var R: Report "XMS Multi"; begin Setup(); R.Run(); end;
    procedure VarRunModalExcel() var R: Report "XMS Multi Excel"; begin Setup(); R.RunModal(); end;
    procedure StaticRunWithPage() begin Setup(); Report.Run(Report::"XMS Multi Excel"); end;
    procedure StaticRunRequestPage(): Text begin Setup(); exit(Report.RunRequestPage(Report::"XMS Multi Excel")); end;
    procedure VarRunRequestPage(): Text var R: Report "XMS Multi"; begin Setup(); exit(R.RunRequestPage()); end;
    procedure StaticExecute() begin Setup(); Report.Execute(Report::"XMS Multi", ''); end;
    procedure StaticPrint() begin Setup(); Report.Print(Report::"XMS Multi", ''); end;
    procedure RunProcessingOnly() begin Setup(); Report.Run(Report::"XMS Proc", false); end;
    local procedure SaveFacts(Id: Integer; Fmt: ReportFormat): Text
    var Tmp: Record "XMS Blob" temporary; Saved: Boolean;
    begin
        Setup();
        if TrySave(Id, Fmt, Tmp, Saved) then;
        exit(Format(Saved));
    end;
    procedure SavePlainXml(): Text begin exit(SaveFacts(Report::"XMS Plain", ReportFormat::Xml)); end;
    procedure SavePdfNoLayout(): Text begin exit(SaveFacts(Report::"XMS Multi", ReportFormat::Pdf)); end;
    procedure SaveExcelOfWord(): Text begin exit(SaveFacts(Report::"XMS Multi Word", ReportFormat::Excel)); end;
    procedure SaveXmlProcessingOnly(): Text begin exit(SaveFacts(Report::"XMS Proc", ReportFormat::Xml)); end;
    procedure SaveXmlTwoLayouts(): Text begin exit(SaveFacts(Report::"XMS Two Layouts", ReportFormat::Xml)); end;
    procedure SaveXmlLegacyLayout(): Text begin exit(SaveFacts(Report::"XMS Legacy", ReportFormat::Xml)); end;
    procedure SaveXmlExtended(): Text begin exit(SaveFacts(Report::"XMS Extended", ReportFormat::Xml)); end;
    procedure SaveXmlOfVariable(): Text
    var R: Report "XMS Multi"; Tmp: Record "XMS Blob" temporary; Out: OutStream;
    begin
        Setup();
        Tmp.Data.CreateOutStream(Out);
        exit(Format(R.SaveAs('', ReportFormat::Xml, Out)));
    end;
    procedure SaveXmlWithRecordRef(): Text
    var Tmp: Record "XMS Blob" temporary; Out: OutStream; RR: RecordRef;
    begin
        Setup();
        RR.Open(Database::"XMS Num");
        Tmp.Data.CreateOutStream(Out);
        exit(Format(Report.SaveAs(Report::"XMS Multi", '', ReportFormat::Xml, Out, RR)));
    end;
    procedure SaveXmlWithParameters(): Text
    var Tmp: Record "XMS Blob" temporary; Out: OutStream;
    begin
        Setup();
        Tmp.Data.CreateOutStream(Out);
        exit(Format(Report.SaveAs(Report::"XMS Multi", '<?xml version="1.0" standalone="yes"?><ReportParameters/>', ReportFormat::Xml, Out)));
    end;
    procedure SavePdfOfWord(): Text
    var Tmp: Record "XMS Blob" temporary; Saved: Boolean;
    begin
        Setup();
        TrySave(Report::"XMS Multi Word", ReportFormat::Pdf, Tmp, Saved);
        exit(Format(Saved));
    end;
}}
"#
    )
}

fn text(proc: &str) -> String {
    al_testkit::text(Built::from_source(&src()).run_in("XMS Tests", proc))
}

#[test]
fn multiple_data_sheets_refuses_every_call() {
    for p in [
        "SameDataset",
        "RunNoLayout",
        "RunModalNoLayout",
        "RunExcel",
        "RunModalExcel",
        "SaveExcelLayout",
        "SaveExcelNoLayout",
        "RunWord",
        "SavePdfOfWord",
        "VarRunWord",
        "VarRunNoLayout",
        "VarRunModalExcel",
        "StaticRunWithPage",
        "StaticRunRequestPage",
        "VarRunRequestPage",
        "StaticExecute",
        "StaticPrint",
        "RunProcessingOnly",
        "SavePdfNoLayout",
        "SaveExcelOfWord",
        "SaveXmlProcessingOnly",
        "SaveXmlTwoLayouts",
        "SaveXmlLegacyLayout",
        "SaveXmlExtended",
        "SaveXmlOfVariable",
        "SaveXmlWithRecordRef",
        "SaveXmlWithParameters",
    ] {
        match Built::from_source(&src()).run_in("XMS Tests", p) {
            Err(Signal::Unsupported(why)) => {
                assert!(
                    why.to_lowercase().contains("excellayoutmultipledatasheets"),
                    "{p}: {why}"
                )
            }
            other => panic!("{p}: expected a refusal, got {other:?}"),
        }
    }
}

#[test]
fn report_without_the_property_saves_its_dataset() {
    assert_eq!(text("SavePlainXml"), "Yes");
}
