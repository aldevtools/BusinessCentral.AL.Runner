//! `CurrReport.Language` and `CurrReport.FormatRegion` in a report run by `SaveAs` (report/W3ReportObjectIdPreviewLanguage.al:
//! 513-583): each run starts with no language set; the formats of another language or region are not built there and refused.

use al_builtins::native::Signal;
use al_testkit::text;

const FIXTURE: &str = r#"
table 50990 "SL Row"
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Id) { } }
}

codeunit 50991 "SL Log"
{
    SingleInstance = true;
    var Entries: Text; Mode: Text; Lang: Integer;
    procedure Add(E: Text) begin Entries += E + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
    procedure SetMode(M: Text; L: Integer) begin Mode := M; Lang := L; end;
    procedure GetMode(): Text begin exit(Mode); end;
    procedure GetLang(): Integer begin exit(Lang); end;
}

report 50992 "SL Report"
{
    dataset { dataitem(T; "SL Row") { column(TId; Id) { } } }
    var L: Codeunit "SL Log";
    trigger OnPreReport()
    begin
        L.Add('before=' + Format(CurrReport.Language));
        case L.GetMode() of
            'lang': CurrReport.Language := L.GetLang();
            'lang format':
                begin
                    CurrReport.Language := L.GetLang();
                    L.Add(Format(1234.5));
                end;
            'region': CurrReport.FormatRegion := 'de-DE';
            'session region': CurrReport.FormatRegion := 'en-US';
        end;
    end;
}

report 50994 "SL Fmt"
{
    dataset
    {
        dataitem(T; "SL Row")
        {
            column(DecC; DecVar) { }
            column(DateC; DateVar) { }
            column(NegC; NegVar) { }
            column(TimeC; TimeVar) { }
            column(BoolC; BoolVar) { }
            column(FmtC; FmtVar) { }
            trigger OnAfterGetRecord()
            begin
                DecVar := 1234.5;
                DateVar := DMY2Date(5, 3, 2024);
                NegVar := -1234567.891;
                TimeVar := 093005T;
                BoolVar := true;
                FmtVar := Format(DecVar) + '|' + Format(DateVar) + '|' + Format(NegVar) + '|' + Format(TimeVar) + '|' + Format(BoolVar);
            end;
        }
    }
    var
        L: Codeunit "SL Log";
        DecVar: Decimal;
        DateVar: Date;
        NegVar: Decimal;
        TimeVar: Time;
        BoolVar: Boolean;
        FmtVar: Text;
    trigger OnPreReport()
    begin
        L.Add('pre language=' + Format(CurrReport.Language) + ' region=[' + CurrReport.FormatRegion + '] global=' + Format(GlobalLanguage()));
    end;
}

report 50995 "SL Fmt Labels"
{
    dataset { dataitem(T; "SL Row") { column(TId; Id) { } } }
    labels { Lbl = 'Label'; }
}

