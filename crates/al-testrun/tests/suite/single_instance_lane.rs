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
        runtime: None,
        app_json_keys: Vec::new(),
        symbols: vec![],
    }
}

fn runtime(src: &str) -> CorpusRuntime {
    runtime_over(src, STATE)
}

fn runtime_over(src: &str, base: &str) -> CorpusRuntime {
    runtime_opts(src, base, &RunOptions::default())
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
