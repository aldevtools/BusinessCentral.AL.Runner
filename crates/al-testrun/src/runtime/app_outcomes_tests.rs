//! Test support: the outcomes of a test codeunit of a program of several apps compiled from source.
use super::*;
use al_sema::AppSource;

/// The outcomes, and per test the ids of the assumptions it hit.
pub(super) type OutcomesAndHits = (Vec<(String, Outcome)>, Vec<(String, Vec<&'static str>)>);

/// The outcomes and the assumption ids each test hit, of the test codeunit `codeunit` of a program of several apps (id,
/// dependencies, source); an app whose id starts with `dep` is a dependency app (ADR-013), the others are under test.
pub(super) fn outcomes_and_hits_of_apps(apps: &[(&str, &[&str], &str)], codeunit: &str) -> OutcomesAndHits {
    outcomes_and_hits_with(apps, &[], codeunit, TestIsolation::Codeunit)
}

/// [`outcomes_and_hits_of_apps`], with the manifest publisher of the apps named in `publishers` (app id, publisher; the
/// others have none) and the test isolation `isolation`.
pub(super) fn outcomes_and_hits_with(
    apps: &[(&str, &[&str], &str)],
    publishers: &[(&str, &str)],
    codeunit: &str,
    isolation: TestIsolation,
) -> OutcomesAndHits {
    let (engine, l) = compiled_apps(apps, publishers);
    let oi = l.program.objects.iter().position(|o| o.name == codeunit).unwrap();
    let mut hits = Vec::new();
    let done = run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        isolation,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
        crate::host::RunClock::begin(),
    );
    let hits = hits
        .into_iter()
        .map(|(n, h)| (n, h.iter().map(|a| a.info().id).collect()))
        .collect();
    (done, hits)
}

/// The engine and the compile of a program of several apps (id, dependencies, source), with the manifest publisher of
/// the apps named in `publishers`; an app whose id starts with `dep` is a dependency app (ADR-013).
pub(super) fn compiled_apps(apps: &[(&str, &[&str], &str)], publishers: &[(&str, &str)]) -> (Arc<Engine>, Lowered) {
    let (ws, mut l) = al_sema::lower::compile_apps(
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
                if id.starts_with("dep") {
                    al_sema::AppKind::Dependency
                } else {
                    al_sema::AppKind::Workspace
                }
            })
            .collect(),
        &Default::default(),
    );
    for app in &mut std::sync::Arc::make_mut(&mut l.program).apps {
        if let Some((_, p)) = publishers.iter().find(|(id, _)| *id == app.id) {
            app.publisher = (*p).into();
        }
    }
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    (Engine::new(l.program.clone(), blocked), l)
}

/// The outcomes and the assumption ids each test hit, of the test codeunit `T` of app `main` on the install baseline of
/// the dependency app `dep`, as the baseline cache keeps it, with the tables it could not seed.
pub(super) fn outcomes_on_dependency_baseline(dep: &str, main: &str) -> OutcomesAndHits {
    // The install baseline's roots: the dependency app's install codeunit is compiled.
    let (ws, l) = al_sema::lower::compile_apps_with(
        vec![
            AppSource {
                id: "dep".into(),
                name: "dep".into(),
                dependencies: vec![],
                files: vec![("dep.al".into(), dep.into())],
            },
            AppSource {
                id: "main".into(),
                name: "main".into(),
                dependencies: vec!["dep".into()],
                files: vec![("main.al".into(), main.into())],
            },
        ],
        vec![al_sema::AppKind::Dependency, al_sema::AppKind::Workspace],
        &Default::default(),
        None,
        al_sema::check::Roots {
            baseline: true,
            ..Default::default()
        },
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let engine = Engine::new(l.program.clone(), Vec::new());
    // One clock for the baseline and the tests over it, as in a run.
    let clock = crate::host::RunClock::begin();
    let base = crate::baseline::run(&engine, &l, None, clock).expect("the dependency app has install code");
    // The baseline as the cache keeps it.
    let back = al_vm::Baseline::from_bytes(&base.baseline.to_bytes().expect("plain rows")).expect("read back");
    // The tests run on an engine that knows the tables the baseline could not seed, as `corpus.rs` gives them.
    let mut engine = Engine::new(l.program.clone(), Vec::new());
    std::sync::Arc::get_mut(&mut engine).expect("a fresh engine").unseeded = base.engine_unseeded();
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let mut hits = Vec::new();
    let done = run_codeunit_full(
        &engine,
        &l,
        oi,
        Some(&back),
        TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
        clock,
    );
    let hits = hits
        .into_iter()
        .map(|(n, h)| (n, h.iter().map(|a| a.info().id).collect()))
        .collect();
    (done, hits)
}
