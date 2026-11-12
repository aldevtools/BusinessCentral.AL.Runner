//! Dutch (Belgium, 2067) month and weekday names. reports/runtime/ReportLanguageNotRunFormats.al:242 pins `december woensdag`
//! for `<Month Text> <Weekday Text>` after `CurrReport.Language := 2067` in a SaveAs; the cut names (`<Month Text,3>`), the other
//! months and days, and `GlobalLanguage(2067)` outside a report are not pinned and stay refused.

use al_builtins::native::Signal;
use al_testkit::text;

const FIXTURE: &str = r#"
table 50960 "NL Row"
{
    fields { field(1; Id; Integer) { } field(3; B; Blob) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

report 50961 "NL Lang"
{
    UsageCategory = None;
    dataset
    {
        dataitem(One; "NL Row")
        {
            column(T; T) { }
            trigger OnAfterGetRecord()
            begin
                T := Format(DMY2Date(25, 12, 2024), 0, Pattern);
            end;
        }
    }
    trigger OnPreReport()
    begin
        CurrReport.Language := 2067;
    end;
    var
        T: Text;
        Pattern: Text;
    procedure SetPattern(P: Text)
    begin
        Pattern := P;
    end;
}

codeunit 50962 "NL Tests"
{
    local procedure Save(P: Text): Text
    var
        R: Report "NL Lang";
        Row: Record "NL Row";
        Tmp: Record "NL Row" temporary;
        OutS: OutStream;
        InS: InStream;
        Line: Text;
        Res: Text;
    begin
        Row.Id := 1;
        Row.Insert();
        R.SetPattern(P);
        Tmp.B.CreateOutStream(OutS);
        R.SaveAs('', ReportFormat::Xml, OutS);
        Tmp.B.CreateInStream(InS);
        while not InS.EOS() do begin
            InS.ReadText(Line);
            if Line.Contains('<Column ') then
                Res += Line.Trim();
        end;
        exit(Res);
    end;

    procedure Names(): Text
    begin
        exit(Save('<Month Text> <Weekday Text>'));
    end;

    procedure CutNames(): Text
    begin
        exit(Save('<Month Text,3> <Weekday Text,3>'));
    end;

    procedure GlobalLanguageNames(): Text
    begin
        GlobalLanguage(2067);
        exit(Format(DMY2Date(25, 12, 2024), 0, '<Month Text> <Weekday Text>'));
    end;
}
"#;

fn assert_refused(routine: &str, needle: &str) {
    match al_testkit::run_in(FIXTURE, "NL Tests", routine) {
        Err(Signal::Unsupported(m)) if m.contains(needle) => {}
        other => panic!("{routine}: expected a refusal containing {needle:?}, got {other:?}"),
    }
}

/// ReportLanguageNotRunFormats.al:242 `mw=december woensdag`.
#[test]
fn the_pinned_dutch_names_in_a_saveas() {
    assert_eq!(
        text(al_testkit::run_in(FIXTURE, "NL Tests", "Names")),
        "<Column name=\"T\">december woensdag</Column>"
    );
}

/// The cut names are not pinned in Dutch (`dec woe` is a guess).
#[test]
fn cut_dutch_names_are_refused() {
    assert_refused("CutNames", "Month Text,3 in the language 2067");
}

/// The pin sets the language with CurrReport.Language in a SaveAs; GlobalLanguage(2067) outside a report is not pinned.
#[test]
fn dutch_names_after_global_language_are_refused() {
    assert_refused("GlobalLanguageNames", "Month Text in the language 2067");
}
