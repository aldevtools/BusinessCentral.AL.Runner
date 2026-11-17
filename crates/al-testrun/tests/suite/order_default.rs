//! ADR-066 amendment 12 (owner, 2026-11-09): by default every test codeunit starts with fresh SingleInstance state and
//! from the install baseline's NumberSequence values; nothing runs again in BC's order. A failed test that read a
//! SingleInstance global an earlier codeunit would have set on BC gets a hint (the SingleInstance codeunit, its app,
//! the global, the writer and the flag), and the tests that take numbers after another codeunit took some are listed.
//! `--strict-order` (`RunOptions::strict_order`) restores the carry, the chain and the re-run in BC's
//! order. BC runs all test codeunits in one session: codeunits/TestCuP7ASetter.al:24-58 against TestCuP7BReader.al:24-37.

use al_builtins::assumptions::Assumption;
use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun, TestResult};

const BASE: &str = r#"
codeunit 50471 "Order State"
{
    SingleInstance = true;
    procedure SetFlag(V: Text) begin Flag := V; end;
    procedure GetFlag(): Text begin exit(Flag); end;
    var Flag: Text;
}
codeunit 50472 OrderInstaller
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() begin NumberSequence.Insert('ORDERSEQ', 1, 1); end;
}
"#;

/// The P7 shape: the reader passes only after the setter ran in the same session.
const PAIR: &str = r#"
codeunit 60471 "Order Setter"
{
    Subtype = Test;
    [Test] procedure SetsTheFlag() var S: Codeunit "Order State"; begin S.SetFlag('left'); end;
}
codeunit 60472 "Order Reader"
{
    Subtype = Test;
    [Test] procedure ReadsTheFlag()
    var S: Codeunit "Order State";
    begin
        if S.GetFlag() <> 'left' then Error('flag is %1', S.GetFlag());
    end;
}
"#;

/// A reader that passes whatever the flag holds.
const LENIENT: &str = r#"
codeunit 60473 "Order Setter Two"
{
    Subtype = Test;
    [Test] procedure SetsItToo() var S: Codeunit "Order State"; begin S.SetFlag('left'); end;
}
codeunit 60474 "Order Lenient Reader"
{
    Subtype = Test;
    [Test] procedure ReadsAnyFlag()
    var S: Codeunit "Order State";
    begin
        if StrLen(S.GetFlag()) > 10 then Error('flag is %1', S.GetFlag());
    end;
}
"#;

/// Two codeunits that take a number of the install code's sequence: on BC the second gets 2 (runner 130450 does not
/// give back numbers a test took: test-framework/onrun-storage/Y35OnRunStorage.al:30-31).
const NUMBERS: &str = r#"
codeunit 60475 "Order First Taker"
{
    Subtype = Test;
    [Test] procedure TakesOne() var N: BigInteger; begin N := NumberSequence.Next('ORDERSEQ'); if N <> 1 then Error('got %1', N); end;
}
codeunit 60476 "Order Second Taker"
{
    Subtype = Test;
    [Test] procedure TakesTwo() var N: BigInteger; begin N := NumberSequence.Next('ORDERSEQ'); if N <> 2 then Error('got %1', N); end;
}
codeunit 60477 "Order No Taker"
{
    Subtype = Test;
    [Test] procedure TakesNothing() begin end;
}
"#;

fn run(src: &str, opts: &RunOptions) -> CorpusRuntime {
    CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], src), app("base", &[], BASE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Base Publisher")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        opts,
    )
    .expect("the run starts")
}

fn strict() -> RunOptions {
    RunOptions {
        strict_order: true,
        ..Default::default()
    }
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
        platform_internals_visible_to: Vec::new(),
        symbols: vec![],
    }
}

fn result<'a>(rt: &'a CorpusRuntime, test: &str) -> &'a TestResult {
    rt.results()
        .iter()
        .find(|r| r.test == test)
        .unwrap_or_else(|| panic!("{test} ran: {:?}", rt.results()))
}

fn order_flagged(r: &TestResult) -> bool {
    r.assumptions.contains(&Assumption::SingleInstanceOrder)
}

/// Test plan 1 and 4: by default the reader runs with fresh state and fails, nothing runs again, and its result names
/// the SingleInstance codeunit, its app, the global, the codeunit that would have set it and the flag.
#[test]
fn by_default_a_reader_of_state_an_earlier_codeunit_left_fails_with_a_hint_and_nothing_runs_again() {
    let rt = run(PAIR, &RunOptions::default());
    let reader = result(&rt, "ReadsTheFlag");
    assert!(matches!(reader.outcome, Outcome::Fail(_)), "{reader:?}");
    assert!(!order_flagged(reader), "the default claims no BC order: {reader:?}");
    let hint = reader.order_hint.as_ref().expect("a hint");
    let text = hint.text();
    for part in [
        "50471",
        "Order State",
        "base",
        "Base Publisher",
        "Flag",
        "60471",
        "Order Setter",
        "--strict-order",
    ] {
        assert!(text.contains(part), "{part} in {text}");
    }
    assert_eq!(hint.reads.len(), 1, "{hint:?}");
    assert_eq!(hint.reads[0].global, "Flag");
    assert_eq!(hint.reads[0].last_writer, Some((60471, "Order Setter".to_string())));
    let setter = result(&rt, "SetsTheFlag");
    assert!(setter.outcome == Outcome::Pass && setter.order_hint.is_none());
    let lane = &rt.phases.chain.single_lane;
    assert_eq!((lane.rerun, lane.lane.len()), (0, 0), "{lane:?}");
    assert!(rt.phases.chain.members.is_empty() && rt.phases.chain.reruns.is_empty());
}

