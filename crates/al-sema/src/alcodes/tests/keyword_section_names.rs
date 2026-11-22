//! al-runner issues 8630 and 8634: a section named, unquoted, by one of the words the AL compiler rejects as a parameter name
//! does not compile. AL0107 at the name is the first diagnostic the AL compilers of the BC artifacts of every supported major and
//! a BC container gave: a table or table extension field (assumption `keyword-field-names`), and the 44 other sections below
//! (assumption `keyword-section-names`).

use super::*;

const WORDS: [&str; 24] = [
    "Actions",
    "Area",
    "Begin",
    "ChartPart",
    "CueGroup",
    "End",
    "Event",
    "Extends",
    "False",
    "FieldAttribute",
    "FieldElement",
    "FieldGroups",
    "Key",
    "Of",
    "PageExtension",
    "Procedure",
    "ProfileExtension",
    "TableElement",
    "TableExtension",
    "TextAttribute",
    "TextElement",
    "Trigger",
    "True",
    "Var",
];

/// The AL0107 diagnostics of `src`, each as the offset of the word it names (`None` for another offset).
fn al0107(src: &str, word: &str) -> Vec<Option<usize>> {
    let ws = load(&[src]);
    let at = src.rfind(&format!("; {word};")).map(|i| i + 2);
    check_workspace(&ws)
        .into_iter()
        .filter(|d| d.code == "AL0107")
        .map(|d| {
            assert_eq!(d.message, IDENTIFIER_EXPECTED);
            (Some(d.offset) == at).then_some(d.offset)
        })
        .collect()
}

fn table(field: &str) -> String {
    format!(
        "table 50100 \"R\"\n{{\n    fields\n    {{\n        field(1; \"No.\"; Code[20])\n        {{\n        }}\n        field(2; {field}; Integer)\n        {{\n        }}\n    }}\n    keys\n    {{\n        key(PK; \"No.\")\n        {{\n            Clustered = true;\n        }}\n    }}\n}}\n"
    )
}

fn table_extension(field: &str) -> String {
    format!(
        "tableextension 50100 \"R\" extends \"Base\"\n{{\n    fields\n    {{\n        field(50100; {field}; Integer)\n        {{\n        }}\n    }}\n}}\ntable 50101 \"Base\"\n{{\n    fields\n    {{\n        field(1; \"No.\"; Code[20])\n        {{\n        }}\n    }}\n}}\n"
    )
}

#[test]
fn a_keyword_as_an_unquoted_field_name_is_al0107_at_the_name() {
    for w in WORDS {
        let got = al0107(&table(w), w);
        assert_eq!(got.len(), 1, "table {w}");
        assert!(got[0].is_some(), "table {w}: not at the name");
        let got = al0107(&table_extension(w), w);
        assert_eq!(got.len(), 1, "table extension {w}");
        assert!(got[0].is_some(), "table extension {w}: not at the name");
    }
    // Any case.
    assert_eq!(al0107(&table("AREA"), "AREA").len(), 1);
    assert_eq!(al0107(&table("key"), "key").len(), 1);
}

#[test]
fn quoted_names_and_other_words_compile_as_field_names() {
    for w in [
        "\"Area\"",
        "\"Key\"",
        "\"Begin\"",
        "Local",
        "If",
        "Exit",
        "Keys",
        "Fields",
        "Layout",
        "Value",
    ] {
        assert!(al0107(&table(w), w).is_empty(), "table {w}");
        assert!(al0107(&table_extension(w), w).is_empty(), "table extension {w}");
    }
}

const T: &str = "table 50100 \"R\" { fields { field(1; \"No.\"; Code[20]) { } field(2; Amount; Integer) { } } keys { key(PK; \"No.\") { Clustered = true; } } }\n";
const CARD: &str = "page 50110 \"C\" { PageType = Card; SourceTable = \"R\"; layout { area(Content) { field(\"No.\"; Rec.\"No.\") { } } area(FactBoxes) { } } actions { area(Processing) { action(Act) { trigger OnAction() begin end; } } area(Promoted) { } } }\n";
const PART: &str = "page 50111 \"P\" { PageType = ListPart; SourceTable = \"R\"; layout { area(Content) { repeater(Lines) { field(\"No.\"; Rec.\"No.\") { } } } } }\n";
const REPORT: &str = "report 50112 \"Rp\" { ProcessingOnly = true; dataset { dataitem(Row; \"R\") { column(No; \"No.\") { } } } requestpage { layout { area(Content) { } } } }\n";

