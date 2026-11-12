//! A parameters string with the declaration `XmlDocument.WriteTo(Text)` writes (al-runner issue 7908, follow-up to 7590): the
//! reporter's repro, with the data item on a table of its own (the virtual table Integer needs the System app) and the test
//! codeunit run as a plain one. The repro itself, unchanged, was run with al-runner against BC 29's apps. Under SaveAs the
//! string is read as the same string with the plain declaration (assumption report-params-written-declaration); every other
//! shape keeps main's refusal.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::Built;

const SRC: &str = r#"
table 50280 "PX Blob"
{
    TableType = Temporary;
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50283 "PX Row"
{
    fields { field(1; Number; Integer) { } }
    keys { key(PK; Number) { } }
}

report 50281 "PX Param Report"
{
    UsageCategory = None;
    dataset { dataitem(R; "PX Row") { column(Num; Number) { } } }
    requestpage
    {
        layout { area(Content) { field(StartCtl; StartDate) { ApplicationArea = All; } field(NoteCtl; NoteText) { ApplicationArea = All; } } }
    }
    trigger OnInitReport()
    begin
        NoteText := 'init';
    end;
    trigger OnPreReport()
    begin
        Error('%1|%2', StartDate, NoteText);
    end;
    var
        StartDate: Date;
        NoteText: Text;
}

codeunit 50282 "PX Xml Decl Tests"
{
    local procedure Saved(Xml: Text): Text
    var B: Record "PX Blob"; Out: OutStream;
    begin
        B.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"PX Param Report", Xml, ReportFormat::Xml, Out);
    end;

    procedure WrittenByWriteTo(): Text
    var Doc: XmlDocument; Node: XmlNode; Xml: Text;
    begin
        XmlDocument.ReadFrom('<?xml version="1.0" standalone="yes"?><ReportParameters><Options><Field name="StartDate" /><Field name="NoteText" /></Options></ReportParameters>', Doc);
        Doc.SelectSingleNode('ReportParameters/Options/Field[@name="StartDate"]', Node);
        Node.AsXmlElement().Add(XmlText.Create('2026-01-15'));
        Doc.WriteTo(Xml);
        exit(Saved(Xml));
    end;

    procedure WriteToText(): Text
    var Doc: XmlDocument; Node: XmlNode; Xml: Text;
    begin
        XmlDocument.ReadFrom('<?xml version="1.0" standalone="yes"?><ReportParameters><Options><Field name="StartDate" /><Field name="NoteText" /></Options></ReportParameters>', Doc);
        Doc.SelectSingleNode('ReportParameters/Options/Field[@name="StartDate"]', Node);
        Node.AsXmlElement().Add(XmlText.Create('2026-01-15'));
        Doc.WriteTo(Xml);
        exit(Xml);
    end;

    procedure Utf16DeclarationOneLine(): Text
    begin
        exit(Saved('<?xml version="1.0" encoding="utf-16" standalone="yes"?><ReportParameters><Options><Field name="StartDate">2026-01-15</Field></Options></ReportParameters>'));
    end;

    procedure Utf8Declaration(): Text
    begin
        exit(Saved('<?xml version="1.0" encoding="utf-8" standalone="yes"?><ReportParameters><Options><Field name="StartDate">2026-01-15</Field></Options></ReportParameters>'));
    end;

    procedure WhitespaceValue(): Text
    var Xml: Text; CrLf: Text[2];
    begin
        CrLf[1] := 13; CrLf[2] := 10;
        Xml := '<?xml version="1.0" encoding="utf-16" standalone="yes"?><ReportParameters><Options><Field name="NoteText">' + CrLf + '  </Field></Options></ReportParameters>';
        exit(Saved(Xml));
    end;

    // Breaks the probe did not measure (review shapes P03, P04, P05, P17, P26).
    local procedure Crlf(): Text
    var C: Text[2];
    begin
        C[1] := 13; C[2] := 10;
        exit(C);
    end;

    procedure BreaksWithoutSpaces(): Text
    var D: Text;
    begin
        D := '<?xml version="1.0" encoding="utf-16" standalone="yes"?>';
        exit(Saved(D + Crlf() + '<ReportParameters>' + Crlf() + '<Options>' + Crlf() + '<Field name="StartDate">2026-01-15</Field>' + Crlf() + '</Options>' + Crlf() + '</ReportParameters>'));
    end;

    procedure BreaksWithOtherSpaces(): Text
    var D: Text;
    begin
        D := '<?xml version="1.0" encoding="utf-16" standalone="yes"?>';
        exit(Saved(D + Crlf() + '<ReportParameters>' + Crlf() + '       <Options>' + Crlf() + ' <Field name="StartDate">2026-01-15</Field>' + Crlf() + '   </Options>' + Crlf() + '</ReportParameters>'));
    end;

    procedure OneBreakOnly(): Text
    begin
        exit(Saved('<?xml version="1.0" encoding="utf-16" standalone="yes"?><ReportParameters><Options>' + Crlf() + '    <Field name="StartDate">2026-01-15</Field></Options></ReportParameters>'));
    end;

    procedure BreakAfterDeclarationOnly(): Text
    begin
        exit(Saved('<?xml version="1.0" encoding="utf-16" standalone="yes"?>' + Crlf() + '<ReportParameters><Options><Field name="StartDate">2026-01-15</Field></Options></ReportParameters>'));
    end;

    procedure BreakInsideAnAttribute(): Text
    begin
        exit(Saved('<?xml version="1.0" encoding="utf-16" standalone="yes"?><ReportParameters><Options><Field name="Start>' + Crlf() + '  Date">2026-01-15</Field></Options></ReportParameters>'));
    end;

    procedure WithDataItems(): Text
    begin
        exit(Saved('<?xml version="1.0" encoding="utf-16" standalone="yes"?><ReportParameters><Options><Field name="StartDate">2026-01-15</Field></Options><DataItems><DataItem name="R">VERSION(1) SORTING(Field1)</DataItem></DataItems></ReportParameters>'));
    end;

    procedure NamedRoot(): Text
    begin
        exit(Saved('<?xml version="1.0" encoding="utf-16" standalone="yes"?><ReportParameters name="PX Param Report" id="50281"><Options><Field name="StartDate">2026-01-15</Field></Options></ReportParameters>'));
    end;

    procedure Executed()
    begin
        Report.Execute(Report::"PX Param Report", '<?xml version="1.0" encoding="utf-16" standalone="yes"?><ReportParameters><Options><Field name="StartDate">2026-01-15</Field></Options></ReportParameters>');
    end;
}
"#;

