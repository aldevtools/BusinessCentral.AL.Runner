//! ADR-066 amendment 2 (al-runner issue 6924): a test codeunit that reads SingleInstance state an earlier test codeunit
//! left runs, with the codeunits that wrote it, in one serial session in BC's order, and its tests are flagged. A
//! lazy cache that gives the same value either way, and a run without such a dependence, cost no second run.
//! BC runs all test codeunits in one session: codeunits/TestCuP7ASetter.al:24-58 against TestCuP7BReader.al:24-37.

use al_builtins::assumptions::Assumption;
use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};

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

const SAME_CACHE: &str = r#"
codeunit 60461 "Lane Cache A"
{
    Subtype = Test;
    [Test] procedure FillsIt() var C: Codeunit "Lane Cache"; begin if C.Get() <> 5 then Error('a'); end;
}
codeunit 60462 "Lane Cache B"
{
    Subtype = Test;
    [Test] procedure ReadsIt() var C: Codeunit "Lane Cache"; begin if C.Get() <> 5 then Error('b'); end;
}
"#;

const OTHER_CACHE: &str = r#"
codeunit 60471 "Lane Moving A"
{
    Subtype = Test;
    [Test] procedure FillsIt() var C: Codeunit "Lane Cache"; begin if C.GetMoving(1) <> 1 then Error('a'); end;
}
codeunit 60472 "Lane Moving B"
{
    Subtype = Test;
    // Its own fill would give 2; BC's one session still holds the 1 that codeunit 60471 cached.
    [Test] procedure SeesTheFirstFill() var C: Codeunit "Lane Cache"; begin if C.GetMoving(2) <> 1 then Error('b %1', C.GetMoving(2)); end;
}
"#;

const NO_DEPENDENCE: &str = r#"
codeunit 60481 "Lane Alone A"
{
    Subtype = Test;
    [Test] procedure Bumps() var A: Codeunit "Lane Alone"; begin A.Bump(); end;
}
codeunit 60482 "Lane Plain B"
{
    Subtype = Test;
    [Test] procedure UsesNone() begin end;
}
"#;

/// ADR-066 amendment 3: a keyed record cache that each codeunit checks and refills (Microsoft's Unit of Measure
/// Management shape), and a global record that a procedure overwrites with `:=` before it reads it.
const KEYED: &str = r#"
table 50454 "Lane Row"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; Name; Text[30]) { }
    }
    keys { key(PK; "No.") { } }
}
codeunit 50455 "Lane Keyed Cache"
{
    SingleInstance = true;
    procedure NameOf(No: Integer): Text
    begin
        if Cached."No." <> No then
            Cached.Get(No);
        exit(Cached.Name);
    end;
    procedure Remember(R: Record "Lane Row"): Text
    begin
        Last := R;
        exit(Last.Name);
    end;
    var
        Cached: Record "Lane Row";
        Last: Record "Lane Row";
}
codeunit 50456 "Lane Rows"
{
    procedure Seed()
    var R: Record "Lane Row";
    begin
        R."No." := 1; R.Name := 'one'; R.Insert();
        R."No." := 2; R.Name := 'two'; R.Insert();
    end;
}
"#;

const KEYED_OTHER_KEYS: &str = r#"
codeunit 60501 "Lane Keyed A"
{
    Subtype = Test;
    [Test] procedure AsksForOne()
    var C: Codeunit "Lane Keyed Cache"; Rows: Codeunit "Lane Rows";
    begin
        Rows.Seed();
        if C.NameOf(1) <> 'one' then Error('a');
    end;
}
codeunit 60502 "Lane Keyed B"
{
    Subtype = Test;
    [Test] procedure AsksForTwo()
    var C: Codeunit "Lane Keyed Cache"; Rows: Codeunit "Lane Rows";
    begin
        Rows.Seed();
        if C.NameOf(2) <> 'two' then Error('b');
    end;
}
codeunit 60503 "Lane Keyed C"
{
    Subtype = Test;
    [Test] procedure AsksForTwoAgain()
    var C: Codeunit "Lane Keyed Cache"; Rows: Codeunit "Lane Rows";
    begin
        Rows.Seed();
        if C.NameOf(2) <> 'two' then Error('c');
    end;
}
"#;

const OVERWRITTEN_FIRST: &str = r#"
codeunit 60511 "Lane Remember A"
{
    Subtype = Test;
    [Test] procedure RemembersOne()
    var C: Codeunit "Lane Keyed Cache"; R: Record "Lane Row";
    begin
        R."No." := 1; R.Name := 'one';
        if C.Remember(R) <> 'one' then Error('a');
    end;
}
codeunit 60512 "Lane Remember B"
{
    Subtype = Test;
    [Test] procedure RemembersTwo()
    var C: Codeunit "Lane Keyed Cache"; R: Record "Lane Row";
    begin
        R."No." := 2; R.Name := 'two';
        if C.Remember(R) <> 'two' then Error('b');
    end;
}
"#;

fn refill_flagged(rt: &CorpusRuntime, test: &str) -> bool {
    rt.results()
        .iter()
        .find(|r| r.test == test)
        .unwrap_or_else(|| panic!("{test} ran"))
        .assumptions
        .contains(&Assumption::SingleInstanceRefill)
}

#[test]
fn a_keyed_cache_each_codeunit_checks_and_refills_costs_no_lane() {
    let rt = runtime_over(KEYED_OTHER_KEYS, KEYED);
    assert!(
        rt.results().iter().all(|r| r.outcome == Outcome::Pass),
        "{:?}",
        rt.results()
    );
    let lane = &rt.phases.chain.single_lane;
    assert!(lane.lane.is_empty(), "{:?}", lane.lane);
    // B refilled the cache with another key than A left: independent under the guard-then-refill assumption, flagged.
    assert!(refill_flagged(&rt, "AsksForTwo") && !flagged(&rt, "AsksForTwo"));
    // C refilled it with the key B left: the same value either way, nothing assumed.
    assert!(!refill_flagged(&rt, "AsksForTwoAgain"));
    assert!(!refill_flagged(&rt, "AsksForOne"));
}

