//! A selection of pure tests is compiled lean (`al_sema::check::Roots::lean`): the workspace objects no selected test
//! names are left out, so the tables, pages, listeners and install code beside the tests do not pull the dependency
//! apps into the compile. The outcomes are the ones the complete compile gives: a run that meets a routine the lean
//! compile left out runs again on the complete one.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::compile_cache::CompileCache;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};

const BASE: &str = r#"
table 50100 Setting { fields { field(1; Code; Code[20]) { } field(2; Value; Text[50]) { } } keys { key(PK; Code) { } } }
codeunit 50101 Installer
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() var S: Record Setting; begin S.Code := 'SEEDED'; S.Value := 'yes'; S.Insert(); end;
}
"#;

/// Beside the tests: a table with a trigger, a page on it, a listener, and install code that writes the table and
/// counts in a single-instance codeunit.
const BESIDE: &str = r#"
table 60110 Row { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { } } trigger OnInsert() begin end; }
page 60111 Rows { SourceTable = Row; layout { area(Content) { field(No; Rec."No.") { } } } }
codeunit 60112 "Row Listener"
{
    [EventSubscriber(ObjectType::Table, Database::Row, 'OnAfterInsertEvent', '', false, false)]
    local procedure Inserted(var Rec: Record Row) begin end;
}
codeunit 60113 "Main Installer"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() var R: Record Row; C: Codeunit Counter; begin R."No." := 1; R.Insert(); C.Bump(); end;
}
codeunit 60120 Counter
{
    SingleInstance = true;
    var N: Integer;
    procedure Bump() begin N += 1; end;
    procedure Value(): Integer begin exit(N); end;
}
"#;

const TESTS: &str = r#"
codeunit 60101 PureTests
{
    Subtype = Test;
    [Test] procedure Adds() begin if 1 + 1 <> 2 then Error('add'); end;
    [Test] procedure Fails() begin Error('on purpose'); end;
}
codeunit 60103 RowTests
{
    Subtype = Test;
    [Test] procedure SeesTheInstalledRow() var R: Record Row; begin if not R.Get(1) then Error('no row'); end;
}
codeunit 60104 CounterTests
{
    Subtype = Test;
    var C: Codeunit Counter;
    [Test] procedure StartsFresh() begin if C.Value() <> 0 then Error('the install code counted %1 here', C.Value()); end;
}
"#;

/// A test whose own text names no data, and a helper of the workspace that reads a row.
const HELPER_TESTS: &str = r#"
codeunit 60106 "Row Helper" { procedure Seeded(): Boolean var R: Record Row; begin exit(R.Get(1)); end; }
codeunit 60105 HelperTests
{
    Subtype = Test;
    var H: Codeunit "Row Helper";
    [Test] procedure SeesTheInstalledRowThroughTheHelper() begin if not H.Seeded() then Error('no row'); end;
}
"#;

/// A workspace codeunit BC's test runner runs by id around every test method (`runner_reset.rs`): no test names it.
const RUNNER_CODEUNIT: &str = r#"
codeunit 130301 "Reset State Before Test Run" { trigger OnRun() begin end; }
"#;

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    }
}

fn run(main: &str, only: &[u32], cache: Option<&std::path::Path>) -> CorpusRuntime {
    let manifest = |p: &str| Manifest {
        publisher: p.into(),
        version: "1.0.0.0".into(),
        resources: vec![],
        include_source: false,
        target: String::new(),
        layout_files: Vec::new(),
        id_ranges: Vec::new(),
        symbols: vec![],
    };
    CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], main), app("base", &[], BASE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions {
            only: (!only.is_empty()).then(|| only.iter().map(|&id| (0, id)).collect()),
            compile_cache: cache.map(|dir| CompileCache {
                dir: dir.to_path_buf(),
                require: false,
                rebuild: false,
                country: String::new(),
            }),
            ..Default::default()
        },
    )
    .expect("the run starts")
}

fn outcome(rt: &CorpusRuntime, test: &str) -> Outcome {
    rt.results()
        .iter()
        .find(|r| r.test == test)
        .unwrap_or_else(|| panic!("{test} did not run"))
        .outcome
        .clone()
}