fn run(routine: &str) -> (Result<Value, Signal>, Vec<Assumption>) {
    let mut b = Built::from_source(SRC);
    clear_hits();
    let r = b.run_in("PX Xml Decl Tests", routine);
    (r, take_hits())
}

/// The text of the error OnPreReport raises with the two option values.
fn raised(r: Result<Value, Signal>) -> String {
    match r {
        Err(Signal::Error(e)) => e.message.to_string(),
        other => panic!("expected the error of OnPreReport, got {other:?}"),
    }
}

#[test]
fn the_reporters_two_strings_set_and_blank_the_options_as_on_bc() {
    // BC 29 (issue 7908): WriteTo gives StartDate 2026-01-15 and NoteText blank; the one-line string keeps NoteText 'init'.
    let (r, hits) = run("WrittenByWriteTo");
    assert_eq!(raised(r), "01/15/26|");
    assert!(hits.contains(&Assumption::ReportParamsWrittenDeclaration), "{hits:?}");
    let (r, hits) = run("Utf16DeclarationOneLine");
    assert_eq!(raised(r), "01/15/26|init");
    assert!(hits.contains(&Assumption::ReportParamsWrittenDeclaration), "{hits:?}");
}

#[test]
fn the_runtimes_write_to_gives_the_text_the_reporter_measured() {
    // BC 29 (issue 7908): 203 characters, the utf-16 declaration, then CR LF and two spaces of indentation per level.
    let (r, _) = run("WriteToText");
    let t = al_testkit::text(r);
    assert_eq!(t.chars().count(), 203, "{t:?}");
    assert!(t.starts_with(
        "<?xml version=\"1.0\" encoding=\"utf-16\" standalone=\"yes\"?>\r\n<ReportParameters>\r\n  <Options>"
    ));
}

#[track_caller]
fn keeps_mains_refusal(routine: &str) {
    let (r, hits) = run(routine);
    assert!(matches!(&r, Err(Signal::Unsupported(_))), "{routine}: {r:?}");
    assert!(
        !hits.contains(&Assumption::ReportParamsWrittenDeclaration),
        "{routine}: {hits:?}"
    );
}

#[test]
fn other_declarations_values_data_items_roots_and_calls_keep_mains_refusal() {
    keeps_mains_refusal("Utf8Declaration");
    keeps_mains_refusal("WhitespaceValue");
    keeps_mains_refusal("WithDataItems");
    keeps_mains_refusal("NamedRoot");
    keeps_mains_refusal("Executed");
    keeps_mains_refusal("BreaksWithoutSpaces");
    keeps_mains_refusal("BreaksWithOtherSpaces");
    keeps_mains_refusal("OneBreakOnly");
    keeps_mains_refusal("BreakAfterDeclarationOnly");
    keeps_mains_refusal("BreakInsideAnAttribute");
}
