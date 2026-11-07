//! al-runner issue 6924: chain codeunits whose tests take numbers from a sequence of the install code hand those numbers
//! on, so each one starts where the one before it stopped. Helper threads run them ahead from a predicted start: the
//! chain's start moved on by the numbers the codeunits before it are predicted to take (`chain_walk.rs`). The walk keeps
//! such a run only when the actual start is that predicted start, so every outcome is the serial one. Here each
//! codeunit checks the exact numbers it gets, and one takes more numbers when its first number is odd, so a prediction
//! made from the chain's start is wrong for it.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SerialWhy, SuiteRun};

const BASE: &str = r#"
codeunit 50241 SpecInstaller
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() begin NumberSequence.Insert('SHARED'); end;
}
"#;

/// The codeunit ids, in chain order.
const IDS: [u32; 10] = [60240, 60241, 60242, 60243, 60244, 60245, 60246, 60247, 60248, 60249];
/// The codeunit (index into `IDS`) whose count of numbers depends on the number it gets first.
const ODD: usize = 4;

/// The test codeunits, each checking the numbers the serial run gives it.
fn source() -> String {
    let mut next: i64 = 0;
    let mut out = String::new();
    for (i, id) in IDS.iter().enumerate() {
        let mut body = String::new();
        if i == ODD {
            let first = next;
            body.push_str(&format!(
                "N := NumberSequence.Next('SHARED'); if N <> {first} then Error('codeunit {id}: expected {first}, got %1', N);\n\
                 if N mod 2 = 1 then begin N := NumberSequence.Next('SHARED'); N := NumberSequence.Next('SHARED'); end;\n"
            ));
            next += if first % 2 == 1 { 3 } else { 1 };
        } else {
            for _ in 0..(i % 3 + 1) {
                body.push_str(&format!(
                    "N := NumberSequence.Next('SHARED'); if N <> {next} then Error('codeunit {id}: expected {next}, got %1', N);\n"
                ));
                next += 1;
            }
        }
        out.push_str(&format!(
            r#"
codeunit {id} "Spec {id}"
{{
    Subtype = Test;
    [Test] procedure Takes{id}()
    var I: Integer; N: BigInteger; W: BigInteger;
    begin
        // Some work, so that the helper threads have time to run ahead of the walk.
        for I := 1 to 200000 do W += I;
        {body}
    end;
}}
"#
        ));
    }
    out
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
fn chain_codeunits_that_hand_on_numbers_run_ahead_from_a_predicted_start_with_the_serial_outcome() {
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
    assert_eq!(rt.results().len(), IDS.len());
    let chain = &rt.phases.chain;
    assert_eq!(chain.members.len(), IDS.len(), "every codeunit takes numbers");
    assert!(chain.members.iter().all(|m| m.left), "every codeunit hands on numbers");
    if workers() > 2 {
        assert!(
            chain.members.iter().any(|m| m.ahead),
            "no chain codeunit ran ahead: {:?}",
            chain.members.iter().map(|m| (m.id, m.ahead)).collect::<Vec<_>>()
        );
    }
}

/// `--timings` says why each chain codeunit that ran on the walk's thread did not run ahead: one cause per serial
/// codeunit, the first from the chain's start (which the helpers do not run) among them.
#[test]
fn each_chain_codeunit_that_ran_serially_has_a_cause() {
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
    let chain = &rt.phases.chain;
    let serial: Vec<u32> = chain.members.iter().filter(|m| !m.ahead).map(|m| m.id).collect();
    let caused: Vec<u32> = chain.not_ahead.iter().map(|m| m.id).collect();
    assert_eq!(caused, serial, "{:?}", chain.not_ahead);
    if workers() > 2 {
        assert_eq!(
            chain.not_ahead.first().map(|m| (m.id, m.why)),
            Some((chain.members[0].id, SerialWhy::Closed)),
            "{:?}",
            chain.not_ahead
        );
    }
    let lines = chain.lines();
    assert!(
        lines.iter().any(|l| l.starts_with("serial carry chain by cause: ")),
        "{lines:?}"
    );
}