#[test]
fn a_global_record_overwritten_before_it_is_read_is_no_dependence() {
    let rt = runtime_over(OVERWRITTEN_FIRST, KEYED);
    assert!(
        rt.results().iter().all(|r| r.outcome == Outcome::Pass),
        "{:?}",
        rt.results()
    );
    let lane = &rt.phases.chain.single_lane;
    assert!(lane.lane.is_empty() && lane.notes.is_empty(), "{:?}", lane.notes);
    assert!(!flagged(&rt, "RemembersTwo") && !refill_flagged(&rt, "RemembersTwo"));
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

fn runtime(src: &str) -> CorpusRuntime {
    runtime_over(src, STATE)
}

fn runtime_over(src: &str, base: &str) -> CorpusRuntime {
    runtime_opts(
        src,
        base,
        &RunOptions {
            strict_order: true,
            ..Default::default()
        },
    )
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
        .contains(&Assumption::SingleInstanceOrder)
}

fn passed(rt: &CorpusRuntime, test: &str) -> bool {
    rt.results()
        .iter()
        .any(|r| r.test == test && r.outcome == Outcome::Pass)
}

#[test]
fn a_reader_of_state_an_earlier_codeunit_left_passes_and_is_flagged() {
    let rt = runtime(DEPENDENT_PAIR);
    assert!(passed(&rt, "ReadsTheFlag"), "{:?}", rt.results());
    assert!(flagged(&rt, "ReadsTheFlag"));
    // The writer and a bystander keep their outcome and are not flagged.
    assert!(passed(&rt, "SetsTheFlag") && !flagged(&rt, "SetsTheFlag"));
    assert!(passed(&rt, "UsesNothingShared") && !flagged(&rt, "UsesNothingShared"));
    let lane = &rt.phases.chain.single_lane;
    assert_eq!(lane.lane.len(), 2, "{:?}", lane.lane);
    // ADR-066 amendment 4: the reader fails in its parallel run (fresh state), so it ran again serially in BC's order.
    assert_eq!((lane.dependent, lane.rerun), (1, 1), "{:?}", lane);
    assert!(
        lane.notes
            .iter()
            .any(|n| n.contains("Lane Reader") && n.contains("Lane Setter") && n.contains("fix the test")),
        "{:?}",
        lane.notes
    );
}

#[test]
fn a_lazy_cache_that_gives_the_same_value_is_not_flagged_and_costs_no_second_run() {
    let rt = runtime(SAME_CACHE);
    assert!(passed(&rt, "FillsIt") && passed(&rt, "ReadsIt"));
    assert!(!flagged(&rt, "ReadsIt"));
    assert!(rt.phases.chain.single_lane.lane.is_empty());
}

#[test]
fn a_lazy_cache_another_codeunit_filled_with_another_value_is_a_dependence() {
    let rt = runtime(OTHER_CACHE);
    assert!(passed(&rt, "SeesTheFirstFill"), "{:?}", rt.results());
    assert!(flagged(&rt, "SeesTheFirstFill"));
}

#[test]
fn a_run_without_a_dependence_has_no_lane() {
    let rt = runtime(NO_DEPENDENCE);
    assert!(rt.results().iter().all(|r| r.outcome == Outcome::Pass));
    let lane = &rt.phases.chain.single_lane;
    // One SingleInstance codeunit, used by one test codeunit only: nothing to run again.
    assert!(lane.lane.is_empty() && lane.notes.is_empty());
}

#[test]
fn an_app_without_a_single_instance_codeunit_that_has_globals_has_no_lane() {
    let src = r#"
codeunit 60491 "Lane Plain C" { Subtype = Test; [Test] procedure One() begin end; }
codeunit 60492 "Lane Plain D" { Subtype = Test; [Test] procedure Two() begin end; }
"#;
    // The one SingleInstance codeunit of the base app has no globals: nothing can pass from one codeunit to the next.
    let rt = runtime_over(
        src,
        "codeunit 50499 \"Lane Plain Base\" { SingleInstance = true; procedure Hi() begin end; }",
    );
    assert!(rt.results().iter().all(|r| r.outcome == Outcome::Pass));
    let lane = &rt.phases.chain.single_lane;
    assert!(lane.lane.is_empty());
}

/// ADR-066 amendment 4: a dependent codeunit whose tests pass with fresh SingleInstance state (as when it runs alone or
/// first on BC).
const DEPENDENT_BUT_PASSES: &str = r#"
codeunit 60521 "Lane Setter Two"
{
    Subtype = Test;
    [Test] procedure SetsItToo() var S: Codeunit "Lane State"; begin S.SetFlag('left'); end;
}
codeunit 60522 "Lane Lenient Reader"
{
    Subtype = Test;
    [Test] procedure ReadsAnyFlag()
    var S: Codeunit "Lane State";
    begin
        if StrLen(S.GetFlag()) > 10 then Error('flag is %1', S.GetFlag());
    end;
}
"#;

/// ADR-066 amendment 4, decisions 1 and 3: a dependent codeunit that passes in its parallel run is not run again, and
/// its tests are flagged passes (order dependent).
#[test]
fn a_dependent_codeunit_that_passes_in_parallel_is_not_run_again_and_is_flagged() {
    let rt = runtime(DEPENDENT_BUT_PASSES);
    assert!(passed(&rt, "ReadsAnyFlag"), "{:?}", rt.results());
    assert!(flagged(&rt, "ReadsAnyFlag"));
    assert!(passed(&rt, "SetsItToo") && !flagged(&rt, "SetsItToo"));
    let lane = &rt.phases.chain.single_lane;
    assert!(lane.lane.is_empty(), "{:?}", lane.lane);
    assert_eq!((lane.dependent, lane.rerun, lane.strict), (1, 0, false), "{:?}", lane);
    // The note still names what it relies on, and the --timings line counts it.
    assert!(
        lane.notes
            .iter()
            .any(|n| n.contains("Lane Lenient Reader") && n.contains("Lane Setter Two")),
        "{:?}",
        lane.notes
    );
    let head = &lane.lines()[0];
    assert!(head.contains("1 dependent") && head.contains("0 ran again"), "{head}");
}

/// ADR-066 amendment 4, decision 4: `--strict-order` runs every dependent codeunit again serially in BC's order, with
/// the codeunit it depends on, as amendment 3 did.
#[test]
fn strict_order_runs_every_dependent_codeunit_again_serially() {
    let opts = RunOptions {
        strict_order: true,
        replay_every_dependent: true,
        ..Default::default()
    };
    let rt = runtime_opts(DEPENDENT_BUT_PASSES, STATE, &opts);
    assert!(passed(&rt, "ReadsAnyFlag") && flagged(&rt, "ReadsAnyFlag"));
    let lane = &rt.phases.chain.single_lane;
    assert_eq!(lane.lane.len(), 2, "{:?}", lane.lane);
    assert_eq!((lane.dependent, lane.rerun, lane.strict), (1, 1, true), "{:?}", lane);
    assert!(lane.lines()[0].contains("strict order"), "{:?}", lane.lines());
}

/// ADR-066 amendment 5: two dependent codeunits that fail with fresh state and rely on different writers.
const TWO_CHAINS: &str = r#"
codeunit 50457 "Lane Other State"
{
    SingleInstance = true;
    procedure SetFlag(V: Text) begin Flag := V; end;
    procedure GetFlag(): Text begin exit(Flag); end;
    var Flag: Text;
}
codeunit 60531 "Lane Chain Setter A"
{
    Subtype = Test;
    [Test] procedure SetsA() var S: Codeunit "Lane State"; begin S.SetFlag('a'); end;
}
codeunit 60532 "Lane Chain Reader A"
{
    Subtype = Test;
    [Test] procedure ReadsA() var S: Codeunit "Lane State"; begin if S.GetFlag() <> 'a' then Error('a is %1', S.GetFlag()); end;
}
codeunit 60533 "Lane Chain Setter B"
{
    Subtype = Test;
    [Test] procedure SetsB() var S: Codeunit "Lane Other State"; begin S.SetFlag('b'); end;
}
codeunit 60534 "Lane Chain Reader B"
{
    Subtype = Test;
    [Test] procedure ReadsB() var S: Codeunit "Lane Other State"; begin if S.GetFlag() <> 'b' then Error('b is %1', S.GetFlag()); end;
}
"#;

/// ADR-066 amendment 5, decision 1: each serial re-run is a session of its own (the dependent codeunit and its writer
/// chain); two chains that share no codeunit are two sessions.
#[test]
fn two_dependents_with_their_own_writers_run_as_two_serial_sessions() {
    let rt = runtime(TWO_CHAINS);
    for t in ["ReadsA", "ReadsB"] {
        assert!(passed(&rt, t) && flagged(&rt, t), "{t}: {:?}", rt.results());
    }
    let lane = &rt.phases.chain.single_lane;
    assert_eq!((lane.dependent, lane.rerun), (2, 2), "{:?}", lane);
    assert_eq!(lane.chains, 2, "{:?}", lane);
    assert_eq!(lane.lane.len(), 4, "{:?}", lane.lane);
    let head = &lane.lines()[0];
    assert!(head.contains("2 serial sessions") && head.contains("longest"), "{head}");
}

/// Two dependent codeunits that read what one writer left.
const SHARED_WRITER: &str = r#"
codeunit 60541 "Lane Shared Setter"
{
    Subtype = Test;
    [Test] procedure SetsShared() var S: Codeunit "Lane State"; begin S.SetFlag('s'); end;
}
codeunit 60542 "Lane Shared Reader A"
{
    Subtype = Test;
    [Test] procedure ReadsSharedA() var S: Codeunit "Lane State"; begin if S.GetFlag() <> 's' then Error('a %1', S.GetFlag()); end;
}
codeunit 60543 "Lane Shared Reader B"
{
    Subtype = Test;
    [Test] procedure ReadsSharedB() var S: Codeunit "Lane State"; begin if S.GetFlag() <> 's' then Error('b %1', S.GetFlag()); end;
}
"#;

/// ADR-066 amendment 5: two chains that share a writer are two sessions; the writer runs in each, so neither reader
/// sees what the other left.
#[test]
fn chains_that_share_a_writer_run_as_sessions_of_their_own() {
    let rt = runtime(SHARED_WRITER);
    for t in ["ReadsSharedA", "ReadsSharedB"] {
        assert!(passed(&rt, t) && flagged(&rt, t), "{t}: {:?}", rt.results());
    }
    let lane = &rt.phases.chain.single_lane;
    assert_eq!((lane.dependent, lane.rerun, lane.chains), (2, 2, 2), "{:?}", lane);
    assert_eq!(lane.lane.len(), 4, "the writer runs in both sessions: {:?}", lane.lane);
    assert_eq!(
        lane.lane.iter().filter(|(id, _)| *id == 60541).count(),
        2,
        "{:?}",
        lane.lane
    );
}

/// A dependent codeunit that passes with fresh state, besides a test the lowering refuses (a ternary of a label and an
/// integer into a Variant) and so never starts.
const DEPENDENT_WITH_STATIC_REFUSAL: &str = r#"
codeunit 60551 "Lane Static Setter"
{
    Subtype = Test;
    [Test] procedure SetsStatic() var S: Codeunit "Lane State"; begin S.SetFlag('left'); end;
}
codeunit 60552 "Lane Static Reader"
{
    Subtype = Test;
    [Test] procedure ReadsAnyFlagStatic()
    var S: Codeunit "Lane State";
    begin
        if StrLen(S.GetFlag()) > 10 then Error('flag is %1', S.GetFlag());
    end;
    [Test] procedure RefusedBeforeItStarts()
    var V: Variant; L: Label 'abc', Locked = true; B: Boolean;
    begin
        V := B ? L : 5;
    end;
}
"#;

/// ADR-066 amendment 5, decision 2: a test refused before it starts does not make its dependent codeunit run again.
#[test]
fn a_test_refused_before_it_starts_does_not_trigger_a_serial_run() {
    let rt = runtime(DEPENDENT_WITH_STATIC_REFUSAL);
    assert!(
        rt.results()
            .iter()
            .any(|r| r.test == "RefusedBeforeItStarts" && matches!(r.outcome, Outcome::NotRun(_))),
        "{:?}",
        rt.results()
    );
    assert!(passed(&rt, "ReadsAnyFlagStatic") && flagged(&rt, "ReadsAnyFlagStatic"));
    let lane = &rt.phases.chain.single_lane;
    assert_eq!((lane.dependent, lane.rerun, lane.chains), (1, 0, 0), "{:?}", lane);
    assert!(lane.lane.is_empty(), "{:?}", lane.lane);
}

/// The same codeunit, but its other test is refused while it runs (`Sleep(-1)`).
const DEPENDENT_WITH_MID_RUN_REFUSAL: &str = r#"
codeunit 60561 "Lane MidRun Setter"
{
    Subtype = Test;
    [Test] procedure SetsMidRun() var S: Codeunit "Lane State"; begin S.SetFlag('left'); end;
}
codeunit 60562 "Lane MidRun Reader"
{
    Subtype = Test;
    [Test] procedure ReadsAnyFlagMidRun()
    var S: Codeunit "Lane State";
    begin
        if StrLen(S.GetFlag()) > 10 then Error('flag is %1', S.GetFlag());
    end;
    [Test] procedure RefusedWhileItRuns()
    begin
        Sleep(-1);
    end;
}
"#;

/// ADR-066 amendment 5, decision 2: a test refused while it runs does make its dependent codeunit run again.
#[test]
fn a_test_refused_while_it_runs_triggers_a_serial_run() {
    let rt = runtime(DEPENDENT_WITH_MID_RUN_REFUSAL);
    assert!(
        rt.results()
            .iter()
            .any(|r| r.test == "RefusedWhileItRuns" && matches!(r.outcome, Outcome::NotRun(_))),
        "{:?}",
        rt.results()
    );
    assert!(passed(&rt, "ReadsAnyFlagMidRun") && flagged(&rt, "ReadsAnyFlagMidRun"));
    let lane = &rt.phases.chain.single_lane;
    assert_eq!((lane.dependent, lane.rerun, lane.chains), (1, 1, 1), "{:?}", lane);
    assert_eq!(lane.lane.len(), 2, "{:?}", lane.lane);
}

/// ADR-066 amendment 6 (al-runner issue 6924): a run with `--test-suites` treats each suite as a BC session of its own.
fn with_suites(src: &str, suites: &str) -> CorpusRuntime {
    let opts = RunOptions {
        strict_order: true,
        test_suites: Some(std::sync::Arc::new(
            al_testrun::test_suites::TestSuites::parse(suites).expect("a valid suites file"),
        )),
        ..Default::default()
    };
    runtime_opts(src, STATE, &opts)
}

/// Amendment 6: a writer in one suite and a reader in another are not linked. On BC each suite runs in a client
/// session of its own, so the reader starts with fresh SingleInstance state and fails, unflagged.
#[test]
fn a_writer_in_one_suite_and_a_reader_in_another_are_not_linked() {
    let rt = with_suites(DEPENDENT_PAIR, "A: 60451\nB: 60452 60453\n");
    assert!(!passed(&rt, "ReadsTheFlag"), "{:?}", rt.results());
    assert!(!flagged(&rt, "ReadsTheFlag"));
    let lane = &rt.phases.chain.single_lane;
    assert_eq!((lane.dependent, lane.rerun), (0, 0), "{:?}", lane);
    assert!(lane.lane.is_empty(), "{:?}", lane.lane);
}

/// Amendment 6: within a suite amendments 4 and 5 apply unchanged: the reader fails with fresh state, runs again
/// serially after its writer and passes flagged.
#[test]
fn a_writer_and_a_reader_in_one_suite_are_still_linked() {
    let rt = with_suites(DEPENDENT_PAIR, "DATACHECK: 60451 60452\nOTHER: 60453\n");
    assert!(
        passed(&rt, "ReadsTheFlag") && flagged(&rt, "ReadsTheFlag"),
        "{:?}",
        rt.results()
    );
    assert!(passed(&rt, "SetsTheFlag") && !flagged(&rt, "SetsTheFlag"));
    let lane = &rt.phases.chain.single_lane;
    assert_eq!((lane.dependent, lane.rerun, lane.lane.len()), (1, 1, 2), "{:?}", lane);
}

/// Amendment 6: BC's order is the order in the suite, not the id order: a reader listed before its writer sees
/// nothing the writer left.
#[test]
fn the_order_within_a_suite_is_the_order_of_the_file() {
    let rt = with_suites(DEPENDENT_PAIR, "A: 60452 60451\n");
    assert!(
        !passed(&rt, "ReadsTheFlag") && !flagged(&rt, "ReadsTheFlag"),
        "{:?}",
        rt.results()
    );
    assert_eq!(rt.phases.chain.single_lane.dependent, 0);
}

/// Amendment 6: a codeunit in no suite runs as a session of its own: no earlier codeunit, no link.
#[test]
fn a_codeunit_in_no_suite_is_a_session_of_its_own() {
    let rt = with_suites(DEPENDENT_PAIR, "A: 60451 60453\n");
    assert!(
        !passed(&rt, "ReadsTheFlag") && !flagged(&rt, "ReadsTheFlag"),
        "{:?}",
        rt.results()
    );
    assert!(passed(&rt, "UsesNothingShared"));
    assert_eq!(rt.phases.chain.single_lane.dependent, 0);
}

/// Amendment 6: a codeunit listed in two suites (issue 6924: suites 5 and 6 are the same) runs and is reported once.
/// It is dependent when it is in any suite it is listed in; the first suite where it is decides its serial run.
#[test]
fn a_codeunit_listed_in_two_suites_is_reported_once() {
    let rt = with_suites(DEPENDENT_PAIR, "A: 60451 60452\nB: 60451 60452\n");
    assert!(
        passed(&rt, "ReadsTheFlag") && flagged(&rt, "ReadsTheFlag"),
        "{:?}",
        rt.results()
    );
    assert_eq!(rt.results().iter().filter(|r| r.test == "ReadsTheFlag").count(), 1);
    let lane = &rt.phases.chain.single_lane;
    assert_eq!(
        (lane.dependent, lane.rerun, lane.chains, lane.notes.len()),
        (1, 1, 1, 1),
        "{:?}",
        lane
    );
    // Not dependent in its first suite (alone there), dependent in the second: flagged, run again after its writer.
    let rt = with_suites(DEPENDENT_PAIR, "B: 60452\nA: 60451 60452\n");
    assert!(
        passed(&rt, "ReadsTheFlag") && flagged(&rt, "ReadsTheFlag"),
        "{:?}",
        rt.results()
    );
}

/// Amendment 6 (owner, 2026-11-07): suite boundaries are session boundaries for SingleInstance state only. The compile,
/// the install code and the install baseline are made once per run and shared by every suite and every session.
const INSTALLED: &str = r#"
table 50458 "Lane Setting" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { } } }
codeunit 50459 "Lane Installer"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() var S: Record "Lane Setting"; begin S.Code := 'SEEDED'; S.Insert(); end;
}
"#;

