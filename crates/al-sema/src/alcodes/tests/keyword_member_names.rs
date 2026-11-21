//! al-runner issue 8635: one of 32 words of the AL language, unquoted after `.` in the code of a procedure or trigger, does not
//! compile. AL0107 at the word is the first diagnostic the AL compilers of the BC artifacts of every supported major and a BC
//! container gave, in every place below (assumption `keyword-member-names`).

use super::*;

const WORDS: [&str; 32] = [
    "And",
    "AssertError",
    "Begin",
    "Case",
    "Div",
    "Do",
    "DownTo",
    "Else",
    "End",
    "Event",
    "Exit",
    "False",
    "For",
    "ForEach",
    "If",
    "In",
    "Mod",
    "Not",
    "Of",
    "Or",
    "Procedure",
    "Repeat",
    "Then",
    "This",
    "To",
    "Trigger",
    "True",
    "Until",
    "Var",
    "While",
    "With",
    "Xor",
];

/// The table and callee every shape may use; `@` is the member name, declared quoted.
const T: &str = "table 50100 \"R\" { fields { field(1; \"No.\"; Code[20]) { } field(2; \"@\"; Integer) { } } keys { key(PK; \"No.\") { Clustered = true; } } }\n";
const CALLEE: &str = "codeunit 50102 \"Callee\" { procedure \"@\"(): Integer begin exit(1); end; }\n";
const PAGE: &str = "page 50103 \"P\" { PageType = Card; SourceTable = \"R\"; layout { area(Content) { field(\"@\"; Rec.\"@\") { } } } }\n";

/// One shape per place the AL compilers reject such a member in; `.#` is the member under test.
const SHAPES: [(&str, &str); 14] = [
    (
        "field written",
        "codeunit 50101 \"U\" { procedure P() var Row: Record \"R\"; begin Row.# := 1; end; }",
    ),
    (
        "field read",
        "codeunit 50101 \"U\" { procedure P() var Row: Record \"R\"; V: Integer; begin V := Row.#; end; }",
    ),
    (
        "field as an argument",
        "codeunit 50101 \"U\" { procedure P() var Row: Record \"R\"; begin Row.SetRange(\"No.\", Format(Row.#)); end; }",
    ),
    (
        "method called",
        "codeunit 50101 \"U\" { procedure P() var C: Codeunit \"Callee\"; V: Integer; begin V := C.#(); end; }",
    ),
    (
        "method called as a statement",
        "codeunit 50101 \"U\" { procedure P() var C: Codeunit \"Callee\"; begin C.#(); end; }",
    ),
    (
        "OnRun",
        "codeunit 50101 \"U\" { trigger OnRun() var Row: Record \"R\"; begin Row.# := 1; end; }",
    ),
    (
        "table trigger",
        "table 50104 \"S\" { fields { field(1; \"No.\"; Code[20]) { } field(2; \"@\"; Integer) { } } keys { key(PK; \"No.\") { Clustered = true; } } trigger OnInsert() begin Rec.# := 1; end; }",
    ),
    (
        "field trigger, xRec",
        "table 50104 \"S\" { fields { field(1; \"No.\"; Code[20]) { trigger OnValidate() begin if xRec.# = 0 then; end; } field(2; \"@\"; Integer) { } } keys { key(PK; \"No.\") { Clustered = true; } } }",
    ),
    (
        "page trigger",
        "page 50101 \"U\" { PageType = Card; SourceTable = \"R\"; trigger OnAfterGetRecord() begin Rec.# := 1; end; }",
    ),
    (
        "page action",
        "page 50101 \"U\" { PageType = Card; SourceTable = \"R\"; actions { area(Processing) { action(Act) { trigger OnAction() begin Rec.# := 1; end; } } } }",
    ),
    (
        "report trigger",
        "report 50101 \"U\" { ProcessingOnly = true; dataset { dataitem(Row; \"R\") { trigger OnAfterGetRecord() begin Row.# := 1; end; } } }",
    ),
    (
        "TestPage control",
        "codeunit 50101 \"U\" { Subtype = Test; [Test] procedure P() var Card: TestPage \"P\"; begin Card.OpenView(); Card.#.SetValue(1); end; }",
    ),
    (
        "interface method",
        "interface \"I\" { procedure \"@\"(): Integer; }\ncodeunit 50105 \"Impl\" implements \"I\" { procedure \"@\"(): Integer begin exit(1); end; }\ncodeunit 50101 \"U\" { procedure P(Iface: Interface \"I\") var V: Integer; begin V := Iface.#(); end; }",
    ),
    (
        "query column",
        "query 50106 \"Q\" { elements { dataitem(Row; \"R\") { column(\"@\"; \"@\") { } } } }\ncodeunit 50101 \"U\" { procedure P() var Qy: Query \"Q\"; V: Integer; begin Qy.Open(); if Qy.Read() then V := Qy.#; end; }",
    ),
];

