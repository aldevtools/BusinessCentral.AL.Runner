//! ADR-066 amendment 2 (al-runner issue 6924): a test codeunit that reads SingleInstance state an earlier test codeunit
//! left runs, with the codeunits that wrote it, in one serial session in BC's order, and its tests are flagged. A
//! lazy cache that gives the same value either way, and a run without such a dependence, cost no second run.
//! BC runs all test codeunits in one session: codeunits/TestCuP7ASetter.al:24-58 against TestCuP7BReader.al:24-37.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};
use std::ops::Not;

const STATE: &str = r#"
codeunit 50451 "Lane State"
{
    SingleInstance = true;
    procedure SetFlag(V: Text) begin Flag := V; end;
    procedure GetFlag(): Text begin exit(Flag); end;
    var Flag: Text;
}
codeunit 50452 "Lane Cache"
{
    SingleInstance = true;
    procedure Get(): Integer
    begin
        if not Loaded then begin
            Value := 5;
            Loaded := true;
        end;
        exit(Value);
    end;
    procedure GetMoving(N: Integer): Integer
    begin
        if not Loaded then begin
            Value := N;
            Loaded := true;
        end;
        exit(Value);
    end;
    var
        Loaded: Boolean;
        Value: Integer;
}
codeunit 50453 "Lane Alone"
{
    SingleInstance = true;
    procedure Bump() begin Count += 1; end;
    procedure Get(): Integer begin exit(Count); end;
    var Count: Integer;
}
"#;

const DEPENDENT_PAIR: &str = r#"
codeunit 60451 "Lane Setter"
{
    Subtype = Test;
    [Test] procedure SetsTheFlag() var S: Codeunit "Lane State"; begin S.SetFlag('left'); end;
}
codeunit 60452 "Lane Reader"
{
    Subtype = Test;
    [Test] procedure ReadsTheFlag()
    var S: Codeunit "Lane State";
    begin
        if S.GetFlag() <> 'left' then Error('flag is %1', S.GetFlag());
    end;
}
codeunit 60453 "Lane Bystander"
{
    Subtype = Test;
    [Test] procedure UsesNothingShared() begin end;
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
        platform_internals_visible_to: Vec::new(),
        symbols: vec![],
    }
}

fn runtime_opts(src: &str, base: &str, opts: &RunOptions) -> CorpusRuntime {
    CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], src), app("base", &[], base)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        opts,
    )
    .expect("the run starts")
}

fn flagged(rt: &CorpusRuntime, test: &str) -> bool {
    rt.results()
        .iter()
        .find(|r| r.test == test)
        .unwrap_or_else(|| panic!("{test} ran"))
        .assumptions
        .is_empty()
        .not()
}

fn passed(rt: &CorpusRuntime, test: &str) -> bool {
    rt.results()
        .iter()
        .any(|r| r.test == test && r.outcome == Outcome::Pass)
}

/// ADR-066 amendment 8: the carry chain's walk keeps the SingleInstance state from member to
/// member of a BC session, as BC's one test-runner session does (P7: codeunits/TestCuP7BReader.al:24-37 against
/// TestCuP7ASetter.al:24-58). A chain member that reads what the member before it left passes in the walk itself:
/// no serial re-run, and its result is the run in BC's order.
#[test]
fn a_chain_member_sees_the_single_instance_state_the_member_before_it_left() {
    let src = r#"
codeunit 60700 "Chain SI Setter"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure ChainSetsTheFlag() var S: Codeunit "Lane State"; begin S.SetFlag('left'); end;
}
codeunit 60701 "Chain SI Reader"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure ChainReadsTheFlag()
    var S: Codeunit "Lane State";
    begin
        if S.GetFlag() <> 'left' then Error('flag is %1', S.GetFlag());
    end;
}
"#;
    let opts = RunOptions {
        strict_order: true,
        required_isolation: true,
        ..Default::default()
    };
    let rt = runtime_opts(src, STATE, &opts);
    assert!(passed(&rt, "ChainReadsTheFlag"), "{:?}", rt.results());
    let chain = &rt.phases.chain;
    assert_eq!((chain.singles_kept, chain.singles_read_carried), (2, 1), "{:?}", chain);
}

