//! An Xml port with `Namespaces`, `DefaultNamespace` and `UseDefaultNamespace = true` (al-runner issue 7131; corpus
//! xmlports/shapes-properties/XpUseDefaultNsWithDefault.al from corpus PR 1158). The export writes the root
//! without its prefix, with `xmlns` set to the uri of that prefix (the DefaultNamespace for a root without a prefix); an element
//! without a prefix is in the DefaultNamespace and declares it where the namespace in scope differs.

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const SOAP: &str = "http://schemas.xmlsoap.org/soap/envelope/";
const TEM: &str = "http://tempuri.org/";
const DN: &str = "urn:lt:xdn:input";

fn src(encoding: &str, schema: &str, call: &str) -> String {
    src_with(encoding, &format!("DefaultNamespace = '{DN}';"), schema, call)
}

fn src_with(encoding: &str, default_ns: &str, schema: &str, call: &str) -> String {
    format!(
        r#"
table 69980 "DN Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69981 "DN Item" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Name; Text[50]) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
xmlport 69982 "DN Port"
{{
    Direction = Both; Format = Xml; {encoding} UseRequestPage = false;
    {default_ns}
    Namespaces = soap = '{SOAP}', tem = '{TEM}';
    UseDefaultNamespace = true;
    schema {{ {schema} }}
}}
codeunit 69983 C
{{
    procedure Go(): Text begin exit({call}); end;

    procedure Export(Utf16: Boolean): Text
    var B: Record "DN Blob" temporary; R: Record "DN Item"; OutS: OutStream; InS: InStream; Line: Text; T: Text;
    begin
        R.Id := 1; R.Name := 'one'; R.Insert();
        R.Id := 2; R.Name := '  two  '; R.Insert();
        B.Data.CreateOutStream(OutS);
        Xmlport.Export(Xmlport::"DN Port", OutS);
        if Utf16 then B.Data.CreateInStream(InS, TextEncoding::UTF16) else B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin InS.ReadText(Line); T += Line + '|'; end;
        exit(T);
    end;

    procedure Import(): Text
    var B: Record "DN Blob" temporary; OutS: OutStream; InS: InStream;
    begin
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('<Envelope><Body><Id>9</Id><Name>n9</Name></Body></Envelope>');
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        Xmlport.Import(Xmlport::"DN Port", InS);
        exit('');
    end;
}}
"#
    )
}

const UTF8: &str = "Encoding = UTF8;";

fn export(encoding: &str, schema: &str) -> String {
    let utf16 = encoding.is_empty();
    text(run_in(&src(encoding, schema, &format!("Export({utf16})")), "C", "Go"))
}

fn head(encoding: &str, default_ns: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"{encoding}\" standalone=\"no\"?>|<Envelope xmlns:soap=\"{SOAP}\" xmlns:tem=\"{TEM}\" xmlns=\"{default_ns}\">|"
    )
}

const ITEM: &str = "tableelement(Rw; \"DN Item\") { XmlName = 'Item'; fieldelement(Id; Rw.Id) { NamespacePrefix = 'tem'; } fieldelement(Name; Rw.Name) { } }";

fn in_input_msg(inner: &str) -> String {
    format!(
        "textelement(Envelope) {{ NamespacePrefix = 'soap'; textelement(Body) {{ NamespacePrefix = 'soap'; textelement(ReceiveMessage) {{ NamespacePrefix = 'tem'; textelement(InputMsg) {{ {inner} }} }} }} }}"
    )
}

// XDN_TextLeaf_Export: the reporter's shape.
#[test]
fn the_reporters_input_msg_declares_the_default_namespace() {
    let leaf = "textelement(Value) { trigger OnBeforePassVariable() begin Value := 'V1'; end; }";
    assert_eq!(
        export(UTF8, &in_input_msg(leaf)),
        format!(
            "{}  <soap:Body>|    <tem:ReceiveMessage>|      <InputMsg xmlns=\"{DN}\">|        <Value>V1</Value>|      </InputMsg>|    </tem:ReceiveMessage>|  </soap:Body>|</Envelope>|",
            head("UTF-8", SOAP)
        )
    );
}

fn items_in_input_msg(encoding: &str) -> String {
    format!(
        "{}  <soap:Body>|    <tem:ReceiveMessage>|      <InputMsg xmlns=\"{DN}\">|        <Item>|          <tem:Id>1</tem:Id>|          <Name>one</Name>|        </Item>|        <Item>|          <tem:Id>2</tem:Id>|          <Name>  two  </Name>|        </Item>|      </InputMsg>|    </tem:ReceiveMessage>|  </soap:Body>|</Envelope>|",
        head(encoding, SOAP)
    )
}

// XDN_Item_Export, XDN_ItemUtf16_Export.
#[test]
fn a_table_element_below_input_msg_inherits_the_default_namespace() {
    assert_eq!(export(UTF8, &in_input_msg(ITEM)), items_in_input_msg("UTF-8"));
    assert_eq!(export("", &in_input_msg(ITEM)), items_in_input_msg("UTF-16"));
}

