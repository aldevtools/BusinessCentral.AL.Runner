//! `CurrReport.FormatRegion := r` after `CurrReport.Language := l` in a trigger, in a report run by `SaveAs` and by `Run`
//! (reports/runtime/BdRegionTests.al:1346-1480 BdSetterLanguage_*, :1712-1912 Y89_Lang_*): Format follows the region, the long
//! date takes the month name of the language, and the dataset names the language's culture and the region.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;

const FIXTURE: &str = r#"
table 50970 "RL Row"
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Id) { } }
}

codeunit 50971 "RL Log"
{
    SingleInstance = true;
    var Entries: Text; Region: Text; Lang: Integer;
    procedure Add(E: Text) begin Entries += E + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
    procedure SetCulture(R: Text; L: Integer) begin Region := R; Lang := L; end;
    procedure GetRegion(): Text begin exit(Region); end;
    procedure GetLang(): Integer begin exit(Lang); end;
}

report 50972 "RL Report"
{
    UseRequestPage = false;
    dataset
    {
        dataitem(T; "RL Row")
        {
            DataItemTableView = where(Id = const(1));
            column(AmtC; Amt) { }
            column(DateC; D) { }
        }
    }
    var
        L: Codeunit "RL Log";
        Amt: Decimal;
        D: Date;
    trigger OnPreReport()
    begin
        Amt := 1234567.891;
        D := DMY2Date(5, 3, 2024);
        CurrReport.Language := L.GetLang();
        if L.GetRegion() <> '' then
            CurrReport.FormatRegion := L.GetRegion();
        L.Add(Format(Amt) + '#' + Format(D) + '#' + Format(D, 0, 4) + ' language=' + Format(CurrReport.Language) + ' region=[' + CurrReport.FormatRegion + ']');
    end;
}

report 50974 "RL Proc"
{
    UseRequestPage = false;
    ProcessingOnly = true;
    var
        L: Codeunit "RL Log";
        Amt: Decimal;
        D: Date;
    trigger OnPreReport()
    begin
        Amt := 1234567.891;
        D := DMY2Date(5, 3, 2024);
        CurrReport.Language := L.GetLang();
        if L.GetRegion() <> '' then
            CurrReport.FormatRegion := L.GetRegion();
        L.Add(Format(Amt) + '#' + Format(D) + '#' + Format(D, 0, 4) + ' language=' + Format(CurrReport.Language) + ' region=[' + CurrReport.FormatRegion + ']');
    end;
}

codeunit 50973 "RL Tests"
{
    local procedure Lines(var B: Record "RL Row"): Text
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
            if Line.StartsWith('<ReportDataSet') or Line.StartsWith('<Column ') or Line.StartsWith('<Language>') or Line.StartsWith('<FormatRegion>') then
                R += Line + '|';
        end;
        exit(R);
    end;

    procedure SaveAs(Region: Text; Lang: Integer): Text
    var
        B: Record "RL Row";
        L: Codeunit "RL Log";
        Rpt: Report "RL Report";
        OutS: OutStream;
    begin
        L.SetCulture(Region, Lang);
        if not B.Get(1) then begin
            B.Id := 1;
            B.Insert();
        end;
        B.Data.CreateOutStream(OutS);
        Rpt.SaveAs('', ReportFormat::Xml, OutS);
        B.Modify();
        exit(L.Take() + Lines(B));
    end;

    procedure Run(Region: Text; Lang: Integer): Text
    var
        B: Record "RL Row";
        L: Codeunit "RL Log";
    begin
        L.SetCulture(Region, Lang);
        if not B.Get(1) then begin
            B.Id := 1;
            B.Insert();
        end;
        Report.Run(Report::"RL Proc", false);
        exit(L.Take());
    end;
}
"#;

