//! al-runner issue 8657: a member of an option type named, unquoted, by one of the words the AL compiler rejects as a
//! parameter name does not compile (`O: Option Zero,Key;`), and a table field's OptionMembers property rejects a list of
//! its own (`OptionMembers = Zero,If;`). The AL compilers of the BC artifacts of every supported major and a BC container
//! give the first diagnostic below at the word (assumption `keyword-option-members`).

use super::*;

/// The words of `PARAM_NAME_KEYWORDS` this checker judges in an option type: `begin` and `var` stop the parser already.
const TYPE_WORDS: [&str; 22] = [
    "Actions",
    "Area",
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
];
const PROPERTY_WORDS: [&str; 7] = [
    "If",
    "Else",
    "True",
    "False",
    "PageExtension",
    "ProfileExtension",
    "TableExtension",
];

/// The places an option type is written; `W` is the member under test.
const TYPE_SHAPES: [(&str, &str); 6] = [
    (
        "local variable",
        "codeunit 50101 \"U\" { procedure P() var O: Option Zero,W; begin end; }",
    ),
    (
        "first member",
        "codeunit 50101 \"U\" { procedure P() var O: Option W,Zero; begin end; }",
    ),
    (
        "parameter",
        "codeunit 50101 \"U\" { procedure P(O: Option Zero,W) begin end; }",
    ),
    (
        "global variable",
        "codeunit 50101 \"U\" { var O: Option Zero,W; procedure P() begin end; }",
    ),
    (
        "table global variable",
        "table 50102 \"T\" { fields { field(1; \"No.\"; Code[20]) { } } keys { key(PK; \"No.\") { Clustered = true; } } var O: Option Zero,W; }",
    ),
    (
        "return type",
        "codeunit 50101 \"U\" { procedure P(): Option Zero,W begin end; }",
    ),
];
const PROPERTY_SHAPES: [(&str, &str); 3] = [
    (
        "table field",
        "table 50102 \"T\" { fields { field(1; \"No.\"; Code[20]) { } field(2; Opt; Option) { OptionMembers = Zero,W; } } keys { key(PK; \"No.\") { Clustered = true; } } }",
    ),
    (
        "table field, first member",
        "table 50102 \"T\" { fields { field(1; \"No.\"; Code[20]) { } field(2; Opt; Option) { OptionMembers = W,Zero; } } keys { key(PK; \"No.\") { Clustered = true; } } }",
    ),
    (
        "table extension field",
        "table 50102 \"T\" { fields { field(1; \"No.\"; Code[20]) { } } keys { key(PK; \"No.\") { Clustered = true; } } }\ntableextension 50103 \"TX\" extends \"T\" { fields { field(50100; Opt; Option) { OptionMembers = Zero,W; } } }",
    ),
];

/// The diagnostics of `shape` with `name` in place of `W` that sit at the name, as (code, message).
fn at_name(shape: &str, name: &str) -> Vec<(String, String)> {
    let at = shape.find('W').expect("W");
    let src = format!("{}{name}{}\n", &shape[..at], &shape[at + 1..]);
    let ws = load(&[&src]);
    check_workspace(&ws)
        .into_iter()
        .filter(|d| d.offset == at)
        .map(|d| (d.code.to_string(), d.message))
        .collect()
}

#[test]
fn a_keyword_member_of_an_option_type_is_rejected_at_the_word() {
    for (what, shape) in TYPE_SHAPES {
        for w in TYPE_WORDS {
            let want = match (w, what) {
                ("End", "return type") => ("AL0519", "'End' is not valid value in this context.".to_string()),
                ("End", _) => ("AL0104", "Syntax error, ';' expected".to_string()),
                _ => (
                    "AL0105",
                    format!(
                        "Syntax error, identifier expected; '{}' is a keyword",
                        w.to_ascii_lowercase()
                    ),
                ),
            };
            assert_eq!(at_name(shape, w), [(want.0.to_string(), want.1)], "{what}: {w}");
        }
        assert_eq!(at_name(shape, "kEY").len(), 1, "{what}: kEY");
    }
}

#[test]
fn a_keyword_member_of_the_option_members_property_is_al0105_at_the_word() {
    for (what, shape) in PROPERTY_SHAPES {
        for w in PROPERTY_WORDS {
            let want = format!(
                "Syntax error, identifier expected; '{}' is a keyword",
                w.to_ascii_lowercase()
            );
            assert_eq!(at_name(shape, w), [("AL0105".to_string(), want)], "{what}: {w}");
        }
    }
}

/// The places of the cell table below; `M` is the member list.
const PLACES: [(&str, &str); 5] = [
    (
        "local",
        "codeunit 50101 \"U\" { procedure P() var O: Option M; begin end; }",
    ),
    (
        "global",
        "codeunit 50101 \"U\" { var O: Option M; procedure P() begin end; }",
    ),
    (
        "table global",
        "table 50102 \"T\" { fields { field(1; \"No.\"; Code[20]) { } } keys { key(PK; \"No.\") { Clustered = true; } } var O: Option M; }",
    ),
    (
        "parameter",
        "codeunit 50101 \"U\" { procedure P(O: Option M) begin end; }",
    ),
    ("return", "codeunit 50101 \"U\" { procedure P(): Option M begin end; }"),
];
/// The positions of the cell table below; `W` is the word.
const POSITIONS: [(&str, &str); 3] = [("first", "W,Zero"), ("middle", "Zero,W,One"), ("last", "Zero,W")];