const SEEDED_READERS: &str = r#"
codeunit 60491 "Lane Seed Reader A"
{
    Subtype = Test;
    [Test] procedure SeesTheSeedA() var S: Record "Lane Setting"; begin S.Get('SEEDED'); end;
}
codeunit 60492 "Lane Seed Reader B"
{
    Subtype = Test;
    [Test] procedure SeesTheSeedB() var S: Record "Lane Setting"; begin S.Get('SEEDED'); end;
}
"#;

#[test]
fn suites_share_one_compile_and_one_install_baseline() {
    let src = format!("{DEPENDENT_PAIR}{SEEDED_READERS}");
    let base = format!("{STATE}{INSTALLED}");
    let plain = runtime_opts(
        &src,
        &base,
        &RunOptions {
            strict_order: true,
            ..Default::default()
        },
    );
    let opts = RunOptions {
        strict_order: true,
        test_suites: Some(std::sync::Arc::new(
            al_testrun::test_suites::TestSuites::parse("A: 60451 60491\nB: 60452 60492\nC: 60451 60452\n").unwrap(),
        )),
        ..Default::default()
    };
    let split = runtime_opts(&src, &base, &opts);
    for rt in [&plain, &split] {
        // Both readers, in different suites, see the row the install code wrote once.
        assert!(
            passed(rt, "SeesTheSeedA") && passed(rt, "SeesTheSeedB"),
            "{:?}",
            rt.results()
        );
        assert_eq!(rt.phases.baseline_source, Some("ran"));
        assert_eq!(rt.phases.work.compiles, 1);
    }
    // The same work as one session: one compile, one baseline with the same rows, the same tests run, one pass.
    assert_eq!(format!("{:?}", split.phases.work), format!("{:?}", plain.phases.work));
    assert_eq!(split.phases.passes.len(), plain.phases.passes.len());
    // Suite C links the pair, so the reader still runs again; its serial session starts from the shared baseline.
    assert!(passed(&split, "ReadsTheFlag") && flagged(&split, "ReadsTheFlag"));
}

