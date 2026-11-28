//! ADR-066 amendment 12 (owner, 2026-11-09): by default every test codeunit starts with fresh SingleInstance state and
//! from the install baseline's NumberSequence values; nothing runs again in BC's order. A failed test that read a
//! SingleInstance global an earlier codeunit would have set on BC gets a hint (the SingleInstance codeunit, its app,
//! the global, the writer and the flag); a failed test that took numbers from a sequence an earlier codeunit of its
//! session also took from gets a numbers hint (ADR-070 decision 3, which dropped amendment 12's static numbers list).
//! `--strict-order` (`RunOptions::strict_order`) restores the carry, the chain and the re-run in BC's
//! order. BC runs all test codeunits in one session: codeunits/TestCuP7ASetter.al:24-58 against TestCuP7BReader.al:24-37.

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
    !r.assumptions.is_empty()
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
    assert!(rt.phases.chain.members.is_empty() && rt.phases.chain.reruns.is_empty());
}

/// Test plan 1: with `--strict-order` the reader passes in BC's order (ADR-070: clean, no hint; P7:
/// codeunits/TestCuP7BReader.al:24-37 against TestCuP7ASetter.al:24-58).
#[test]
fn with_strict_order_the_reader_runs_in_bcs_order_and_passes() {
    let rt = run(PAIR, &strict());
    let reader = result(&rt, "ReadsTheFlag");
    assert_eq!(reader.outcome, Outcome::Pass, "{reader:?}");
    assert!(!order_flagged(reader) && reader.order_hint.is_none(), "{reader:?}");
}

/// Test plan 2 and owner answer 1: a reader that passes with fresh state gets no hint and is not flagged. ADR-070
/// decision 4: it keeps the globals it read (the JUnit property `singleInstanceCarried` and the summary line; the
/// console and JSON show no hint on a pass), and nothing else.
#[test]
fn a_reader_that_passes_with_fresh_state_gets_no_hint() {
    let rt = run(LENIENT, &RunOptions::default());
    let reader = result(&rt, "ReadsAnyFlag");
    assert_eq!(reader.outcome, Outcome::Pass);
    let hint = reader.order_hint.as_ref().expect("decision 4: the reads");
    assert!(
        hint.numbers.is_empty() && hint.reads.len() == 1 && reader.assumptions.is_empty(),
        "{reader:?}"
    );
}

/// The numbers hint of a test (ADR-070 decision 3): the sequences and the earlier taker of each.
fn numbers_hint(rt: &CorpusRuntime, test: &str) -> Vec<(String, u32)> {
    let hint = result(rt, test).order_hint.as_ref();
    hint.map(|h| h.numbers.iter().map(|n| (n.sequence.clone(), n.after.0)).collect())
        .unwrap_or_default()
}

/// Test plan 3: by default each codeunit starts from the install baseline's sequence value (no carry chain), and the
/// failed test that took numbers after another codeunit took some gets the numbers hint (ADR-070 decision 3).
#[test]
fn by_default_each_codeunit_takes_numbers_from_the_baseline_and_the_later_taker_gets_a_hint() {
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
    assert_eq!(numbers_hint(&rt, "TakesTwo"), vec![("ORDERSEQ".to_string(), 60475)]);
    let hint = result(&rt, "TakesTwo").order_hint.as_ref().unwrap();
    assert_eq!(hint.numbers[0].after, (60475, "Order First Taker".to_string()));
    assert!(hint.text().contains("--strict-order"), "{}", hint.text());
    assert!(result(&rt, "TakesOne").order_hint.is_none());
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
    assert!(result(&rt, "TakesTwo").order_hint.is_none());
    assert!(!rt.phases.chain.members.is_empty());
}