/// Every cell of Begin, End and Var in an option type that the AL compilers of every supported major measured the same,
/// each a must-not-compile case of corpus PR 1502: (word, place, position, first diagnostic as (code, message, at the
/// comma after the word rather than the word)). `None` is an internal error of the AL compiler (AL0999 at the start of the
/// file), not judged. Begin and Var as the last member are left out: they stop this parser already.
const CELLS: [(&str, &str, &str, Option<(&str, &str, bool)>); 35] = [
    (
        "Begin",
        "local",
        "first",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    (
        "Begin",
        "local",
        "middle",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    (
        "Begin",
        "global",
        "first",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    (
        "Begin",
        "global",
        "middle",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    (
        "Begin",
        "table global",
        "first",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    (
        "Begin",
        "table global",
        "middle",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    (
        "Begin",
        "parameter",
        "first",
        Some(("AL0104", "Syntax error, ')' expected", false)),
    ),
    (
        "Begin",
        "parameter",
        "middle",
        Some(("AL0104", "Syntax error, ')' expected", false)),
    ),
    (
        "Begin",
        "return",
        "first",
        Some(("AL0104", "Syntax error, 'end' expected", true)),
    ),
    (
        "Begin",
        "return",
        "middle",
        Some(("AL0104", "Syntax error, 'end' expected", true)),
    ),
    (
        "End",
        "local",
        "first",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    (
        "End",
        "local",
        "middle",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    (
        "End",
        "local",
        "last",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    (
        "End",
        "global",
        "first",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    (
        "End",
        "global",
        "middle",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    (
        "End",
        "global",
        "last",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    (
        "End",
        "table global",
        "first",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    (
        "End",
        "table global",
        "middle",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    (
        "End",
        "table global",
        "last",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    ("End", "parameter", "first", None),
    (
        "End",
        "parameter",
        "middle",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    (
        "End",
        "parameter",
        "last",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    ("End", "return", "first", None),
    (
        "End",
        "return",
        "middle",
        Some(("AL0519", "'End' is not valid value in this context.", false)),
    ),
    (
        "End",
        "return",
        "last",
        Some(("AL0519", "'End' is not valid value in this context.", false)),
    ),
    (
        "Var",
        "local",
        "first",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    (
        "Var",
        "local",
        "middle",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    (
        "Var",
        "global",
        "first",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    (
        "Var",
        "global",
        "middle",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    (
        "Var",
        "table global",
        "first",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    (
        "Var",
        "table global",
        "middle",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    ("Var", "parameter", "first", None),
    (
        "Var",
        "parameter",
        "middle",
        Some(("AL0104", "Syntax error, ';' expected", false)),
    ),
    (
        "Var",
        "return",
        "first",
        Some(("AL0519", "',' is not valid value in this context.", true)),
    ),
    (
        "Var",
        "return",
        "middle",
        Some(("AL0519", "',' is not valid value in this context.", true)),
    ),
];

#[test]
fn begin_end_and_var_give_the_first_diagnostic_alc_gives_in_each_cell() {
    for (word, place, position, want) in CELLS {
        let shape = PLACES.iter().find(|(p, _)| *p == place).expect("place").1;
        let members = POSITIONS.iter().find(|(p, _)| *p == position).expect("position").1;
        let src = format!("{}\n", shape.replacen('M', &members.replacen('W', word, 1), 1));
        let at = src.find(&members.replacen('W', word, 1)).expect("members") + members.find('W').expect("W");
        let ws = load(&[&src]);
        let got: Vec<_> = check_workspace(&ws)
            .into_iter()
            .map(|d| (d.code.to_string(), d.message, d.offset))
            .collect();
        let want: Vec<_> = want
            .map(|(code, message, at_comma)| {
                (
                    code.to_string(),
                    message.to_string(),
                    at + if at_comma { word.len() } else { 0 },
                )
            })
            .into_iter()
            .collect();
        assert_eq!(got, want, "{word}, {place}, {position}");
    }
}

/// An option type inside an array (`array[2] of Option Zero,Key`) is not measured and keeps main's answer: it compiles.
#[test]
fn an_option_inside_an_array_is_not_judged() {
    for shape in [
        "codeunit 50101 \"U\" { procedure P() var A: array[2] of Option Zero,W; begin end; }",
        "codeunit 50101 \"U\" { procedure P(A: array[2] of Option Zero,W,One) begin end; }",
    ] {
        for w in ["Key", "End", "Begin", "Var"] {
            assert!(at_name(shape, w).is_empty(), "{w} in {shape}");
        }
    }
}

#[test]
fn quoted_members_and_other_words_compile() {
    for (what, shape) in TYPE_SHAPES {
        for name in ["\"Key\"", "\"End\"", "Local", "Exit", "If", "Foo"] {
            assert!(at_name(shape, name).is_empty(), "{what}: {name}");
        }
    }
    for (what, shape) in PROPERTY_SHAPES {
        for name in ["\"If\"", "Key", "Local", "Exit", "Foo"] {
            assert!(at_name(shape, name).is_empty(), "{what}: {name}");
        }
    }
}
