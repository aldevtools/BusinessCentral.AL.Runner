//! A text element with a value next to a table element, two text levels below the root table element or in a text element inside
//! the row of a table element (al-runner issue 7263; corpus xmlports/shapes-properties/XpTextValueNestedTable.al from corpus PR 1197,
//! BC 27.5, 28.5, 29.0). BC writes the plain tree in schema order; the value element once.

use al_testkit::{run_in, text};

/// `props` go into the xmlport header; `orders` and `refs` rows are inserted before the export (one root row always).
fn src(props: &str, schema: &str, orders: usize, refs: usize) -> String {
    format!(
        r#"
table 69980 "TV Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69981 "TV Comm" {{ fields {{ field(1; Id; Code[10]) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
table 69982 "TV Order" {{ fields {{ field(1; "No."; Code[10]) {{ }} }} keys {{ key(PK; "No.") {{ }} }} }}
table 69983 "TV Ref" {{ fields {{ field(1; "Ref"; Code[10]) {{ }} }} keys {{ key(PK; "Ref") {{ }} }} }}
xmlport 69984 "TV Port"
{{
    Direction = Export; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    {props}
    schema {{ {schema} }}
}}
codeunit 69985 C
{{
    procedure Go(): Text
    var B: Record "TV Blob" temporary; Comm: Record "TV Comm"; O: Record "TV Order"; R: Record "TV Ref"; i: Integer;
        OutS: OutStream; InS: InStream; Line: Text; T: Text;
    begin
        Comm.Id := 'C1'; Comm.Insert();
        for i := 1 to {orders} do begin O."No." := 'O' + Format(i); O.Insert(); end;
        for i := 1 to {refs} do begin R."Ref" := 'R' + Format(i); R.Insert(); end;
        B.Data.CreateOutStream(OutS);
        Xmlport.Export(Xmlport::"TV Port", OutS);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin InS.ReadText(Line); T += Line + '|'; end;
        exit(T);
    end;
}}
"#
    )
}

pub(super) fn export(props: &str, schema: &str, orders: usize, refs: usize) -> String {
    text(run_in(&src(props, schema, orders, refs), "C", "Go"))
}

pub(super) const DECL: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>|";
pub(super) const ORDERS: &str =
    "tableelement(orders; \"TV Order\") { XmlName = 'order'; fieldelement(No; orders.\"No.\") { } }";
pub(super) const USERNAME: &str =
    "textelement(username) { trigger OnBeforePassVariable() begin username := 'x'; end; }";
pub(super) const REFS: &str = "tableelement(rf; \"TV Ref\") { XmlName = 'r'; fieldelement(Ref; rf.\"Ref\") { } }";
pub(super) const T1: &str = "textelement(T1) { trigger OnBeforePassVariable() begin T1 := 'x'; end; }";

/// A: root table element > L1 > L0 > `inner`.
fn a(prefix: bool, inner: &str) -> String {
    let (root, tem) = if prefix {
        ("NamespacePrefix = 'soap';", "NamespacePrefix = 'tem';")
    } else {
        ("", "")
    };
    format!(
        "tableelement(comm; \"TV Comm\") {{ MaxOccurs = Once; XmlName = 'Envelope'; {root} \
         textelement(L1) {{ {tem} textelement(L0) {{ {tem} {inner} }} }} }}"
    )
}

/// B: root text element > table element (MaxOccurs = Once) > TI > `inner`.
fn b(inner: &str) -> String {
    format!(
        "textelement(Envelope) {{ tableelement(orders; \"TV Order\") {{ MaxOccurs = Once; textelement(TI) {{ {inner} }} }} }}"
    )
}

// XpTextValueNestedTable.al XTV_RootTextsTableThenValue_Export (0, 1 and 2 rows).
#[test]
fn a_value_element_after_a_table_element_two_text_levels_below_the_root_table_element() {
    let schema = a(false, &format!("{ORDERS} {USERNAME}"));
    let doc = |rows: &str| {
        format!("{DECL}<Envelope>|  <L1>|    <L0>|{rows}      <username>x</username>|    </L0>|  </L1>|</Envelope>|")
    };
    assert_eq!(
        export("", &schema, 0, 0),
        doc("      <order>|        <No />|      </order>|")
    );
    assert_eq!(
        export("", &schema, 2, 0),
        doc("      <order>|        <No>O1</No>|      </order>|      <order>|        <No>O2</No>|      </order>|")
    );
}

// XpTextValueNestedTable.al XTV_RootTextsValueThenTable_Export (1 row).
#[test]
fn a_value_element_before_a_table_element_two_text_levels_below_the_root_table_element() {
    let schema = a(false, &format!("{USERNAME} {ORDERS}"));
    assert_eq!(
        export("", &schema, 1, 0),
        format!(
            "{DECL}<Envelope>|  <L1>|    <L0>|      <username>x</username>|      <order>|        <No>O1</No>|      </order>|    </L0>|  </L1>|</Envelope>|"
        )
    );
}