/// A carry chain member that reports what it found in the flag of "Lane State", then leaves `sets` in it and fails with
/// "saw [<what it found>]". SingleInstance state is not rolled back with the failure, so the failed test hands it on
/// (ADR-066 amendment 8, decision 2).
fn chain_reporter(id: u32, extra: &str, sets: &str) -> String {
    format!(
        r#"
codeunit {id} "Chain Reporter {id}"
{{
    Subtype = Test;
    {extra}
    [Test] procedure Reports{id}()
    var S: Codeunit "Lane State"; Saw: Text;
    begin
        Saw := S.GetFlag();
        S.SetFlag('{sets}');
        Error('saw [%1]', Saw);
    end;
}}
"#
    )
}

/// As [`chain_reporter`], but passes when it found `expect` (a failed Disabled codeunit before a codeunit that reads
/// a table it may have written leaves that codeunit not run, `corpus.rs`).
fn chain_checker(id: u32, extra: &str, expect: &str, sets: &str) -> String {
    chain_reporter(id, extra, sets).replace(
        "Error('saw [%1]', Saw);",
        &format!("if Saw <> '{expect}' then Error('saw [%1]', Saw);"),
    )
}

/// The failure text of test `test`.
fn failure(rt: &CorpusRuntime, test: &str) -> String {
    match rt.results().iter().find(|r| r.test == test).map(|r| &r.outcome) {
        Some(Outcome::Fail(m)) => m.clone(),
        other => panic!("{test}: {other:?}"),
    }
}

const DISABLED: &str = "RequiredTestIsolation = Disabled;";

/// ADR-066 amendment 8, decision 2: the walk hands the single instances from member to member of one BC session, and
/// a failed test hands its state on as on BC. The third member sees what the second left, the second what the first left.
#[test]
fn a_failed_chain_member_hands_its_single_instance_state_on() {
    let src = [
        chain_reporter(60710, DISABLED, "one"),
        chain_reporter(60711, DISABLED, "two"),
        chain_reporter(60712, DISABLED, "three"),
    ]
    .concat();
    let opts = RunOptions {
        strict_order: true,
        required_isolation: true,
        ..Default::default()
    };
    let rt = runtime_opts(&src, STATE, &opts);
    assert_eq!(failure(&rt, "Reports60710"), "saw []");
    assert_eq!(failure(&rt, "Reports60711"), "saw [one]");
    assert_eq!(failure(&rt, "Reports60712"), "saw [two]");
    for t in ["Reports60710", "Reports60711", "Reports60712"] {
        assert!(!flagged(&rt, t), "{t}");
    }
}

/// ADR-066 amendment 8, decision 2: a new BC session starts with fresh single instances. A Disabled codeunit runs on
/// the TestRunnerHub, a codeunit whose OnRun writes a table under runner 130450 (`Unit::carry`); both are chain
/// members of different BC sessions (amendment 6, `single_lane.rs` `streams`). Two of each, so each session is watched:
/// the second of a session sees what the first left, the first of a session sees fresh state.
#[test]
fn a_chain_member_of_another_hub_leg_starts_with_fresh_single_instances() {
    let on_run =
        |k: u32| format!("trigger OnRun() var R: Record \"Chain Leg Row\"; begin R.K := {k}; R.Insert(); end;");
    let src = format!(
        "table 60720 \"Chain Leg Row\" {{ fields {{ field(1; K; Integer) {{ }} }} keys {{ key(PK; K) {{ Clustered = true; }} }} }}\n{}{}{}{}",
        chain_checker(60721, DISABLED, "", "hub"),
        chain_reporter(60722, &on_run(1), "runner"),
        chain_checker(60723, DISABLED, "hub", "hub2"),
        chain_reporter(60724, &on_run(2), "runner2"),
    );
    let opts = RunOptions {
        strict_order: true,
        required_isolation: true,
        ..Default::default()
    };
    let rt = runtime_opts(&src, STATE, &opts);
    let members: Vec<u32> = rt.phases.chain.members.iter().map(|m| m.id).collect();
    for id in 60721..=60724 {
        assert!(members.contains(&id), "{id}: {members:?}");
    }
    assert!(
        passed(&rt, "Reports60721") && passed(&rt, "Reports60723"),
        "{:?}",
        rt.results()
    );
    assert_eq!(failure(&rt, "Reports60722"), "saw []");
    assert_eq!(failure(&rt, "Reports60724"), "saw [runner]");
}

