//! Text elements with a value next to a table element, three or more text levels below the root table element, several of them in
//! one text element, and deeper inside the row of a table element (al-runner issue 7309; corpus
//! xmlports/shapes-properties/XpTextValueNestedTable.al on branch al-runtime-rs-staging-7309, corpus PR 1218, BC 27.5, 28.5, 29.0).
//! BC writes the plain tree in schema order; each value element once.

use super::xmlport_text_value_nested_table::{DECL, ORDERS, REFS, T1, USERNAME, export};

const PASSWORD: &str = "textelement(password) { trigger OnBeforePassVariable() begin password := 'y'; end; }";
const T2: &str = "textelement(T2) { trigger OnBeforePassVariable() begin T2 := 'y'; end; }";
const T3: &str = "textelement(T3) { trigger OnBeforePassVariable() begin T3 := 'z'; end; }";

/// Text elements `levels` nested in each other around `inner`, each with `props`.
fn nest(levels: &[&str], props: &str, inner: &str) -> String {
    levels.iter().rev().fold(inner.to_string(), |acc, l| {
        format!("textelement({l}) {{ {props} {acc} }}")
    })
}

/// A: root table element > `levels` > `inner`.
fn a(levels: &[&str], prefix: bool, inner: &str) -> String {
    let (root, tem) = if prefix {
        ("NamespacePrefix = 'soap';", "NamespacePrefix = 'tem';")
    } else {
        ("", "")
    };
    format!(
        "tableelement(comm; \"TV Comm\") {{ MaxOccurs = Once; XmlName = 'Envelope'; {root} {} }}",
        nest(levels, tem, inner)
    )
}

/// B: root text element > table element (MaxOccurs = Once) > `levels` > `inner`.
fn b(levels: &[&str], inner: &str) -> String {
    format!(
        "textelement(Envelope) {{ tableelement(orders; \"TV Order\") {{ MaxOccurs = Once; {} }} }}",
        nest(levels, "", inner)
    )
}

/// The exported lines of `levels` (with `tag` in front of each name) around `inner`, starting at indent `depth`; `inner` holds
/// lines without indent, each ending with `|`, and each gets the indent of the innermost level plus one.
fn doc(levels: &[&str], tag: &str, depth: usize, inner: &[&str]) -> String {
    let pad = |d: usize| "  ".repeat(d);
    let mut s = String::new();
    for (i, l) in levels.iter().enumerate() {
        s += &format!("{}<{tag}{l}>|", pad(depth + i));
    }
    for line in inner {
        let trimmed = line.trim_start();
        let own = (line.len() - trimmed.len()) / 2;
        s += &format!("{}{trimmed}|", pad(depth + levels.len() + own));
    }
    for (i, l) in levels.iter().enumerate().rev() {
        s += &format!("{}</{tag}{l}>|", pad(depth + i));
    }
    s
}

fn order(no: &str) -> [String; 3] {
    ["<order>".into(), format!("  <No>{no}</No>"), "</order>".into()]
}

fn r(no: &str) -> [String; 3] {
    ["<r>".into(), format!("  <Ref>{no}</Ref>"), "</r>".into()]
}

fn lines(parts: &[&[String]]) -> Vec<String> {
    parts.iter().flat_map(|p| p.iter().cloned()).collect()
}

fn a_doc(levels: &[&str], inner: &[String]) -> String {
    let inner: Vec<&str> = inner.iter().map(String::as_str).collect();
    format!("{DECL}<Envelope>|{}</Envelope>|", doc(levels, "", 1, &inner))
}

fn b_doc(levels: &[&str], inner: &[String]) -> String {
    let inner: Vec<&str> = inner.iter().map(String::as_str).collect();
    format!(
        "{DECL}<Envelope>|  <orders>|{}  </orders>|</Envelope>|",
        doc(levels, "", 2, &inner)
    )
}

fn v(s: &str) -> [String; 1] {
    [s.to_string()]
}

const L3: &[&str] = &["L2", "L1", "L0"];

// XpTextValueNestedTable.al XTV_Root3TextsTableThenValue_Export (1 and 2 rows).
#[test]
fn a_value_element_after_a_table_element_three_text_levels_below_the_root_table_element() {
    let schema = a(L3, false, &format!("{ORDERS} {USERNAME}"));
    let user = v("<username>x</username>");
    assert_eq!(export("", &schema, 1, 0), a_doc(L3, &lines(&[&order("O1"), &user])));
    assert_eq!(
        export("", &schema, 2, 0),
        a_doc(L3, &lines(&[&order("O1"), &order("O2"), &user]))
    );
}

