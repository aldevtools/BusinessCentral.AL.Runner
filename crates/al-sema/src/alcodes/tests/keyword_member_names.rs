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

/// al-runner issue 8649: the same 32 words after `::` (corpus must-not-compile 3158-3160 pin `in`). `@` is the value or
/// object name, declared quoted; `#` is the use under test.
const SCOPE_SHAPES: [(&str, &str); 6] = [
    (
        "enum variable",
        "enum 50100 \"E\" { value(0; Zero) { } value(1; \"@\") { } }\ncodeunit 50101 \"U\" { procedure P() var K: Enum \"E\"; begin K := K::#; end; }",
    ),
    (
        "enum type",
        "enum 50100 \"E\" { value(0; Zero) { } value(1; \"@\") { } }\ncodeunit 50101 \"U\" { procedure P(): Integer begin exit(\"E\"::#.AsInteger()); end; }",
    ),
    (
        "option field",
        "table 50102 \"T\" { fields { field(1; \"No.\"; Code[20]) { } field(2; Opt; Option) { OptionMembers = Zero,\"@\"; } } keys { key(PK; \"No.\") { Clustered = true; } } }\ncodeunit 50101 \"U\" { procedure P() var Row: Record \"T\"; begin Row.Opt := Row.Opt::#; end; }",
    ),
    (
        "option variable",
        "codeunit 50101 \"U\" { procedure P() var O: Option Zero,\"@\"; begin O := O::#; end; }",
    ),
    (
        "case label",
        "enum 50100 \"E\" { value(0; Zero) { } value(1; \"@\") { } }\ncodeunit 50101 \"U\" { procedure P(K: Enum \"E\"): Integer begin case K of K::#: exit(1); end; exit(0); end; }",
    ),
    (
        "object id",
        "table 50102 \"@\" { fields { field(1; \"No.\"; Code[20]) { } } keys { key(PK; \"No.\") { Clustered = true; } } }\ncodeunit 50101 \"U\" { procedure P(): Integer begin exit(Database::#); end; }",
    ),
];