/// ADR-066 amendment 8, decision 2 (amendment 6: a suite is a BC session of its own): the first member of the second
/// suite starts with fresh single instances, although the walk ran the first suite's members just before it.
#[test]
fn a_chain_member_of_the_next_suite_starts_with_fresh_single_instances() {
    let rt = CorpusRuntime::try_run_suites_with(
        vec![
            app(
                "first",
                &["base"],
                &[
                    chain_reporter(60730, DISABLED, "first"),
                    chain_reporter(60732, DISABLED, "first2"),
                ]
                .concat(),
            ),
            app(
                "second",
                &["base"],
                &[
                    chain_reporter(60731, DISABLED, "second"),
                    chain_reporter(60733, DISABLED, "second2"),
                ]
                .concat(),
            ),
            app("base", &[], STATE),
        ],
        vec![
            al_sema::AppKind::Workspace,
            al_sema::AppKind::Workspace,
            al_sema::AppKind::Dependency,
        ],
        &[manifest("Me"), manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[
            SuiteRun {
                name: "first".into(),
                app: Some(0),
                runner: None,
                driver: None,
            },
            SuiteRun {
                name: "second".into(),
                app: Some(1),
                runner: None,
                driver: None,
            },
        ],
        &RunOptions {
            strict_order: true,
            required_isolation: true,
            ..Default::default()
        },
    )
    .expect("the run starts");
    assert_eq!(failure(&rt, "Reports60730"), "saw []");
    assert_eq!(failure(&rt, "Reports60732"), "saw [first]");
    assert_eq!(failure(&rt, "Reports60731"), "saw []");
    assert_eq!(failure(&rt, "Reports60733"), "saw [second]");
}

/// ADR-070 (review of 2026-11-10): the single instances move on past a test refused while it ran, as what ran before
/// the stop ran on BC too; the next member sees what the refusing member left.
#[test]
fn the_chain_member_after_a_test_refused_while_it_ran_sees_the_single_instances_it_left() {
    let refusing = r#"
codeunit 60741 "Chain Refusing Member"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure SetsThenIsRefused1() var S: Codeunit "Lane State"; begin S.SetFlag('refusing'); end;
    [Test] procedure SetsThenIsRefused2() begin Sleep(-1); end;
}
"#;
    let src = [
        chain_reporter(60740, DISABLED, "first"),
        refusing.to_string(),
        chain_reporter(60742, DISABLED, "after"),
        chain_reporter(60743, DISABLED, "last"),
    ]
    .concat();
    let opts = RunOptions {
        strict_order: true,
        required_isolation: true,
        ..Default::default()
    };
    let rt = runtime_opts(&src, STATE, &opts);
    assert!(
        rt.results()
            .iter()
            .any(|r| r.test == "SetsThenIsRefused2" && matches!(r.outcome, Outcome::NotRun(_))),
        "{:?}",
        rt.results()
    );
    assert_eq!(failure(&rt, "Reports60740"), "saw []");
    assert_eq!(failure(&rt, "Reports60742"), "saw [refusing]");
    assert_eq!(failure(&rt, "Reports60743"), "saw [after]");
}

// ADR-058 amendment 1: order dependence through SingleInstance state, in stability mode.

const CHAIN_PAIR: &str = r#"
codeunit 60710 "Chain Fresh Setter"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure FreshSetsTheFlag() var S: Codeunit "Lane State"; begin S.SetFlag('left'); end;
}
codeunit 60711 "Chain Fresh Reader"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure FreshReadsTheFlag()
    var S: Codeunit "Lane State";
    begin
        if S.GetFlag() <> 'left' then Error('flag is [%1]', S.GetFlag());
    end;
}
codeunit 60712 "Chain Fresh Bystander"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure FreshUsesNothing() begin end;
}
"#;

fn strict_opts() -> RunOptions {
    RunOptions {
        required_isolation: true,
        strict_order: true,
        ..Default::default()
    }
}

