//! Base Application's login code (`LogInManagement.GetDefaultWorkDate`, codeunit 40) sets WorkDate when the test
//! session opens the company (al-runner issue 5743). Its demand reaches most of Base Application, so the lean compile
//! of tests that look free of data leaves it out unless a test can see the WorkDate it sets (al-runner issue 5872: every
//! warm run of ten empty tests took 4 s of compile). Tests that read WorkDate still get the login code's date, and the
//! compile cache is still used when the apps are compiled completely, and renewed when a dependency app changes.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::compile_cache::CompileCache;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};

/// The dependency's login code: it sets WorkDate to 15 January 2024.
const LOGIN: &str = r#"
codeunit 40 LogInManagement
{
    SingleInstance = true;
    procedure GetDefaultWorkDate(): Date begin exit(20240115D); end;
}
"#;

/// Login code the runtime cannot run: `Debugger.GetLastErrorText` is refused when it is lowered.
const LOGIN_REFUSED: &str = r#"
codeunit 40 LogInManagement
{
    SingleInstance = true;
    procedure GetDefaultWorkDate(): Date begin if Debugger.GetLastErrorText() = '' then; exit(20240115D); end;
}
"#;

const TABLE: &str = r#"
table 50100 Setting { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { } } }
"#;

const PURE: &str = r#"
codeunit 60101 PureTests
{
    Subtype = Test;
    [Test] procedure Adds() begin if 1 + 1 <> 2 then Error('add'); end;
}
"#;

const READS_WORKDATE: &str = r#"
codeunit 60102 WorkDateTests
{
    Subtype = Test;
    [Test] procedure SeesTheLoginDate() begin if WorkDate() <> 20240115D then Error('WorkDate %1', WorkDate()); end;
}
"#;

/// `Evaluate` of `w` into a Date gives the session's WorkDate without naming `WorkDate`.
const EVALUATES_W: &str = r#"
codeunit 60103 EvaluateTests
{
    Subtype = Test;
    [Test] procedure WIsTheLoginDate() var D: Date; begin Evaluate(D, 'w'); if D <> 20240115D then Error('w %1', D); end;
}
"#;

const READS_TABLE: &str = r#"
codeunit 60104 TableTests
{
    Subtype = Test;
    [Test] procedure Counts() var S: Record Setting; begin if S.Count() <> 0 then Error('rows'); end;
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

fn run(main: &str, base: &str, cache: Option<&std::path::Path>) -> CorpusRuntime {
    let manifest = |p: &str| Manifest {
        publisher: p.into(),
        version: "1.0.0.0".into(),
        resources: vec![],
        include_source: false,
        target: String::new(),
        layout_files: Vec::new(),
        id_ranges: Vec::new(),
        translations: false,
        translation_files: Vec::new(),
        symbols: vec![],
    };
    CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], main), app("base", &[], base)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions {
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

/// Both compile paths: the demand-driven one and the one with the compile cache, cold and warm.
fn both(main: &str, base: &str, check: impl Fn(&CorpusRuntime)) {
    check(&run(main, base, None));
    let dir = tempfile::tempdir().unwrap();
    check(&run(main, base, Some(dir.path())));
    check(&run(main, base, Some(dir.path())));
}

/// A test that reads no WorkDate cannot see what the login code sets: the lean compile leaves the login code out, so
/// login code the runtime cannot run does not refuse it (before, the lean compile reached the login code and its demand,
/// most of Base Application).
#[test]
fn a_pure_test_that_reads_no_workdate_is_compiled_without_the_login_code() {
    let base = format!("{TABLE}{LOGIN_REFUSED}");
    both(PURE, &base, |rt| {
        assert!(rt.phases.baseline_skipped, "{:?}", rt.phases.baseline_why);
        assert_eq!(rt.phases.compiles, 1, "{:?}", rt.phases.compile_line());
        assert!(rt.phases.compile_cache_skipped || rt.phases.cache.is_none());
        assert_eq!(outcome(rt, "Adds"), Outcome::Pass);
    });
}

/// A test that reads WorkDate sees the login code's date, as before; login code that cannot run refuses it.
#[test]
fn a_test_that_reads_workdate_still_sees_the_login_date() {
    let base = format!("{TABLE}{LOGIN}");
    both(&format!("{PURE}{READS_WORKDATE}"), &base, |rt| {
        assert_eq!(
            outcome(rt, "SeesTheLoginDate"),
            Outcome::Pass,
            "{:?}",
            rt.phases.compile_line()
        );
        assert_eq!(outcome(rt, "Adds"), Outcome::Pass);
    });
    let base = format!("{TABLE}{LOGIN_REFUSED}");
    both(READS_WORKDATE, &base, |rt| {
        assert!(
            matches!(outcome(rt, "SeesTheLoginDate"), Outcome::NotRun(r) if r.contains("login code")),
            "{:?}",
            outcome(rt, "SeesTheLoginDate")
        );
    });
}

/// `Evaluate(D, 'w')` reads the session's WorkDate without naming it: the test gets the login code's date.
#[test]
fn a_test_that_evaluates_w_into_a_date_still_sees_the_login_date() {
    let base = format!("{TABLE}{LOGIN}");
    both(EVALUATES_W, &base, |rt| {
        assert_eq!(
            outcome(rt, "WIsTheLoginDate"),
            Outcome::Pass,
            "{:?}",
            rt.phases.compile_line()
        );
    });
}

/// Tests that read a table are compiled completely: a warm run takes the dependency apps from the compile cache, and a
/// change of a dependency app or of the app under test compiles again.
#[test]
fn the_compile_cache_is_reused_warm_and_renewed_when_an_app_changes() {
    let dir = tempfile::tempdir().unwrap();
    let base = format!("{TABLE}{LOGIN}");
    let main = format!("{READS_TABLE}{READS_WORKDATE}");
    let cold = run(&main, &base, Some(dir.path()));
    let report = cold.phases.cache.as_ref().expect("the compile cache is used");
    assert!(report.built && !report.hit, "{report:?}");
    assert_eq!(outcome(&cold, "SeesTheLoginDate"), Outcome::Pass);

    let warm = run(&main, &base, Some(dir.path()));
    let report = warm.phases.cache.as_ref().expect("the compile cache is used");
    assert!(report.hit && !report.built, "{report:?}");
    assert_eq!(outcome(&warm, "Counts"), Outcome::Pass);
    assert_eq!(outcome(&warm, "SeesTheLoginDate"), Outcome::Pass);

    // The dependency's login code now gives another date: its layer is built again and the test sees the new date.
    let changed = base.replace("20240115D", "20240116D");
    let rt = run(&main, &changed, Some(dir.path()));
    let report = rt.phases.cache.as_ref().expect("the compile cache is used");
    assert!(report.built && !report.hit, "{report:?}");
    assert!(matches!(outcome(&rt, "SeesTheLoginDate"), Outcome::Fail(m) if m.contains("WorkDate")));

    // The app under test changes: it is compiled again on the cached dependency apps.
    let main = main.replace("<> 20240115D", "<> 20240116D");
    let rt = run(&main, &changed, Some(dir.path()));
    let report = rt.phases.cache.as_ref().expect("the compile cache is used");
    assert!(report.hit && !report.built, "{report:?}");
    assert_eq!(outcome(&rt, "SeesTheLoginDate"), Outcome::Pass);
}
