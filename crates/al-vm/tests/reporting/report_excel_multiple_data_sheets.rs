//! `ExcelLayoutMultipleDataSheets = true` on a report (al-runner issue 7836): the shapes the corpus probe measured run as the
//! same report without the property (assumption excel-layout-multiple-data-sheets, ExcelMultipleDataSheetsReport.al); every
//! other run of such a report refuses. Rust tests until the probe is pinned.

use al_builtins::assumptions::{self, Assumption};
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
fn multiple_data_sheets_saveas_xml_gives_the_dataset_of_the_report_without_it() {
    assumptions::take_hits();
    assert_eq!(text("SameDataset"), "Yes|Yes|Yes|Yes|Yes");
    assert!(assumptions::take_hits().contains(&Assumption::ExcelLayoutMultipleDataSheets));
}

#[test]
fn multiple_data_sheets_run_without_a_request_page_raises_as_measured() {
    let no_layout = "No|Report 50961 does not have a valid layout. Valid layout types are: Word, RDLC, Excel, and Custom.|ReportNoLayout";
    assert_eq!(text("RunNoLayout"), no_layout);
    assert_eq!(text("RunModalNoLayout"), no_layout);
    let excel = "No|The current report operation is not supported for Excel Layout reports.|ReportUnsupportedOperation";
    assert_eq!(text("RunExcel"), excel);
    assert_eq!(text("RunModalExcel"), excel);
}

#[test]
fn multiple_data_sheets_saveas_excel_answers_as_measured() {
    assert_eq!(text("SaveExcelLayout"), "Yes|Yes|Yes");
    assert_eq!(text("SaveExcelNoLayout"), "Yes|No|No");
}

#[test]
fn multiple_data_sheets_shapes_not_measured_refuse() {
    for p in ["RunWord", "SavePdfOfWord"] {
        match Built::from_source(&src()).run_in("XMS Tests", p) {
            Err(Signal::Unsupported(why)) => assert!(why.contains("ExcelLayoutMultipleDataSheets"), "{p}: {why}"),
            other => panic!("{p}: expected a refusal, got {other:?}"),
        }
    }
}