/// ADR-070 decision 7: nothing runs again; the walk names the global and its writer, and the `fresh` variant of a
/// stability run (the default isolated run) gives the reader its fresh outcome.
#[test]
fn a_chain_member_that_reads_a_carried_value_names_the_global_and_its_writer() {
    let rt = runtime_opts(CHAIN_PAIR, STATE, &strict_opts());
    // The base result stays BC's order.
    assert!(passed(&rt, "FreshReadsTheFlag"), "{:?}", rt.results());
    let carried: Vec<_> = rt.single_carried().collect();
    assert_eq!(carried.len(), 1, "only the reader is a suspect: {carried:?}");
    let c = carried[0];
    assert_eq!(
        (c.codeunit, c.in_chain, c.isolation.as_str()),
        (60711, true, "Disabled")
    );
    assert_eq!(c.reads.len(), 1, "{c:?}");
    let r = &c.reads[0];
    assert_eq!(
        (r.single_id, r.single_name.as_str(), r.global.as_str()),
        (50451, "Lane State", "Flag")
    );
    assert_eq!(r.last_writer, Some((60710, "Chain Fresh Setter".to_string())));
    let fresh = al_testrun::stability::Perturb {
        single_fresh: true,
        ..Default::default()
    };
    let rts = variants_of(CHAIN_PAIR, &[fresh]);
    assert!(
        matches!(outcome_of(&rts[0], "FreshReadsTheFlag"), Outcome::Fail(m) if m.contains("flag is []")),
        "{:?}",
        rts[0].results()
    );
}

#[test]
fn a_plain_dependent_codeunit_is_a_walk_member_and_is_listed() {
    let rt = runtime_opts(DEPENDENT_PAIR, STATE, &strict_opts());
    assert!(passed(&rt, "ReadsTheFlag"), "{:?}", rt.results());
    let carried: Vec<_> = rt.single_carried().collect();
    assert_eq!(carried.len(), 1, "{carried:?}");
    let c = carried[0];
    assert!(
        c.in_chain,
        "ADR-070: every codeunit is a member of the strict walk: {c:?}"
    );
    assert_eq!(c.reads[0].global, "Flag");
    assert!(
        c.reads[0]
            .last_writer
            .as_ref()
            .is_some_and(|w| w.1.contains("Lane Setter")),
        "{c:?}"
    );
}

fn variants_of(src: &str, perturbs: &[al_testrun::stability::Perturb]) -> Vec<CorpusRuntime> {
    CorpusRuntime::try_run_variants(
        vec![app("main", &["base"], src), app("base", &[], STATE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions {
            strict_order: true,
            required_isolation: true,
            ..Default::default()
        },
        perturbs,
        |_| {},
    )
    .expect("the run starts")
}

fn outcome_of<'a>(rt: &'a CorpusRuntime, test: &str) -> &'a Outcome {
    &rt.results().iter().find(|r| r.test == test).expect("ran").outcome
}

/// ADR-070 decision 7: the `fresh` variant is a plain default (isolated) run, the base variant a plain run in BC's order.
#[test]
fn the_fresh_variant_is_a_plain_default_run_and_the_base_variant_a_strict_run() {
    let base = al_testrun::stability::Perturb::default();
    let fresh = al_testrun::stability::Perturb {
        single_fresh: true,
        ..Default::default()
    };
    let src = format!("{CHAIN_PAIR}{DEPENDENT_PAIR}");
    let rts = variants_of(&src, &[base, fresh]);
    assert_eq!(rts.len(), 2);
    for t in [
        "FreshReadsTheFlag",
        "ReadsTheFlag",
        "FreshSetsTheFlag",
        "FreshUsesNothing",
    ] {
        assert_eq!(outcome_of(&rts[0], t), &Outcome::Pass, "{t} in the base variant");
    }
    assert!(matches!(outcome_of(&rts[1], "FreshReadsTheFlag"), Outcome::Fail(_)));
    assert!(matches!(outcome_of(&rts[1], "ReadsTheFlag"), Outcome::Fail(_)));
    assert_eq!(outcome_of(&rts[1], "FreshUsesNothing"), &Outcome::Pass);
    let plain = RunOptions {
        required_isolation: true,
        ..Default::default()
    };
    assert_eq!(outcomes(&rts[1]), outcomes(&runtime_opts(&src, STATE, &plain)));
}

