//! XmlPort `InlineSchema = true`: the export writes an XSD inside the root element, then the rows without
//! indentation (corpus xmlport/lt-y-xmlport-props/YXmlPortProps.al:563 InlineSchema_Export_ContainsSchema, a root text
//! element with one table element of an Integer and a Text field), and xmlport/lt-bd-xmlport/BdXmlPort.al:425-492
//! (more field types, attributes, an optional text element, an optional table element without rows). Other shapes are refused.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50560 "XI Item" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } field(3; Amount; Decimal) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50561 "XI Sink" { fields { field(1; Id; Integer) { } field(3; Sink; Blob) { } } keys { key(PK; Id) { Clustered = true; } } }
xmlport 50560 "XI Inline" { Format = Xml; Direction = Both; UseRequestPage = false; InlineSchema = true;
    schema { textelement(Root) { tableelement(Rw; "XI Item") { XmlName = 'Item';
        fieldelement(Id; Rw.Id) { } fieldelement(Name; Rw.Name) { } } } } }
xmlport 50561 "XI Decimal" { Format = Xml; Direction = Export; UseRequestPage = false; InlineSchema = true;
    schema { textelement(Root) { tableelement(Rw; "XI Item") { fieldelement(Amount; Rw.Amount) { } } } } }
xmlport 50562 "XI Attr" { Format = Xml; Direction = Export; UseRequestPage = false; InlineSchema = true;
    schema { textelement(Root) { tableelement(Rw; "XI Item") { MinOccurs = Zero; fieldattribute(Id; Rw.Id) { } textattribute(Note) { } fieldelement(Name; Rw.Name) { } textelement(Extra) { MinOccurs = Zero; } } } } }
xmlport 50564 "XI Text Attr" { Format = Xml; Direction = Export; UseRequestPage = false; InlineSchema = true;
    schema { textelement(Root) { tableelement(Rw; "XI Item") { fieldattribute(Name; Rw.Name) { } fieldelement(Id; Rw.Id) { } } } } }
xmlport 50565 "XI Optional Field" { Format = Xml; Direction = Export; UseRequestPage = false; InlineSchema = true;
    schema { textelement(Root) { tableelement(Rw; "XI Item") { fieldelement(Id; Rw.Id) { MinOccurs = Zero; } } } } }