/// The NUMBERS shape with codeunits that only read the sequence: one before the first taker, one after it.
const READERS: &str = r#"
codeunit 60470 "Order Early Reader"
{
    Subtype = Test;
    [Test] procedure ReadsEarly() begin if NumberSequence.Current('ORDERSEQ') <> 1 then Error('early'); end;
}
codeunit 60478 "Order Current Reader"
{
    Subtype = Test;
    [Test] procedure ReadsCurrent() var N: BigInteger; begin N := NumberSequence.Current('ORDERSEQ'); if N <> 2 then Error('current %1', N); end;
}
"#;

fn with_readers() -> String {
    format!("{NUMBERS}{READERS}")
}

/// Readers of the sequence after the takers that the tests' static reach does not show (review of al-runner PR 8241,
/// probe 8152 codeunits 51015, 51017, 51018): one in a message handler, one in a table event behind a RecordRef insert, and
/// a codeunit that reads nothing but runs a codeunit by a variable id. `Order Current Reader` gets a test that reads nothing.
const HIDDEN_READERS: &str = r#"
codeunit 60479 "Order Handler Reader"
{
    Subtype = Test;
    var Seen: BigInteger;
    [Test] [HandlerFunctions('Msg')] procedure ReadsInHandler() begin Message('x'); if Seen <> 2 then Error('handler %1', Seen); end;
    [MessageHandler] procedure Msg(T: Text[1024]) begin Seen := NumberSequence.Current('ORDERSEQ'); end;
}
table 60480 "Order Ref Row" { fields { field(1; K; Code[10]) { } field(2; Seen; BigInteger) { } } keys { key(PK; K) { } } }
codeunit 60481 "Order Ref Sub"
{
    [EventSubscriber(ObjectType::Table, Database::"Order Ref Row", OnBeforeInsertEvent, '', false, false)]
    local procedure OnIns(var Rec: Record "Order Ref Row") begin Rec.Seen := NumberSequence.Current('ORDERSEQ'); end;
}
codeunit 60482 "Order RecRef Reader"
{
    Subtype = Test;
    [Test] procedure ReadsThroughRecRef() var R: RecordRef; F: FieldRef; begin R.Open(60480); F := R.Field(1); F.Value := 'A'; R.Insert(); end;
}
codeunit 60483 "Order Unknown Reach"
{
    Subtype = Test;
    [Test] procedure RunsById() var Id: Integer; begin Id := Codeunit::"Order Noop"; Codeunit.Run(Id); end;
}
codeunit 60484 "Order Noop" { trigger OnRun() begin end; }
"#;

fn with_hidden_readers() -> String {
    let readers = READERS.replace(
        "[Test] procedure ReadsCurrent()",
        "[Test] procedure ReadsNothing() begin end;\n    [Test] procedure ReadsCurrent()",
    );
    format!("{NUMBERS}{readers}{HIDDEN_READERS}")
}

/// ADR-070 decision 3: the numbers hint goes to a failed test that took numbers. A test that only reads the sequence
/// after the takers fails without one (the hint is for takers); the later taker gets it after the first.
#[test]
fn by_default_only_a_later_taker_gets_the_numbers_hint_and_a_reader_does_not() {
    let rt = run(&with_readers(), &RunOptions::default());
    assert!(
        matches!(&result(&rt, "ReadsCurrent").outcome, Outcome::Fail(m) if m.contains("current 1")),
        "{:?}",
        result(&rt, "ReadsCurrent")
    );
    assert!(numbers_hint(&rt, "ReadsCurrent").is_empty());
    assert_eq!(numbers_hint(&rt, "TakesTwo"), vec![("ORDERSEQ".to_string(), 60475)]);
}

