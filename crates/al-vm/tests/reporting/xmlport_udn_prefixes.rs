//! An Xml port with `Namespaces` and `UseDefaultNamespace = true` but no `DefaultNamespace` (al-runner issue 7091; corpus
//! xmlports/shapes-properties/XpUseDefaultNsPrefixes.al, BC 27.5, 28.5, 29.0). The export
//! writes the root without its prefix, with `xmlns` set to the uri of that prefix (BC's namespace of the port for a root without a
//! prefix); an element without a prefix is in the port's namespace and declares it where the namespace in scope differs.

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const SOAP: &str = "http://schemas.xmlsoap.org/soap/envelope/";
const TEM: &str = "http://tempuri.org/";

fn pfx(p: Option<&str>) -> String {
    p.map(|p| format!(" NamespacePrefix = '{p}';")).unwrap_or_default()
}

/// A port like those of XpUseDefaultNsPrefixes.al: Envelope > Body > Item (Id, Name), or the reporter's table element root
/// (`table_root`: Envelope is the table element, MaxOccurs = Once, with Body inside). `Go()` gives `call`.
#[allow(clippy::too_many_arguments)]
fn src(
    encoding: &str,
    table_root: bool,
    root: Option<&str>,
    body: Option<&str>,
    item: Option<&str>,
    id: Option<&str>,
    call: &str,
) -> String {
    let fields = format!(
        "fieldelement(Id; Rw.Id) {{{} }} fieldelement(Name; Rw.Name) {{ }}",
        pfx(id)
    );
    let schema = if table_root {
        format!(
            "tableelement(Rw; \"UP Item\") {{ MaxOccurs = Once;{} XmlName = 'Envelope'; textelement(Body) {{{} {fields} }} }}",
            pfx(root),
            pfx(body)
        )
    } else {
        format!(
            "textelement(Envelope) {{{} textelement(Body) {{{} tableelement(Rw; \"UP Item\") {{ XmlName = 'Item';{} {fields} }} }} }}",
            pfx(root),
            pfx(body),
            pfx(item)
        )
    };
    format!(
        r#"
table 69970 "UP Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69971 "UP Item" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Name; Text[50]) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
xmlport 69972 "UP Port"
{{
    Direction = Both; Format = Xml; {encoding} UseRequestPage = false;
    Namespaces = soap = '{SOAP}', tem = '{TEM}';
    UseDefaultNamespace = true;
    schema {{ {schema} }}
}}
codeunit 69973 C
{{
    procedure Go(): Text begin exit({call}); end;

    procedure Export(Utf16: Boolean): Text
    var B: Record "UP Blob" temporary; R: Record "UP Item"; OutS: OutStream; InS: InStream; Line: Text; T: Text;
    begin
        R.Id := 1; R.Name := 'one'; R.Insert();
        R.Id := 2; R.Name := '  two  '; R.Insert();
        B.Data.CreateOutStream(OutS);
        Xmlport.Export(Xmlport::"UP Port", OutS);
        if Utf16 then B.Data.CreateInStream(InS, TextEncoding::UTF16) else B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin InS.ReadText(Line); T += Line + '|'; end;
        exit(T);
    end;

    procedure Import(): Text
    var B: Record "UP Blob" temporary; OutS: OutStream; InS: InStream;
    begin
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('<Envelope><Body><Item><Id>9</Id><Name>n9</Name></Item></Body></Envelope>');
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        Xmlport.Import(Xmlport::"UP Port", InS);
        exit('');
    end;
}}
"#
    )
}

const UTF8: &str = "Encoding = UTF8;";

fn export(encoding: &str, table_root: bool, prefixes: [Option<&str>; 4]) -> String {
    let [root, body, item, id] = prefixes;
    let utf16 = encoding.is_empty();
    text(run_in(
        &src(encoding, table_root, root, body, item, id, &format!("Export({utf16})")),
        "C",
        "Go",
    ))
}

fn head(encoding: &str, default_ns: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"{encoding}\" standalone=\"no\"?>|<Envelope xmlns:soap=\"{SOAP}\" xmlns:tem=\"{TEM}\" xmlns=\"{default_ns}\">|"
    )
}

const PORT_NS: &str = "urn:microsoft-dynamics-nav/xmlports/x69972";

