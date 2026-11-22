//! al-runner issue 8539: a variable named, unquoted, by a word the AL compiler reads as a keyword there does not compile.
//! Every expectation is the first diagnostic the AL compilers of the BC artifacts of every supported major gave for that shape
//! (the same code and place on all three; the measurement is in the issue). Assumption `keyword-variable-names`; the page
//! global `area` is must-not-compile 0334-variable-named-area.al:1.

use super::*;

const AL0107: &str =
    "AL0107 Syntax error, identifier expected. Provide a valid name (letters, digits, and underscores only).";
const BRACE: &str = "AL0104 Syntax error, '}' expected";
const BEGIN: &str = "AL0104 Syntax error, 'begin' expected";
const PROCEDURE: &str = "AL0104 Syntax error, 'procedure' expected";
const AL0303: &str = "AL0303 Attributes can only be defined on variables and methods.";

/// The words that the compiler rejects as the first name of a declaration (and that the parser takes as a name).
const WORDS: [&str; 18] = [
    "Actions",
    "Area",
    "ChartPart",
    "CueGroup",
    "Event",
    "Extends",
    "False",
    "FieldAttribute",
    "FieldElement",
    "FieldGroups",
    "Key",
    "PageExtension",
    "ProfileExtension",
    "TableElement",
    "TableExtension",
    "TextAttribute",
    "TextElement",
    "True",
];

/// AL0519 for these as the first declaration of a procedure's var section.
const AL0519_FIRST: [&str; 6] = ["Area", "Extends", "False", "FieldGroups", "Key", "True"];
/// AL0519 for these as a later declaration of any var section.
const AL0519_LATER: [&str; 9] = [
    "Area",
    "Extends",
    "False",
    "FieldGroups",
    "Key",
    "True",
    "PageExtension",
    "ProfileExtension",
    "TableExtension",
];

fn al0519(w: &str) -> String {
    format!("AL0519 '{w}' is not valid value in this context.")
}

/// The keyword-name diagnostics of a compile of `src`, each with where it stands: `@word` at the offset of `word` (its
/// last occurrence before `: `), `@after` right after it, `@attr` at the first `[`, else the offset.
fn diags(src: &str, word: &str) -> Vec<String> {
    let ws = load(&[src]);
    let name = src
        .rfind(&format!("{word}:"))
        .or_else(|| src.rfind(&format!("{word},")))
        .expect("the word");
    check_workspace(&ws)
        .into_iter()
        .filter(|d| matches!(d.code, "AL0104" | "AL0107" | "AL0303" | "AL0519"))
        .map(|d| {
            let place = if d.offset == name {
                "@word".to_string()
            } else if d.offset == name + word.len() {
                "@after".to_string()
            } else if src.find('[') == Some(d.offset) {
                "@attr".to_string()
            } else {
                format!("@{}", d.offset)
            };
            format!("{} {} {place}", d.code, d.message)
        })
        .collect()
}

fn one(expected: &str, place: &str) -> Vec<String> {
    vec![format!("{expected} {place}")]
}

fn proc_var(decls: &str) -> String {
    format!("codeunit 50100 \"R\"\n{{\n    procedure P()\n    var\n{decls}    begin\n    end;\n}}\n")
}

fn global_var(decls: &str) -> String {
    format!("codeunit 50100 \"R\"\n{{\n    var\n{decls}\n    procedure P()\n    begin\n    end;\n}}\n")
}

fn trigger_var(decls: &str) -> String {
    format!("codeunit 50100 \"R\"\n{{\n    trigger OnRun()\n    var\n{decls}    begin\n    end;\n}}\n")
}

#[test]
fn first_declaration_of_a_procedure_var_section() {
    for w in WORDS {
        let got = diags(&proc_var(&format!("        {w}: Integer;\n")), w);
        let want = match w {
            "Event" => one(AL0107, "@after"),
            _ if AL0519_FIRST.contains(&w) => one(&al0519(w), "@word"),
            _ => one(BRACE, "@word"),
        };
        assert_eq!(got, want, "{w}");
    }
}

#[test]
fn later_declaration_of_a_procedure_var_section() {
    for w in WORDS {
        let got = diags(
            &proc_var(&format!("        Other: Integer;\n        {w}: Integer;\n")),
            w,
        );
        let want = match w {
            "Event" => one(AL0107, "@after"),
            _ if AL0519_LATER.contains(&w) => one(&al0519(w), "@word"),
            _ => one(BRACE, "@word"),
        };
        assert_eq!(got, want, "{w}");
    }
}

#[test]
fn global_var_section() {
    for w in WORDS {
        let first = diags(&global_var(&format!("        {w}: Integer;\n")), w);
        let later = diags(
            &global_var(&format!("        Other: Integer;\n        {w}: Integer;\n")),
            w,
        );
        let (want_first, want_later) = match w {
            "Event" => (one(AL0107, "@after"), one(AL0107, "@after")),
            _ if AL0519_LATER.contains(&w) => (one(BRACE, "@word"), one(&al0519(w), "@word")),
            _ => (one(BRACE, "@word"), one(BRACE, "@word")),
        };
        assert_eq!(first, want_first, "first {w}");
        assert_eq!(later, want_later, "later {w}");
    }
}