/// A carry chain (`RequiredTestIsolation = Disabled`, ADR-055) of `n` writers, each inserting a row and setting a
/// SingleInstance flag of its own, and `n` readers outside the chain (`RequiredTestIsolation = Function`, on the same
/// TestRunnerHub session) that each read one writer's flag and pass either way. ADR-066 amendment 8: the walk keeps the
/// single instances for the writers, so only the readers are dependent. Under `--strict-order`, `n` serial sessions, each
/// replaying its writer from the rows the writers before it left (a carried start of its own, apart from the first).
fn carry_chain(n: usize) -> String {
    let mut src = String::from(
        r#"
table 60600 "Lane Carry Row" { fields { field(1; K; Integer) { } } keys { key(PK; K) { Clustered = true; } } }
"#,
    );
    for i in 0..n {
        src.push_str(&format!(
            r#"
codeunit {si} "Lane Carry Flag {i}"
{{
    SingleInstance = true;
    procedure SetFlag(V: Text) begin Flag := V; end;
    procedure GetFlag(): Text begin exit(Flag); end;
    var Flag: Text;
}}
codeunit {w} "Lane Carry Setter {i}"
{{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure SetsCarried{i}()
    var S: Codeunit "Lane Carry Flag {i}"; R: Record "Lane Carry Row";
    begin
        S.SetFlag('left');
        R.K := {i};
        R.Insert();
    end;
}}
codeunit {r} "Lane Carry Reader {n1}"
{{
    Subtype = Test;
    RequiredTestIsolation = Function;
    [Test] procedure ReadsCarried{n1}()
    var S: Codeunit "Lane Carry Flag {i}";
    begin
        if StrLen(S.GetFlag()) > 10 then Error('flag is %1', S.GetFlag());
    end;
}}
"#,
            si = 60660 + i,
            w = 60600 + i,
            r = 60631 + i,
            n1 = i + 1,
        ));
    }
    src
}

