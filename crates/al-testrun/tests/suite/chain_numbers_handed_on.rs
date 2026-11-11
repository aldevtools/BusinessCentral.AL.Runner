//! al-runner issue 6924: chain codeunits whose tests take numbers from a sequence of the install code hand those numbers
//! on, so each one starts where the one before it stopped. The walk is serial (`chain_walk.rs`, ADR-070), so every
//! outcome is the serial one. Here each codeunit checks the exact numbers it gets, and one takes more numbers when its
//! first number is odd.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};

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
        translation_source: None,
        runtime: None,
        app_json_keys: Vec::new(),
        symbols: vec![],
    }
}

/// Runs the chain codeunits with `opts` and checks the outcome is the serial one: every test passes (each checks the
/// exact numbers the serial run gives it), and every codeunit takes numbers and hands them on.
fn run_chain(opts: &RunOptions) -> CorpusRuntime {
    let rt = CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], &source()), app("base", &[], BASE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        opts,
    )
    .expect("the run starts");
    for r in rt.results() {
        assert_eq!(r.outcome, Outcome::Pass, "{}", r.test);
    }
    assert_eq!(rt.results().len(), IDS.len());
    let chain = &rt.phases.chain;
    assert_eq!(chain.members.len(), IDS.len(), "every codeunit takes numbers");
    assert!(chain.members.iter().all(|m| m.left), "every codeunit hands on numbers");
    rt
}

/// The walk is serial: every codeunit gets the exact numbers the serial run gives it, and `--timings` says so.
#[test]
fn chain_codeunits_that_hand_on_numbers_give_the_serial_outcome() {
    let rt = run_chain(&RunOptions {
        strict_order: true,
        ..Default::default()
    });
    let lines = rt.phases.chain.lines();
    assert!(
        lines[0].starts_with(&format!(
            "carry chain: {} of {} test codeunits are in it, all ran serially",
            IDS.len(),
            IDS.len()
        )),
        "{lines:?}"
    );
}