/// One shape per section the AL compilers reject such a word in (`W` is the name), with the objects it needs.
const SHAPES: [(&str, &str); 44] = [
    (
        "page field",
        "page 50101 \"X\" { PageType = Card; SourceTable = \"R\"; layout { area(Content) { field(W; Rec.Amount) { } } } }",
    ),
    (
        "page action",
        "page 50101 \"X\" { PageType = Card; actions { area(Processing) { action(W) { trigger OnAction() begin end; } } } }",
    ),
    (
        "page group",
        "page 50101 \"X\" { PageType = Card; SourceTable = \"R\"; layout { area(Content) { group(W) { field(\"No.\"; Rec.\"No.\") { } } } } }",
    ),
    (
        "page action group",
        "page 50101 \"X\" { PageType = Card; actions { area(Processing) { group(W) { action(Act) { trigger OnAction() begin end; } } } } }",
    ),
    (
        "page repeater",
        "page 50101 \"X\" { PageType = List; SourceTable = \"R\"; layout { area(Content) { repeater(W) { field(\"No.\"; Rec.\"No.\") { } } } } }",
    ),
    (
        "page part",
        "page 50101 \"X\" { PageType = Card; SourceTable = \"R\"; layout { area(Content) { part(W; \"P\") { } } } }",
    ),
    (
        "page systempart",
        "page 50101 \"X\" { PageType = Card; layout { area(FactBoxes) { systempart(W; Notes) { } } } }",
    ),
    (
        "page cuegroup",
        "page 50101 \"X\" { PageType = CardPart; SourceTable = \"R\"; layout { area(Content) { cuegroup(W) { field(\"No.\"; Rec.\"No.\") { } } } } }",
    ),
    (
        "page grid",
        "page 50101 \"X\" { PageType = Card; SourceTable = \"R\"; layout { area(Content) { grid(W) { field(\"No.\"; Rec.\"No.\") { } } } } }",
    ),
    (
        "page fixed",
        "page 50101 \"X\" { PageType = Card; SourceTable = \"R\"; layout { area(Content) { fixed(W) { group(Inner) { field(\"No.\"; Rec.\"No.\") { } } } } } }",
    ),
    (
        "page label",
        "page 50101 \"X\" { PageType = Card; layout { area(Content) { label(W) { Caption = 'L'; } } } }",
    ),
    (
        "page actionref",
        "page 50101 \"X\" { PageType = Card; actions { area(Processing) { action(Act) { trigger OnAction() begin end; } } area(Promoted) { actionref(W; Act) { } } } }",
    ),
    (
        "page separator",
        "page 50101 \"X\" { PageType = Card; actions { area(Processing) { separator(W) { } action(Act) { trigger OnAction() begin end; } } } }",
    ),
    (
        "page view",
        "page 50101 \"X\" { PageType = List; SourceTable = \"R\"; layout { area(Content) { repeater(Lines) { field(\"No.\"; Rec.\"No.\") { } } } } views { view(W) { Caption = 'V'; } } }",
    ),
    (
        "pageextension field",
        "pageextension 50101 \"X\" extends \"C\" { layout { addlast(Content) { field(W; Rec.Amount) { } } } }",
    ),
    (
        "pageextension group",
        "pageextension 50101 \"X\" extends \"C\" { layout { addlast(Content) { group(W) { } } } }",
    ),
    (
        "pageextension action",
        "pageextension 50101 \"X\" extends \"C\" { actions { addlast(Processing) { action(W) { trigger OnAction() begin end; } } } }",
    ),
    (
        "pageextension action group",
        "pageextension 50101 \"X\" extends \"C\" { actions { addlast(Processing) { group(W) { } } } }",
    ),
    (
        "pageextension part",
        "pageextension 50101 \"X\" extends \"C\" { layout { addlast(FactBoxes) { part(W; \"P\") { } } } }",
    ),
    (
        "pageextension systempart",
        "pageextension 50101 \"X\" extends \"C\" { layout { addlast(FactBoxes) { systempart(W; Notes) { } } } }",
    ),
    (
        "pageextension grid",
        "pageextension 50101 \"X\" extends \"C\" { layout { addlast(Content) { grid(W) { } } } }",
    ),
    (
        "pageextension fixed",
        "pageextension 50101 \"X\" extends \"C\" { layout { addlast(Content) { fixed(W) { group(Inner) { } } } } }",
    ),
    (
        "pageextension actionref",
        "pageextension 50101 \"X\" extends \"C\" { actions { addlast(Promoted) { actionref(W; Act) { } } } }",
    ),
    (
        "pageextension separator",
        "pageextension 50101 \"X\" extends \"C\" { actions { addlast(Processing) { separator(W) { } } } }",
    ),
    ("enum value", "enum 50101 \"X\" { value(0; W) { } }"),
    (
        "enumextension value",
        "enum 50102 \"E\" { Extensible = true; value(0; Zero) { } }\nenumextension 50101 \"X\" extends \"E\" { value(50100; W) { } }",
    ),
    (
        "report column",
        "report 50101 \"X\" { ProcessingOnly = true; dataset { dataitem(Row; \"R\") { column(W; Amount) { } } } }",
    ),
    (
        "report dataitem",
        "report 50101 \"X\" { ProcessingOnly = true; dataset { dataitem(W; \"R\") { } } }",
    ),
    (
        "report request page field",
        "report 50101 \"X\" { ProcessingOnly = true; dataset { dataitem(Row; \"R\") { } } requestpage { layout { area(Content) { field(W; Opt) { } } } } var Opt: Integer; }",
    ),
    (
        "reportextension column",
        "reportextension 50101 \"X\" extends \"Rp\" { dataset { add(Row) { column(W; Amount) { } } } }",
    ),
    (
        "reportextension dataitem",
        "reportextension 50101 \"X\" extends \"Rp\" { dataset { addlast(Row) { dataitem(W; \"R\") { } } } }",
    ),
    (
        "reportextension request page field",
        "reportextension 50101 \"X\" extends \"Rp\" { requestpage { layout { addlast(Content) { field(W; Opt) { } } } } var Opt: Integer; }",
    ),
    (
        "query column",
        "query 50101 \"X\" { elements { dataitem(Row; \"R\") { column(W; Amount) { } } } }",
    ),
    (
        "query dataitem",
        "query 50101 \"X\" { elements { dataitem(W; \"R\") { column(No; \"No.\") { } } } }",
    ),
    (
        "query filter",
        "query 50101 \"X\" { elements { dataitem(Row; \"R\") { column(No; \"No.\") { } filter(W; Amount) { } } } }",
    ),
    (
        "xmlport fieldelement",
        "xmlport 50101 \"X\" { schema { textelement(Root) { tableelement(Row; \"R\") { fieldelement(W; Row.Amount) { } } } } }",
    ),
    (
        "xmlport tableelement",
        "xmlport 50101 \"X\" { schema { textelement(Root) { tableelement(W; \"R\") { } } } }",
    ),
    (
        "xmlport textelement",
        "xmlport 50101 \"X\" { schema { textelement(Root) { textelement(W) { } } } }",
    ),
    (
        "xmlport fieldattribute",
        "xmlport 50101 \"X\" { schema { textelement(Root) { tableelement(Row; \"R\") { fieldattribute(W; Row.Amount) { } } } } }",
    ),
    (
        "xmlport textattribute",
        "xmlport 50101 \"X\" { schema { textelement(Root) { textattribute(W) { } } } }",
    ),
    (
        "xmlport request page field",
        "xmlport 50101 \"X\" { schema { textelement(Root) { } } requestpage { layout { area(Content) { field(W; Opt) { } } } } var Opt: Integer; }",
    ),
    (
        "table key",
        "table 50101 \"X\" { fields { field(1; \"No.\"; Code[20]) { } } keys { key(W; \"No.\") { Clustered = true; } } }",
    ),
    (
        "table fieldgroup",
        "table 50101 \"X\" { fields { field(1; \"No.\"; Code[20]) { } } keys { key(PK; \"No.\") { Clustered = true; } } fieldgroups { fieldgroup(W; \"No.\") { } } }",
    ),
    (
        "tableextension key",
        "tableextension 50101 \"X\" extends \"R\" { fields { field(50100; Extra; Integer) { } } keys { key(W; Extra) { } } }",
    ),
];

