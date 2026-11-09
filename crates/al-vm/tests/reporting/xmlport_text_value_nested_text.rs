//! A text element with content (a value element, or a field element and a value element) nested in a text element that also holds a
//! table element, inside the row of a table element (al-runner issue 7349; corpus xmlports/shapes-properties/XpTextValueNestedTable.al
//! XTN_*, corpus PR 1226, BC 27.5, 28.5, 29.0). BC writes the plain tree in schema order; the text element with content once and whole
//! where the schema places it; the inner table element one `r` per row, or one `r` with the record as it stands when it has none.

use super::xmlport_text_value_nested_table::{DECL, REFS, T1, export};

const TT: &str = "textelement(TT) { trigger OnBeforePassVariable() begin TT := 'x'; end; }";
const TU: &str = "textelement(TU) { trigger OnBeforePassVariable() begin TU := 'y'; end; }";
const T2: &str = "textelement(T2) { trigger OnBeforePassVariable() begin T2 := 'b'; end; }";
const QS: &str = "tableelement(qs; \"TV Ref\") { XmlName = 'q'; fieldelement(Ref; qs.\"Ref\") { } }";

fn field(name: &str) -> String {
    format!("fieldelement({name}; orders.\"No.\") {{ }}")
}

fn text(name: &str, inner: &str) -> String {
    format!("textelement({name}) {{ {inner} }}")
}

/// Root text element > table element (MaxOccurs = Once) > text element TI > `inner`.
fn schema(inner: &str) -> String {
    format!(
        "textelement(Envelope) {{ tableelement(orders; \"TV Order\") {{ MaxOccurs = Once; {} }} }}",
        text("TI", inner)
    )
}

/// The exported document with `body` inside TI; each body line is indented by two spaces per level below TI.
fn doc(body: &[String]) -> String {
    let mut s = format!("{DECL}<Envelope>|  <orders>|    <TI>|");
    for l in body {
        s += &format!("      {l}|");
    }
    s + "    </TI>|  </orders>|</Envelope>|"
}

/// `n` rows of the inner table element `tag` (one with the record as it stands when `n` is 0), indented by `pad`.
fn rows(tag: &str, n: usize, pad: &str) -> Vec<String> {
    let refs: Vec<String> = if n == 0 {
        vec![format!("{pad}  <Ref />")]
    } else {
        (1..=n).map(|i| format!("{pad}  <Ref>R{i}</Ref>")).collect()
    };
    refs.into_iter()
        .flat_map(|r| [format!("{pad}<{tag}>"), r, format!("{pad}</{tag}>")])
        .collect()
}

fn ls(parts: &[&[&str]]) -> Vec<String> {
    parts.iter().flat_map(|p| p.iter().map(|s| s.to_string())).collect()
}

const RD_X: &[&str] = &["<RD>", "  <TT>x</TT>", "</RD>"];
const RE_Y: &[&str] = &["<RE>", "  <TU>y</TU>", "</RE>"];
const RD_DX: &[&str] = &["<RD>", "  <D>O1</D>", "  <TT>x</TT>", "</RD>"];

/// Runs `inner` with 0, 1 and 2 inner rows and compares with `expect(rows)`.
fn check(inner: &str, expect: impl Fn(usize) -> Vec<String>) {
    let s = schema(inner);
    for n in 0..=2 {
        assert_eq!(export("", &s, 1, n), doc(&expect(n)), "{n} inner rows");
    }
}

fn join(parts: Vec<Vec<String>>) -> Vec<String> {
    parts.concat()
}

// XpTextValueNestedTable.al XTN_R1_Export (issue 7349 R1).
#[test]
fn a_text_element_with_a_value_then_a_table_element_in_a_text_element_inside_a_row() {
    check(&format!("{} {REFS}", text("RD", TT)), |n| {
        join(vec![ls(&[RD_X]), rows("r", n, "")])
    });
}

// XpTextValueNestedTable.al XTN_R2_Export (issue 7349 R2).
#[test]
fn a_table_element_then_a_text_element_with_a_value_in_a_text_element_inside_a_row() {
    check(&format!("{REFS} {}", text("RD", TT)), |n| {
        join(vec![rows("r", n, ""), ls(&[RD_X])])
    });
}

// XpTextValueNestedTable.al XTN_R3_Export (issue 7349 R3).
#[test]
fn a_text_element_with_a_field_and_a_value_then_a_table_element_inside_a_row() {
    let rd = text("RD", &format!("{} {TT}", field("D")));
    check(&format!("{rd} {REFS}"), |n| join(vec![ls(&[RD_DX]), rows("r", n, "")]));
}