/// Test plan 1: with `--strict-order` the reader passes in BC's order, as amendment 8 pins (flagged, no hint).
#[test]
fn with_strict_order_the_reader_runs_again_in_bcs_order_and_passes() {
    let rt = run(PAIR, &strict());
    let reader = result(&rt, "ReadsTheFlag");
    assert_eq!(reader.outcome, Outcome::Pass, "{reader:?}");
    assert!(order_flagged(reader) && reader.order_hint.is_none());
    assert_eq!(rt.phases.chain.single_lane.rerun, 1);
}

/// Test plan 2 and owner answer 1: a reader that passes with fresh state gets no hint and is not flagged.
#[test]
fn a_reader_that_passes_with_fresh_state_gets_no_hint() {
    let rt = run(LENIENT, &RunOptions::default());
    let reader = result(&rt, "ReadsAnyFlag");
    assert_eq!(reader.outcome, Outcome::Pass);
    assert!(
        reader.order_hint.is_none() && reader.assumptions.is_empty(),
        "{reader:?}"
    );
}

/// Test plan 3: by default each codeunit starts from the install baseline's sequence value (no carry chain), and the
/// codeunit that takes numbers after another taker is listed.
#[test]
fn by_default_each_codeunit_takes_numbers_from_the_baseline_and_the_later_taker_is_listed() {
    let rt = run(NUMBERS, &RunOptions::default());
    assert_eq!(result(&rt, "TakesOne").outcome, Outcome::Pass);
    assert!(
        matches!(&result(&rt, "TakesTwo").outcome, Outcome::Fail(m) if m.contains("got 1")),
        "{:?}",
        result(&rt, "TakesTwo")
    );
    let chain = &rt.phases.chain;
    assert!(
        chain.members.is_empty(),
        "no carry chain for numbers: {:?}",
        chain.members
    );
    let listed: Vec<u32> = chain.numbers_order.iter().map(|n| n.codeunit).collect();
    assert_eq!(listed, vec![60476], "{:?}", chain.numbers_order);
    assert_eq!(chain.numbers_order[0].after, (60475, "Order First Taker".to_string()));
    assert!(
        chain
            .lines()
            .iter()
            .any(|l| l.contains("Order Second Taker") && l.contains("--strict-order"))
    );
}

/// Test plan 3: with `--strict-order` the numbers chain as on BC.
#[test]
fn with_strict_order_the_numbers_chain_as_on_bc() {
    let rt = run(NUMBERS, &strict());
    assert_eq!(result(&rt, "TakesOne").outcome, Outcome::Pass);
    assert_eq!(
        result(&rt, "TakesTwo").outcome,
        Outcome::Pass,
        "{:?}",
        result(&rt, "TakesTwo")
    );
    assert!(rt.phases.chain.numbers_order.is_empty());
    assert!(!rt.phases.chain.members.is_empty());
}

/// The corpus's order probes (`metrics/strict-order-files.txt`): a default run takes the outcomes of the listed
/// codeunits from a `--strict-order` run of them alone.
#[test]
fn a_default_run_takes_the_outcomes_of_listed_codeunits_from_a_strict_run() {
    let mut rt = run(PAIR, &RunOptions::default());
    assert!(matches!(result(&rt, "ReadsTheFlag").outcome, Outcome::Fail(_)));
    let only = RunOptions {
        strict_order: true,
        only: Some(vec![(0, 60471), (0, 60472)]),
        ..Default::default()
    };
    let strict = run(PAIR, &only);
    let listed: std::collections::BTreeSet<(String, u32)> = [(String::new(), 60471), (String::new(), 60472)].into();
    rt.adopt_cases(&strict, &listed);
    let case = |name: &str, codeunit| al_testrun::CaseId {
        codeunit,
        name: name.into(),
    };
    assert_eq!(rt.outcome_in("", &case("ReadsTheFlag", 60472)), Outcome::Pass);
    assert!(
        rt.assumptions_in("", &case("ReadsTheFlag", 60472))
            .contains(&Assumption::SingleInstanceOrder)
    );
    assert_eq!(rt.outcome_in("", &case("SetsTheFlag", 60471)), Outcome::Pass);
}

/// The same over one compile: `RunOptions::strict_after` runs the listed codeunits again inside the run, so the pass
/// compiles once and the listed cases take the strict outcomes.
#[test]
fn strict_after_runs_the_listed_codeunits_again_over_the_same_compile() {
    let opts = RunOptions {
        strict_after: Some(al_testrun::runtime::StrictAfter {
            only: vec![(0, 60471), (0, 60472)],
            listed: [(String::new(), 60471), (String::new(), 60472)].into(),
        }),
        ..Default::default()
    };
    let rt = run(PAIR, &opts);
    let case = |name: &str, codeunit| al_testrun::CaseId {
        codeunit,
        name: name.into(),
    };
    assert_eq!(rt.outcome_in("", &case("ReadsTheFlag", 60472)), Outcome::Pass);
    assert_eq!(rt.phases.compiles, 1);
}
