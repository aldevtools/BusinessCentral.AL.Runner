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
    CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], src), app("base", &[], base)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
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
