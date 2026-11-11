//! al-runner issue 6924: test codeunits runner 130450 runs whose tests can take numbers join the carry chain, which runs
//! one codeunit after the other (`chain_walk.rs`, always serial: ADR-070). A codeunit that left nothing for
//! the later ones (its tests' own sequences are rolled back with it, its OnRun changes nothing) does not hold them back,
//! while numbers taken from a sequence the install code made still reach the later codeunits, in id order.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};

const BASE: &str = r#"
codeunit 50221 SharedInstaller
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() begin NumberSequence.Insert('SHARED'); end;
}
"#;

/// Codeunits that take numbers only from a sequence of their own, each with an OnRun that changes nothing.
fn own(id: u32) -> String {
    format!(
        r#"
codeunit {id} "Own {id}"
{{
    Subtype = Test;
    trigger OnRun() begin end;
    [Test] procedure TakesOwn{id}()
    var I: Integer; N: BigInteger;
    begin
        NumberSequence.Insert('OWN');
        if NumberSequence.Next('OWN') <> 0 then Error('the sequence of an earlier codeunit was not rolled back');
    end;
}}
"#
    )
}

const SHARED: &str = r#"
codeunit 60224 "Takes Shared"
{
    Subtype = Test;
    [Test] procedure TakesTwoShared()
    begin
        if NumberSequence.Next('SHARED') <> 0 then Error('first');
        if NumberSequence.Next('SHARED') <> 1 then Error('second');
    end;
}
codeunit 60228 "Reads Shared"
{
    Subtype = Test;
    [Test] procedure ContinuesShared()
    var N: BigInteger;
    begin
        N := NumberSequence.Next('SHARED');
        if N <> 2 then Error('the numbers an earlier codeunit took did not carry: %1', N);
    end;
}
"#;

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    }
}

fn manifest(p: &str) -> Manifest {
    Manifest {
        publisher: p.into(),
        version: "1.0.0.0".into(),
        resources: vec![],
        include_source: false,
        target: String::new(),
        logo: None,
        description: None,
        layout_files: Vec::new(),
        id_ranges: Vec::new(),
        translations: false,
        package_data: false,
        package_data_files: Vec::new(),
        translation_files: Vec::new(),
        translation_source: None,
        runtime: None,
        app_json_keys: Vec::new(),
        symbols: vec![],
    }
}

fn run() -> CorpusRuntime {
    let mut main: String = [60220, 60221, 60222, 60223, 60225, 60226, 60227, 60229, 60230, 60231]
        .into_iter()
        .map(own)
        .collect();
    main.push_str(SHARED);
    CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], &main), app("base", &[], BASE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions {
            strict_order: true,
            ..Default::default()
        },
    )
    .expect("the run starts")
}

#[test]
fn chain_codeunits_that_leave_nothing_do_not_hold_back_the_later_ones_and_taken_numbers_still_carry() {
    let rt = run();
    for r in rt.results() {
        assert_eq!(r.outcome, Outcome::Pass, "{}", r.test);
    }
    assert_eq!(rt.results().len(), 12);
    let chain = &rt.phases.chain;
    assert_eq!(chain.members.len(), 12, "every codeunit can take numbers");
    let left: Vec<u32> = chain.members.iter().filter(|m| m.left).map(|m| m.id).collect();
    assert_eq!(
        left,
        vec![60224, 60228],
        "only the codeunits that took shared numbers left something"
    );
}
