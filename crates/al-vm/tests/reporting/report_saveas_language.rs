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

codeunit 50993 "SL Tests"
{
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
