//! An Xml port whose root is a table element with text elements below it that hold content (al-runner issue 7130; corpus
//! xmlports/shapes-properties/XpTableRootNestedText.al from corpus PR 1158, BC 27.5, 28.5, 29.0). BC writes the plain nested tree:
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

/// The import of `schema` from `doc`: the item rows, then the child rows, after it.
fn imported(schema: &str, doc: &str) -> String {
    match import(schema, doc) {
        Ok(t) => t,
        Err(e) => panic!("{e:?}"),
    }
}

const ITEMS_AFTER: &str = "1/one;";

// XNT_TxtField_Import, XNT_TxtTbl_Import, XNT_TxtTxtField_Import, XNT_TxtTxtTbl_Import: BC imports these documents; the root table
// element inserts its row, a root without fields of its own a blank row 0, the inner table rows are inserted.
#[test]
fn imports_of_the_plain_tree_insert_the_rows() {
    let t = imported(
        &root(
            true,
            "textelement(Body) { fieldelement(Id; Rw.Id) { } fieldelement(Name; Rw.Name) { } }",
        ),
        "<Envelope><Body><Id>7</Id><Name>n7</Name></Body></Envelope>",
    );
    assert_eq!(t, "1/one;7/n7;|");
    let t = imported(
        &root(true, &format!("textelement(Body) {{ {LINES} }}")),
        "<Envelope><Body><Line><Id>7</Id><Txt>t7</Txt></Line><Line><Id>8</Id><Txt>t8</Txt></Line></Body></Envelope>",
    );
    assert_eq!(t, format!("0/;{ITEMS_AFTER}|7/t7;8/t8;"));
    let t = imported(
        &root(
            true,
            "textelement(Body) { textelement(Inner) { fieldelement(Id; Rw.Id) { } fieldelement(Name; Rw.Name) { } } }",
        ),
        "<Envelope><Body><Inner><Id>7</Id><Name>n7</Name></Inner></Body></Envelope>",
    );
    assert_eq!(t, "1/one;7/n7;|");
    let t = imported(
        &root(
            true,
            &format!(
                "textelement(Header) {{ }} textelement(Body) {{ textelement(Msg) {{ textelement(Refs) {{ {LINES} }} }} }}"
            ),
        ),
        "<Envelope><Header /><Body><Msg><Refs><Line><Id>7</Id><Txt>t7</Txt></Line></Refs></Msg></Body></Envelope>",
    );
    assert_eq!(t, format!("0/;{ITEMS_AFTER}|7/t7;"));
}

const USERNAME: &str = "textelement(username) { trigger OnBeforePassVariable() begin username := 'U1'; end; }";
const ORDERS: &str =
    "tableelement(Ch; \"NT Child\") { XmlName = 'order'; fieldelement(Id; Ch.Id) { } fieldelement(Txt; Ch.Txt) { } }";

fn order(id: u32, txt: &str, pad: &str) -> String {
    format!("{pad}<order>|{pad}  <Id>{id}</Id>|{pad}  <Txt>{txt}</Txt>|{pad}</order>|")
}

const ORDERS_TEXT: [(u32, &str); 3] = [(1, "c1a"), (2, "c1b"), (3, "c2a")];

fn orders(pad: &str) -> String {
    ORDERS_TEXT.iter().map(|(i, t)| order(*i, t, pad)).collect()
}

// XNT_PlainLeaf_Export, XNT_PlainLeaf_Import: the root table element holds a text element with its own value.
#[test]
fn a_text_element_with_a_value_below_the_root_table_element() {
    let schema = root(true, USERNAME);
    let once = format!("{DECL}<Envelope>|  <username>U1</username>|</Envelope>|");
    assert_eq!(export("", &schema, ""), once);
    assert_eq!(export("", &schema, NO_ITEMS), once);
    assert_eq!(
        imported(&schema, "<Envelope><username>U7</username></Envelope>"),
        format!("0/;{ITEMS_AFTER}|")
    );
    // A child element the port does not have fails the schema check; the rows stay.
    let r = import(&schema, "<Envelope><username>U7</username><Id>9</Id></Envelope>");
    let Err(al_builtins::native::Signal::Error(e)) = r else {
        panic!("{r:?}")
    };
    assert_eq!(
        e.message,
        "Imported XML cannot validate with the schema: The element 'Envelope' has invalid child element 'Id'. List of possible elements expected: 'username'."
    );
    assert_eq!(e.code, "XmlPortInvalidXml");
}

// XNT_LeafRows_Export: without MaxOccurs one root per row.
#[test]
fn a_text_element_with_a_value_below_a_root_table_element_with_two_rows() {
    let schema = root(false, USERNAME);
    let one = "<Envelope>|  <username>U1</username>|</Envelope>|";
    assert_eq!(export("", &schema, ""), format!("{DECL}{one}{one}"));
    assert_eq!(export("", &schema, NO_ITEMS), format!("{DECL}{one}"));
}