codeunit 50993 "SL Tests"
{
    local procedure Lines(var B: Record "SL Row"): Text
    var
        InS: InStream;
        Line: Text;
        R: Text;
    begin
        B.CalcFields(Data);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin
            InS.ReadText(Line);
            Line := Line.Trim();
            if Line.Contains(' tableId=') then
                Line := Line.Substring(1, Line.IndexOf(' tableId=') - 1) + '>';
            if Line.StartsWith('<ReportDataSet') or Line.StartsWith('<DataItem ') or Line.StartsWith('<Column ') or Line.StartsWith('<Language>') or Line.StartsWith('<FormatRegion>') then
                R += Line + '|';
        end;
        exit(R);
    end;

    local procedure SaveFmt(var Rpt: Report "SL Fmt"): Text
    var
        B: Record "SL Row";
        L: Codeunit "SL Log";
        OutS: OutStream;
    begin
        if not B.Get(1) then begin
            B.Id := 1;
            B.Insert();
        end;
        B.Data.CreateOutStream(OutS);
        Rpt.SaveAs('', ReportFormat::Xml, OutS);
        B.Modify();
        exit(L.Take() + Lines(B));
    end;

    procedure VariableLanguage(): Text
    var
        Rpt: Report "SL Fmt";
        Read: Text;
    begin
        Rpt.Language := 1031;
        Read := 'read=' + Format(Rpt.Language) + ';';
        exit(Read + SaveFmt(Rpt));
    end;

    procedure VariableRegion(): Text
    var
        Rpt: Report "SL Fmt";
        Read: Text;
    begin
        Rpt.FormatRegion := 'de-DE';
        Read := 'read=' + Rpt.FormatRegion + ';';
        exit(Read + SaveFmt(Rpt));
    end;

    procedure AfterGlobalLanguage(): Text
    var
        Rpt: Report "SL Fmt";
        R: Text;
    begin
        GlobalLanguage(1031);
        R := SaveFmt(Rpt);
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure VariableLanguageOther(): Text
    var
        Rpt: Report "SL Fmt";
    begin
        Rpt.Language := 1036;
        exit(SaveFmt(Rpt));
    end;

    procedure VariableRegionOther(): Text
    var
        Rpt: Report "SL Fmt";
    begin
        Rpt.FormatRegion := 'fr-FR';
        exit(Rpt.FormatRegion);
    end;

    procedure VariableLanguageAndRegion(): Text
    var
        Rpt: Report "SL Fmt";
    begin
        Rpt.Language := 1031;
        Rpt.FormatRegion := 'de-DE';
        exit(SaveFmt(Rpt));
    end;

    procedure VariableLanguageRun(): Text
    var
        Rpt: Report "SL Fmt";
    begin
        Rpt.Language := 1031;
        Rpt.UseRequestPage(false);
        Rpt.Run();
        exit('ran');
    end;

    procedure GlobalLanguageLabels(): Text
    var
        B: Record "SL Row";
        Rpt: Report "SL Fmt Labels";
        OutS: OutStream;
    begin
        if not B.Get(1) then begin
            B.Id := 1;
            B.Insert();
        end;
        B.Data.CreateOutStream(OutS);
        GlobalLanguage(1031);
        Rpt.SaveAs('', ReportFormat::Xml, OutS);
        exit('saved');
    end;

    local procedure Save()
    var
        B: Record "SL Row";
        Rpt: Report "SL Report";
        OutS: OutStream;
    begin
        if not B.Get(1) then begin
            B.Id := 1;
            B.Insert();
        end;
        B.Data.CreateOutStream(OutS);
        Rpt.SaveAs('', ReportFormat::Xml, OutS);
        B.Modify();
    end;

    procedure OtherLanguage(): Text
    var
        L: Codeunit "SL Log";
    begin
        L.SetMode('lang', 1031);
        Save();
        exit(L.Take());
    end;

    procedure OtherLanguageFormat(): Text
    var
        L: Codeunit "SL Log";
    begin
        L.SetMode('lang format', 1031);
        Save();
        exit(L.Take());
    end;

    // A language one run sets is not what the next run starts with.
    procedure NextRunStartsUnset(): Text
    var
        L: Codeunit "SL Log";
    begin
        L.SetMode('lang', 1031);
        Save();
        L.SetMode('', 0);
        Save();
        exit(L.Take());
    end;

    // CurrReport.Language := n in OnPreReport: the dataset's language.
    procedure TriggerLanguageXml(Lang: Integer): Text
    var
        B: Record "SL Row";
        L: Codeunit "SL Log";
    begin
        L.SetMode('lang', Lang);
        Save();
        L.Take();
        B.Get(1);
        exit(Lines(B));
    end;

    procedure OtherRegion(): Text
    var
        L: Codeunit "SL Log";
    begin
        L.SetMode('region', 0);
        Save();
        exit(L.Take());
    end;

    procedure SessionRegion(): Text
    var
        L: Codeunit "SL Log";
    begin
        L.SetMode('session region', 0);
        Save();
        exit(L.Take());
    end;
}
"#;