/// The two items below `<soap:Body>` as XUD_TextRoot_Export writes them: `<Item>` declares the port's namespace.
fn items_declared() -> String {
    format!(
        "    <Item xmlns=\"{PORT_NS}\">|      <tem:Id>1</tem:Id>|      <Name>one</Name>|    </Item>|    <Item xmlns=\"{PORT_NS}\">|      <tem:Id>2</tem:Id>|      <Name>  two  </Name>|    </Item>|"
    )
}

// XUD_TableRoot_Export: the reporter's shape.
#[test]
fn the_reporters_table_root_drops_its_prefix_and_takes_its_uri_as_default() {
    assert_eq!(
        export(UTF8, true, [Some("soap"), Some("soap"), None, Some("tem")]),
        format!(
            "{}  <soap:Body>|    <tem:Id>1</tem:Id>|    <Name xmlns=\"{PORT_NS}\">one</Name>|  </soap:Body>|</Envelope>|",
            head("UTF-8", SOAP)
        )
    );
}

// XUD_TextRoot_Export, XUD_TextRootUtf16_Export, XUD_RootSecond_Export.
#[test]
fn a_text_root_takes_the_uri_of_its_prefix_and_an_unprefixed_element_declares_the_port_namespace() {
    let p = [Some("soap"), Some("soap"), None, Some("tem")];
    let body = format!("  <soap:Body>|{}  </soap:Body>|</Envelope>|", items_declared());
    assert_eq!(export(UTF8, false, p), format!("{}{body}", head("UTF-8", SOAP)));
    assert_eq!(export("", false, p), format!("{}{body}", head("UTF-16", SOAP)));
    assert_eq!(
        export(UTF8, false, [Some("tem"), Some("soap"), None, Some("tem")]),
        format!("{}{body}", head("UTF-8", TEM))
    );
}

// XUD_RootPlain_Export: a root without a prefix gets the port's namespace; nothing below declares it again.
#[test]
fn a_root_without_a_prefix_gets_the_port_namespace() {
    assert_eq!(
        export(UTF8, false, [None, Some("soap"), None, Some("tem")]),
        format!(
            "{}  <soap:Body>|    <Item>|      <tem:Id>1</tem:Id>|      <Name>one</Name>|    </Item>|    <Item>|      <tem:Id>2</tem:Id>|      <Name>  two  </Name>|    </Item>|  </soap:Body>|</Envelope>|",
            head("UTF-8", PORT_NS)
        )
    );
}

// XUD_RootOnly_Export, XUD_ItemPrefix_Export.
#[test]
fn the_port_namespace_is_declared_on_the_first_unprefixed_element_of_each_branch() {
    assert_eq!(
        export(UTF8, false, [Some("soap"), None, None, None]),
        format!(
            "{}  <Body xmlns=\"{PORT_NS}\">|    <Item>|      <Id>1</Id>|      <Name>one</Name>|    </Item>|    <Item>|      <Id>2</Id>|      <Name>  two  </Name>|    </Item>|  </Body>|</Envelope>|",
            head("UTF-8", SOAP)
        )
    );
    assert_eq!(
        export(UTF8, false, [Some("soap"), Some("soap"), Some("tem"), None]),
        format!(
            "{}  <soap:Body>|    <tem:Item>|      <Id xmlns=\"{PORT_NS}\">1</Id>|      <Name xmlns=\"{PORT_NS}\">one</Name>|    </tem:Item>|    <tem:Item>|      <Id xmlns=\"{PORT_NS}\">2</Id>|      <Name xmlns=\"{PORT_NS}\">  two  </Name>|    </tem:Item>|  </soap:Body>|</Envelope>|",
            head("UTF-8", SOAP)
        )
    );
}

// XUD_*_Import: every import of these ports fails with one error, whatever the document (XpUseDefaultNsPrefixes.al:249, :280, :311, :342).
#[test]
fn every_import_fails_with_the_undefined_prefix_error() {
    let r = run_in(
        &src(UTF8, false, Some("soap"), Some("soap"), None, Some("tem"), "Import()"),
        "C",
        "Go",
    );
    let Err(Signal::Error(e)) = r else { panic!("{r:?}") };
    assert_eq!(
        e.message,
        "The prefix  is not defined in the Namespaces property of the XMLPort."
    );
    assert_eq!(e.code, "XmlPortNodeProcess");
}
