//! The XmlPort property `Encoding` on a Format = Xml port: UTF8 writes the declaration `UTF-8` and UTF-8 bytes
//! with no byte order mark, UTF16 (the default) writes `UTF-16` and UTF-16 little endian after a byte order mark
//! (corpus xmlport/lt-af-xmlport/AfXmlPortEncoding.al:AF_Encoding_Export). An import of a UTF-16 document that
//! declares `utf-8` fails in the reader (AF_Encoding_Import_Utf8Property_Utf16DocDeclaredUtf8). Other encodings
//! are refused.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50540 "XE Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } field(3; Sink; Blob) { } } keys { key(PK; Id) { Clustered = true; } } }
xmlport 50541 "XE Utf8" { Direction = Both; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Row; "XE Row") { fieldelement(Id; Row.Id) { } fieldelement(Name; Row.Name) { } } } } }
xmlport 50542 "XE Utf16" { Direction = Both; Format = Xml; Encoding = UTF16; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Row; "XE Row") { fieldelement(Id; Row.Id) { } fieldelement(Name; Row.Name) { } } } } }
xmlport 50543 "XE Dos" { Direction = Both; Format = Xml; Encoding = MSDOS; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Row; "XE Row") { fieldelement(Id; Row.Id) { } } } } }
codeunit 50540 "T"
{
    Subtype = Test;
    local procedure Head(var H: Record "XE Row"): Text
    var InS: InStream; B: Byte; N: Integer; Num: Integer; R: Text;
    begin
        H.CalcFields(Sink);
        H.Sink.CreateInStream(InS);
        R := '';
        while (not InS.EOS()) and (N < 3) do begin InS.Read(B); Num := B; N := N + 1; R += Format(Num) + ','; end;
        exit(R);
    end;
    [Test]
    procedure Utf8Export()
    var Row: Record "XE Row"; H: Record "XE Row"; OutS: OutStream; Xp: XmlPort "XE Utf8"; L: Text;
    begin
        Row.DeleteAll(); Row.Id := 1; Row.Name := 'ab'; Row.Insert();
        H.Id := 99; H.Insert();
        H.Sink.CreateOutStream(OutS);
        Xp.SetDestination(OutS); Xp.Export(); H.Modify();
        L := Head(H);
        if L <> '60,63,120,' then Error('utf8 %1', L);
    end;
    [Test]
    procedure Utf16Export()
    var Row: Record "XE Row"; H: Record "XE Row"; OutS: OutStream; Xp: XmlPort "XE Utf16"; L: Text;
    begin
        Row.DeleteAll(); Row.Id := 1; Row.Name := 'ab'; Row.Insert();
        H.Id := 99; H.Insert();
        H.Sink.CreateOutStream(OutS);
        Xp.SetDestination(OutS); Xp.Export(); H.Modify();
        L := Head(H);
        if L <> '255,254,60,' then Error('utf16 %1', L);
    end;
    [Test]
    procedure ImportUtf16DocDeclaredUtf8()
    var H: Record "XE Row"; OutS: OutStream; InS: InStream; Xp: XmlPort "XE Utf8";
    begin
        H.DeleteAll(); H.Id := 99; H.Insert();
        H.Sink.CreateOutStream(OutS, TextEncoding::UTF16);
        OutS.WriteText('<?xml version="1.0" encoding="utf-8"?><Root><Row><Id>5</Id><Name>x</Name></Row></Root>');
        H.Modify(); H.CalcFields(Sink);
        H.Sink.CreateInStream(InS);
        Xp.SetSource(InS);
        asserterror Xp.Import();
        if GetLastErrorText() <> 'Name cannot begin with the ''.'' character, hexadecimal value 0x00. Line 1, position 40.' then Error('text %1', GetLastErrorText());
        if GetLastErrorCode() <> 'XmlPortImport:Xml' then Error('code %1', GetLastErrorCode());
    end;
    [Test]
    procedure OtherEncodingIsRefused()
    var OutS: OutStream; Xp: XmlPort "XE Dos"; H: Record "XE Row";
    begin
        H.DeleteAll(); H.Id := 99; H.Insert();
        H.Sink.CreateOutStream(OutS);
        Xp.SetDestination(OutS); Xp.Export();
    end;
}
"#;

fn outcome(name: &str) -> Outcome {
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), SRC.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    );
    rt.first_outcome(&CaseId {
        codeunit: 50540,
        name: name.into(),
    })
}

#[test]
fn utf8_encoding_writes_utf8_bytes_without_a_byte_order_mark() {
    assert_eq!(outcome("Utf8Export"), Outcome::Pass);
}

#[test]
fn utf16_encoding_writes_the_default_utf16_bytes() {
    assert_eq!(outcome("Utf16Export"), Outcome::Pass);
}

#[test]
fn a_utf16_document_that_declares_utf8_fails_in_the_reader() {
    assert_eq!(outcome("ImportUtf16DocDeclaredUtf8"), Outcome::Pass);
}

#[test]
fn another_encoding_is_refused() {
    match outcome("OtherEncodingIsRefused") {
        Outcome::NotRun(why) => assert!(why.contains("encoding") && why.contains("MSDOS"), "{why}"),
        other => panic!("{other:?}"),
    }
}
