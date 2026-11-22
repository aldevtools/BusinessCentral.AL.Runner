//! `@` alone with no pattern keeps only the blank values when the database reads the rows, and every row of a temporary record
//! (al-builtins `filter_at_sign`, assumption `at-sign-alone-database-blank`): BQ Tests 9b and 9c of open corpus pull request
//! 1495 (tests/al-language/pages/background-tasks/BgTaskQueryCache.al), and the neighbors. The AL is the neighbor app's
//! (tests/neighbors/at-sign-alone-database), one copy. Delete this file when the pin is in the corpus.

use al_builtins::assumptions::Assumption;
use al_test_cli::report::Outcome;

const PROBE: &str = include_str!("../../../../tests/neighbors/at-sign-alone-database/src/Probe.al");
const NEIGHBORS: &str = include_str!("../../../../tests/neighbors/at-sign-alone-database/src/Neighbors.al");

fn outcomes(src: &str, codeunit: &str) -> Vec<(String, Outcome, Vec<Assumption>)> {
    let src = format!("{src}\n{}", include_str!("../fixtures/outside-sc/Assert.al"));
    let l = al_testkit::compile_apps(vec![al_testkit::app(&src)]);
    let engine = al_vm::Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == codeunit)
        .expect("the codeunit");
    let mut hits = Vec::new();
    let outcomes = al_testrun::runtime::run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        al_testrun::isolation::TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
    );
    assert!(!outcomes.is_empty());
    outcomes
        .into_iter()
        .map(|(n, o)| {
            let marks = hits
                .iter()
                .find(|(h, _)| *h == n)
                .map(|(_, m)| m.clone())
                .unwrap_or_default();
            (n, o, marks)
        })
        .collect()
}

/// The cells of BQ Tests 9b (corpus PR 1495, BgTaskQueryCache.al:5637-5831): @ alone keeps the empty values when the database
/// reads the rows, whatever the field's type (a FlowFilter of either type applied by CalcFields to a source field of either
/// type :5753, :5767; the source's own fields :5780; a lookup FlowField of a host in the database :5809), and every value of a
/// temporary record (:5780, :5809). All flagged.
#[test]
fn at_sign_alone_bq9b_cells() {
    let o = outcomes(PROBE, "BQ Tests 9b");
    assert_eq!(o.len(), 4);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(
            marks.contains(&Assumption::AtSignAloneDatabaseBlank),
            "{name}: {marks:?}"
        );
    }
}

/// The cells of BQ Tests 9c (corpus PR 1495, BgTaskQueryCache.al:6291-6433): a query reads the database, so @ alone on a Code
/// and a Text column, by SetFilter and by a DataItemTableFilter, keeps the empty values. All flagged.
#[test]
fn at_sign_alone_bq9c_cells() {
    let o = outcomes(PROBE, "BQ Tests 9c");
    assert_eq!(o.len(), 4);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(
            marks.contains(&Assumption::AtSignAloneDatabaseBlank),
            "{name}: {marks:?}"
        );
    }
}

/// `<>@` alone on a Code field of a record in the database: BC keeps the rows whose value is not empty (a local BC 29.1
/// container); not measured by a cell, so it stays refused, as on main.
#[test]
fn at_sign_alone_negated_code_field_stays_refused() {
    let src = format!("{PROBE}\n{NEIGHBORS}");
    let o = outcomes(&src, "BQ At Neighbor Tests");
    let [(_, r, _)] = o.as_slice() else { panic!("{o:?}") };
    assert!(matches!(r, Outcome::NotRun(w) if w.contains("`<>@`")), "{r:?}");
}