// XDN_RootPlain_Export, XDN_RootSecond_Export.
#[test]
fn the_root_takes_the_uri_of_its_prefix_or_the_default_namespace() {
    let root = |prefix: &str| {
        format!("textelement(Envelope) {{ {prefix} textelement(Body) {{ NamespacePrefix = 'soap'; {ITEM} }} }}")
    };
    let items = |declared: &str| {
        format!(
            "  <soap:Body>|    <Item{declared}>|      <tem:Id>1</tem:Id>|      <Name>one</Name>|    </Item>|    <Item{declared}>|      <tem:Id>2</tem:Id>|      <Name>  two  </Name>|    </Item>|  </soap:Body>|</Envelope>|"
        )
    };
    assert_eq!(export(UTF8, &root("")), format!("{}{}", head("UTF-8", DN), items("")));
    assert_eq!(
        export(UTF8, &root("NamespacePrefix = 'tem';")),
        format!("{}{}", head("UTF-8", TEM), items(&format!(" xmlns=\"{DN}\"")))
    );
}

const TABLE_ROOT: &str = "tableelement(Rw; \"DN Item\") { MaxOccurs = Once; NamespacePrefix = 'soap'; XmlName = 'Envelope'; textelement(Body) { NamespacePrefix = 'soap'; fieldelement(Id; Rw.Id) { NamespacePrefix = 'tem'; } fieldelement(Name; Rw.Name) { } } }";

// XDN_TableRoot_Export.
#[test]
fn a_table_root_drops_its_prefix() {
    assert_eq!(
        export(UTF8, TABLE_ROOT),
        format!(
            "{}  <soap:Body>|    <tem:Id>1</tem:Id>|    <Name xmlns=\"{DN}\">one</Name>|  </soap:Body>|</Envelope>|",
            head("UTF-8", SOAP)
        )
    );
}

// XDN_TableRoot_Import, XDN_TextLeaf_Import, XDN_Item_Import: every import fails the same way, whatever the document (corpus
// PR 1158 and 1169, XpUseDefaultNsWithDefault.al, BC 27.5, 28.5, 29.0).
#[test]
fn every_import_fails_with_the_undefined_prefix_error() {
    for schema in [TABLE_ROOT.to_string(), in_input_msg(ITEM)] {
        let r = run_in(&src(UTF8, &schema, "Import()"), "C", "Go");
        let Err(Signal::Error(e)) = r else { panic!("{r:?}") };
        assert_eq!(
            e.message,
            "The prefix  is not defined in the Namespaces property of the XMLPort."
        );
        assert_eq!(e.code, "XmlPortNodeProcess");
    }
}

const ATTR_ITEM: &str = "tableelement(Rm; \"DN Item\") { XmlName = 'Remark'; fieldattribute(Type; Rm.Id) { ATTRPREFIX } fieldelement(Text; Rm.Name) { } }";

fn attr_in_input_msg(attr_prefix: &str) -> String {
    in_input_msg(&ATTR_ITEM.replace("ATTRPREFIX", attr_prefix))
}

fn remarks(ns: &str, inner_ns: &str) -> String {
    format!(
        "{}  <soap:Body>|    <tem:ReceiveMessage>|      <InputMsg xmlns=\"{inner_ns}\">|        <Remark Type=\"1\">|          <Text>one</Text>|        </Remark>|        <Remark Type=\"2\">|          <Text>  two  </Text>|        </Remark>|      </InputMsg>|    </tem:ReceiveMessage>|  </soap:Body>|</Envelope>|",
        head("UTF-8", ns)
    )
}

// XDN_AttrDefault_Export: a field attribute on a table element does not change the namespaces.
#[test]
fn a_field_attribute_below_input_msg_keeps_the_namespaces() {
    assert_eq!(export(UTF8, &attr_in_input_msg("")), remarks(SOAP, DN));
}

// XDN_AttrNoDefault_Export: without DefaultNamespace InputMsg takes the namespace of the port (issue 7091).
#[test]
fn without_a_default_namespace_input_msg_takes_the_namespace_of_the_port() {
    let r = text(run_in(
        &src_with(UTF8, "", &attr_in_input_msg(""), "Export(false)"),
        "C",
        "Go",
    ));
    assert_eq!(r, remarks(SOAP, "urn:microsoft-dynamics-nav/xmlports/x69982"));
}

// XDN_AttrPrefixed_Export: an attribute with NamespacePrefix is written without a prefix, a text attribute too.
#[test]
fn an_attribute_is_written_without_a_namespace_prefix() {
    assert_eq!(
        export(UTF8, &attr_in_input_msg("NamespacePrefix = 'soap';")),
        remarks(SOAP, DN)
    );
    let schema = format!(
        "textelement(Envelope) {{ NamespacePrefix = 'soap'; textelement(Body) {{ NamespacePrefix = 'soap'; textelement(InputMsg) {{ NamespacePrefix = 'tem'; textattribute(Ver) {{ trigger OnBeforePassVariable() begin Ver := 'v1'; end; }} {} }} }} }}",
        ATTR_ITEM.replace("ATTRPREFIX", "NamespacePrefix = 'soap';")
    );
    assert_eq!(
        export(UTF8, &schema),
        format!(
            "{}  <soap:Body>|    <tem:InputMsg Ver=\"v1\">|      <Remark Type=\"1\" xmlns=\"{DN}\">|        <Text>one</Text>|      </Remark>|      <Remark Type=\"2\" xmlns=\"{DN}\">|        <Text>  two  </Text>|      </Remark>|    </tem:InputMsg>|  </soap:Body>|</Envelope>|",
            head("UTF-8", SOAP)
        )
    );
}
