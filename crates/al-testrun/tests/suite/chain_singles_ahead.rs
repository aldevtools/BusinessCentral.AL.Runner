//! ADR-066 amendment 8: the carry chain's walk keeps the SingleInstance state from member to member, so it takes a run
//! ahead (fresh single instances, another thread) only when that run read and wrote no watched global. In a chain whose
//! members all use a SingleInstance global, every run ahead is dropped and the member runs again on the walk's thread.
//! Such runs only cost: the helper threads hold their own copy of the install baseline and run a session each next to
//! the walk's (the edge gate measured +10 to +20 % peak memory on Microsoft's SCM and ERM suites). The walk stops
//! running a suite ahead once it dropped more such runs than it took, as it does for wrong predictions (`MISSES`).

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SerialWhy, SuiteRun};

const BASE: &str = r#"
codeunit 50261 SinglesAheadInstaller
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() begin NumberSequence.Insert('SHARED'); end;
}
codeunit 50262 "Singles Ahead Count"
{
    SingleInstance = true;
    procedure Bump() begin Count += 1; end;
    var Count: Integer;
}
"#;

/// Chain members: each takes a number of the install code's sequence (so it is in the carry chain and hands the number
/// on) and reads and writes the SingleInstance count (so no run ahead of it can be taken). Each passes in any order.
const MEMBERS: u32 = 40;

fn source() -> String {
    (0..MEMBERS)
        .map(|i| {
            let id = 60260 + i;
            format!(
                r#"
codeunit {id} "Singles Ahead {id}"
{{
    Subtype = Test;
    [Test] procedure Bumps{id}()
    var I: Integer; N: BigInteger; W: BigInteger; S: Codeunit "Singles Ahead Count";
    begin
        // Some work, so that the helper threads have time to run ahead of the walk.
        for I := 1 to 50000 do W += I;
        N := NumberSequence.Next('SHARED');
        S.Bump();
    end;
}}
"#
            )
        })
        .collect()
}

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
        runtime: None,
        app_json_keys: Vec::new(),
        symbols: vec![],
    }
}

/// The test threads the run uses (`AL_TEST_THREADS`, else one per core).
fn workers() -> usize {
    std::env::var("AL_TEST_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, |n| n.get()))
}

#[test]
fn a_chain_whose_runs_ahead_read_single_instance_state_stops_running_ahead() {
    let rt = CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], &source()), app("base", &[], BASE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    )
    .expect("the run starts");
    for r in rt.results() {
        assert_eq!(r.outcome, Outcome::Pass, "{}", r.test);
    }
    let chain = &rt.phases.chain;
    assert_eq!(chain.members.len(), MEMBERS as usize, "every codeunit takes numbers");
    assert_eq!(chain.singles_kept, MEMBERS as usize, "{:?}", chain);
    // No run ahead can be taken: each read and wrote the count.
    assert!(chain.members.iter().all(|m| !m.ahead), "{:?}", chain.members);
    if workers() > 2 {
        let dropped = chain
            .not_ahead
            .iter()
            .filter(|m| m.why == SerialWhy::NotRebuilt)
            .count();
        assert!(
            chain.not_ahead.iter().any(|m| m.why == SerialWhy::GaveUp),
            "the helpers ran every member ahead ({dropped} runs dropped): {:?}",
            chain.not_ahead.iter().map(|m| (m.id, m.why)).collect::<Vec<_>>()
        );
    }
}