/// ADR-066 amendment 5, decision 5, and al-runner issue 6924 (a partner suite of about 700 codeunits in the carry chain
/// ran out of memory): a carried start is written as bytes when a worker thread needs it and freed once it is read
/// back, so at most one per thread is alive at once, not every start every session needs for the whole lane.
#[test]
fn carried_starts_are_written_when_a_session_needs_them_and_freed_after() {
    const READERS: usize = 24;
    let opts = RunOptions {
        required_isolation: true,
        strict_order: true,
        replay_every_dependent: true,
        ..Default::default()
    };
    let rt = runtime_opts(&carry_chain(READERS), STATE, &opts);
    for i in 1..=READERS {
        let t = format!("ReadsCarried{i}");
        assert!(passed(&rt, &t) && flagged(&rt, &t), "{t}: {:?}", rt.results());
    }
    // The writers are chain members: the walk ran them with the single instances kept (amendment 8), not flagged.
    for i in 0..READERS {
        assert!(!flagged(&rt, &format!("SetsCarried{i}")));
    }
    let lane = &rt.phases.chain.single_lane;
    assert_eq!((lane.rerun, lane.chains), (READERS, READERS), "{:?}", lane);
    if lane.threads > 1 {
        // Every writer but the first replays from a carried start.
        assert!(lane.starts_written >= READERS - 1, "{:?}", lane);
        assert!(lane.starts_peak <= lane.threads, "{:?}", lane);
    }
}