/// Codeunits outside the carry chain and in it whose outcome or message depends on the SingleInstance state that
/// earlier codeunits left: some pass only in BC's order, some fail only in it, one fails either way with a message
/// that names what it read.
const DIFFERENTIAL: &str = r#"
codeunit 60720 "Diff Setter"
{
    Subtype = Test;
    [Test] procedure DiffSetsTheFlag() var S: Codeunit "Lane State"; begin S.SetFlag('left'); end;
}
codeunit 60721 "Diff Needs Flag"
{
    Subtype = Test;
    [Test] procedure DiffNeedsTheFlag() var S: Codeunit "Lane State"; begin if S.GetFlag() <> 'left' then Error('no flag [%1]', S.GetFlag()); end;
}
codeunit 60722 "Diff Hates Flag"
{
    Subtype = Test;
    [Test] procedure DiffFailsAfterTheSetter() var S: Codeunit "Lane State"; begin if S.GetFlag() = 'left' then Error('saw [%1]', S.GetFlag()); end;
}
codeunit 60723 "Diff Always Fails"
{
    Subtype = Test;
    [Test] procedure DiffFailsEitherWay() var S: Codeunit "Lane State"; begin Error('flag [%1]', S.GetFlag()); end;
}
codeunit 60724 "Diff Lenient"
{
    Subtype = Test;
    [Test] procedure DiffReadsAnyFlag() var S: Codeunit "Lane State"; begin if StrLen(S.GetFlag()) > 10 then Error('long'); end;
}
codeunit 60725 "Diff Chain Setter"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure DiffChainSets() var S: Codeunit "Lane State"; begin S.SetFlag('chain'); end;
}
codeunit 60726 "Diff Chain Needs"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure DiffChainNeeds() var S: Codeunit "Lane State"; begin if S.GetFlag() <> 'chain' then Error('chain flag [%1]', S.GetFlag()); end;
}
codeunit 60727 "Diff Chain Hates"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure DiffChainFailsAfter() var S: Codeunit "Lane State"; begin if S.GetFlag() = 'chain' then Error('chain saw [%1]', S.GetFlag()); end;
}
"#;

fn outcomes(rt: &CorpusRuntime) -> Vec<(u32, String, Outcome)> {
    let mut v: Vec<_> = rt
        .results()
        .iter()
        .map(|r| (r.codeunit, r.test.clone(), r.outcome.clone()))
        .collect();
    v.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    v
}

/// ADR-058 amendment 1, decision 3 (differential conformance): the base variant of a stability run with
/// `isolation.singleInstance` gives every test the outcome and message of a normal `--strict-order` run.
#[test]
fn the_base_variant_with_the_single_instance_check_matches_a_strict_order_run() {
    let base = al_testrun::stability::Perturb::default();
    let fresh = al_testrun::stability::Perturb {
        single_fresh: true,
        ..Default::default()
    };
    let rts = variants_of(DIFFERENTIAL, &[base, fresh]);
    let strict = runtime_opts(
        DIFFERENTIAL,
        STATE,
        &RunOptions {
            required_isolation: true,
            strict_order: true,
            ..Default::default()
        },
    );
    let want = outcomes(&strict);
    assert_eq!(want.len(), 8, "{want:?}");
    // The suite is not trivial: in BC's order some tests fail, and the fresh variant changes some outcomes.
    assert!(want.iter().any(|w| matches!(w.2, Outcome::Fail(_))), "{want:?}");
    assert_ne!(outcomes(&rts[1]), want, "the fresh variant differs");
    assert_eq!(outcomes(&rts[0]), want);
}

/// Issue 8593 shape: every codeunit a chain member (RequiredTestIsolation = Disabled). `Lane Carry Refused` is refused
/// while it runs (the order of `æ` is not measured, issue 8195), so what it left is not known; `Lane Carry Counter`
/// counts the rows after it and reads the flag `Lane Carry Setter` left.
const CARRY_AFTER_REFUSED: &str = r#"
table 60810 "Lane Carry Row" { fields { field(1; Name; Text[30]) { } } keys { key(PK; Name) { } } }
codeunit 60810 "Lane Carry Writer"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure WritesCarryNames() var R: Record "Lane Carry Row";
    begin R.Name := 'Alpha'; R.Insert(); R.Name := 'ærø'; R.Insert(); R.Name := 'Bravo'; R.Insert(); end;
}
codeunit 60811 "Lane Carry Setter"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure SetsCarryFlag() var S: Codeunit "Lane State"; begin S.SetFlag('left'); end;
}
codeunit 60812 "Lane Carry Refused"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure RefusedLastName() var S: Codeunit "Lane State"; R: Record "Lane Carry Row";
    begin
        if S.GetFlag() = '' then;
        R.FindLast();
        Error('last=%1', R.Name);
    end;
}
codeunit 60813 "Lane Carry Counter"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure CountsCarriedRows() var S: Codeunit "Lane State"; R: Record "Lane Carry Row";
    begin
        if S.GetFlag() = '' then;
        Error('count=%1 flag=%2', R.Count(), S.GetFlag());
    end;
}
"#;

