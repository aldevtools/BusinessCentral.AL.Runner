//! XmlPort `Namespaces`, the `NamespacePrefix` of a field element, and `AutoCalcField` on a field node
//! (corpus xmlport/lt-y-xmlport-props/YXmlPortProps.al:652-679 NodeProperties_Export_NamesPrefixesAutoCalc,
//! NodeProperties_Import_NamesPrefixes). A prefixed element is written `p:Name` and the root declares `xmlns:p`;
//! the prefix on an attribute is not written. An import needs the prefix on the element. Declared but unused
//! namespaces, a prefix that is not declared and more than one namespace are not pinned and are refused.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50550 "XN Item" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } field(3; Total; Integer) { FieldClass = FlowField; CalcFormula = count("XN Line" where(ItemId = field(Id))); } } keys { key(PK; Id) { Clustered = true; } } }
table 50551 "XN Line" { fields { field(1; Id; Integer) { } field(2; ItemId; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50552 "XN Sink" { fields { field(1; Id; Integer) { } field(3; Sink; Blob) { } } keys { key(PK; Id) { Clustered = true; } } }
xmlport 50550 "XN Attr" { Format = Xml; Direction = Both; UseRequestPage = false; Namespaces = p = 'urn:lt-y';
    schema { textelement(Root) { tableelement(Rw; "XN Item") { XmlName = 'Item';
        fieldattribute(Id; Rw.Id) { XmlName = 'Ident'; NamespacePrefix = 'p'; }
        fieldattribute(Total; Rw.Total) { AutoCalcField = true; }
        textattribute(Note) { NamespacePrefix = 'p'; Width = 10; }
        fieldelement(Name; Rw.Name) { XmlName = 'Label'; NamespacePrefix = 'p'; Width = 20; }
        fieldelement(Count; Rw.Total) { XmlName = 'Lines'; AutoCalcField = true; } } } } }
xmlport 50551 "XN NoCalc" { Format = Xml; Direction = Both; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Rw; "XN Item") { XmlName = 'Item';
        fieldattribute(Total; Rw.Total) { AutoCalcField = false; }
        fieldelement(Count; Rw.Total) { AutoCalcField = false; } } } } }
xmlport 50552 "XN Unused" { Format = Xml; Direction = Export; UseRequestPage = false; Namespaces = p = 'urn:lt-y';
    schema { textelement(Root) { tableelement(Rw; "XN Item") { fieldelement(Id; Rw.Id) { } } } } }
xmlport 50553 "XN Undeclared" { Format = Xml; Direction = Export; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Rw; "XN Item") { fieldelement(Id; Rw.Id) { NamespacePrefix = 'q'; } } } } }
codeunit 50550 "T"
{
    Subtype = Test;
    local procedure Seed()
    var I: Record "XN Item"; L: Record "XN Line";
    begin
        I.DeleteAll(); L.DeleteAll();
        I.Id := 1; I.Name := 'one'; I.Insert();
        L.Id := 1; L.ItemId := 1; L.Insert();
        L.Id := 2; L.ItemId := 1; L.Insert();
    end;
    local procedure Export(Id: Integer): Text
    var S: Record "XN Sink"; OutS: OutStream; InS: InStream; Line: Text; R: Text;
    begin
        S.DeleteAll(); S.Id := 1; S.Insert();
        S.Sink.CreateOutStream(OutS);
        Xmlport.Export(Id, OutS);
        S.Modify(); S.CalcFields(Sink);
        S.Sink.CreateInStream(InS, TextEncoding::UTF16);
        while not InS.EOS() do begin InS.ReadText(Line); R += Line + '|'; end;
        exit(R);
    end;
    local procedure ImportDoc(Id: Integer; Doc: Text)
    var S: Record "XN Sink"; OutS: OutStream; InS: InStream;
    begin
        S.DeleteAll(); S.Id := 1; S.Insert();
        S.Sink.CreateOutStream(OutS);
        OutS.WriteText(Doc);
        S.Modify(); S.CalcFields(Sink);
        S.Sink.CreateInStream(InS);
        Xmlport.Import(Id, InS);
    end;
    [Test]
    procedure PrefixAndAutoCalcOnExport()
    var T: Text;
    begin
        Seed();
        T := Export(50550);
        if T <> '<?xml version="1.0" encoding="UTF-16" standalone="no"?>|<Root xmlns:p="urn:lt-y">|  <Item Ident="1" Total="2" Note="">|    <p:Label>one</p:Label>|    <Lines>2</Lines>|  </Item>|</Root>|' then Error('attr %1', T);
        T := Export(50551);
        if T <> '<?xml version="1.0" encoding="UTF-16" standalone="no"?>|<Root>|  <Item Total="0">|    <Count>0</Count>|  </Item>|</Root>|' then Error('nocalc %1', T);
    end;
    [Test]
    procedure ImportNeedsThePrefix()
    var I: Record "XN Item";
    begin
        Seed();
        ImportDoc(50550, '<Root xmlns:p="urn:lt-y"><Item Ident="11" Total="0" Note="n"><p:Label>eleven</p:Label><Lines>0</Lines></Item></Root>');
        I.Get(11);
        if I.Name <> 'eleven' then Error('name %1', I.Name);
        asserterror ImportDoc(50550, '<Root><Item Ident="12" Total="0" Note=""><Label>twelve</Label><Lines>0</Lines></Item></Root>');
        if GetLastErrorText() <> 'Imported XML cannot validate with the schema: The element ''Item'' has invalid child element ''Label''. List of possible elements expected: ''Label'' in namespace ''urn:lt-y''.' then Error('text %1', GetLastErrorText());
        if GetLastErrorCode() <> 'XmlPortInvalidXml' then Error('code %1', GetLastErrorCode());
    end;
    [Test]
    procedure DeclaredButUnusedIsRefused()
    begin
        Seed();
        Export(50552);
    end;
    [Test]
    procedure UndeclaredPrefixIsRefused()
    begin
        Seed();
        Export(50553);
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
        codeunit: 50550,
        name: name.into(),
    })
}

#[test]
fn a_prefixed_element_is_written_with_its_prefix_and_autocalcfield_decides_the_flowfield() {
    assert_eq!(outcome("PrefixAndAutoCalcOnExport"), Outcome::Pass);
}

#[test]
fn an_import_needs_the_prefix_on_the_element() {
    assert_eq!(outcome("ImportNeedsThePrefix"), Outcome::Pass);
}

#[test]
fn unpinned_namespace_shapes_are_refused() {
    for t in ["DeclaredButUnusedIsRefused", "UndeclaredPrefixIsRefused"] {
        match outcome(t) {
            Outcome::NotRun(why) => assert!(why.contains("namespace"), "{t}: {why}"),
            other => panic!("{t}: {other:?}"),
        }
    }
}