/// Sessions that share writers in a carry chain: `W1` sets the flag, `W2` bumps the count, and each reader reads both,
/// so every reader's session runs `W1` and `W2` from the same carried starts.
fn shared_carry_chain(n: usize) -> String {
    let mut src = String::from(
        r#"
table 60700 "Lane Shared Row" { fields { field(1; K; Integer) { } } keys { key(PK; K) { Clustered = true; } } }
codeunit 60700 "Lane Shared First"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure SharedFirst() var R: Record "Lane Shared Row"; begin R.K := 0; R.Insert(); end;
}
codeunit 60701 "Lane Shared W1"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure SharedW1() var S: Codeunit "Lane State"; R: Record "Lane Shared Row";
    begin S.SetFlag('left'); R.K := 1; R.Insert(); end;
}
codeunit 60702 "Lane Shared W2"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;
    [Test] procedure SharedW2() var A: Codeunit "Lane Alone"; R: Record "Lane Shared Row";
    begin A.Bump(); R.K := 2; R.Insert(); end;
}
"#,
    );
    for i in 1..=n {
        src.push_str(&format!(
            r#"
codeunit {} "Lane Shared Reader {i}"
{{
    Subtype = Test;
    RequiredTestIsolation = Function;
    [Test] procedure ReadsShared{i}()
    var S: Codeunit "Lane State"; A: Codeunit "Lane Alone"; R: Record "Lane Shared Row"; Seed: Record "Lane Setting";
    begin
        Seed.Get('SEEDED');
        R.K := {};
        R.Insert();
        if (StrLen(S.GetFlag()) > 10) or (A.Get() > 10) then Error('flag is %1', S.GetFlag());
    end;
}}
"#,
            60702 + i,
            100 + i
        ));
    }
    src
}

/// ADR-066 amendment 5, decision 5 (al-runner issue 6924, a partner suite of about 700 codeunits in the carry chain
/// took 527 s against 24 s): a carried start that several sessions need is made once on the main thread, not once per
/// session that asks for it.
#[test]
fn a_carried_start_that_sessions_share_is_made_once() {
    const READERS: usize = 24;
    let opts = RunOptions {
        required_isolation: true,
        strict_order: true,
        replay_every_dependent: true,
        ..Default::default()
    };
    // An install baseline, as in the partner's suite: a start goes as the rows it holds apart from it.
    let rt = runtime_opts(&shared_carry_chain(READERS), &format!("{STATE}{INSTALLED}"), &opts);
    for i in 1..=READERS {
        let t = format!("ReadsShared{i}");
        assert!(passed(&rt, &t) && flagged(&rt, &t), "{t}: {:?}", rt.results());
    }
    let lane = &rt.phases.chain.single_lane;
    assert_eq!((lane.rerun, lane.chains), (READERS, READERS), "{:?}", lane);
    if lane.threads > 1 {
        // W1 and W2 of each session start from a carried start (48 asks); there are 3 carries.
        assert!(lane.starts_written <= 3, "{:?}", lane);
        // Each as the rows it holds apart from the start before it, none whole.
        assert_eq!(lane.starts_whole, 0, "{:?}", lane);
    }
}