// XNT_TblText_Export, XNT_TblText_Import: Body holds a table element, then a text element with a value.
#[test]
fn a_table_element_then_a_text_element_with_a_value() {
    let schema = root(true, &format!("textelement(Body) {{ {ORDERS} {USERNAME} }}"));
    assert_eq!(
        export("", &schema, ""),
        format!(
            "{DECL}<Envelope>|  <Body>|{}    <username>U1</username>|  </Body>|</Envelope>|",
            orders("    ")
        )
    );
    assert_eq!(
        export("", &schema, NO_LINES),
        format!(
            "{DECL}<Envelope>|  <Body>|{}    <username>U1</username>|  </Body>|</Envelope>|",
            order(0, "", "    ").replace("<Txt></Txt>", "<Txt />")
        )
    );
    assert_eq!(
        imported(
            &schema,
            "<Envelope><Body><order><Id>7</Id><Txt>t7</Txt></order><username>U7</username></Body></Envelope>"
        ),
        format!("0/;{ITEMS_AFTER}|7/t7;")
    );
}

// XNT_TextTbl_Export, XNT_TextTbl_Import: the other order.
#[test]
fn a_text_element_with_a_value_then_a_table_element() {
    let schema = root(true, &format!("textelement(Body) {{ {USERNAME} {ORDERS} }}"));
    assert_eq!(
        export("", &schema, ""),
        format!(
            "{DECL}<Envelope>|  <Body>|    <username>U1</username>|{}  </Body>|</Envelope>|",
            orders("    ")
        )
    );
    assert_eq!(
        export("", &schema, NO_LINES),
        format!(
            "{DECL}<Envelope>|  <Body>|    <username>U1</username>|    <order>|      <Id>0</Id>|      <Txt />|    </order>|  </Body>|</Envelope>|"
        )
    );
    assert_eq!(
        imported(
            &schema,
            "<Envelope><Body><username>U7</username><order><Id>7</Id><Txt>t7</Txt></order></Body></Envelope>"
        ),
        format!("0/;{ITEMS_AFTER}|7/t7;")
    );
}

// XNT_NsTblText_Export, XNT_NsDfltTblText_Export: the reporter's N11 shape, with and without a DefaultNamespace.
#[test]
fn with_namespaces_a_table_element_then_a_text_element_with_a_value() {
    let schema = "tableelement(Rw; \"NT Item\") { MaxOccurs = Once; NamespacePrefix = 'soap'; XmlName = 'Envelope'; textelement(Body) { NamespacePrefix = 'soap'; tableelement(Ch; \"NT Child\") { NamespacePrefix = 'tem'; XmlName = 'order'; fieldelement(Id; Ch.Id) { } fieldelement(Txt; Ch.Txt) { } } textelement(username) { NamespacePrefix = 'tem'; trigger OnBeforePassVariable() begin username := 'U1'; end; } } }";
    let head = format!("{DECL}<Envelope xmlns:soap=\"{SOAP}\" xmlns:tem=\"{TEM}\" xmlns=\"{SOAP}\">|");
    let row = |ns: &str, id: u32, txt: &str| {
        let txt = if txt.is_empty() {
            format!("<Txt xmlns=\"{ns}\" />")
        } else {
            format!("<Txt xmlns=\"{ns}\">{txt}</Txt>")
        };
        format!("    <tem:order>|      <Id xmlns=\"{ns}\">{id}</Id>|      {txt}|    </tem:order>|")
    };
    for (props, ns) in [
        (
            "Namespaces = soap = 'http://schemas.xmlsoap.org/soap/envelope/', tem = 'http://tempuri.org/'; UseDefaultNamespace = true;",
            "urn:microsoft-dynamics-nav/xmlports/x69993",
        ),
        (
            "DefaultNamespace = 'urn:lt:xnt:input'; Namespaces = soap = 'http://schemas.xmlsoap.org/soap/envelope/', tem = 'http://tempuri.org/'; UseDefaultNamespace = true;",
            "urn:lt:xnt:input",
        ),
    ] {
        let tail = "    <tem:username>U1</tem:username>|  </soap:Body>|</Envelope>|";
        assert_eq!(
            export(props, schema, ""),
            format!(
                "{head}  <soap:Body>|{}{}{}{tail}",
                row(ns, 1, "c1a"),
                row(ns, 2, "c1b"),
                row(ns, 3, "c2a")
            )
        );
        assert_eq!(
            export(props, schema, NO_LINES),
            format!("{head}  <soap:Body>|{}{tail}", row(ns, 0, ""))
        );
    }
}