// XpTextValueNestedTable.al XTV_Root3TextsValueThenTable_Export (0 rows).
#[test]
fn a_value_element_before_a_table_element_three_text_levels_below_the_root_table_element() {
    let schema = a(L3, false, &format!("{USERNAME} {ORDERS}"));
    let empty = ["<order>".to_string(), "  <No />".into(), "</order>".into()];
    assert_eq!(
        export("", &schema, 0, 0),
        a_doc(L3, &lines(&[&v("<username>x</username>"), &empty]))
    );
}

// XpTextValueNestedTable.al XTV_Root3TextsNamespaces_Export (1 row).
#[test]
fn three_text_levels_with_namespaces_and_use_default_namespace() {
    let props = "Namespaces = soap = 'http://schemas.xmlsoap.org/soap/envelope/', tem = 'http://tempuri.org/'; UseDefaultNamespace = true;";
    let schema = a(L3, true, &format!("{ORDERS} {USERNAME}"));
    let port = "urn:microsoft-dynamics-nav/xmlports/x69984";
    let inner = [
        format!("<order xmlns=\"{port}\">"),
        "  <No>O1</No>".into(),
        "</order>".into(),
        format!("<username xmlns=\"{port}\">x</username>"),
    ];
    let inner: Vec<&str> = inner.iter().map(String::as_str).collect();
    assert_eq!(
        export(props, &schema, 1, 0),
        format!(
            "{DECL}<Envelope xmlns:soap=\"http://schemas.xmlsoap.org/soap/envelope/\" xmlns:tem=\"http://tempuri.org/\" xmlns=\"http://schemas.xmlsoap.org/soap/envelope/\">|{}</Envelope>|",
            doc(L3, "tem:", 1, &inner)
        )
    );
}

// XpTextValueNestedTable.al XTV_Root4TextsTableThenValue_Export (1 row).
#[test]
fn a_value_element_after_a_table_element_four_text_levels_below_the_root_table_element() {
    let levels = &["L3", "L2", "L1", "L0"];
    let schema = a(levels, false, &format!("{ORDERS} {USERNAME}"));
    assert_eq!(
        export("", &schema, 1, 0),
        a_doc(levels, &lines(&[&order("O1"), &v("<username>x</username>")]))
    );
}

// XpTextValueNestedTable.al XTV_Root3TextsTwoValues_Export (2 rows).
#[test]
fn two_value_elements_around_a_table_element_three_text_levels_below_the_root_table_element() {
    let schema = a(L3, false, &format!("{USERNAME} {ORDERS} {PASSWORD}"));
    assert_eq!(
        export("", &schema, 2, 0),
        a_doc(
            L3,
            &lines(&[
                &v("<username>x</username>"),
                &order("O1"),
                &order("O2"),
                &v("<password>y</password>")
            ])
        )
    );
}

// XpTextValueNestedTable.al XTV_Root2TextsTwoValues_Export (1 row).
#[test]
fn two_value_elements_before_a_table_element_two_text_levels_below_the_root_table_element() {
    let levels = &["L1", "L0"];
    let schema = a(levels, false, &format!("{USERNAME} {PASSWORD} {ORDERS}"));
    assert_eq!(
        export("", &schema, 1, 0),
        a_doc(
            levels,
            &lines(&[&v("<username>x</username>"), &v("<password>y</password>"), &order("O1")])
        )
    );
}

// XpTextValueNestedTable.al XTV_RowTextTwoValuesThenTable_Export (0 and 1 rows).
#[test]
fn two_value_elements_before_a_table_element_in_a_text_element_inside_a_row() {
    let schema = b(&["TI"], &format!("{T1} {T2} {REFS}"));
    let empty = ["<r>".to_string(), "  <Ref />".into(), "</r>".into()];
    let vals = lines(&[&v("<T1>x</T1>"), &v("<T2>y</T2>")]);
    assert_eq!(export("", &schema, 1, 0), b_doc(&["TI"], &lines(&[&vals, &empty])));
    assert_eq!(export("", &schema, 1, 1), b_doc(&["TI"], &lines(&[&vals, &r("R1")])));
}

// XpTextValueNestedTable.al XTV_RowTextValueTableValue_Export (2 rows).
#[test]
fn value_elements_on_both_sides_of_a_table_element_in_a_text_element_inside_a_row() {
    let schema = b(&["TI"], &format!("{T1} {REFS} {T2}"));
    assert_eq!(
        export("", &schema, 1, 2),
        b_doc(
            &["TI"],
            &lines(&[&v("<T1>x</T1>"), &r("R1"), &r("R2"), &v("<T2>y</T2>")])
        )
    );
}