/// Al-runner issue 8593: a chain member after a member that was refused while it ran reads what that member left, which
/// is not known (`run.rs` `carry_disabled`): it is refused in the `--strict-order` walk as in the default walk, not run from the install baseline.
#[test]
fn a_lane_member_after_a_refused_member_is_refused_as_in_the_walk() {
    let opts = RunOptions {
        required_isolation: true,
        strict_order: true,
        test_suites: Some(std::sync::Arc::new(
            al_testrun::test_suites::TestSuites::parse("S: 60810 60811 60812 60813").expect("a valid suites file"),
        )),
        ..Default::default()
    };
    let rt = runtime_opts(CARRY_AFTER_REFUSED, STATE, &opts);
    let counter = rt
        .results()
        .iter()
        .find(|r| r.test == "CountsCarriedRows")
        .map(|r| r.outcome.clone());
    assert!(
        matches!(&counter, Some(Outcome::NotRun(why)) if why.contains("Lane Carry Refused") && why.contains("not pinned")),
        "{counter:?}"
    );
}

/// Issue 8594 shape: a chain member (`Lane Outside Writer`, RequiredTestIsolation = Disabled) and, outside the chain,
/// a codeunit that sets the flag and two that count the writer's rows and read the flag.
const OUTSIDE_READERS: &str = r#"
table 60820 "Lane Outside Row" { fields { field(1; Name; Text[30]) { } } keys { key(PK; Name) { } } }
codeunit 60820 "Lane Outside Writer"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure WritesOutsideNames() var R: Record "Lane Outside Row";
    begin R.Name := 'Alpha'; R.Insert(); R.Name := 'Bravo'; R.Insert(); R.Name := 'Charlie'; R.Insert(); end;
}
codeunit 60821 "Lane Outside Setter"
{
    Subtype = Test;
    [Test] procedure SetsOutsideFlag() var S: Codeunit "Lane State"; begin S.SetFlag('left'); end;
}
codeunit 60822 "Lane Outside Reader A"
{
    Subtype = Test;
    [Test] procedure CountsOutsideRowsA() var S: Codeunit "Lane State"; R: Record "Lane Outside Row";
    begin
        if S.GetFlag() = '' then;
        Error('count=%1 flag=%2', R.Count(), S.GetFlag());
    end;
}
codeunit 60823 "Lane Outside Reader B"
{
    Subtype = Test;
    [Test] procedure CountsOutsideRowsB() var S: Codeunit "Lane State"; R: Record "Lane Outside Row";
    begin
        if S.GetFlag() = '' then;
        Error('count=%1 flag=%2', R.Count(), S.GetFlag());
    end;
}
"#;

/// Al-runner issue 8594: a codeunit outside the chain that reads a table a member wrote sees the member's rows in the
/// `--strict-order` walk as in the re-run after the parallel part (QuCommitIsolation.al:711-760 QU_D11_NextCodeunitSees),
/// with the flag the walk hands on.
#[test]
fn a_lane_reader_outside_the_chain_sees_the_rows_a_member_left() {
    let opts = RunOptions {
        required_isolation: true,
        strict_order: true,
        ..Default::default()
    };
    let rt = runtime_opts(OUTSIDE_READERS, STATE, &opts);
    for t in ["CountsOutsideRowsA", "CountsOutsideRowsB"] {
        let o = rt.results().iter().find(|r| r.test == t).map(|r| r.outcome.clone());
        assert!(
            matches!(&o, Some(Outcome::Fail(m)) if m == "count=3 flag=left"),
            "{t}: {o:?}"
        );
    }
}