#[test]
fn trigger_var_section() {
    for w in WORDS {
        let first = diags(&trigger_var(&format!("        {w}: Integer;\n")), w);
        let later = diags(
            &trigger_var(&format!("        Other: Integer;\n        {w}: Integer;\n")),
            w,
        );
        assert_eq!(first, one(BEGIN, "@word"), "first {w}");
        let want_later = if AL0519_LATER.contains(&w) {
            one(&al0519(w), "@word")
        } else {
            one(BEGIN, "@word")
        };
        assert_eq!(later, want_later, "later {w}");
    }
}

/// After a comma the parameter-name words are rejected, the six the parser never takes as a first name included; `local`
/// and the statement keywords compile there.
#[test]
fn a_name_after_a_comma() {
    for w in WORDS
        .iter()
        .copied()
        .chain(["Begin", "End", "Of", "Procedure", "Trigger", "Var"])
    {
        let src = format!("        Other, {w}: Integer;\n");
        for (shape, text) in [
            ("procedure", proc_var(&src)),
            ("global", global_var(&src)),
            ("trigger", trigger_var(&src)),
        ] {
            assert_eq!(diags(&text, w), one(AL0107, "@word"), "{shape} {w}");
        }
    }
    for w in ["Local", "If", "Exit", "Foo"] {
        assert!(
            diags(&proc_var(&format!("        Other, {w}: Integer;\n")), w).is_empty(),
            "{w}"
        );
    }
    // A keyword as the first name of a list is the first name of a declaration.
    assert_eq!(
        diags(&proc_var("        Key, Other: Integer;\n"), "Key"),
        one(&al0519("Key"), "@word")
    );
}

#[test]
fn an_attributed_declaration() {
    for w in WORDS {
        let decl = format!("        [NonDebuggable]\n        {w}: Text;\n");
        let (global, local) = match w {
            "Event" => (one(AL0107, "@after"), one(AL0303, "@attr")),
            "True" | "False" => (one(PROCEDURE, "@word"), one(AL0303, "@attr")),
            _ => (vec![], vec![]),
        };
        assert_eq!(diags(&global_var(&decl), w), global, "global {w}");
        assert_eq!(diags(&proc_var(&decl), w), local, "procedure {w}");
        assert_eq!(diags(&trigger_var(&decl), w), local, "trigger {w}");
    }
    // Two attributes: AL0303 at the first.
    let two = "        [NonDebuggable]\n        [SecurityFiltering(SecurityFilter::Filtered)]\n        Event: Record Customer;\n";
    assert_eq!(diags(&proc_var(two), "Event"), one(AL0303, "@attr"));
    // The declaration after an attributed one is a later declaration.
    let after = "        [NonDebuggable]\n        Other: Text;\n        PageExtension: Integer;\n";
    assert_eq!(
        diags(&proc_var(after), "PageExtension"),
        one(&al0519("PageExtension"), "@word")
    );
}

#[test]
fn the_word_is_named_as_written_and_quoted_names_compile() {
    assert_eq!(
        diags(&proc_var("        key: Integer;\n"), "key"),
        one(&al0519("key"), "@word")
    );
    assert_eq!(
        diags(&proc_var("        KEY: Record Customer;\n"), "KEY"),
        one(&al0519("KEY"), "@word")
    );
    for decl in [
        "        \"Key\": Integer;\n",
        "        Keys: Integer;\n",
        "        Fields: Integer;\n",
        "        Field: Integer;\n",
        "        Layout: Integer;\n",
    ] {
        let w = decl.trim().split(':').next().unwrap();
        assert!(diags(&proc_var(decl), w).is_empty(), "{decl}");
    }
}