/// The AL0107 diagnostics of `shape` with member `name`, each as `true` when it is at the member. The shape comes first in the
/// file, so the synthetic routines the workspace adds to the page after it do not move it.
fn member_al0107(shape: &str, name: &str) -> Vec<bool> {
    let decl = name.trim_matches('"');
    let fill = |s: &str| s.replace('@', decl);
    let at = shape.find('#').expect("#");
    let src = format!(
        "{}{name}{}\n{}{}{}",
        fill(&shape[..at]),
        fill(&shape[at + 1..]),
        fill(T),
        fill(CALLEE),
        fill(PAGE)
    );
    let at = fill(&shape[..at]).len();
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
fn a_keyword_as_an_unquoted_member_in_code_is_al0107_at_the_member() {
    for (what, shape) in SHAPES {
        for w in WORDS {
            assert_eq!(member_al0107(shape, w), [true], "{what}: .{w}");
        }
        assert_eq!(member_al0107(shape, "iF"), [true], "{what}: .iF");
    }
}

#[test]
fn quoted_members_and_other_words_compile() {
    for (what, shape) in SHAPES {
        for name in [
            "\"If\"", "\"This\"", "Key", "Area", "Local", "Value", "Column", "Page", "Foo",
        ] {
            assert!(member_al0107(shape, name).is_empty(), "{what}: .{name}");
        }
    }
}

/// A namespace segment that is such a word compiles in the namespace declaration, in `using` and in a variable's type; only a
/// use in code is AL0107 (measured with If, Exit and This).
#[test]
fn a_keyword_namespace_segment_is_judged_in_code_only() {
    let decl = "namespace Ns.If;\ntable 50100 \"R\" { fields { field(1; \"No.\"; Code[20]) { } } keys { key(PK; \"No.\") { Clustered = true; } } }\n";
    let typed = "namespace Other;\nusing Ns.If;\ncodeunit 50101 \"U\" { procedure P() var Row: Record Ns.If.\"R\"; begin end; }\n";
    let ws = load(&[decl, typed]);
    assert!(check_workspace(&ws).iter().all(|d| d.code != "AL0107"));
    let code =
        "namespace Other;\ncodeunit 50101 \"U\" { procedure P(): Integer begin exit(Database::Ns.If.\"R\"); end; }\n";
    let ws = load(&[decl, code]);
    let at = code.find("Ns.If.").unwrap() + 3;
    let got: Vec<_> = check_workspace(&ws)
        .into_iter()
        .filter(|d| d.code == "AL0107")
        .map(|d| (d.file, d.offset))
        .collect();
    assert_eq!(got, [("f1.al".to_string(), at)]);
}

/// A procedure of the author's named like the workspace's synthetic routines is code the AL compilers judge (alc 18.1 gives
/// AL0107 for `Row.If` in a procedure `__visible_12`); the routines the workspace adds for a page's expression properties are
/// not judged.
#[test]
fn an_authored_procedure_named_like_a_synthetic_routine_is_judged() {
    let src = format!(
        "codeunit 50101 \"U\" {{ procedure __visible_12() var Row: Record \"R\"; begin Row.If := 1; end; }}\n{}",
        T.replace('@', "If")
    );
    let at = src.find("Row.If").unwrap() + 4;
    let ws = load(&[&src]);
    let got: Vec<usize> = check_workspace(&ws)
        .into_iter()
        .filter(|d| d.code == "AL0107")
        .map(|d| d.offset)
        .collect();
    assert_eq!(got, [at]);
    let page = format!(
        "page 50103 \"P\" {{ PageType = Card; SourceTable = \"R\"; layout {{ area(Content) {{ field(\"If\"; Rec.\"If\") {{ Visible = Rec.\"If\" > 0; }} }} }} }}\n{}",
        T.replace('@', "If")
    );
    let ws = load(&[&page]);
    assert!(!ws.synthetic(0).is_empty(), "the page has a synthetic routine");
    assert!(check_workspace(&ws).iter().all(|d| d.code != "AL0107"));
}