// XpTextValueNestedTable.al XTN_Around_Export.
#[test]
fn text_elements_with_a_value_on_both_sides_of_a_table_element_inside_a_row() {
    check(&format!("{} {REFS} {}", text("RD", TT), text("RE", TU)), |n| {
        join(vec![ls(&[RD_X]), rows("r", n, ""), ls(&[RE_Y])])
    });
}

// XpTextValueNestedTable.al XTN_TwoTexts_Export.
#[test]
fn two_text_elements_with_a_value_then_a_table_element_inside_a_row() {
    check(&format!("{} {} {REFS}", text("RD", TT), text("RE", TU)), |n| {
        join(vec![ls(&[RD_X, RE_Y]), rows("r", n, "")])
    });
}

// XpTextValueNestedTable.al XTN_TwoTables_Export.
#[test]
fn a_text_element_with_a_field_and_a_value_then_two_table_elements_inside_a_row() {
    let rd = text("RD", &format!("{} {TT}", field("D")));
    check(&format!("{rd} {REFS} {QS}"), |n| {
        join(vec![ls(&[RD_DX]), rows("r", n, ""), rows("q", n, "")])
    });
}

// XpTextValueNestedTable.al XTN_Mixed_Export.
#[test]
fn value_elements_around_a_text_element_with_a_value_and_a_table_element_inside_a_row() {
    let t1 = T1.replace("'x'", "'a'");
    check(&format!("{t1} {} {REFS} {T2}", text("RD", TT)), |n| {
        join(vec![
            ls(&[&["<T1>a</T1>"], RD_X]),
            rows("r", n, ""),
            ls(&[&["<T2>b</T2>"]]),
        ])
    });
}

// XpTextValueNestedTable.al XTN_DeepTable_Export.
#[test]
fn a_text_element_with_a_value_and_a_table_element_one_text_level_deeper_inside_a_row() {
    check(&text("TJ", &format!("{} {REFS}", text("RD", TT))), |n| {
        let rd: Vec<String> = RD_X.iter().map(|l| format!("  {l}")).collect();
        join(vec![vec!["<TJ>".into()], rd, rows("r", n, "  "), vec!["</TJ>".into()]])
    });
}

// XpTextValueNestedTable.al XTN_DeepValue_Export.
#[test]
fn a_value_element_two_text_levels_below_a_text_element_with_a_table_element_inside_a_row() {
    check(&format!("{} {REFS}", text("RD", &text("RC", TT))), |n| {
        let rd = &["<RD>", "  <RC>", "    <TT>x</TT>", "  </RC>", "</RD>"];
        join(vec![ls(&[rd]), rows("r", n, "")])
    });
}

// XpTextValueNestedTable.al XTN_FieldValueAround_Export.
#[test]
fn field_and_value_elements_in_and_around_text_elements_next_to_a_table_element_inside_a_row() {
    let t1 = T1.replace("'x'", "'a'");
    let rd = text("RD", &format!("{} {TT}", field("D2")));
    let re = text("RE", &format!("{} {TU}", field("D3")));
    check(&format!("{} {t1} {rd} {REFS} {re}", field("D1")), |n| {
        let head: &[&str] = &[
            "<D1>O1</D1>",
            "<T1>a</T1>",
            "<RD>",
            "  <D2>O1</D2>",
            "  <TT>x</TT>",
            "</RD>",
        ];
        let tail: &[&str] = &["<RE>", "  <D3>O1</D3>", "  <TU>y</TU>", "</RE>"];
        join(vec![ls(&[head]), rows("r", n, ""), ls(&[tail])])
    });
}

// The import of these shapes is not pinned (XpTextValueNestedTable.al XTN_* probe the export only): it is refused.
#[test]
fn the_import_of_a_text_element_with_a_value_nested_next_to_a_table_element_is_refused() {
    use super::xmlport_text_value_nested_table::import;
    let s = schema(&format!("{} {REFS}", text("RD", TT)));
    let r = import(
        &s,
        "<Envelope><orders><TI><RD><TT>u</TT></RD><r><Ref>R1</Ref></r></TI></orders></Envelope>",
    );
    assert!(matches!(&r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}

// Below the root table element the same shape is not pinned (XTN_* probe the row of a table element below a root text element): it
// stays refused.
#[test]
fn the_same_shape_below_the_root_table_element_is_refused() {
    let s = format!(
        "tableelement(comm; \"TV Comm\") {{ MaxOccurs = Once; XmlName = 'Envelope'; {} }}",
        text("TI", &format!("{} {REFS}", text("RD", TT)))
    );
    let err = std::panic::catch_unwind(|| export("", &s, 1, 1)).expect_err("refused");
    let msg = err.downcast_ref::<String>().cloned().unwrap_or_default();
    assert!(msg.contains("Err(Unsupported("), "{msg}");
}