fn assert_refused(routine: &str, needle: &str) {
    match al_testkit::run_in(FIXTURE, "SL Tests", routine) {
        Err(Signal::Unsupported(m)) if m.contains(needle) => {}
        other => panic!("{routine}: expected a refusal containing {needle:?}, got {other:?}"),
    }
}

/// W3ReportObjectIdPreviewLanguage.al:546-570: a SaveAs in another language or region changes its formats and dataset, which is
/// not built. The language itself is kept (handlers/TestReportCurrReportLanguage.al:44-69 reads 1031 back).
#[test]
fn another_language_or_region_in_a_saveas_is_refused() {
    assert_eq!(
        text(al_testkit::run_in(FIXTURE, "SL Tests", "OtherLanguage")),
        "before=1033;"
    );
    assert_refused("OtherLanguageFormat", "CurrReport.Language := 1031");
    assert_refused("OtherRegion", "CurrReport.FormatRegion := 'de-DE'");
    assert_eq!(
        text(al_testkit::run_in(FIXTURE, "SL Tests", "SessionRegion")),
        "before=1033;"
    );
}

/// W3ReportObjectIdPreviewLanguage.al:572-583: each SaveAs starts with no language set.
#[test]
fn each_saveas_starts_with_no_language_set() {
    assert_eq!(
        text(al_testkit::run_in(FIXTURE, "SL Tests", "NextRunStartsUnset")),
        "before=1033;before=1033;"
    );
}

/// W3ReportObjectIdPreviewLanguage.al:893 RB_SaveAs_ReportVariableLanguage1031: `Language := 1031` on the variable reads back,
/// the run reads it as CurrReport.Language while GlobalLanguage stays 1033, Format writes German (`Ja`), the column values stay
/// en-US, and the dataset says `language="de-DE" formatRegion="en-US"` (BdRegionTests.al:928 for the header).
#[test]
fn a_language_set_on_the_variable_carries_into_the_saveas() {
    assert_eq!(
        text(al_testkit::run_in(FIXTURE, "SL Tests", "VariableLanguage")),
        "read=1031;pre language=1031 region=[] global=1033;\
<ReportDataSet name=\"SL Fmt\" id=\"50994\" language=\"de-DE\" formatRegion=\"en-US\">|<Language>de-DE</Language>|<FormatRegion>en-US</FormatRegion>|\
<DataItem language=\"de-DE\" formatRegion=\"en-US\" name=\"T\">|<Column name=\"DecC\" decimalformatter=\"#,##0.00\">1234.5</Column>|\
<Column name=\"DateC\">3/5/2024</Column>|<Column name=\"NegC\" decimalformatter=\"#,##0.00\">-1234567.891</Column>|<Column name=\"TimeC\">9:30:05 AM</Column>|<Column name=\"BoolC\">True</Column>|<Column name=\"FmtC\">1,234.5|03/05/24|-1,234,567.891| 9:30:05 AM|Ja</Column>|"
    );
}

/// W3ReportObjectIdPreviewLanguage.al:870 RB_SaveAs_AfterGlobalLanguage1031: the same after GlobalLanguage(1031), which the run
/// reads as its language too.
#[test]
fn a_saveas_after_global_language_1031() {
    assert_eq!(
        text(al_testkit::run_in(FIXTURE, "SL Tests", "AfterGlobalLanguage")),
        "pre language=1031 region=[] global=1031;\
<ReportDataSet name=\"SL Fmt\" id=\"50994\" language=\"de-DE\" formatRegion=\"en-US\">|<Language>de-DE</Language>|<FormatRegion>en-US</FormatRegion>|\
<DataItem language=\"de-DE\" formatRegion=\"en-US\" name=\"T\">|<Column name=\"DecC\" decimalformatter=\"#,##0.00\">1234.5</Column>|\
<Column name=\"DateC\">3/5/2024</Column>|<Column name=\"NegC\" decimalformatter=\"#,##0.00\">-1234567.891</Column>|<Column name=\"TimeC\">9:30:05 AM</Column>|<Column name=\"BoolC\">True</Column>|<Column name=\"FmtC\">1,234.5|03/05/24|-1,234,567.891| 9:30:05 AM|Ja</Column>|"
    );
}