// XpTextValueNestedTable.al XTV_RowTextTableThenTwoValues_Export (1 row).
#[test]
fn two_value_elements_after_a_table_element_in_a_text_element_inside_a_row() {
    let schema = b(&["TI"], &format!("{REFS} {T1} {T2}"));
    assert_eq!(
        export("", &schema, 1, 1),
        b_doc(&["TI"], &lines(&[&r("R1"), &v("<T1>x</T1>"), &v("<T2>y</T2>")]))
    );
}

// XpTextValueNestedTable.al XTV_RowTextThreeValuesThenTable_Export (1 row).
#[test]
fn three_value_elements_before_a_table_element_in_a_text_element_inside_a_row() {
    let schema = b(&["TI"], &format!("{T1} {T2} {T3} {REFS}"));
    assert_eq!(
        export("", &schema, 1, 1),
        b_doc(
            &["TI"],
            &lines(&[&v("<T1>x</T1>"), &v("<T2>y</T2>"), &v("<T3>z</T3>"), &r("R1")])
        )
    );
}

// XpTextValueNestedTable.al XTV_RowTextValueEmptyThenTable_Export (1 row).
#[test]
fn a_value_element_and_an_empty_one_before_a_table_element_in_a_text_element_inside_a_row() {
    let schema = b(&["TI"], &format!("{T1} textelement(T2) {{ }} {REFS}"));
    assert_eq!(
        export("", &schema, 1, 1),
        b_doc(&["TI"], &lines(&[&v("<T1>x</T1>"), &v("<T2 />"), &r("R1")]))
    );
}

// XpTextValueNestedTable.al XTV_RowDeepTwoValuesThenTable_Export (2 rows).
#[test]
fn two_value_elements_two_text_levels_inside_a_row() {
    let levels = &["TI", "TJ"];
    let schema = b(levels, &format!("{T1} {T2} {REFS}"));
    assert_eq!(
        export("", &schema, 1, 2),
        b_doc(
            levels,
            &lines(&[&v("<T1>x</T1>"), &v("<T2>y</T2>"), &r("R1"), &r("R2")])
        )
    );
}

// XpTextValueNestedTable.al XTV_RowDeep3OneValueThenTable_Export (1 row).
#[test]
fn a_value_element_three_text_levels_inside_a_row() {
    let levels = &["TI", "TJ", "TK"];
    let schema = b(levels, &format!("{T1} {REFS}"));
    assert_eq!(
        export("", &schema, 1, 1),
        b_doc(levels, &lines(&[&v("<T1>x</T1>"), &r("R1")]))
    );
}

// XpTextValueNestedTable.al XTV_RowFieldTwoValuesThenTable_Export (1 row).
#[test]
fn a_field_element_and_two_value_elements_before_a_table_element_inside_a_row() {
    let schema = b(
        &["TI"],
        &format!("fieldelement(No; orders.\"No.\") {{ }} {T1} {T2} {REFS}"),
    );
    assert_eq!(
        export("", &schema, 1, 1),
        b_doc(
            &["TI"],
            &lines(&[&v("<No>O1</No>"), &v("<T1>x</T1>"), &v("<T2>y</T2>"), &r("R1")])
        )
    );
}

// The import of these shapes is not pinned (XpTextValueNestedTable.al probes the export only): it is refused, also with two value
// elements right below the root table element, where the import of one is pinned (XpTableRootNestedText.al XNT_TblText_Import).
#[test]
fn the_import_of_two_value_elements_or_deeper_ones_is_refused() {
    use super::xmlport_text_value_nested_table::import;
    let cases = [
        (
            a(&["L0"], false, &format!("{ORDERS} {USERNAME} {PASSWORD}")),
            "<Envelope><L0><order><No>O1</No></order><username>u</username><password>p</password></L0></Envelope>",
        ),
        (
            a(L3, false, &format!("{ORDERS} {USERNAME}")),
            "<Envelope><L2><L1><L0><order><No>O1</No></order><username>u</username></L0></L1></L2></Envelope>",
        ),
        (
            b(&["TI"], &format!("{T1} {T2} {REFS}")),
            "<Envelope><orders><TI><T1>u</T1><T2>v</T2><r><Ref>R1</Ref></r></TI></orders></Envelope>",
        ),
    ];
    for (schema, doc) in cases {
        let r = import(&schema, doc);
        assert!(
            matches!(&r, Err(al_builtins::native::Signal::Unsupported(_))),
            "{schema}: {r:?}"
        );
    }
}
