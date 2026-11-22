//! Al-runner issue 8670, through the neighbor app tests/neighbors/8670-bg-worker-filter-group1 (copies of corpus PR 1507,
//! tests/al-language/pages/background-tasks/BgTaskWorkerFilterGroup1.al, and of corpus PR 1514, BgTaskWorkerFilterGroup1Shapes.al):
//! the test session's filter in filter group 1 after a worker took it back, read in a later test, and the worker's calls no
//! cell measures (`al-vm` group1_sessions.rs, group1_sessions/worker.rs).

use al_builtins::assumptions::Assumption::{self, BgTaskWorkerFilterGroupOne};
use al_test_cli::report::Outcome;
use al_testrun::isolation::TestIsolation;

fn outcomes(codeunit: &str) -> Vec<(String, Outcome, Vec<Assumption>)> {
    let src = [
        include_str!("../../../../tests/neighbors/8670-bg-worker-filter-group1/src/Probe.al"),
        include_str!("../../../../tests/neighbors/8670-bg-worker-filter-group1/src/Shapes.al"),
        include_str!("../../../../tests/neighbors/8670-bg-worker-filter-group1/src/Neighbors.al"),
        include_str!("../../../../tests/neighbors/8670-bg-worker-filter-group1/src/Assert.al"),
    ]
    .join("\n");
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
        TestIsolation::Codeunit,
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

/// A worker that takes back the test session's filter in filter group 1 (SetRange(Tag) in group 1) does so in its own
/// session: the test session keeps its filter, in the same test and in the next one (BgTaskWorkerFilterGroup1Shapes.al:657,
/// :683). Main took the filter back for the test session. Both tests read the table through the assumption, the next one too.
#[test]
fn issue_8670_worker_clears_group1_flags_the_later_reads() {
    let o = outcomes("G1S Worker Clears Tests");
    assert_eq!(o.len(), 2);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.contains(&BgTaskWorkerFilterGroupOne), "{name}: {marks:?}");
    }
}

/// The worker's calls no cell measures on a table whose test session entry it runs without refuse with main's wording:
/// another form of the entry, another setter with a read past Count, a read, key or filter of the worker past the cells
/// (Neighbors.al of the neighbor app). Main refused each of them too.
#[test]
fn issue_8670_worker_unmeasured_calls_refuse() {
    for (codeunit, call) in [
        ("G1N Three Values", "Count"),
        ("G1N Text Filter", "Count"),
        ("G1N Range Filter", "Count"),
        ("G1N Worker Find Last", "Find"),
        ("G1N Worker Key", "SetCurrentKey"),
        ("G1N Worker Temp FindFirst", "FindFirst"),
        ("G1N Worker SetFilter", "SetFilter"),
        ("G1N Worker Other Field", "SetRange"),
        ("G1N Temp Setter FindFirst", "FindFirst"),
        ("G1N Worker Next Blank", "Next"),
    ] {
        let o = outcomes(codeunit);
        assert_eq!(o.len(), 1, "{codeunit}");
        let reason = format!(
            "{call} on a record of a table with a filter in filter group 1 (the global group) that another record set"
        );
        assert!(
            matches!(&o[0].1, Outcome::NotRun(why) if why.starts_with(&reason)),
            "{codeunit}: {:?}",
            o[0].1
        );
    }
}
