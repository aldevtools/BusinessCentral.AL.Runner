//! Test support: the outcomes of a test codeunit of a program of several apps compiled from source.
use super::*;
use al_sema::AppSource;

/// The outcomes, and per test the ids of the assumptions it hit.
pub(super) type OutcomesAndHits = (Vec<(String, Outcome)>, Vec<(String, Vec<&'static str>)>);

/// The outcomes and the assumption ids each test hit, of the test codeunit `codeunit` of a program of several apps (id,
/// dependencies, source); the app with id `dep` is a dependency app (ADR-013), the others are under test.
pub(super) fn outcomes_and_hits_of_apps(apps: &[(&str, &[&str], &str)], codeunit: &str) -> OutcomesAndHits {
    let (ws, l) = al_sema::lower::compile_apps(
        apps.iter()
            .map(|(id, deps, src)| AppSource {
                id: (*id).into(),
                name: (*id).into(),
                dependencies: deps.iter().map(|d| (*d).to_string()).collect(),
                files: vec![(format!("{id}.al"), (*src).into())],
            })
            .collect(),
        apps.iter()
            .map(|(id, _, _)| {
                if *id == "dep" {
                    al_sema::AppKind::Dependency
                } else {
                    al_sema::AppKind::Workspace
                }
            })
            .collect(),
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let oi = l.program.objects.iter().position(|o| o.name == codeunit).unwrap();
    let mut hits = Vec::new();
    let done = run_codeunit_full(
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
    let hits = hits
        .into_iter()
        .map(|(n, h)| (n, h.iter().map(|a| a.info().id).collect()))
        .collect();
    (done, hits)
}