/// The AL0107 diagnostics of shape `shape` with `name` in place of `W`, each as `true` when it is at the name. The shape comes
/// first in the file: the synthetic routines the workspace adds to the objects after it do not move it.
fn section_al0107(shape: &str, name: &str) -> Vec<bool> {
    let at = shape.find('W').expect("W");
    let src = format!("{}{name}{}\n{T}{CARD}{PART}{REPORT}", &shape[..at], &shape[at + 1..]);
    let ws = load(&[&src]);
    check_workspace(&ws)
        .into_iter()
        .filter(|d| d.code == "AL0107")
        .map(|d| {
            assert_eq!(d.message, IDENTIFIER_EXPECTED);
            d.offset == at
        })
        .collect()
}

#[test]
fn a_keyword_as_an_unquoted_section_name_is_al0107_at_the_name() {
    for (what, shape) in SHAPES {
        assert!(shape.matches('W').count() == 1, "{what}: one W");
        for w in WORDS {
            assert_eq!(section_al0107(shape, w), [true], "{what} named {w}");
        }
        assert_eq!(section_al0107(shape, "kEY"), [true], "{what} named kEY");
    }
}

#[test]
fn quoted_names_and_other_words_compile_as_section_names() {
    for (what, shape) in SHAPES {
        for name in ["Foo", "\"Key\"", "\"Area\"", "If", "Exit", "Local", "Keys", "Value"] {
            assert!(section_al0107(shape, name).is_empty(), "{what} named {name}");
        }
    }
}

/// Sections no probe measured keep compiling: a word in a place that names no section of its own (an anchor or `area`), and
/// sections of object kinds no cell holds.
#[test]
fn unmeasured_sections_are_not_judged() {
    for shape in [
        "pageextension 50101 \"X\" extends \"C\" { layout { addafter(W) { } } }",
        "pageextension 50101 \"X\" extends \"C\" { layout { modify(W) { Visible = false; } } }",
        "pageextension 50101 \"X\" extends \"C\" { layout { addlast(Content) { repeater(W) { } } } }",
        "pageextension 50101 \"X\" extends \"C\" { views { addfirst { view(W) { } } } }",
        "report 50101 \"X\" { ProcessingOnly = true; dataset { dataitem(Row; \"R\") { } } requestpage { layout { area(Content) { group(W) { } } } } }",
        "page 50101 \"X\" { PageType = Card; layout { area(Content) { usercontrol(W; Ctl) { } } } }",
    ] {
        assert!(section_al0107(shape, "Key").is_empty(), "{shape}");
    }
}
