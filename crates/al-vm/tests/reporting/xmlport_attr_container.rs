//! A text element directly below a table element that holds only attributes, with a field attribute of the row
//! (corpus xmlport/lt-sg-xmlport-occurs/SgXmlPortOccurs.al:78-175 SG_AttrBelowText_*, xmlport 89361).

use al_builtins::native::Signal;
use al_testkit::run;

const SRC: &str = r#"
table 69950 "XA Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
table 69951 "XA Row" { fields { field(1; Id; Integer) { } field(2; EntryNo; Integer) { } field(3; Note; Text[30]) { } } keys { key(PK; Id) { } } }
xmlport 69952 "XA Port"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Row; "XA Row")
            {
                MinOccurs = Zero;
                fieldattribute(Id; Row.Id) { Occurrence = Required; }
                textelement(Sched)
                {
                    fieldattribute(EntryNo; Row.EntryNo) { Occurrence = Required; }
                    fieldattribute(Note; Row.Note) { Occurrence = Optional; }
                }
            }
        }
    }
}
codeunit 69953 C
{
    procedure Export(): Text
    var B: Record "XA Blob" temporary; R: Record "XA Row"; OutS: OutStream; InS: InStream; Line: Text; T: Text;
    begin
        R.Id := 1; R.EntryNo := 7; R.Note := 'n'; R.Insert();
        R.Id := 2; R.EntryNo := 0; R.Note := ''; R.Insert();
        B.Data.CreateOutStream(OutS);
        Xmlport.Export(Xmlport::"XA Port", OutS);
        B.Data.CreateInStream(InS, TextEncoding::UTF16);
        while not InS.EOS() do begin InS.ReadText(Line); T += Line + '|'; end;
        exit(T);
    end;

    procedure Import(Doc: Text): Text
    var B: Record "XA Blob" temporary; R: Record "XA Row"; OutS: OutStream; InS: InStream; T: Text;
    begin
        B.Data.CreateOutStream(OutS);
        OutS.WriteText(Doc);
        B.Data.CreateInStream(InS);
        Xmlport.Import(Xmlport::"XA Port", InS);
        if R.FindSet() then
            repeat T += Format(R.Id) + ':' + Format(R.EntryNo) + ':' + R.Note + ','; until R.Next() = 0;
        exit(T);
    end;

    procedure ImportAll(): Text
    begin
        exit(Import('<Root><Row Id="1"><Sched EntryNo="7" Note="n" /></Row><Row Id="2"><Sched EntryNo="8" /></Row></Root>'));
    end;

    procedure ImportMissing(): Text
    begin
        exit(Import('<Root><Row Id="1"><Sched Note="n" /></Row></Root>'));
    end;
}
"#;

#[test]
fn the_export_writes_the_attributes_on_the_text_element() {
    assert_eq!(
        al_testkit::text(run(SRC, "Export")),
        "<?xml version=\"1.0\" encoding=\"UTF-16\" standalone=\"no\"?>|<Root>|  <Row Id=\"1\">|    <Sched EntryNo=\"7\" Note=\"n\" />|  </Row>|  <Row Id=\"2\">|    <Sched EntryNo=\"0\" />|  </Row>|</Root>|"
    );
}

#[test]
fn the_import_reads_the_attributes_into_the_row() {
    assert_eq!(al_testkit::text(run(SRC, "ImportAll")), "1:7:n,2:8:,");
    match run(SRC, "ImportMissing") {
        Err(Signal::Error(e)) => assert_eq!(
            (e.message.as_str(), e.code.as_str()),
            (
                "Imported XML cannot validate with the schema: The required attribute 'EntryNo' is missing.",
                "XmlPortInvalidXml"
            )
        ),
        other => panic!("{other:?}"),
    }
}