/// al-runner issue 8152: under `--strict-order` with TestIsolation = Function (`--isolation test`) what a later codeunit
/// reads of the numbers an earlier codeunit's tests took is not measured: the NumberSequence call is refused, from any
/// code (a handler, a table event behind a RecordRef insert). A codeunit that reads before every taker, a test that reads
/// nothing, one whose reach is not known and the first taker run; under Codeunit isolation the reader keeps main's result.
#[test]
fn strict_order_under_function_isolation_refuses_a_later_reader_of_the_numbers() {
    let opts = RunOptions {
        strict_order: true,
        isolation: Some(al_testrun::isolation::TestIsolation::Function),
        ..Default::default()
    };
    let rt = run(&with_hidden_readers(), &opts);
    for test in ["ReadsCurrent", "TakesTwo", "ReadsInHandler", "ReadsThroughRecRef"] {
        assert!(
            matches!(&result(&rt, test).outcome, Outcome::NotRun(m) if m.contains("TestIsolation = Function")),
            "{test}: {:?}",
            result(&rt, test)
        );
    }
    for test in ["ReadsEarly", "TakesOne", "TakesNothing", "ReadsNothing", "RunsById"] {
        assert!(
            !matches!(&result(&rt, test).outcome, Outcome::NotRun(_)),
            "{test}: {:?}",
            result(&rt, test)
        );
    }
    let rt = run(&with_readers(), &strict());
    assert_eq!(
        result(&rt, "ReadsCurrent").outcome,
        Outcome::Pass,
        "{:?}",
        result(&rt, "ReadsCurrent")
    );
    assert_eq!(result(&rt, "TakesTwo").outcome, Outcome::Pass);
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
    assert!(rt.assumptions_in("", &case("ReadsTheFlag", 60472)).is_empty());
    assert_eq!(rt.outcome_in("", &case("SetsTheFlag", 60471)), Outcome::Pass);
    // Al-runner issue 8943: the default-order outcome stays readable after the strict one replaced it.
    assert!(matches!(
        rt.default_order_recorded(None, &case("ReadsTheFlag", 60472)),
        Some(Outcome::Fail(_))
    ));
    assert_eq!(
        rt.default_order_recorded(None, &case("SetsTheFlag", 60471)),
        Some(Outcome::Pass)
    );
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

/// al-runner issue 8247: under `--strict-order` with Codeunit isolation a later codeunit reads the numbers an earlier
/// codeunit took (Y35OnRunStorage.al:30-31) also in a message handler of its test and in a table event behind a RecordRef
/// insert: the codeunit joins the chain whichever of its routines reads the sequence.
#[test]
fn strict_order_chains_the_numbers_to_a_handler_and_a_recordref_reader() {
    let rt = run(&with_hidden_readers(), &strict());
    for test in ["ReadsCurrent", "TakesTwo", "ReadsInHandler", "ReadsThroughRecRef"] {
        assert_eq!(
            result(&rt, test).outcome,
            Outcome::Pass,
            "{test}: {:?}",
            result(&rt, test)
        );
    }
}

/// Review of al-runner PR 8363 (probe roots): a codeunit that takes numbers in a message handler, and one that reads them in
/// a confirm handler, join the chain as the takers and readers of a test body do: under `--strict-order` the handler taker
/// draws 3 after the two takers and the confirm reader sees it. By default the handler taker fails with the numbers hint
/// (it took numbers in its handler after the takers, ADR-070 decision 3); the confirm reader only reads, so it gets none.
const HANDLER_TAKERS: &str = r#"
codeunit 60485 "Order Handler Taker"
{
    Subtype = Test;
    var Seen: BigInteger;
    [Test] [HandlerFunctions('Msg')] procedure TakesInHandler() begin Message('x'); if Seen <> 3 then Error('handler taker %1', Seen); end;
    [MessageHandler] procedure Msg(T: Text[1024]) begin Seen := NumberSequence.Next('ORDERSEQ'); end;
}
codeunit 60486 "Order Confirm Reader"
{
    Subtype = Test;
    var Seen: BigInteger;
    [Test] [HandlerFunctions('Conf')] procedure ReadsInConfirm() begin if Confirm('q') then; if Seen <> 3 then Error('confirm %1', Seen); end;
    [ConfirmHandler] procedure Conf(Q: Text[1024]; var Reply: Boolean) begin Seen := NumberSequence.Current('ORDERSEQ'); Reply := true; end;
}
codeunit 60487 "Order Names A Plain Routine"
{
    Subtype = Test;
    [Test] [HandlerFunctions('Plain')] procedure NamesAPlainRoutine() begin end;
    procedure Plain() begin if NumberSequence.Current('ORDERSEQ') = 0 then; end;
}
"#;

#[test]
fn a_taker_and_a_reader_in_handlers_join_the_chain_and_a_named_plain_routine_does_not() {
    let src = format!("{NUMBERS}{READERS}{HANDLER_TAKERS}");
    let rt = run(&src, &strict());
    for test in [
        "TakesOne",
        "TakesTwo",
        "ReadsCurrent",
        "TakesInHandler",
        "ReadsInConfirm",
    ] {
        assert_eq!(
            result(&rt, test).outcome,
            Outcome::Pass,
            "{test}: {:?}",
            result(&rt, test)
        );
    }
    let rt = run(&src, &RunOptions::default());
    assert_eq!(
        numbers_hint(&rt, "TakesInHandler"),
        vec![("ORDERSEQ".to_string(), 60476)]
    );
    assert!(numbers_hint(&rt, "ReadsInConfirm").is_empty());
    assert!(numbers_hint(&rt, "NamesAPlainRoutine").is_empty());
}

/// Two codeunits whose OnRun writes a row (so they are carry-chain members under runner 130450: each hands on the numbers
/// its tests took, `numbers_taken.rs`) and a codeunit outside the chain after them that takes numbers (run again from the
/// chain's carry, `carry_after.rs`). With [`NUMBERS`] before them.
const CARRY_TAKERS: &str = r#"
table 60478 "Order Carry Row" { fields { field(1; Id; Integer) { } } }
codeunit 60479 "Order Carry First"
{
    Subtype = Test;
    trigger OnRun() var R: Record "Order Carry Row"; begin R.Id := 1; R.Insert(); end;
    [Test] procedure CarryTakesOne() var N: BigInteger; begin N := NumberSequence.Next('ORDERSEQ'); end;
}
codeunit 60480 "Order Carry Second"
{
    Subtype = Test;
    trigger OnRun() var R: Record "Order Carry Row"; begin R.Id := 2; R.Insert(); end;
    [Test] procedure CarryTakesAndFails() var N: BigInteger; begin N := NumberSequence.Next('ORDERSEQ'); Error('unrelated %1', N); end;
}
codeunit 60481 "Order Carry Free"
{
    Subtype = Test;
    [Test] procedure FreeTakesAndFails() var N: BigInteger; begin N := NumberSequence.Next('ORDERSEQ'); Error('unrelated %1', N); end;
}
"#;

/// Review of al-runner PR 8947, LOW-1: the numbers hint names only an earlier taker whose numbers did not reach the test
/// here. The second chain member starts from the first's carry and the codeunit outside the chain runs again from it, so
/// both saw what the chain members took; the last earlier taker they did not see is the free codeunit 60476. MED-1: the
/// passing chain member's numbers are kept for the stability cause (`numbers_taken_after`), not as a hint.
#[test]
fn the_numbers_hint_leaves_out_a_chain_member_whose_numbers_reached_the_test() {
    let rt = run(&format!("{NUMBERS}{CARRY_TAKERS}"), &RunOptions::default());
    let members: Vec<u32> = rt.phases.chain.members.iter().map(|m| m.id).collect();
    assert_eq!(members, vec![60479, 60480], "{:?}", rt.phases.chain.members);
    assert!(
        rt.phases.chain.reruns.iter().any(|e| e.id == 60481),
        "{:?}",
        rt.phases.chain.reruns
    );
    for test in ["CarryTakesAndFails", "FreeTakesAndFails"] {
        assert!(
            matches!(&result(&rt, test).outcome, Outcome::Fail(m) if m.contains("unrelated")),
            "{:?}",
            result(&rt, test)
        );
        assert_eq!(numbers_hint(&rt, test), vec![("ORDERSEQ".to_string(), 60476)], "{test}");
    }
    assert!(result(&rt, "CarryTakesOne").order_hint.is_none());
    let after: Vec<(u32, &str, u32)> = rt
        .numbers_taken_after()
        .iter()
        .flat_map(|(cu, t, ns)| ns.iter().map(move |n| (*cu, t.as_str(), n.after.0)))
        .collect();
    assert!(after.contains(&(60479, "CarryTakesOne", 60476)), "{after:?}");
}

/// Two sequences and three takers: A takes X, B and C take Y. C always fails, so only the numbers hint says where BC's
/// order differs.
const TWO_SESSIONS: &str = r#"
codeunit 60490 "Order Y Installer"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() begin NumberSequence.Insert('ORDERSEQY', 1, 1); end;
}
codeunit 60491 "Order X Taker"
{
    Subtype = Test;
    [Test] procedure TakesX() var N: BigInteger; begin N := NumberSequence.Next('ORDERSEQ'); end;
}
codeunit 60492 "Order Y Taker"
{
    Subtype = Test;
    [Test] procedure TakesY() var N: BigInteger; begin N := NumberSequence.Next('ORDERSEQY'); end;
}
codeunit 60493 "Order Y Late Taker"
{
    Subtype = Test;
    [Test] procedure TakesYAndFails() var N: BigInteger; begin N := NumberSequence.Next('ORDERSEQY'); Error('unrelated %1', N); end;
}
"#;