/// Both compile paths: the demand-driven one and the one with the compile cache.
fn both(main: &str, only: &[u32], check: impl Fn(&CorpusRuntime)) {
    check(&run(main, only, None));
    let dir = tempfile::tempdir().unwrap();
    check(&run(main, only, Some(dir.path())));
    check(&run(main, only, Some(dir.path())));
}

#[test]
fn a_pure_selection_compiles_only_the_workspace_objects_its_tests_name() {
    let main = format!("{BESIDE}{TESTS}");
    both(&main, &[60101], |rt| {
        assert!(rt.phases.baseline_skipped, "{:?}", rt.phases.baseline_why);
        assert_eq!(outcome(rt, "Adds"), Outcome::Pass);
        assert_eq!(outcome(rt, "Fails"), Outcome::Fail("on purpose".into()));
        // Of the 8 workspace objects, the two other test codeunits are not selected and the 5 beside the tests are
        // named by no selected test.
        assert_eq!(rt.phases.lean, Some((5, 8)), "{:?}", rt.phases.compile_line());
        assert!(!rt.phases.lean_redone);
        assert_eq!(rt.phases.compiles, 1);
        assert!(
            rt.phases
                .compile_line()
                .is_some_and(|l| l.starts_with("compile: lean, 5 of 8 workspace objects left out")),
            "{:?}",
            rt.phases.compile_line()
        );
    });
}

/// The install code of the workspace runs once, in the session that publishes the app (the baseline's): the
/// single-instance codeunit it counted in is fresh in each test's session. A lean selection leaves the install code
/// out and gets the outcome of the unselected run.
#[test]
fn a_lean_selection_gets_the_outcome_of_the_complete_run_when_install_code_counts_in_a_single_instance_codeunit() {
    let main = format!("{BESIDE}{TESTS}");
    both(&main, &[], |rt| {
        assert!(!rt.phases.baseline_skipped);
        assert_eq!(outcome(rt, "StartsFresh"), Outcome::Pass);
        assert_eq!(outcome(rt, "SeesTheInstalledRow"), Outcome::Pass);
    });
    both(&main, &[60104], |rt| {
        assert!(rt.phases.baseline_skipped, "{:?}", rt.phases.baseline_why);
        assert_eq!(
            outcome(rt, "StartsFresh"),
            Outcome::Pass,
            "{:?}",
            rt.phases.compile_line()
        );
        assert!(rt.phases.lean.is_some());
    });
}

#[test]
fn a_selection_that_needs_the_baseline_is_compiled_completely() {
    let main = format!("{BESIDE}{TESTS}");
    both(&main, &[60103], |rt| {
        assert!(!rt.phases.baseline_skipped);
        assert_eq!(rt.phases.lean, None);
        assert_eq!(outcome(rt, "SeesTheInstalledRow"), Outcome::Pass);
    });
}

/// The test runner runs codeunit 130301 by id when the apps hold one; the lean compile left it out, as no test names
/// it. The run meets its refusal and runs the tests again on the complete compile.
#[test]
fn a_lean_run_that_meets_a_routine_it_left_out_runs_again_on_the_complete_compile() {
    let main = format!("{BESIDE}{TESTS}{RUNNER_CODEUNIT}");
    both(&main, &[60101], |rt| {
        assert!(rt.phases.lean_redone, "{:?}", rt.phases.compile_line());
        assert_eq!(rt.phases.lean, None, "the run that counts is the complete one");
        assert_eq!(rt.phases.compiles, 2);
        assert_eq!(outcome(rt, "Adds"), Outcome::Pass);
        assert_eq!(outcome(rt, "Fails"), Outcome::Fail("on purpose".into()));
        assert!(
            rt.phases
                .compile_line()
                .is_some_and(|l| l.contains("ran again on the complete compile")),
            "{:?}",
            rt.phases.compile_line()
        );
    });
}

/// The helper a test names touches data: a compile without the baseline would reach the dependency apps and be thrown
/// away when the rule says "may observe", so it is not tried and the apps are compiled once.
#[test]
fn a_selection_whose_helper_reads_a_table_is_compiled_once() {
    let main = format!("{BESIDE}{TESTS}{HELPER_TESTS}");
    both(&main, &[60105], |rt| {
        assert!(!rt.phases.baseline_skipped);
        assert_eq!(rt.phases.compiles, 1);
        assert_eq!(outcome(rt, "SeesTheInstalledRowThroughTheHelper"), Outcome::Pass);
    });
}
