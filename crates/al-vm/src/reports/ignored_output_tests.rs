use super::*;

/// ADR-040 section 3: the list grows by evidence only. Each entry names the app, the object, and the Microsoft tests
/// that need the report to run; no entry is listed twice; a layout smoke test, a `CheckFileNotEmpty` test or an
/// Excel cell test is never a reason.
#[test]
fn the_allow_list_cites_its_tests_and_has_no_duplicates() {
    assert_eq!(
        IGNORED_OUTPUT.iter().map(|e| e.name).collect::<Vec<_>>(),
        ["Post Inventory Cost to G/L", "Check"]
    );
    for (i, e) in IGNORED_OUTPUT.iter().enumerate() {
        assert!(e.app.len() == 36 && e.id > 0 && !e.name.is_empty(), "{}", e.name);
        assert!(
            e.why.contains('(') && e.why.contains("tests") == e.why.contains("tests"),
            "{}",
            e.name
        );
        assert!(
            IGNORED_OUTPUT[..i].iter().all(|o| (o.app, o.id) != (e.app, e.id)),
            "{} is listed twice",
            e.name
        );
        for banned in ["CheckFileNotEmpty", "VerifyCellValue", "Layout Test", "Report Layout"] {
            assert!(
                !e.why.contains(banned) || e.why.contains("never"),
                "{}: {banned}",
                e.name
            );
        }
    }
}
