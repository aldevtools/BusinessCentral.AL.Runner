//! `Database::"X"` or `Report::"X"` of an object no loaded app declares (al-runner issues 8101, 8099): an object of
//! another country's Base Application without `--country`, or of a dependency skipped as not found. The reason says so
//! instead of "is not lowered yet", which reads as a missing feature (a runner text, ADR-009).
use super::*;

fn reasons(src: &str) -> Vec<String> {
    let (_, l) = lower_files(&[("f.al", src)], false);
    l.unsupported
        .iter()
        .flatten()
        .flatten()
        .map(|r| r.to_string())
        .collect()
}

#[test]
fn an_object_literal_that_names_no_loaded_object_says_so() {
    for (literal, kind) in [
        ("Database::\"Delivery Reminder Header\"", "table"),
        ("Report::\"Vendor Payment List\"", "report"),
        ("Page::\"Missing Card\"", "page"),
        ("Codeunit::\"Missing Unit\"", "codeunit"),
        ("Query::\"Missing Query\"", "query"),
        ("XmlPort::\"Missing Port\"", "xmlport"),
    ] {
        let src = format!("codeunit 50100 C {{ procedure P(): Integer begin exit({literal}); end; }}");
        let r = reasons(&src);
        assert!(
            r.iter().any(
                |r| r.contains(&format!("`{literal}` names no {kind} of the loaded apps"))
                    && r.contains("--country")
                    && !r.contains("not lowered yet")
            ),
            "{literal}: {r:?}"
        );
    }
}

#[test]
fn an_object_literal_of_a_loaded_object_is_lowered() {
    let r = reasons(
        "table 50101 K { fields { field(1; A; Integer) { } } } codeunit 50100 C { procedure P(): Integer begin exit(Database::K); end; }",
    );
    assert!(r.is_empty(), "{r:?}");
}
