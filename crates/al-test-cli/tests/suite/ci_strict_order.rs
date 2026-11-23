//! ADR-070 decision 5: the nightly run of the whole corpus in BC's order checks the lists (never `--no-ratchet`, which
//! makes it exit 0 whatever it found), and a pull request that adds a line to a strict list fails.

use std::path::Path;

fn workflow() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows/ci.yml"))
        .expect("ci.yml")
}

fn step<'a>(ci: &'a str, name: &str) -> &'a str {
    let start = ci
        .find(&format!("name: {name}"))
        .unwrap_or_else(|| panic!("no step `{name}` in ci.yml"));
    let rest = &ci[start..];
    let end = rest[1..].find("\n      - name: ").map_or(rest.len(), |i| i + 1);
    &rest[..end]
}

#[test]
fn the_whole_corpus_in_bcs_order_checks_the_lists() {
    let ci = workflow();
    let s = step(&ci, "Corpus in BC's order (whole corpus with --strict-order)");
    assert!(s.contains("AL_STRICT_ORDER_ALL=1 ./target/release/al-test-cli"), "{s}");
    assert!(!s.contains("--no-ratchet"), "the step must check the lists:\n{s}");
    assert!(!s.contains("|| true") && !s.contains("continue-on-error"), "{s}");
}

#[test]
fn a_line_added_to_a_strict_list_fails_the_pull_request() {
    let ci = workflow();
    let s = step(&ci, "Not-passing lists against the base branch");
    let at = s
        .find(r#"[ "$status" = 3 ]"#)
        .unwrap_or_else(|| panic!("exit 3 of --list-changes is not handled:\n{s}"));
    assert!(s[at..].lines().take(3).any(|l| l.trim() == "exit 1"), "{s}");
}