/// The AL0107 diagnostics of scope shape `shape` with `name` after `::`, each as `true` when it is at the name.
fn scope_al0107(shape: &str, name: &str) -> Vec<bool> {
    let decl = name.trim_matches('"');
    let fill = |s: &str| s.replace('@', decl);
    let at = shape.find('#').expect("#");
    let src = format!("{}{name}{}\n", fill(&shape[..at]), fill(&shape[at + 1..]));
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
fn a_keyword_unquoted_after_the_scope_operator_is_al0107_at_the_word() {
    for (what, shape) in SCOPE_SHAPES {
        for w in WORDS {
            assert_eq!(scope_al0107(shape, w), [true], "{what}: ::{w}");
        }
        assert_eq!(scope_al0107(shape, "eXIT"), [true], "{what}: ::eXIT");
    }
}

#[test]
fn quoted_names_and_other_words_after_the_scope_operator_compile() {
    for (what, shape) in SCOPE_SHAPES {
        for name in ["\"Exit\"", "\"In\"", "Local", "Foo"] {
            assert!(scope_al0107(shape, name).is_empty(), "{what}: ::{name}");
        }
    }
}

/// al-runner issue 8650: the same words after `.` or `::` in the expressions the AL compilers parse from properties and section
/// arguments. `#` is the expression under test (`Rec.@` or `Rec.Opt::@`); the table declares `@` quoted.
const PROPERTY_TABLE: &str = "table 50100 \"R\" { fields { field(1; \"No.\"; Code[20]) { } field(2; \"@\"; Integer) { } field(3; Opt; Option) { OptionMembers = Zero,\"@\"; } } keys { key(PK; \"No.\") { Clustered = true; } } }\npage 50105 \"Part\" { PageType = ListPart; SourceTable = \"R\"; layout { area(Content) { repeater(L) { field(\"No.\"; Rec.\"No.\") { } } } } }\n";
const PROPERTY_SHAPES: [(&str, &str); 20] = [
    (
        "control Visible",
        "page 50101 \"P\" { PageType = Card; SourceTable = \"R\"; layout { area(Content) { field(\"No.\"; Rec.\"No.\") { Visible = # > 0; } } } }",
    ),
    (
        "control Editable",
        "page 50101 \"P\" { PageType = Card; SourceTable = \"R\"; layout { area(Content) { field(\"No.\"; Rec.\"No.\") { Editable = # > 0; } } } }",
    ),
    (
        "control Enabled",
        "page 50101 \"P\" { PageType = Card; SourceTable = \"R\"; layout { area(Content) { field(\"No.\"; Rec.\"No.\") { Enabled = # > 0; } } } }",
    ),
    (
        "control CaptionClass",
        "page 50101 \"P\" { PageType = Card; SourceTable = \"R\"; layout { area(Content) { field(\"No.\"; Rec.\"No.\") { CaptionClass = Format(#); } } } }",
    ),
    (
        "control HideValue",
        "page 50101 \"P\" { PageType = Card; SourceTable = \"R\"; layout { area(Content) { field(\"No.\"; Rec.\"No.\") { HideValue = # > 0; } } } }",
    ),
    (
        "control ShowMandatory",
        "page 50101 \"P\" { PageType = Card; SourceTable = \"R\"; layout { area(Content) { field(\"No.\"; Rec.\"No.\") { ShowMandatory = # > 0; } } } }",
    ),
    (
        "control QuickEntry",
        "page 50101 \"P\" { PageType = Card; SourceTable = \"R\"; layout { area(Content) { field(\"No.\"; Rec.\"No.\") { QuickEntry = # > 0; } } } }",
    ),
    (
        "control source",
        "page 50101 \"P\" { PageType = Card; SourceTable = \"R\"; layout { area(Content) { field(Shown; #) { } } } }",
    ),
    (
        "action Visible",
        "page 50101 \"P\" { PageType = Card; SourceTable = \"R\"; actions { area(Processing) { action(Act) { Visible = # > 0; trigger OnAction() begin end; } } } }",
    ),
    (
        "action Enabled",
        "page 50101 \"P\" { PageType = Card; SourceTable = \"R\"; actions { area(Processing) { action(Act) { Enabled = # > 0; trigger OnAction() begin end; } } } }",
    ),
    (
        "group Visible",
        "page 50101 \"P\" { PageType = Card; SourceTable = \"R\"; layout { area(Content) { group(G) { Visible = # > 0; field(\"No.\"; Rec.\"No.\") { } } } } }",
    ),
    (
        "part Visible",
        "page 50101 \"P\" { PageType = Card; SourceTable = \"R\"; layout { area(Content) { part(Lines; \"Part\") { Visible = # > 0; } } } }",
    ),
    (
        "page DataCaptionExpression",
        "page 50101 \"P\" { PageType = Card; SourceTable = \"R\"; DataCaptionExpression = Format(#); layout { area(Content) { field(\"No.\"; Rec.\"No.\") { } } } }",
    ),
    (
        "page extension control Visible",
        "pageextension 50103 \"PX\" extends \"P\" { layout { modify(\"No.\") { Visible = # > 0; } } }",
    ),
    (
        "page extension control source",
        "pageextension 50103 \"PX\" extends \"P\" { layout { addlast(Content) { field(Shown; #) { } } } }",
    ),
    (
        "page extension action Visible",
        "pageextension 50103 \"PX\" extends \"P\" { actions { addlast(Processing) { action(Act2) { Visible = # > 0; trigger OnAction() begin end; } } } }",
    ),
    (
        "report column source",
        "report 50102 \"Rp\" { ProcessingOnly = true; dataset { dataitem(Row; \"R\") { column(C; #) { } } } }",
    ),
    (
        "report extension column source",
        "reportextension 50103 \"RX\" extends \"Rp\" { dataset { add(Row) { column(C; #) { } } } }",
    ),
    (
        "action Visible, second expression",
        "page 50101 \"P\" { PageType = Card; SourceTable = \"R\"; actions { area(Processing) { action(Act) { Visible = (1 > 0) and (# > 0); trigger OnAction() begin end; } } } }",
    ),
    (
        "control Visible in a repeater",
        "page 50101 \"P\" { PageType = List; SourceTable = \"R\"; layout { area(Content) { repeater(L) { field(\"No.\"; Rec.\"No.\") { Visible = # > 0; } } } } }",
    ),
];
/// The objects a shape extends, when it does not declare them: a card page and a report.
const BASES: &str = "page 50101 \"P\" { PageType = Card; SourceTable = \"R\"; layout { area(Content) { field(\"No.\"; Rec.\"No.\") { } } } actions { area(Processing) { action(Act) { trigger OnAction() begin end; } } } }\nreport 50102 \"Rp\" { ProcessingOnly = true; dataset { dataitem(Row; \"R\") { column(No; \"No.\") { } } } }\n";

/// The AL0107 diagnostics of property shape `shape` with the expression `expr` in place of `#` (`@` in it is `name`), each as
/// `true` when it is at the name.
fn property_al0107(shape: &str, expr: &str, name: &str) -> Vec<bool> {
    let decl = name.trim_matches('"');
    let at = shape.find('#').expect("#");
    let expr = expr.replace('@', name);
    let body = format!("{}{expr}{}", &shape[..at], &shape[at + 1..]);
    let bases = if shape.starts_with("pageextension") || shape.starts_with("reportextension") {
        BASES
    } else {
        ""
    };
    let src = format!("{body}\n{}{bases}", PROPERTY_TABLE.replace('@', decl));
    let at = at + expr.rfind(name).expect("name");
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
fn a_keyword_unquoted_in_a_property_or_source_expression_is_al0107_at_the_word() {
    for (what, shape) in PROPERTY_SHAPES {
        for expr in ["Rec.@", "Rec.Opt::@"] {
            let expr = if what.starts_with("report") {
                expr.replace("Rec.", "Row.")
            } else {
                expr.to_string()
            };
            for w in WORDS {
                assert_eq!(property_al0107(shape, &expr, w), [true], "{what}: {expr} {w}");
            }
        }
    }
}

#[test]
fn quoted_names_and_other_words_in_a_property_or_source_expression_compile() {
    for (what, shape) in PROPERTY_SHAPES {
        for expr in ["Rec.@", "Rec.Opt::@"] {
            let expr = if what.starts_with("report") {
                expr.replace("Rec.", "Row.")
            } else {
                expr.to_string()
            };
            for name in ["\"If\"", "Local", "Key", "Foo"] {
                assert!(property_al0107(shape, &expr, name).is_empty(), "{what}: {expr} {name}");
            }
        }
    }
}

/// Properties no cell measured keep compiling, and so do the properties that name a field after `.` with a grammar of their
/// own (TableRelation and CalcFormula reject only some of the words).
#[test]
fn unmeasured_properties_are_not_judged() {
    let table = PROPERTY_TABLE.replace('@', "Exit");
    for src in [
        "page 50101 \"P\" { PageType = Card; SourceTable = \"R\"; layout { area(Content) { field(\"No.\"; Rec.\"No.\") { StyleExpr = Rec.Exit; } } } }".to_string(),
        "table 50103 \"S\" { fields { field(1; Ref; Integer) { TableRelation = \"R\".Exit; } } keys { key(PK; Ref) { Clustered = true; } } }".to_string(),
        "report 50102 \"Rp\" { ProcessingOnly = true; dataset { dataitem(Row; \"R\") { } } requestpage { layout { area(Content) { field(Opt; Row.Exit) { } } } } }".to_string(),
    ] {
        let ws = load(&[&format!("{src}\n{table}")]);
        assert!(check_workspace(&ws).iter().all(|d| d.code != "AL0107"), "{src}");
    }
}