/// ADR-066 amendment 7, option 1: a dependent codeunit whose failing test runs before any test of it reads the global
/// an earlier codeunit left otherwise.
const FAILS_BEFORE_IT_READS: &str = r#"
codeunit 60571 "Lane Early Setter"
{
    Subtype = Test;
    [Test] procedure SetsEarly() var S: Codeunit "Lane State"; begin S.SetFlag('left'); end;
}
codeunit 60572 "Lane Early Failer"
{
    Subtype = Test;
    [Test] procedure FailsBeforeItReads() begin Error('unrelated'); end;
    [Test] procedure ReadsAnyFlagLate()
    var S: Codeunit "Lane State";
    begin
        if StrLen(S.GetFlag()) > 10 then Error('flag is %1', S.GetFlag());
    end;
}
"#;

/// ADR-066 amendment 7, option 1 (owner, 2026-11-08): the failed test read no global the chain would have left with
/// another value, and no test before it did: the serial re-run cannot change it, so the parallel run stays (flagged).
#[test]
fn a_failed_test_that_read_no_changed_global_triggers_no_serial_run() {
    let rt = runtime(FAILS_BEFORE_IT_READS);
    assert!(
        rt.results()
            .iter()
            .any(|r| r.test == "FailsBeforeItReads" && matches!(r.outcome, Outcome::Fail(_))),
        "{:?}",
        rt.results()
    );
    assert!(passed(&rt, "ReadsAnyFlagLate") && flagged(&rt, "ReadsAnyFlagLate"));
    assert!(flagged(&rt, "FailsBeforeItReads"));
    let lane = &rt.phases.chain.single_lane;
    assert_eq!((lane.dependent, lane.rerun, lane.chains), (1, 0, 0), "{:?}", lane);
    assert_eq!(lane.kept, 1, "{:?}", lane);
    assert!(lane.lane.is_empty(), "{:?}", lane.lane);
    // `--strict-order` still runs it again.
    let opts = RunOptions {
        strict_order: true,
        replay_every_dependent: true,
        ..Default::default()
    };
    let rt = runtime_opts(FAILS_BEFORE_IT_READS, STATE, &opts);
    let lane = &rt.phases.chain.single_lane;
    assert_eq!((lane.dependent, lane.rerun, lane.kept), (1, 1, 0), "{:?}", lane);
}

/// `n` test codeunits that each raise the counter of "Lane Alone" by a number of their own (so each reads what the one
/// before it left and ends with another value: each depends on the one before it), then one that fails unless an
/// earlier codeunit left the counter set: its replay chain holds `n + 1` codeunits.
fn appending_chain(n: usize) -> String {
    let mut s = String::new();
    for k in 0..n {
        s.push_str(&format!(
            "codeunit {} \"Lane Append {k}\" {{ Subtype = Test; [Test] procedure Appends{k}() var A: Codeunit \"Lane Alone\"; I: Integer; begin for I := 0 to {k} do A.Bump(); end; }}\n",
            61000 + k
        ));
    }
    s.push_str(
        "codeunit 61999 \"Lane Append Checker\" { Subtype = Test; [Test] procedure NeedsTheChain() var A: Codeunit \"Lane Alone\"; begin if A.Get() = 0 then Error('alone'); end; }\n",
    );
    s
}

fn rerun_skipped(rt: &CorpusRuntime, test: &str) -> bool {
    rt.results()
        .iter()
        .find(|r| r.test == test)
        .unwrap_or_else(|| panic!("{test} ran"))
        .assumptions
        .contains(&Assumption::SingleInstanceRerunSkipped)
}

/// The limit the tests of `--max-replay` use (NP Retail's measured run used 50).
const MAX_REPLAY: usize = 50;

fn max_replay(n: usize) -> RunOptions {
    RunOptions {
        strict_order: true,
        max_replay: Some(n),
        ..Default::default()
    }
}

/// ADR-066 amendment 7 as the owner changed it on 2026-11-08: without `--max-replay` there is no cap. Every re-run the
/// per-test rule asks for is replayed in full, however long its chain, so the result is exact.
#[test]
fn a_long_chain_is_replayed_in_full_by_default() {
    let rt = runtime(&appending_chain(MAX_REPLAY * 2));
    assert!(passed(&rt, "NeedsTheChain"), "{:?}", rt.results());
    assert!(flagged(&rt, "NeedsTheChain") && !rerun_skipped(&rt, "NeedsTheChain"));
    let lane = &rt.phases.chain.single_lane;
    assert_eq!((lane.rerun, lane.chains), (1, 1), "{:?}", lane.lines());
    assert_eq!(
        lane.lane.len(),
        MAX_REPLAY * 2 + 1,
        "the whole chain replayed: {:?}",
        lane.lane
    );
    assert!(lane.skipped.is_empty(), "{:?}", lane.skipped);
    assert!(!lane.lines().iter().any(|l| l.contains("re-run")), "{:?}", lane.lines());
}

/// ADR-066 amendment 7, option 2, opt-in since 2026-11-08 (`--max-replay N`): a re-run whose chain holds at most `N`
/// codeunits is replayed.
#[test]
fn a_rerun_whose_chain_is_at_the_max_replay_limit_is_replayed() {
    let limit = MAX_REPLAY;
    let rt = runtime_opts(&appending_chain(limit - 1), STATE, &max_replay(limit));
    assert!(passed(&rt, "NeedsTheChain"), "{:?}", rt.results());
    assert!(flagged(&rt, "NeedsTheChain") && !rerun_skipped(&rt, "NeedsTheChain"));
    let lane = &rt.phases.chain.single_lane;
    assert_eq!((lane.rerun, lane.chains), (1, 1), "{:?}", lane.lines());
    assert_eq!(lane.lane.len(), limit, "the whole chain replayed: {:?}", lane.lane);
    assert!(lane.skipped.is_empty(), "{:?}", lane.skipped);
}

