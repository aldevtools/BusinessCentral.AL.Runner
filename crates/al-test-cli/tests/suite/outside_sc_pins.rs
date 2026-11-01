//! The corpus writer's probes of al-runner issues 5843 and 5844 (corpus branch al-runtime-rs-staging-pub-sc @2150ff97, green
//! on BC 27.5, 28.5 and 29.0), run here until the corpus bump lands: copies of the files in fixtures/outside-sc, with a
//! stand-in for the Assert codeunit. Every test of them must pass. The probe of issue 5842 runs in platform_tables.rs, which
//! loads the System app. Delete this file and the fixtures when the bump lands.

use al_test_cli::report::Outcome;

fn all_pass(file: &str, codeunit: &str) {
    let src = format!("{}\n{}", file, include_str!("../fixtures/outside-sc/Assert.al"));
    let o = al_testkit::outcomes_in(&src, codeunit);
    assert!(!o.is_empty());
    let bad: Vec<_> = o.iter().filter(|(_, r)| *r != Outcome::Pass).collect();
    assert!(bad.is_empty(), "{bad:#?}");
}

#[test]
fn lookup_global_record_field_pins_pass() {
    all_pass(
        include_str!("../fixtures/outside-sc/ScLookupGlobalRecordField.al"),
        "LT SC Lookup Global Rec Tests",
    );
}

#[test]
fn drilldown_marked_only_pins_pass() {
    let file = include_str!("../fixtures/outside-sc/ScDrillDownMarkedOnly.al");
    let name = file
        .lines()
        .find_map(|l| l.strip_prefix("codeunit 89016 \"").and_then(|r| r.split('"').next()))
        .expect("codeunit 89016");
    all_pass(file, name);
}