/// W3ReportObjectIdPreviewLanguage.al:912 RB_SaveAs_ReportVariableFormatRegionDeDE: `FormatRegion := 'de-DE'` on the variable
/// reads back and is the region of the run: German columns and Format (a negative decimal, a Time and a one-digit day and hour as
/// :912 pins them), `formatRegion="de-DE"` (BdRegionTests.al:809 for the header).
#[test]
fn a_region_set_on_the_variable_carries_into_the_saveas() {
    assert_eq!(
        text(al_testkit::run_in(FIXTURE, "SL Tests", "VariableRegion")),
        "read=de-DE;pre language=1033 region=[de-DE] global=1033;\
<ReportDataSet name=\"SL Fmt\" id=\"50994\" language=\"en-US\" formatRegion=\"de-DE\">|<Language>en-US</Language>|<FormatRegion>de-DE</FormatRegion>|\
<DataItem language=\"en-US\" formatRegion=\"de-DE\" name=\"T\">|<Column name=\"DecC\" decimalformatter=\"#,##0.00\">1234,5</Column>|\
<Column name=\"DateC\">05.03.2024</Column>|<Column name=\"NegC\" decimalformatter=\"#,##0.00\">-1234567,891</Column>|<Column name=\"TimeC\">09:30:05</Column>|<Column name=\"BoolC\">True</Column>|<Column name=\"FmtC\">1.234,5|05.03.24|-1.234.567,891|09:30:05|Yes</Column>|"
    );
}

/// Not pinned, refused: another language or region on the variable, both together, a Run of a variable with a language, and the
/// Labels block in another language.
#[test]
fn unpinned_variable_cultures_are_refused() {
    assert_refused("VariableLanguageOther", "1036");
    assert_refused("VariableRegionOther", "fr-FR");
    assert_refused("VariableLanguageAndRegion", "de-DE");
    assert_refused("VariableLanguageRun", "Language");
    assert_refused("GlobalLanguageLabels", "Labels");
}

/// W3ReportObjectIdPreviewLanguage.al:605 Language_Set1031_FormatAndColumns and BdRegionTests.al:928: `CurrReport.Language := 1031`
/// in a trigger of a SaveAs makes the dataset's language de-DE, on the data items and in the header. Another language there is
/// refused when the dataset is written.
#[test]
fn a_language_a_trigger_sets_is_the_language_of_the_dataset() {
    let r = al_testkit::run_with(
        FIXTURE,
        "SL Tests",
        "TriggerLanguageXml",
        vec![al_builtins::value::Value::Integer(1031)],
    )
    .map(|(v, _)| v);
    let lines = text(r);
    assert!(
        lines.starts_with("<ReportDataSet name=\"SL Report\" id=\"50992\" language=\"de-DE\" formatRegion=\"en-US\">|<Language>de-DE</Language>|<FormatRegion>en-US</FormatRegion>|<DataItem language=\"de-DE\" formatRegion=\"en-US\" name=\"T\">|"),
        "{lines}"
    );
    match al_testkit::run_with(
        FIXTURE,
        "SL Tests",
        "TriggerLanguageXml",
        vec![al_builtins::value::Value::Integer(1036)],
    ) {
        Err(Signal::Unsupported(m)) if m.contains("1036") => {}
        other => panic!("expected a refusal, got {other:?}"),
    }
}