/// Review of al-runner PR 9321 (Copilot, order_hint.rs): a codeunit in two BC sessions (`--test-suites`) gets its
/// numbers hint from the session where an earlier codeunit took from the same sequence. In the first session the only
/// earlier taker took from another sequence, so that session gives no hint and must not use up the codeunit.
#[test]
fn a_codeunit_in_two_sessions_gets_the_numbers_hint_from_the_session_that_matches() {
    let opts = RunOptions {
        test_suites: Some(std::sync::Arc::new(
            al_testrun::test_suites::TestSuites::parse("S1: 60491 60493\nS2: 60492 60493")
                .expect("a valid suites file"),
        )),
        ..Default::default()
    };
    let rt = run(TWO_SESSIONS, &opts);
    assert!(
        matches!(&result(&rt, "TakesYAndFails").outcome, Outcome::Fail(m) if m.contains("unrelated")),
        "{:?}",
        result(&rt, "TakesYAndFails")
    );
    assert_eq!(
        numbers_hint(&rt, "TakesYAndFails"),
        vec![("ORDERSEQY".to_string(), 60492)]
    );
}

/// A codeunit whose passing test takes from one sequence and whose failed test takes from the other.
const SPLIT_TAKER: &str = r#"
codeunit 60494 "Order Split Taker"
{
    Subtype = Test;
    [Test] procedure TakesXAndPasses() var N: BigInteger; begin N := NumberSequence.Next('ORDERSEQ'); end;
    [Test] procedure TakesYAndFailsToo() var N: BigInteger; begin N := NumberSequence.Next('ORDERSEQY'); Error('unrelated %1', N); end;
}
"#;

