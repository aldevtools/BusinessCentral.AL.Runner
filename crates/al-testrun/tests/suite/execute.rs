//! A scratch run (`al-runner serve`'s `execute`): the `OnRun` trigger of one codeunit per app runs, and the
//! text of its `Message` calls comes back with the result.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};

fn app(id: &str, files: &[(&str, &str)]) -> AppSource {
    AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: vec![],
        files: files.iter().map(|(p, t)| (p.to_string(), t.to_string())).collect(),
    }
}

fn execute(apps: Vec<AppSource>) -> CorpusRuntime {
    let kinds = vec![al_sema::AppKind::Workspace; apps.len()];
    let suites: Vec<SuiteRun> = (0..apps.len())
        .map(|a| SuiteRun {
            name: format!("app{a}"),
            app: Some(a),
            runner: None,
            driver: None,
        })
        .collect();
    CorpusRuntime::try_run_suites_with(
        apps,
        kinds,
        &[],
        &Default::default(),
        None,
        Default::default(),
        &suites,
        &RunOptions {
            execute: true,
            ..Default::default()
        },
    )
    .unwrap()
}

#[test]
fn the_on_run_trigger_runs_and_its_messages_come_back() {
    let rt = execute(vec![app(
        "a",
        &[(
            "a.al",
            "codeunit 50100 Scratch { trigger OnRun() begin Message('hi'); Message('%1 and %2', 1, 'two'); end; }",
        )],
    )]);
    let r = rt.results();
    assert_eq!(r.len(), 1, "{r:?}");
    assert_eq!((r[0].codeunit, r[0].test.as_str()), (50100, "OnRun"));
    assert_eq!(r[0].outcome, Outcome::Pass, "{:?}", r[0].outcome);
    assert_eq!(r[0].messages, vec!["hi".to_string(), "1 and two".to_string()]);
}

#[test]
fn an_error_in_on_run_is_a_failure_with_its_text() {
    let rt = execute(vec![app(
        "a",
        &[(
            "a.al",
            "codeunit 50100 Scratch { trigger OnRun() begin Message('before'); Error('boom %1', 7); end; }",
        )],
    )]);
    let r = rt.results();
    assert_eq!(r[0].outcome, Outcome::Fail("boom 7".into()), "{:?}", r[0].outcome);
    assert_eq!(r[0].messages, vec!["before".to_string()]);
}

#[test]
fn the_codeunit_with_the_lowest_id_that_is_not_a_test_is_the_one_that_runs() {
    let rt = execute(vec![app(
        "a",
        &[(
            "a.al",
            "codeunit 50001 FirstButATest { Subtype = Test; trigger OnRun() begin Message('test'); end; [Test] procedure T() begin end; }
codeunit 50003 NoOnRun { procedure P() begin end; }
codeunit 50009 Later { trigger OnRun() begin Message('later'); end; }
codeunit 50005 Earlier { trigger OnRun() begin Message('earlier'); end; }",
        )],
    )]);
    let r = rt.results();
    assert_eq!(r.len(), 1, "{r:?}");
    assert_eq!(
        (r[0].codeunit, r[0].messages.clone()),
        (50005, vec!["earlier".to_string()])
    );
}

#[test]
fn a_test_codeunit_runs_when_nothing_else_has_an_on_run() {
    let rt = execute(vec![app(
        "a",
        &[(
            "a.al",
            "codeunit 50001 OnlyATest { Subtype = Test; trigger OnRun() begin Message('test'); end; [Test] procedure T() begin end; }",
        )],
    )]);
    let r = rt.results();
    assert_eq!(r.len(), 1, "{r:?}");
    assert_eq!(r[0].codeunit, 50001);
}

#[test]
fn every_app_gives_a_result_and_an_app_without_on_run_gives_none() {
    let rt = execute(vec![
        app(
            "a",
            &[("a.al", "codeunit 50100 A { trigger OnRun() begin Message('a'); end; }")],
        ),
        app("b", &[("b.al", "codeunit 50200 B { procedure P() begin end; }")]),
        app(
            "c",
            &[("c.al", "codeunit 50300 C { trigger OnRun() begin Message('c'); end; }")],
        ),
    ]);
    let ids: Vec<u32> = rt.results().iter().map(|r| r.codeunit).collect();
    assert_eq!(ids, vec![50100, 50300]);
}

#[test]
fn a_message_in_a_test_run_is_still_refused_outside_a_test_method() {
    // The same call from a normal run (not a scratch run) keeps its refusal: nothing pins what BC does.
    let apps = vec![app(
        "a",
        &[(
            "a.al",
            "codeunit 50100 T { Subtype = Test; trigger OnRun() begin Message('x'); end; [Test] procedure P() begin end; }",
        )],
    )];
    let kinds = vec![al_sema::AppKind::Workspace];
    let rt = CorpusRuntime::try_run_suites_with(
        apps,
        kinds,
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    )
    .unwrap();
    let r = rt.results();
    assert!(
        matches!(&r[0].outcome, Outcome::NotRun(m) if m.contains("outside a test method")),
        "{:?}",
        r[0].outcome
    );
}

/// A scratch run runs the install code in each codeunit's session, and `--timings` says why: two apps, each with an
/// install codeunit and an `OnRun` trigger, run it twice (al-testrun `install_once.rs`).
#[test]
fn a_scratch_run_names_why_the_install_code_ran_for_each_codeunit() {
    let with_install = |id: &str, n: u32| {
        let src = format!(
            "codeunit {n} R{n} {{ trigger OnRun() begin Message('{id}'); end; }}
            codeunit {} I{n} {{ Subtype = Install; trigger OnInstallAppPerCompany() begin end; }}",
            n + 1
        );
        app(id, &[(&format!("{id}.al"), &src)])
    };
    let rt = execute(vec![with_install("a", 50100), with_install("c", 50300)]);
    assert_eq!(rt.phases.install_runs, 2);
    let lines = rt.phases.baseline_line().unwrap_or_default();
    assert!(
        lines.lines().any(|l| l
            == "install code: ran 2 times for the test codeunits; first reason noted that its run cannot be shared: a scratch run"),
        "{lines}"
    );
}
