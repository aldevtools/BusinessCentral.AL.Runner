//! A VariableText import whose record separator is CR/LF while the document holds a lone LF: the line break lands inside a value
//! (corpus xmlports/shapes-properties/Y67XpSeparators.al:Assign_Import `record sep crlf token` reads `[1|a|b 2]` on real BC).
//! A line feed inside a text field value reads as a space; a carriage return inside one is not pinned and refused.

use al_builtins::native::Signal;
use al_testkit::run;

const SRC: &str = r#"
table 69970 "XL Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
table 69971 "XL Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } field(3; Note; Text[30]) { } } keys { key(PK; Id) { } } }
xmlport 69972 "XL Port"
{
    Direction = Import; Format = VariableText; FieldSeparator = ','; FieldDelimiter = '"'; RecordSeparator = '<CR/LF>'; UseRequestPage = false;
    schema { textelement(Root) { tableelement(R; "XL Row") { fieldelement(Id; R.Id) { } fieldelement(Name; R.Name) { } fieldelement(Note; R.Note) { } } } }
}
codeunit 69973 C
{
    procedure F(): Text
    var B: Record "XL Blob" temporary; OutS: OutStream; InS: InStream; Lf: Text; Ch: Char; Row: Record "XL Row";
    begin
        Ch := 10; Lf += Ch;
        B.Code := 'X'; B.Insert();
        B.Data.CreateOutStream(OutS);
        OutS.WriteText('1,a,b' + Lf + '2,c,d');
        B.Data.CreateInStream(InS);
        Xmlport.Import(Xmlport::"XL Port", InS);
        Row.FindFirst();
        exit(Row.Note);
    end;
}
"#;

#[test]
fn a_line_feed_inside_a_variable_text_value_reads_as_a_space() {
    assert_eq!(al_testkit::text(run(SRC, "F")), "b 2");
}

#[test]
fn a_carriage_return_inside_a_variable_text_value_is_refused() {
    let src = SRC.replace("Ch := 10;", "Ch := 13;");
    match run(&src, "F") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("carriage return"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_line_feed_inside_a_decimal_value_reads_as_a_space_in_its_error() {
    // Y83XpMiscTests.al:VariableText_ImportValueCount_2 `vt lf only`: `7.25<LF>8` fails as `7.25 8`.
    let src = SRC
        .replace("field(3; Note; Text[30])", "field(3; Note; Decimal)")
        .replace("'1,a,b' + Lf + '2,c,d'", "'1,a,7.25' + Lf + '8,c,d'");
    match run(&src, "F") {
        Err(Signal::Error(e)) => assert_eq!(e.message, "The value \"7.25 8\" can't be evaluated into type Decimal."),
        other => panic!("expected the evaluate error, got {other:?}"),
    }
}

#[test]
fn a_field_separator_that_holds_the_record_separator_is_read_separator_by_separator() {
    // Y83XpMiscTests.al:VariableText_NewLineSeparator `newline doc crlf`: with FieldSeparator <NewLine> and RecordSeparator <LF>,
    // `7<CR/LF>seven<CR/LF>x<CR/LF>` is one record.
    let src = SRC
        .replace(
            "FieldDelimiter = '\"';",
            "FieldDelimiter = '\"'; FieldSeparator = '<NewLine>'; RecordSeparator = '<LF>';",
        )
        .replace("RecordSeparator = '<CR/LF>'; ", "")
        .replace(
            "'1,a,b' + Lf + '2,c,d'",
            "'7' + Cr + Lf + 'seven' + Cr + Lf + 'x' + Cr + Lf",
        )
        .replace("Ch := 10; Lf += Ch;", "Ch := 10; Lf += Ch; Ch := 13; Cr += Ch;")
        .replace("Lf: Text;", "Lf: Text; Cr: Text;");
    assert_eq!(al_testkit::text(run(&src, "F")), "x");
}