/// A member on the TestRunnerHub (`Lane Hub Writer`), then, outside the chain, a codeunit that sets the flag and one with
/// handlers that counts the writer's rows and reads the flag. BC's CI runs a codeunit with handlers through another runner
/// than the hub (QuCommitIsolation.al:11-13), so whether it runs before or after the member is not pinned.
const HANDLERS_AFTER_HUB: &str = r#"
table 60830 "Lane Hub Row" { fields { field(1; Name; Text[30]) { } } keys { key(PK; Name) { } } }
codeunit 60830 "Lane Hub Writer"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure WritesHubNames() var R: Record "Lane Hub Row";
    begin R.Name := 'Alpha'; R.Insert(); R.Name := 'Bravo'; R.Insert(); end;
}
codeunit 60831 "Lane Hub Setter"
{
    Subtype = Test;
    [Test] procedure SetsHubFlag() var S: Codeunit "Lane State"; begin S.SetFlag('left'); end;
}
codeunit 60832 "Lane Hub Handler Reader"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('LaneHubMessage')]
    procedure CountsWithHandler() var S: Codeunit "Lane State"; R: Record "Lane Hub Row";
    begin
        Message('counting');
        if S.GetFlag() = '' then;
        Error('count=%1 flag=%2', R.Count(), S.GetFlag());
    end;
    [MessageHandler] procedure LaneHubMessage(Msg: Text[1024]) begin end;
}
"#;

/// A codeunit with handlers that reads a table a hub member wrote is refused: under `--strict-order` the walk marks it
/// (`strict_walk.rs` `Unpinned`, `carry_after.rs` `place_unpinned`), as main's default run refuses it
/// (`After::outside_refusal`), even when a plain codeunit runs between the hub member and it.
#[test]
fn a_lane_reader_with_handlers_after_a_hub_member_stays_refused() {
    let opts = RunOptions {
        required_isolation: true,
        strict_order: true,
        ..Default::default()
    };
    let rt = runtime_opts(HANDLERS_AFTER_HUB, STATE, &opts);
    let o = rt
        .results()
        .iter()
        .find(|r| r.test == "CountsWithHandler")
        .map(|r| r.outcome.clone());
    assert!(
        matches!(&o, Some(Outcome::NotRun(why)) if why.contains("which has handlers, before or after it is not pinned")),
        "{o:?}"
    );
}

/// A chain member (`Lane Storage Writer`) changes isolated storage and sets the flag; a later member reads the flag and
/// the storage.
const MEMBER_AFTER_STORAGE: &str = r#"
codeunit 60840 "Lane Storage Writer"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure SetsStorageAndFlag() var S: Codeunit "Lane State";
    begin IsolatedStorage.Set('lane-key', 'kept', DataScope::Module); S.SetFlag('left'); end;
}
codeunit 60841 "Lane Storage Member Reader"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure MemberReadsStorage() var S: Codeunit "Lane State"; V: Text;
    begin
        if S.GetFlag() = '' then;
        if not IsolatedStorage.Get('lane-key', DataScope::Module, V) then V := '';
        Error('storage=%1 flag=%2', V, S.GetFlag());
    end;
}
"#;

/// A chain member whose reach touches the isolated storage an earlier member changed is refused in the `--strict-order`
/// walk as in the default walk (`carry_after.rs` `After::refusal`, `Carried::storage_why`), not run from what that member left.
#[test]
fn a_lane_member_after_a_storage_carry_that_reaches_the_storage_is_refused() {
    let opts = RunOptions {
        required_isolation: true,
        strict_order: true,
        test_suites: Some(std::sync::Arc::new(
            al_testrun::test_suites::TestSuites::parse("S: 60840 60841").expect("a valid suites file"),
        )),
        ..Default::default()
    };
    let rt = runtime_opts(MEMBER_AFTER_STORAGE, STATE, &opts);
    let o = rt
        .results()
        .iter()
        .find(|r| r.test == "MemberReadsStorage")
        .map(|r| r.outcome.clone());
    assert!(
        matches!(&o, Some(Outcome::NotRun(why)) if why.contains("changed isolated storage or a number sequence")),
        "{o:?}"
    );
}