/// Al-runner issue 9332, item 1: in the first session a passing test of the codeunit gets numbers; the failed test of
/// the same codeunit still gets its numbers hint from the second session, where its sequence was taken before.
#[test]
fn a_passing_test_with_numbers_in_one_session_leaves_the_failed_test_its_hint_from_the_other() {
    let opts = RunOptions {
        test_suites: Some(std::sync::Arc::new(
            al_testrun::test_suites::TestSuites::parse("S1: 60491 60494\nS2: 60492 60494")
                .expect("a valid suites file"),
        )),
        ..Default::default()
    };
    let rt = run(&format!("{TWO_SESSIONS}{SPLIT_TAKER}"), &opts);
    assert!(result(&rt, "TakesXAndPasses").order_hint.is_none());
    assert_eq!(
        numbers_hint(&rt, "TakesYAndFailsToo"),
        vec![("ORDERSEQY".to_string(), 60492)]
    );
    let after: Vec<(u32, &str, u32)> = rt
        .numbers_taken_after()
        .iter()
        .flat_map(|(cu, t, ns)| ns.iter().map(move |n| (*cu, t.as_str(), n.after.0)))
        .collect();
    assert!(after.contains(&(60494, "TakesXAndPasses", 60491)), "{after:?}");
    assert!(after.contains(&(60494, "TakesYAndFailsToo", 60492)), "{after:?}");
}

