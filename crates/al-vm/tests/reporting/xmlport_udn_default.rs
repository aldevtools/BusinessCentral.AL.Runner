//! An Xml port with `Namespaces`, `DefaultNamespace` and `UseDefaultNamespace = true` (al-runner issue 7131; corpus
//! xmlports/shapes-properties/XpUseDefaultNsWithDefault.al on branch al-runtime-rs-staging-r7130). The export writes the root
//! without its prefix, with `xmlns` set to the uri of that prefix (the DefaultNamespace for a root without a prefix); an element
//! without a prefix is in the DefaultNamespace and declares it where the namespace in scope differs.

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const SOAP: &str = "http://schemas.xmlsoap.org/soap/envelope/";
const TEM: &str = "http://tempuri.org/";
const DN: &str = "urn:lt:xdn:input";

fn src(encoding: &str, schema: &str, call: &str) -> String {
    format!(
        r#"
table 69980 "DN Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69981 "DN Item" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Name; Text[50]) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
xmlport 69982 "DN Port"
{{
    Direction = Both; Format = Xml; {encoding} UseRequestPage = false;
    DefaultNamespace = '{DN}';
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

// XDN_*_Import: the import is refused.
#[test]
fn the_import_stays_refused() {
    let r = run_in(&src(UTF8, TABLE_ROOT, "Import()"), "C", "Go");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}