/// A chain member (`Lane Storage Row Writer`) changes isolated storage and writes a row; outside the chain a codeunit
/// sets the flag and one, whose reach does not touch the storage, counts the rows and reads the flag.
const OUTSIDE_AFTER_STORAGE: &str = r#"
table 60850 "Lane Storage Row" { fields { field(1; Name; Text[30]) { } } keys { key(PK; Name) { } } }
codeunit 60850 "Lane Storage Row Writer"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure SetsStorageAndRow() var R: Record "Lane Storage Row";
    begin IsolatedStorage.Set('lane-key', 'kept', DataScope::Module); R.Name := 'Alpha'; R.Insert(); end;
}
codeunit 60851 "Lane Storage Setter"
{
    Subtype = Test;
    [Test] procedure SetsStorageFlag() var S: Codeunit "Lane State"; begin S.SetFlag('left'); end;
}
codeunit 60852 "Lane Storage Row Reader"
{
    Subtype = Test;
    [Test] procedure CountsRowsNotStorage() var S: Codeunit "Lane State"; R: Record "Lane Storage Row";
    begin
        if S.GetFlag() = '' then;
        Error('count=%1 flag=%2', R.Count(), S.GetFlag());
    end;
}
"#;

/// A codeunit outside the chain whose reach does not touch the isolated storage a member changed, and that reads the
/// member's rows, runs in the `--strict-order` walk from what the member left (QuCommitIsolation.al:711-760
/// QU_D11_NextCodeunitSees), as after the parallel part.
#[test]
fn a_lane_reader_outside_the_chain_after_a_storage_carry_runs_again_from_it() {
    let opts = RunOptions {
        required_isolation: true,
        strict_order: true,
        ..Default::default()
    };
    let rt = runtime_opts(OUTSIDE_AFTER_STORAGE, STATE, &opts);
    let o = rt
        .results()
        .iter()
        .find(|r| r.test == "CountsRowsNotStorage")
        .map(|r| r.outcome.clone());
    assert!(
        matches!(&o, Some(Outcome::Fail(m)) if m == "count=1 flag=left"),
        "{o:?}"
    );
}

/// A codeunit whose OnRun writes rows (a member on the leg of runner 130450, not the hub), then, outside the chain, a
/// codeunit that sets the flag and one with handlers that counts the rows and reads the flag.
const HANDLERS_OTHER_LEG: &str = r#"
table 60860 "Lane Leg Row" { fields { field(1; Name; Text[30]) { } } keys { key(PK; Name) { } } }
codeunit 60860 "Lane Leg Writer"
{
    Subtype = Test;
    trigger OnRun() var R: Record "Lane Leg Row";
    begin R.Name := 'Alpha'; R.Insert(); R.Name := 'Bravo'; R.Insert(); end;
    [Test] procedure LegWriterTest() begin end;
}
codeunit 60861 "Lane Leg Setter"
{
    Subtype = Test;
    [Test] procedure SetsLegFlag() var S: Codeunit "Lane State"; begin S.SetFlag('left'); end;
}
codeunit 60862 "Lane Leg Handler Reader"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('LaneLegMessage')]
    procedure CountsLegRowsWithHandler() var S: Codeunit "Lane State"; R: Record "Lane Leg Row";
    begin
        Message('counting');
        if S.GetFlag() = '' then;
        Error('count=%1 flag=%2', R.Count(), S.GetFlag());
    end;
    [MessageHandler] procedure LaneLegMessage(Msg: Text[1024]) begin end;
}
"#;

/// The refusal of a codeunit with handlers holds only after a member on the TestRunnerHub (`carry_after.rs`
/// `handlers_unpinned`, `strict_walk.rs` `Unpinned`): after a member on the other leg it runs from what the member left.
#[test]
fn a_lane_reader_with_handlers_on_the_other_leg_runs_again() {
    let opts = RunOptions {
        required_isolation: true,
        strict_order: true,
        ..Default::default()
    };
    let rt = runtime_opts(HANDLERS_OTHER_LEG, STATE, &opts);
    let o = rt
        .results()
        .iter()
        .find(|r| r.test == "CountsLegRowsWithHandler")
        .map(|r| r.outcome.clone());
    assert!(
        matches!(&o, Some(Outcome::Fail(m)) if m == "count=2 flag=left"),
        "{o:?}"
    );
}