/// A failed test that takes from the second sequence and reads the flag of PAIR's setter.
const READS_AND_TAKES: &str = r#"
codeunit 60495 "Order Reads And Takes"
{
    Subtype = Test;
    [Test] procedure ReadsAndTakesY()
    var S: Codeunit "Order State"; N: BigInteger;
    begin
        N := NumberSequence.Next('ORDERSEQY');
        if S.GetFlag() <> 'left' then Error('flag is %1', S.GetFlag());
    end;
}
"#;

/// Al-runner issue 9332, item 2: the first session gives the failed test a numbers hint, the second finds the
/// SingleInstance read. The test gets one hint with both, and counts once.
#[test]
fn a_numbers_hint_and_a_single_instance_hint_from_two_sessions_merge_into_one() {
    let opts = RunOptions {
        test_suites: Some(std::sync::Arc::new(
            al_testrun::test_suites::TestSuites::parse("S1: 60492 60495\nS2: 60471 60495")
                .expect("a valid suites file"),
        )),
        ..Default::default()
    };
    let rt = run(&format!("{PAIR}{TWO_SESSIONS}{READS_AND_TAKES}"), &opts);
    let r = result(&rt, "ReadsAndTakesY");
    assert!(matches!(&r.outcome, Outcome::Fail(m) if m.contains("flag is")), "{r:?}");
    let hint = r.order_hint.as_ref().expect("a hint");
    assert!(!hint.reads.is_empty(), "{hint:?}");
    assert_eq!(
        numbers_hint(&rt, "ReadsAndTakesY"),
        vec![("ORDERSEQY".to_string(), 60492)]
    );
    let failed_hinted = rt
        .results()
        .iter()
        .filter(|r| matches!(r.outcome, Outcome::Fail(_)) && r.order_hint.is_some())
        .count();
    assert_eq!(rt.phases.chain.single_lane.hinted, failed_hinted);
}

/// A codeunit that fails only after the setter of PAIR ran in its session.
const INVERSE: &str = r#"
codeunit 60482 "Order Inverse Reader"
{
    Subtype = Test;
    [Test] procedure FailsAfterTheSetter()
    var S: Codeunit "Order State";
    begin
        if S.GetFlag() = 'left' then Error('the setter ran first');
    end;
}
"#;

/// Review of al-runner PR 9321 (Copilot, al-test-cli main.rs): in a run that is already in BC's order (the nightly
/// whole-corpus run, `AL_STRICT_ORDER_ALL`) `strict_after` runs nothing again, so a listed codeunit keeps the outcome it
/// had after the codeunits before it, not the one it gets alone.
#[test]
fn a_strict_run_keeps_its_outcomes_and_runs_nothing_again_for_strict_after() {
    let opts = RunOptions {
        strict_after: Some(al_testrun::runtime::StrictAfter {
            only: vec![(0, 60482)],
            listed: [(String::new(), 60482)].into(),
        }),
        ..strict()
    };
    let rt = run(&format!("{PAIR}{INVERSE}"), &opts);
    let case = al_testrun::CaseId {
        codeunit: 60482,
        name: "FailsAfterTheSetter".into(),
    };
    assert!(
        matches!(rt.outcome_in("", &case), Outcome::Fail(m) if m.contains("the setter ran first")),
        "{:?}",
        rt.outcome_in("", &case)
    );
}