// XpTextValueNestedTable.al XTV_RootTextsNamespaces_Export (1 row).
#[test]
fn the_same_with_namespaces_and_use_default_namespace() {
    let props = "Namespaces = soap = 'http://schemas.xmlsoap.org/soap/envelope/', tem = 'http://tempuri.org/'; UseDefaultNamespace = true;";
    let schema = a(true, &format!("{ORDERS} {USERNAME}"));
    let port = "urn:microsoft-dynamics-nav/xmlports/x69984";
    assert_eq!(
        export(props, &schema, 1, 0),
        format!(
            "{DECL}<Envelope xmlns:soap=\"http://schemas.xmlsoap.org/soap/envelope/\" xmlns:tem=\"http://tempuri.org/\" xmlns=\"http://schemas.xmlsoap.org/soap/envelope/\">|  <tem:L1>|    <tem:L0>|      <order xmlns=\"{port}\">|        <No>O1</No>|      </order>|      <username xmlns=\"{port}\">x</username>|    </tem:L0>|  </tem:L1>|</Envelope>|"
        )
    );
}

// XpTextValueNestedTable.al XTV_RowTextValueThenTable_Export (0 and 2 rows).
#[test]
fn a_value_element_before_a_table_element_in_a_text_element_inside_a_row() {
    let schema = b(&format!("{T1} {REFS}"));
    let doc = |rows: &str| {
        format!("{DECL}<Envelope>|  <orders>|    <TI>|      <T1>x</T1>|{rows}    </TI>|  </orders>|</Envelope>|")
    };
    assert_eq!(export("", &schema, 1, 0), doc("      <r>|        <Ref />|      </r>|"));
    assert_eq!(
        export("", &schema, 1, 2),
        doc("      <r>|        <Ref>R1</Ref>|      </r>|      <r>|        <Ref>R2</Ref>|      </r>|")
    );
}

// XpTextValueNestedTable.al XTV_RowTextEmptyValueThenTable_Export (1 row).
#[test]
fn an_empty_value_element_before_a_table_element_in_a_text_element_inside_a_row() {
    let schema = b(&format!("textelement(T1) {{ }} {REFS}"));
    assert_eq!(
        export("", &schema, 1, 1),
        format!(
            "{DECL}<Envelope>|  <orders>|    <TI>|      <T1 />|      <r>|        <Ref>R1</Ref>|      </r>|    </TI>|  </orders>|</Envelope>|"
        )
    );
}

// XpTextValueNestedTable.al XTV_RowTextTableThenValue_Export (1 row).
#[test]
fn a_value_element_after_a_table_element_in_a_text_element_inside_a_row() {
    let schema = b(&format!("{REFS} {T1}"));
    assert_eq!(
        export("", &schema, 1, 1),
        format!(
            "{DECL}<Envelope>|  <orders>|    <TI>|      <r>|        <Ref>R1</Ref>|      </r>|      <T1>x</T1>|    </TI>|  </orders>|</Envelope>|"
        )
    );
}

// XpTextValueNestedTable.al XTV_RowTextFieldValueThenTable_Export (1 row).
#[test]
fn a_field_element_a_value_element_and_a_table_element_in_a_text_element_inside_a_row() {
    let schema = b(&format!("fieldelement(No; orders.\"No.\") {{ }} {T1} {REFS}"));
    assert_eq!(
        export("", &schema, 1, 1),
        format!(
            "{DECL}<Envelope>|  <orders>|    <TI>|      <No>O1</No>|      <T1>x</T1>|      <r>|        <Ref>R1</Ref>|      </r>|    </TI>|  </orders>|</Envelope>|"
        )
    );
}

/// The import of `schema` (in a port with Direction = Both) from `doc`, or its refusal.
pub(super) fn import(schema: &str, doc: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    let src = format!(
        r#"
table 69980 "TV Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69981 "TV Comm" {{ fields {{ field(1; Id; Code[10]) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
table 69982 "TV Order" {{ fields {{ field(1; "No."; Code[10]) {{ }} }} keys {{ key(PK; "No.") {{ }} }} }}
table 69983 "TV Ref" {{ fields {{ field(1; "Ref"; Code[10]) {{ }} }} keys {{ key(PK; "Ref") {{ }} }} }}
xmlport 69984 "TV Port" {{ Direction = Both; Format = Xml; Encoding = UTF8; UseRequestPage = false; schema {{ {schema} }} }}
codeunit 69985 C
{{
    procedure Go(): Text
    var B: Record "TV Blob" temporary; OutS: OutStream; InS: InStream;
    begin
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('{doc}');
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        Xmlport.Import(Xmlport::"TV Port", InS);
        exit('imported');
    end;
}}
"#
    );
    run_in(&src, "C", "Go")
}

// The import of these shapes is not pinned (XpTextValueNestedTable.al probes the export only): it is refused.
#[test]
fn the_import_of_a_value_element_two_text_levels_below_the_root_table_element_is_refused() {
    let r = import(
        &a(false, &format!("{ORDERS} {USERNAME}")),
        "<Envelope><L1><L0><order><No>O1</No></order><username>u</username></L0></L1></Envelope>",
    );
    assert!(
        matches!(&r, Err(al_builtins::native::Signal::Unsupported(m)) if m.contains("not pinned")),
        "{r:?}"
    );
    let r = import(
        &b(&format!("{T1} {REFS}")),
        "<Envelope><orders><TI><T1>u</T1><r><Ref>R1</Ref></r></TI></orders></Envelope>",
    );
    assert!(matches!(&r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}