xmlport 50563 "XI Plain" { Format = Xml; Direction = Export; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Rw; "XI Item") { XmlName = 'Item'; fieldelement(Id; Rw.Id) { } fieldelement(Name; Rw.Name) { } } } } }
codeunit 50560 "T"
{
    Subtype = Test;
    local procedure Seed()
    var I: Record "XI Item";
    begin
        I.DeleteAll();
        I.Id := 1; I.Name := 'one'; I.Insert();
        I.Id := 2; I.Name := 'two'; I.Insert();
    end;
    local procedure Export(Id: Integer): Text
    var S: Record "XI Sink"; OutS: OutStream; InS: InStream; Line: Text; R: Text;
    begin
        S.DeleteAll(); S.Id := 1; S.Insert();
        S.Sink.CreateOutStream(OutS);
        Xmlport.Export(Id, OutS);
        S.Modify(); S.CalcFields(Sink);
        S.Sink.CreateInStream(InS, TextEncoding::UTF16);
        while not InS.EOS() do begin InS.ReadText(Line); R += Line + '|'; end;
        exit(R);
    end;
    [Test]
    procedure Inline()
    var T: Text;
    begin
        Seed();
        T := Export(50560);
        if T <> '<?xml version="1.0" encoding="UTF-16" standalone="no"?>|<Root>|<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">|  <xs:complexType name="Item">|    <xs:sequence>|      <xs:element minOccurs="1" maxOccurs="1" default="0" name="Id" type="xs:int" />|      <xs:element minOccurs="1" maxOccurs="1" name="Name" type="xs:string" />|    </xs:sequence>|  </xs:complexType>|  <xs:complexType name="Root" mixed="true">|    <xs:sequence>|      <xs:element minOccurs="1" maxOccurs="unbounded" name="Item" type="Item" />|    </xs:sequence>|  </xs:complexType>|  <xs:element name="Root" type="Root" />|</xs:schema><Item><Id>1</Id><Name>one</Name></Item><Item><Id>2</Id><Name>two</Name></Item></Root>|' then Error('inline %1', T);
        T := Export(50563);
        if T <> '<?xml version="1.0" encoding="UTF-16" standalone="no"?>|<Root>|  <Item>|    <Id>1</Id>|    <Name>one</Name>|  </Item>|  <Item>|    <Id>2</Id>|    <Name>two</Name>|  </Item>|</Root>|' then Error('plain %1', T);
    end;
    [Test]
    procedure DecimalAndAttributes()
    var T: Text; I: Record "XI Item";
    begin
        Seed();
        T := Export(50561);
        if T <> '<?xml version="1.0" encoding="UTF-16" standalone="no"?>|<Root>|<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">|  <xs:complexType name="Rw">|    <xs:sequence>|      <xs:element minOccurs="1" maxOccurs="1" name="Amount" type="xs:string" />|    </xs:sequence>|  </xs:complexType>|  <xs:complexType name="Root" mixed="true">|    <xs:sequence>|      <xs:element minOccurs="1" maxOccurs="unbounded" name="Rw" type="Rw" />|    </xs:sequence>|  </xs:complexType>|  <xs:element name="Root" type="Root" />|</xs:schema><Rw><Amount>0.00</Amount></Rw><Rw><Amount>0.00</Amount></Rw></Root>|' then Error('decimal %1', T);
        T := Export(50562);
        if T <> '<?xml version="1.0" encoding="UTF-16" standalone="no"?>|<Root>|<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">|  <xs:complexType name="Rw">|    <xs:sequence>|      <xs:element minOccurs="1" maxOccurs="1" name="Name" type="xs:string" />|      <xs:element minOccurs="0" maxOccurs="unbounded" name="Extra" type="xs:string" />|    </xs:sequence>|    <xs:attribute name="Id" type="xs:string" use="required" />|    <xs:attribute name="Note" type="xs:string" use="required" />|  </xs:complexType>|  <xs:complexType name="Root" mixed="true">|    <xs:sequence>|      <xs:element minOccurs="0" maxOccurs="unbounded" name="Rw" type="Rw" />|    </xs:sequence>|  </xs:complexType>|  <xs:element name="Root" type="Root" />|</xs:schema><Rw Id="1" Note=""><Name>one</Name><Extra /></Rw><Rw Id="2" Note=""><Name>two</Name><Extra /></Rw></Root>|' then Error('attributes %1', T);
        I.DeleteAll();
        T := Export(50562);
        if StrPos(T, '</xs:schema></Root>|') = 0 then Error('no rows %1', T);
    end;
    [Test]
    procedure TextAttributeIsRefused() begin Seed(); Export(50564); end;
    [Test]
    procedure OptionalFieldIsRefused() begin Seed(); Export(50565); end;
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
        codeunit: 50560,
        name: name.into(),
    })
}

#[test]
fn the_export_writes_the_schema_inside_the_root() {
    assert_eq!(outcome("Inline"), Outcome::Pass);
}

#[test]
fn more_types_attributes_and_optional_elements_are_written() {
    assert_eq!(outcome("DecimalAndAttributes"), Outcome::Pass);
}

#[test]
fn other_shapes_are_refused() {
    for t in ["TextAttributeIsRefused", "OptionalFieldIsRefused"] {
        match outcome(t) {
            Outcome::NotRun(why) => assert!(
                why.to_lowercase().contains("inlineschema") || why.contains("inline"),
                "{t}: {why}"
            ),
            other => panic!("{t}: {other:?}"),
        }
    }
}