fn call(routine: &str, region: &str, lang: i32) -> Result<Value, Signal> {
    al_testkit::run_with(
        FIXTURE,
        "RL Tests",
        routine,
        vec![Value::Text(region.into()), Value::Integer(lang)],
    )
    .map(|(v, _)| v)
}

/// BdRegionTests.al:1350 BdSetterLanguage_1034_EsEsTradnl and :1774 Y89_Lang_EsES_1034_D: Spanish month, the region's order and
/// separators, the header names `es-ES` (the culture of 1034) and the region as typed.
#[test]
fn a_saveas_in_a_language_and_its_region() {
    assert_eq!(
        text(call("SaveAs", "es-ES", 1034)),
        "1.234.567,891#05/03/24#5 de marzo de 2024 language=1034 region=[es-ES];\
<ReportDataSet name=\"RL Report\" id=\"50972\" language=\"es-ES\" formatRegion=\"es-ES\">|<Language>es-ES</Language>|<FormatRegion>es-ES</FormatRegion>|\
<Column name=\"AmtC\" decimalformatter=\"#,##0.00\">1234567,891</Column>|<Column name=\"DateC\">05/03/2024</Column>|"
    );
    // :1890 Y89_Lang_DeAT_3079_D: de-AT groups with U+00A0 and writes the German month.
    assert_eq!(
        text(call("SaveAs", "de-AT", 3079)),
        "1\u{A0}234\u{A0}567,891#05.03.24#5. März 2024 language=3079 region=[de-AT];\
<ReportDataSet name=\"RL Report\" id=\"50972\" language=\"de-AT\" formatRegion=\"de-AT\">|<Language>de-AT</Language>|<FormatRegion>de-AT</FormatRegion>|\
<Column name=\"AmtC\" decimalformatter=\"#,##0.00\">1234567,891</Column>|<Column name=\"DateC\">05.03.2024</Column>|"
    );
}

/// BdRegionTests.al:1082 BdSetterLanguage_1030_FrFR: Danish month in the order of fr-FR, the header names `da-DK`.
#[test]
fn a_saveas_in_a_language_and_another_region() {
    assert_eq!(
        text(call("SaveAs", "fr-FR", 1030)),
        "1\u{A0}234\u{A0}567,891#05/03/24#5 marts 2024 language=1030 region=[fr-FR];\
<ReportDataSet name=\"RL Report\" id=\"50972\" language=\"da-DK\" formatRegion=\"fr-FR\">|<Language>da-DK</Language>|<FormatRegion>fr-FR</FormatRegion>|\
<Column name=\"AmtC\" decimalformatter=\"#,##0.00\">1234567,891</Column>|<Column name=\"DateC\">05/03/2024</Column>|"
    );
}

/// BdRegionTests.al:1819 Y89_Lang_SvSE_1053_R, :1848 Y89_Lang_NbNO_1044_R, :1761 Y89_Lang_NlNL_1043_R: the same texts in a
/// report started by Run.
#[test]
fn a_run_in_a_language_and_its_region() {
    assert_eq!(
        text(call("Run", "sv-SE", 1053)),
        "1\u{A0}234\u{A0}567,891#24-03-05#den 5 mars 2024 language=1053 region=[sv-SE];"
    );
    assert_eq!(
        text(call("Run", "nb-NO", 1044)),
        "1\u{A0}234\u{A0}567,891#05.03.24#5. mars 2024 language=1044 region=[nb-NO];"
    );
    assert_eq!(
        text(call("Run", "nl-NL", 1043)),
        "1.234.567,891#05-03-24#5 maart 2024 language=1043 region=[nl-NL];"
    );
}

/// A language the corpus pins only with its region: alone in a SaveAs it stays refused (the formats in that language are not
/// built), as does a month whose name in that language no test pins.
#[test]
fn a_language_without_its_region_is_refused() {
    match call("SaveAs", "", 1034) {
        Err(Signal::Unsupported(_)) => {}
        other => panic!("expected a refusal, got {other:?}"),
    }
}
