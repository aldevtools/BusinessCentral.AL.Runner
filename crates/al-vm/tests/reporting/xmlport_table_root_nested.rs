//! An Xml port whose root is a table element with text elements below it that hold content (al-runner issue 7130; corpus
//! xmlports/shapes-properties/XpTableRootNestedText.al on branch al-runtime-rs-staging-r7130, BC 27.5, 28.5, 29.0). BC writes the plain nested tree:
//! a text element inside the table element is written once per row, around its content.

use al_testkit::{run_in, text};

const SOAP: &str = "http://schemas.xmlsoap.org/soap/envelope/";
const TEM: &str = "http://tempuri.org/";
const MSG: &str = "urn:lt:xnt:msg";

/// `props` go into the xmlport header; `prep` runs before the export (after two items and three lines were inserted).
fn src(props: &str, schema: &str, prep: &str) -> String {
    format!(
        r#"
table 69990 "NT Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69991 "NT Item" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Name; Text[50]) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
table 69992 "NT Child" {{ fields {{ field(1; Id; Integer) {{ }} field(2; ItemId; Integer) {{ }} field(3; Txt; Text[30]) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
xmlport 69993 "NT Port"
{{
    Direction = Both; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    {props}
    schema {{ {schema} }}
}}
codeunit 69994 C
{{
    procedure Go(): Text
    var B: Record "NT Blob" temporary; R: Record "NT Item"; Ch: Record "NT Child"; OutS: OutStream; InS: InStream; Line: Text; T: Text;
    begin
        R.Id := 1; R.Name := 'one'; R.Insert();
        R.Id := 2; R.Name := '  two  '; R.Insert();
        Ch.Id := 1; Ch.ItemId := 1; Ch.Txt := 'c1a'; Ch.Insert();
        Ch.Id := 2; Ch.ItemId := 1; Ch.Txt := 'c1b'; Ch.Insert();
        Ch.Id := 3; Ch.ItemId := 2; Ch.Txt := 'c2a'; Ch.Insert();
        {prep}
        B.Data.CreateOutStream(OutS);
        Xmlport.Export(Xmlport::"NT Port", OutS);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin InS.ReadText(Line); T += Line + '|'; end;
        exit(T);
    end;
}}
"#
    )
}

fn export(props: &str, schema: &str, prep: &str) -> String {
    text(run_in(&src(props, schema, prep), "C", "Go"))
}

const DECL: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>|";
const EMPTY_NAME: &str = "R.Get(1); R.Name := ''; R.Modify();";
const NO_ITEMS: &str = "R.DeleteAll();";
const NO_LINES: &str = "Ch.DeleteAll();";
const LINES: &str =
    "tableelement(Ch; \"NT Child\") { XmlName = 'Line'; fieldelement(Id; Ch.Id) { } fieldelement(Txt; Ch.Txt) { } }";

fn root(once: bool, body: &str) -> String {
    let once = if once { "MaxOccurs = Once;" } else { "" };
    format!("tableelement(Rw; \"NT Item\") {{ {once} XmlName = 'Envelope'; {body} }}")
}

// XNT_TxtField_Export.
#[test]
fn a_text_element_with_field_elements_below_the_root_table_element() {
    let schema = root(
        true,
        "textelement(Body) { fieldelement(Id; Rw.Id) { } fieldelement(Name; Rw.Name) { } }",
    );
    assert_eq!(
        export("", &schema, ""),
        format!("{DECL}<Envelope>|  <Body>|    <Id>1</Id>|    <Name>one</Name>|  </Body>|</Envelope>|")
    );
    assert_eq!(
        export("", &schema, EMPTY_NAME),
        format!("{DECL}<Envelope>|  <Body>|    <Id>1</Id>|    <Name />|  </Body>|</Envelope>|")
    );
    assert_eq!(
        export("", &schema, NO_ITEMS),
        format!("{DECL}<Envelope>|  <Body>|    <Id>0</Id>|    <Name />|  </Body>|</Envelope>|")
    );
}

// XNT_TxtTbl_Export.
#[test]
fn a_text_element_with_a_table_element_below_the_root_table_element() {
    let schema = root(true, &format!("textelement(Body) {{ {LINES} }}"));
    assert_eq!(
        export("", &schema, ""),
        format!(
            "{DECL}<Envelope>|  <Body>|    <Line>|      <Id>1</Id>|      <Txt>c1a</Txt>|    </Line>|    <Line>|      <Id>2</Id>|      <Txt>c1b</Txt>|    </Line>|    <Line>|      <Id>3</Id>|      <Txt>c2a</Txt>|    </Line>|  </Body>|</Envelope>|"
        )
    );
    let blank = format!(
        "{DECL}<Envelope>|  <Body>|    <Line>|      <Id>0</Id>|      <Txt />|    </Line>|  </Body>|</Envelope>|"
    );
    assert_eq!(export("", &schema, NO_LINES), blank);
    assert_eq!(export("", &schema, &format!("{NO_LINES} {NO_ITEMS}")), blank);
}

// XNT_TxtTxtField_Export.
#[test]
fn two_text_levels_with_field_elements_below_the_root_table_element() {
    let schema = root(
        true,
        "textelement(Body) { textelement(Inner) { fieldelement(Id; Rw.Id) { } fieldelement(Name; Rw.Name) { } } }",
    );
    assert_eq!(
        export("", &schema, ""),
        format!(
            "{DECL}<Envelope>|  <Body>|    <Inner>|      <Id>1</Id>|      <Name>one</Name>|    </Inner>|  </Body>|</Envelope>|"
        )
    );
    assert_eq!(
        export("", &schema, EMPTY_NAME),
        format!(
            "{DECL}<Envelope>|  <Body>|    <Inner>|      <Id>1</Id>|      <Name />|    </Inner>|  </Body>|</Envelope>|"
        )
    );
}

// XNT_Rows_Export: without MaxOccurs, one root per row; with no row, once with the record as it stands.
#[test]
fn without_max_occurs_one_root_per_row() {
    let schema = root(
        false,
        "textelement(Body) { textelement(Inner) { fieldelement(Id; Rw.Id) { } fieldelement(Name; Rw.Name) { } } }",
    );
    let one = |id: &str, name: &str| {
        format!("<Envelope>|  <Body>|    <Inner>|      <Id>{id}</Id>|      {name}|    </Inner>|  </Body>|</Envelope>|")
    };
    assert_eq!(
        export("", &schema, ""),
        format!(
            "{DECL}{}{}",
            one("1", "<Name>one</Name>"),
            one("2", "<Name>  two  </Name>")
        )
    );
    assert_eq!(export("", &schema, NO_ITEMS), format!("{DECL}{}", one("0", "<Name />")));
}

// XNT_TxtTxtTbl_Export: the reporter's shape.
#[test]
fn the_reporters_shape_without_namespaces() {
    let schema = root(
        true,
        &format!(
            "textelement(Header) {{ }} textelement(Body) {{ textelement(Msg) {{ textelement(Refs) {{ {LINES} }} }} }}"
        ),
    );
    assert_eq!(
        export("", &schema, ""),
        format!(
            "{DECL}<Envelope>|  <Header />|  <Body>|    <Msg>|      <Refs>|        <Line>|          <Id>1</Id>|          <Txt>c1a</Txt>|        </Line>|        <Line>|          <Id>2</Id>|          <Txt>c1b</Txt>|        </Line>|        <Line>|          <Id>3</Id>|          <Txt>c2a</Txt>|        </Line>|      </Refs>|    </Msg>|  </Body>|</Envelope>|"
        )
    );
    let blank = format!(
        "{DECL}<Envelope>|  <Header />|  <Body>|    <Msg>|      <Refs>|        <Line>|          <Id>0</Id>|          <Txt />|        </Line>|      </Refs>|    </Msg>|  </Body>|</Envelope>|"
    );
    assert_eq!(export("", &schema, NO_LINES), blank);
    assert_eq!(export("", &schema, &format!("{NO_LINES} {NO_ITEMS}")), blank);
}

// XNT_FieldTxt_Export.
#[test]
fn a_field_element_of_the_root_before_a_nested_text_element() {
    let schema = root(
        true,
        "fieldelement(Id; Rw.Id) { } textelement(Body) { textelement(Inner) { fieldelement(Name; Rw.Name) { } } }",
    );
    assert_eq!(
        export("", &schema, ""),
        format!(
            "{DECL}<Envelope>|  <Id>1</Id>|  <Body>|    <Inner>|      <Name>one</Name>|    </Inner>|  </Body>|</Envelope>|"
        )
    );
}

const NS: &str = "Namespaces = soap = 'http://schemas.xmlsoap.org/soap/envelope/', tem = 'http://tempuri.org/', msg = 'urn:lt:xnt:msg'; UseDefaultNamespace = true;";

fn ns_head() -> String {
    format!("{DECL}<Envelope xmlns:soap=\"{SOAP}\" xmlns:tem=\"{TEM}\" xmlns:msg=\"{MSG}\" xmlns=\"{SOAP}\">|")
}

// XNT_NsTxtField_Export.
#[test]
fn with_namespaces_body_and_receive_message_hold_field_elements() {
    let schema = "tableelement(Rw; \"NT Item\") { MaxOccurs = Once; NamespacePrefix = 'soap'; XmlName = 'Envelope'; textelement(Body) { NamespacePrefix = 'soap'; textelement(ReceiveMessage) { NamespacePrefix = 'tem'; fieldelement(Id; Rw.Id) { NamespacePrefix = 'msg'; } fieldelement(Name; Rw.Name) { } } } }";
    assert_eq!(
        export(NS, schema, ""),
        format!(
            "{}  <soap:Body>|    <tem:ReceiveMessage>|      <msg:Id>1</msg:Id>|      <Name xmlns=\"urn:microsoft-dynamics-nav/xmlports/x69993\">one</Name>|    </tem:ReceiveMessage>|  </soap:Body>|</Envelope>|",
            ns_head()
        )
    );
    assert_eq!(
        export(NS, schema, EMPTY_NAME),
        format!(
            "{}  <soap:Body>|    <tem:ReceiveMessage>|      <msg:Id>1</msg:Id>|      <Name xmlns=\"urn:microsoft-dynamics-nav/xmlports/x69993\" />|    </tem:ReceiveMessage>|  </soap:Body>|</Envelope>|",
            ns_head()
        )
    );
}

// XNT_NsTxtTbl_Export: the reporter's shape with namespaces.
#[test]
fn with_namespaces_the_reporters_shape() {
    let schema = "tableelement(Rw; \"NT Item\") { MaxOccurs = Once; NamespacePrefix = 'soap'; XmlName = 'Envelope'; textelement(Header) { NamespacePrefix = 'soap'; } textelement(Body) { NamespacePrefix = 'soap'; textelement(ReceiveMessage) { NamespacePrefix = 'tem'; tableelement(Ch; \"NT Child\") { NamespacePrefix = 'tem'; XmlName = 'Line'; fieldelement(Id; Ch.Id) { NamespacePrefix = 'msg'; } fieldelement(Txt; Ch.Txt) { } } } } }";
    let line = |id: u32, txt: &str| {
        format!(
            "      <tem:Line>|        <msg:Id>{id}</msg:Id>|        <Txt xmlns=\"urn:microsoft-dynamics-nav/xmlports/x69993\">{txt}</Txt>|      </tem:Line>|"
        )
    };
    assert_eq!(
        export(NS, schema, ""),
        format!(
            "{}  <soap:Header />|  <soap:Body>|    <tem:ReceiveMessage>|{}{}{}    </tem:ReceiveMessage>|  </soap:Body>|</Envelope>|",
            ns_head(),
            line(1, "c1a"),
            line(2, "c1b"),
            line(3, "c2a")
        )
    );
    assert_eq!(
        export(NS, schema, NO_LINES),
        format!(
            "{}  <soap:Header />|  <soap:Body>|    <tem:ReceiveMessage>|      <tem:Line>|        <msg:Id>0</msg:Id>|        <Txt xmlns=\"urn:microsoft-dynamics-nav/xmlports/x69993\" />|      </tem:Line>|    </tem:ReceiveMessage>|  </soap:Body>|</Envelope>|",
            ns_head()
        )
    );
}

/// The import of `schema` from `doc`: the item rows after it, or the refusal.
fn import(schema: &str, doc: &str) -> Result<String, al_builtins::native::Signal> {
    let src = format!(
        r#"
table 69991 "NT Item" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Name; Text[50]) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
table 69992 "NT Child" {{ fields {{ field(1; Id; Integer) {{ }} field(2; ItemId; Integer) {{ }} field(3; Txt; Text[30]) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
table 69990 "NT Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
xmlport 69993 "NT Port" {{ Direction = Both; Format = Xml; Encoding = UTF8; UseRequestPage = false; schema {{ {schema} }} }}
codeunit 69994 C
{{
    procedure Go(): Text
    var B: Record "NT Blob" temporary; R: Record "NT Item"; Ch: Record "NT Child"; OutS: OutStream; InS: InStream; T: Text;
    begin
        R.Id := 1; R.Name := 'one'; R.Insert();
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('{doc}');
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        Xmlport.Import(Xmlport::"NT Port", InS);
        if R.FindSet() then repeat T += Format(R.Id) + '/' + R.Name + ';'; until R.Next() = 0;
        T += '|';
        if Ch.FindSet() then repeat T += Format(Ch.Id) + '/' + Ch.Txt + ';'; until Ch.Next() = 0;
        exit(T);
    end;
}}
"#
    );
    run_in(&src, "C", "Go").map(|v| text(Ok(v)))
}

// XNT_TxtField_Import, XNT_TxtTbl_Import, XNT_TxtTxtField_Import, XNT_TxtTxtTbl_Import: BC imports these documents (the root
// table element inserts its row, a root without fields of its own a blank row 0); the import is not built and stays refused.
#[test]
fn imports_of_the_plain_tree_stay_refused() {
    let r = import(
        &root(
            true,
            "textelement(Body) { fieldelement(Id; Rw.Id) { } fieldelement(Name; Rw.Name) { } }",
        ),
        "<Envelope><Body><Id>7</Id><Name>n7</Name></Body></Envelope>",
    );
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
    let r = import(
        &root(true, &format!("textelement(Body) {{ {LINES} }}")),
        "<Envelope><Body><Line><Id>7</Id><Txt>t7</Txt></Line><Line><Id>8</Id><Txt>t8</Txt></Line></Body></Envelope>",
    );
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
    let r = import(
        &root(
            true,
            "textelement(Body) { textelement(Inner) { fieldelement(Id; Rw.Id) { } fieldelement(Name; Rw.Name) { } } }",
        ),
        "<Envelope><Body><Inner><Id>7</Id><Name>n7</Name></Inner></Body></Envelope>",
    );
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
    let r = import(
        &root(
            true,
            &format!(
                "textelement(Header) {{ }} textelement(Body) {{ textelement(Msg) {{ textelement(Refs) {{ {LINES} }} }} }}"
            ),
        ),
        "<Envelope><Header /><Body><Msg><Refs><Line><Id>7</Id><Txt>t7</Txt></Line></Refs></Msg></Body></Envelope>",
    );
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}