/// The section kinds of the other object types: a procedure, an object trigger, a field or element or action trigger, the
/// global var section and a report's request page var section.
#[test]
fn every_object_type_with_a_var_section() {
    let p = "    procedure P()\n    var\n        Key: Integer;\n    begin\n    end;\n";
    let g = "    var\n        Key: Integer;\n";
    let tr = |name: &str| format!("    trigger {name}()\n    var\n        Key: Integer;\n    begin\n    end;\n");
    let table = |body: &str, ft: &str| {
        format!(
            "table 50100 \"R\"\n{{\n    fields\n    {{\n        field(1; \"No.\"; Code[20])\n        {{\n{ft}        }}\n    }}\n    keys\n    {{\n        key(PK; \"No.\") {{ Clustered = true; }}\n    }}\n{body}}}\n"
        )
    };
    let page = |body: &str, ft: &str| {
        format!(
            "page 50100 \"R\"\n{{\n    PageType = Card;\n    SourceTable = Customer;\n\n    layout\n    {{\n        area(Content)\n        {{\n            field(No; Rec.\"No.\")\n            {{\n{ft}            }}\n        }}\n    }}\n{body}}}\n"
        )
    };
    let report = |body: &str, rp: &str| {
        format!(
            "report 50100 \"R\"\n{{\n    ProcessingOnly = true;\n\n    dataset\n    {{\n        dataitem(Cust; Customer)\n        {{\n        }}\n    }}\n    requestpage\n    {{\n{rp}    }}\n{body}}}\n"
        )
    };
    let xmlport = |body: &str, et: &str| {
        format!(
            "xmlport 50100 \"R\"\n{{\n    schema\n    {{\n        textelement(Root)\n        {{\n{et}        }}\n    }}\n{body}}}\n"
        )
    };
    let query = |body: &str| {
        format!(
            "query 50100 \"R\"\n{{\n    elements\n    {{\n        dataitem(Cust; Customer)\n        {{\n            column(No; \"No.\")\n            {{\n            }}\n        }}\n    }}\n{body}}}\n"
        )
    };
    let ext = |kind: &str, target: &str, body: &str| format!("{kind} 50100 \"R\" extends {target}\n{{\n{body}}}\n");
    let field_trigger = "            trigger OnValidate()\n            var\n                Key: Integer;\n            begin\n            end;\n";
    let cases = [
        (table(p, ""), al0519("Key")),
        (table(g, ""), BRACE.to_string()),
        (table(&tr("OnInsert"), ""), BEGIN.to_string()),
        (table("", field_trigger), BEGIN.to_string()),
        (ext("tableextension", "Customer", p), al0519("Key")),
        (page(p, ""), al0519("Key")),
        (
            page("    protected var\n        Key: Integer;\n", ""),
            BRACE.to_string(),
        ),
        (page(&tr("OnOpenPage"), ""), BEGIN.to_string()),
        (page("", field_trigger), BEGIN.to_string()),
        (ext("pageextension", "\"Customer Card\"", p), al0519("Key")),
        (report(p, ""), al0519("Key")),
        (
            report("", "        var\n            Key: Integer;\n"),
            BRACE.to_string(),
        ),
        (report("", &tr("OnOpenPage")), BEGIN.to_string()),
        (ext("reportextension", "\"Customer - List\"", p), al0519("Key")),
        (xmlport(p, ""), al0519("Key")),
        (xmlport("", &tr("OnBeforePassVariable")), BEGIN.to_string()),
        (query(p), al0519("Key")),
        (query(&tr("OnBeforeOpen")), BEGIN.to_string()),
    ];
    for (src, want) in cases {
        assert_eq!(diags(&src, "Key"), one(&want, "@word"), "{src}");
    }
}

/// The request page var section of a report, a report extension and an xmlport follows the rule of a global var section,
/// for every word, as the first or a later declaration, after a comma and after an attribute; a request page trigger
/// follows the trigger rule.
#[test]
fn request_page_var_sections() {
    let hosts = [
        "report 50100 \"R\"\n{\n    ProcessingOnly = true;\n\n    dataset\n    {\n        dataitem(Cust; Customer)\n        {\n        }\n    }\n    requestpage\n    {\nRP    }\n}\n",
        "reportextension 50100 \"R\" extends \"Customer - List\"\n{\n    requestpage\n    {\nRP    }\n}\n",
        "xmlport 50100 \"R\"\n{\n    schema\n    {\n        textelement(Root)\n        {\n        }\n    }\n    requestpage\n    {\nRP    }\n}\n",
    ];
    for host in hosts {
        let src = |rp: &str| host.replace("RP", rp);
        for w in WORDS {
            let first = diags(&src(&format!("        var\n            {w}: Integer;\n")), w);
            let later = diags(
                &src(&format!(
                    "        var\n            Other: Integer;\n            {w}: Integer;\n"
                )),
                w,
            );
            let (want_first, want_later) = match w {
                "Event" => (one(AL0107, "@after"), one(AL0107, "@after")),
                _ if AL0519_LATER.contains(&w) => (one(BRACE, "@word"), one(&al0519(w), "@word")),
                _ => (one(BRACE, "@word"), one(BRACE, "@word")),
            };
            assert_eq!(first, want_first, "first {w} {host}");
            assert_eq!(later, want_later, "later {w} {host}");
            let list = diags(&src(&format!("        var\n            Other, {w}: Integer;\n")), w);
            assert_eq!(list, one(AL0107, "@word"), "list {w} {host}");
            let attributed = diags(
                &src(&format!(
                    "        var\n            [NonDebuggable]\n            {w}: Text;\n"
                )),
                w,
            );
            let want_attributed = match w {
                "Event" => one(AL0107, "@after"),
                "True" | "False" => one(PROCEDURE, "@word"),
                _ => vec![],
            };
            assert_eq!(attributed, want_attributed, "attributed {w} {host}");
            let trigger = diags(
                &src(&format!(
                    "        trigger OnOpenPage()\n        var\n            {w}: Integer;\n        begin\n        end;\n"
                )),
                w,
            );
            assert_eq!(trigger, one(BEGIN, "@word"), "trigger {w} {host}");
        }
    }
}
