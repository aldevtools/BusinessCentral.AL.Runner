//! al-runner issue 8630: a table field named, unquoted, by one of the words the AL compiler rejects as a parameter name does
//! not compile. AL0107 at the name is the first diagnostic the AL compilers of the BC artifacts of every supported major and a
//! BC container gave, in a table and in a table extension (assumption `keyword-field-names`).

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