/// ADR-066 amendment 7, option 2 (`--max-replay N`): a re-run whose chain holds more codeunits than `N` is not replayed.
/// The codeunit keeps its parallel run and is flagged with why; `--strict-order` replays it all the same.
#[test]
fn a_rerun_whose_chain_is_over_max_replay_is_skipped_and_flagged_unless_strict_order() {
    let limit = MAX_REPLAY;
    let src = appending_chain(limit);
    let rt = runtime_opts(&src, STATE, &max_replay(limit));
    assert!(
        rt.results()
            .iter()
            .any(|r| r.test == "NeedsTheChain" && r.outcome == Outcome::Fail("alone".into())),
        "{:?}",
        rt.results()
    );
    assert!(flagged(&rt, "NeedsTheChain") && rerun_skipped(&rt, "NeedsTheChain"));
    assert!(!rerun_skipped(&rt, "Appends3"));
    let lane = &rt.phases.chain.single_lane;
    assert_eq!((lane.rerun, lane.chains), (0, 0), "{:?}", lane.lines());
    assert!(lane.lane.is_empty(), "{:?}", lane.lane);
    assert_eq!(lane.skipped.len(), 1, "{:?}", lane.skipped);
    assert_eq!((lane.skipped[0].codeunit, lane.skipped[0].chain), (61999, limit + 1));
    let text = format!(
        "re-run skipped (chain of {} codeunits over the limit of {limit})",
        limit + 1
    );
    assert_eq!(al_testrun::runtime::rerun_skipped_text(limit + 1, limit), text);
    assert!(
        lane.lines()
            .iter()
            .any(|l| l.contains("61999 Lane Append Checker") && l.contains(&text)),
        "{:?}",
        lane.lines()
    );
    // `--strict-order` wins over `--max-replay`.
    let opts = RunOptions {
        strict_order: true,
        replay_every_dependent: true,
        ..max_replay(limit)
    };
    let rt = runtime_opts(&src, STATE, &opts);
    assert!(passed(&rt, "NeedsTheChain"), "{:?}", rt.results());
    assert!(!rerun_skipped(&rt, "NeedsTheChain"));
    assert!(rt.phases.chain.single_lane.skipped.is_empty());
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
    assert_eq!(chain.single_lane.rerun, 0, "{:?}", chain.single_lane);
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
    assert_eq!(
        rt.phases.chain.single_lane.rerun, 0,
        "{:?}",
        rt.phases.chain.single_lane
    );
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

/// ADR-066 amendment 8, decision 2 (R3 of amendment 3): a member with a test refused while it ran hands nothing on, so
/// the next member starts with fresh single instances; the member after that sees what the fresh one left.
#[test]
fn the_chain_member_after_a_test_refused_while_it_ran_starts_with_fresh_single_instances() {
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
    assert_eq!(failure(&rt, "Reports60742"), "saw []");
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

fn fresh_opts() -> RunOptions {
    RunOptions {
        required_isolation: true,
        single_fresh_check: true,
        strict_order: true,
        replay_every_dependent: true,
        ..Default::default()
    }
}

#[test]
fn a_chain_member_that_reads_a_carried_value_runs_again_fresh_and_names_the_global_and_its_writer() {
    let rt = runtime_opts(CHAIN_PAIR, STATE, &fresh_opts());
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
    let fresh = c.fresh.as_ref().expect("checked");
    assert!(
        matches!(&fresh[..], [(t, Outcome::Fail(m))] if t == "FreshReadsTheFlag" && m.contains("flag is []")),
        "{fresh:?}"
    );
}

#[test]
fn without_the_check_a_suspect_is_listed_but_not_run_again() {
    let opts = RunOptions {
        strict_order: true,
        required_isolation: true,
        ..Default::default()
    };
    let rt = runtime_opts(CHAIN_PAIR, STATE, &opts);
    let carried: Vec<_> = rt.single_carried().collect();
    assert_eq!(carried.len(), 1, "{carried:?}");
    assert!(carried[0].fresh.is_none());
}

#[test]
fn a_chain_member_rerun_hands_nothing_on() {
    // The reader's fresh run sets nothing, so a later member that checks the flag still sees what the setter left.
    let src = format!(
        "{CHAIN_PAIR}{}",
        chain_checker(60713, "RequiredTestIsolation = Disabled;", "left", "left")
    );
    let rt = runtime_opts(&src, STATE, &fresh_opts());
    assert!(passed(&rt, "Reports60713"), "{:?}", rt.results());
}

#[test]
fn a_dependent_codeunit_outside_the_chain_keeps_its_parallel_run_as_the_fresh_outcome() {
    let rt = runtime_opts(DEPENDENT_PAIR, STATE, &fresh_opts());
    assert!(passed(&rt, "ReadsTheFlag"), "{:?}", rt.results());
    let carried: Vec<_> = rt.single_carried().collect();
    assert_eq!(carried.len(), 1, "{carried:?}");
    let c = carried[0];
    assert!(!c.in_chain);
    assert_eq!(c.reads[0].global, "Flag");
    assert!(
        c.reads[0]
            .last_writer
            .as_ref()
            .is_some_and(|w| w.1.contains("Lane Setter")),
        "{c:?}"
    );
    let fresh = c.fresh.as_ref().expect("checked");
    assert!(
        matches!(&fresh[..], [(t, Outcome::Fail(_))] if t == "ReadsTheFlag"),
        "{fresh:?}"
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

/// ADR-058 amendment 1, decisions 2 and 3: the `fresh` variant is formed from the base variant's run: a suspect takes
/// its fresh outcome, every other codeunit its base outcome.
#[test]
fn the_fresh_variant_takes_each_suspects_fresh_outcome_and_every_other_base_outcome() {
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
    // The base variant ran with the check, so its list holds the fresh outcomes ("checked").
    assert!(rts[0].single_carried().all(|c| c.fresh.is_some()));
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
            replay_every_dependent: true,
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
