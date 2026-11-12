//! ADR-074 (owner decision 2026-11-12, option 1): the test driver. `bc-linux`, the default, runs a test codeunit's OnRun
//! once; `bccontainerhelper` first runs BcContainerHelper's discovery pass (the OnRun of every test codeunit, without
//! test isolation, what it writes staying), then OnRun again inside the runner's isolation. Pinned by corpus
//! tests/al-language-isolation-probe/src/SwOnRunRunner.al:19-26 (Windows nightly run 43058375098 and BC 27.5, 28.5,
//! 29.0); the corpus case itself (`OnRunWrites_DiscoveryRowAndRollback`) is the red/green test, run with
//! `al-runner test --test-driver bccontainerhelper --strict-order`. These tests pin the switch and the refusals.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun, TestDriver};

const LOG: &str = r#"
table 50700 "Log" { fields { field(1; N; Integer) { } } keys { key(PK; N) { Clustered = true; } } }
codeunit 50701 "Writes"
{
    Subtype = Test;
    trigger OnRun()
    var
        L: Record "Log";
    begin
        L.N := L.Count() + 1;
        L.Insert();
    end;

    [Test]
    procedure SeesTwoRows()
    var
        L: Record "Log";
    begin
        if L.Count() <> 2 then
            Error('rows %1', L.Count());
    end;
}
codeunit 50702 "Later"
{
    Subtype = Test;
    [Test]
    procedure SeesTheDiscoveryRowOnly()
    var
        L: Record "Log";
    begin
        if L.Count() <> 1 then
            Error('rows %1', L.Count());
    end;
}
"#;

fn run(src: &str, driver: Option<TestDriver>, strict: bool) -> Vec<(u32, String, Outcome)> {
    let opts = RunOptions {
        test_driver: driver.unwrap_or_default(),
        strict_order: strict,
        strict_sequential: strict,
        ..Default::default()
    };
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), src.to_string())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &opts,
    );
    let mut out: Vec<_> = rt
        .results()
        .iter()
        .map(|r| (r.codeunit, r.test.clone(), r.outcome.clone()))
        .collect();
    out.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    out
}

fn outcome(out: &[(u32, String, Outcome)], test: &str) -> Outcome {
    out.iter()
        .find(|(_, t, _)| t == test)
        .map(|(_, _, o)| o.clone())
        .unwrap_or_else(|| panic!("{test} has no row: {out:?}"))
}

#[test]
fn bc_linux_is_the_default_and_runs_onrun_once() {
    let out = run(LOG, None, false);
    assert!(
        matches!(outcome(&out, "SeesTwoRows"), Outcome::Fail(ref m) if m == "rows 1"),
        "{out:?}"
    );
    assert_eq!(run(LOG, Some(TestDriver::BcLinux), false), out);
}

#[test]
fn bccontainerhelper_keeps_the_discovery_row_and_rolls_back_the_runs_row() {
    for strict in [false, true] {
        let out = run(LOG, Some(TestDriver::BcContainerHelper), strict);
        assert_eq!(outcome(&out, "SeesTwoRows"), Outcome::Pass, "strict {strict}: {out:?}");
        assert_eq!(
            outcome(&out, "SeesTheDiscoveryRowOnly"),
            Outcome::Pass,
            "strict {strict}: {out:?}"
        );
    }
}

#[test]
fn an_earlier_codeunit_that_reads_what_a_later_discovery_wrote_is_refused() {
    let src = format!(
        "{LOG}\ncodeunit 50699 \"Earlier\" {{ Subtype = Test; [Test] procedure ReadsLog() var L: Record \"Log\"; begin if L.IsEmpty() then; end; }}"
    );
    let out = run(&src, Some(TestDriver::BcContainerHelper), false);
    assert!(
        matches!(outcome(&out, "ReadsLog"), Outcome::NotRun(ref why) if why.contains("later codeunit 50701")),
        "{out:?}"
    );
    assert_eq!(outcome(&out, "SeesTwoRows"), Outcome::Pass, "{out:?}");
    // bc-linux has no discovery pass: the earlier codeunit runs.
    assert_eq!(outcome(&run(&src, None, false), "ReadsLog"), Outcome::Pass);
}

const RAISES: &str = r#"
codeunit 50710 "State"
{
    SingleInstance = true;
    var
        Calls: Integer;
    procedure Bump(): Integer begin Calls += 1; exit(Calls); end;
    procedure Get(): Integer begin exit(Calls); end;
}
codeunit 50711 "Raises First"
{
    Subtype = Test;
    var
        S: Codeunit "State";
    trigger OnRun()
    begin
        if S.Bump() = 1 then
            Error('only the first call raises');
    end;

    [Test]
    procedure NoRow()
    begin
    end;
}
codeunit 50712 "Reader"
{
    Subtype = Test;
    var
        S: Codeunit "State";
    [Test]
    procedure OnRunRanTwice()
    begin
        if S.Get() <> 2 then
            Error('calls %1', S.Get());
    end;
}
"#;

#[test]
fn a_codeunit_whose_onrun_raised_in_discovery_reports_no_rows_and_its_onrun_runs_again() {
    let out = run(RAISES, Some(TestDriver::BcContainerHelper), true);
    assert!(out.iter().all(|(id, _, _)| *id != 50711), "{out:?}");
    assert_eq!(outcome(&out, "OnRunRanTwice"), Outcome::Pass, "{out:?}");
}

#[test]
fn without_strict_order_a_test_that_reads_a_single_instance_the_pass_changed_is_refused() {
    let out = run(RAISES, Some(TestDriver::BcContainerHelper), false);
    assert!(
        matches!(outcome(&out, "OnRunRanTwice"), Outcome::NotRun(ref why) if why.contains("--strict-order")),
        "{out:?}"
    );
}

/// A run of `apps` (name, source, dependency names; `Dependency` when the name starts with `dep:`) with `opts` over
/// `suites`.
fn run_apps(apps: &[(&str, &str)], opts: RunOptions, suites: &[SuiteRun]) -> Vec<(u32, String, Outcome)> {
    let mut sources = Vec::new();
    let mut kinds = Vec::new();
    let deps: Vec<String> = apps
        .iter()
        .filter_map(|(n, _)| n.strip_prefix("dep:").map(str::to_string))
        .collect();
    for (name, src) in apps {
        let (name, kind) = match name.strip_prefix("dep:") {
            Some(n) => (n.to_string(), al_sema::AppKind::Dependency),
            None => (name.to_string(), al_sema::AppKind::Workspace),
        };
        let dependencies = match kind {
            al_sema::AppKind::Workspace => deps.clone(),
            al_sema::AppKind::Dependency => vec![],
        };
        sources.push(AppSource {
            id: name.clone(),
            name: name.clone(),
            dependencies,
            files: vec![("main.al".into(), src.to_string())],
        });
        kinds.push(kind);
    }
    let rt = CorpusRuntime::run_suites_with(
        sources,
        kinds,
        &[],
        &Default::default(),
        None,
        Default::default(),
        suites,
        &opts,
    );
    let mut out: Vec<_> = rt
        .results()
        .iter()
        .map(|r| (r.codeunit, r.test.clone(), r.outcome.clone()))
        .collect();
    out.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    out
}

fn bcch(strict: bool) -> RunOptions {
    RunOptions {
        test_driver: TestDriver::BcContainerHelper,
        strict_order: strict,
        strict_sequential: strict,
        ..Default::default()
    }
}

fn not_run(out: &[(u32, String, Outcome)], test: &str, part: &str) -> bool {
    matches!(outcome(out, test), Outcome::NotRun(ref why) if why.contains(part))
}

/// Review finding (round 2, HIGH): an earlier codeunit that reads, through a RecordRef, the table a later codeunit's OnRun
/// writes in the discovery pass is refused, as the typed-record reader is: the static reach does not see which table a
/// RecordRef opens.
#[test]
fn an_earlier_codeunit_that_reads_through_a_recordref_what_a_later_discovery_wrote_is_refused() {
    let src = format!(
        "{LOG}\ncodeunit 50699 \"Earlier RecRef\" {{ Subtype = Test; [Test] procedure ReadsViaRecRef() var R: RecordRef; begin R.Open(50700); if R.Count() <> 0 then Error('rows %1', R.Count()); end; }}\n\
         table 50705 \"Other\" {{ fields {{ field(1; N; Integer) {{ }} }} keys {{ key(PK; N) {{ Clustered = true; }} }} }}\n\
         codeunit 50698 \"Earlier Other\" {{ Subtype = Test; [Test] procedure ReadsOtherViaRecRef() var R: RecordRef; begin R.Open(50705); if R.Count() <> 0 then Error('rows %1', R.Count()); end; }}"
    );
    for strict in [false, true] {
        let out = run_apps(&[("a", &src)], bcch(strict), &[SuiteRun::everything()]);
        assert!(
            not_run(&out, "ReadsViaRecRef", "later codeunit 50701"),
            "strict {strict}: {out:?}"
        );
        assert_eq!(outcome(&out, "SeesTwoRows"), Outcome::Pass, "strict {strict}: {out:?}");
        // What the RecordRef reads is measured: a codeunit that reads only another table runs.
        assert_eq!(
            outcome(&out, "ReadsOtherViaRecRef"),
            Outcome::Pass,
            "strict {strict}: {out:?}"
        );
    }
    assert_eq!(outcome(&run(&src, None, false), "ReadsViaRecRef"), Outcome::Pass);
}

const RUNNER: &str = r#"
table 130450 "Test Method Line" { fields { field(1; "Line No."; Integer) { } } keys { key(PK; "Line No.") { Clustered = true; } } }
codeunit 50790 "Own Runner"
{
    Subtype = TestRunner;
    TableNo = "Test Method Line";
    TestIsolation = Codeunit;
    trigger OnRun()
    begin
    end;
}
"#;

/// Review finding (round 2, MED 1): the discovery pass is measured under runner 130450 (TestIsolation = Codeunit,
/// SwOnRunRunner.al:3-6) and under the runners the isolation suites' Windows oracle was recorded with; any other
/// isolation or runner is refused under bccontainerhelper.
#[test]
fn bccontainerhelper_refuses_every_isolation_and_runner_it_was_not_measured_under() {
    use al_testrun::isolation::TestIsolation;
    let src = format!("{LOG}\n{RUNNER}");
    let apps = [("a", src.as_str())];
    for isolation in [TestIsolation::Disabled, TestIsolation::Function] {
        let opts = RunOptions {
            isolation: Some(isolation),
            ..bcch(false)
        };
        let out = run_apps(&apps, opts, &[SuiteRun::everything()]);
        for t in ["SeesTwoRows", "SeesTheDiscoveryRowOnly"] {
            assert!(not_run(&out, t, "discovery pass"), "{isolation:?}: {out:?}");
        }
    }
    let opts = RunOptions {
        test_runner: al_testrun::runtime::TestRunner::Codeunit(50790),
        ..bcch(false)
    };
    let out = run_apps(&apps, opts, &[SuiteRun::everything()]);
    assert!(not_run(&out, "SeesTheDiscoveryRowOnly", "discovery pass"), "{out:?}");
    // `--isolation codeunit` is runner 130450's own isolation: it runs.
    let opts = RunOptions {
        isolation: Some(TestIsolation::Codeunit),
        ..bcch(false)
    };
    let out = run_apps(&apps, opts, &[SuiteRun::everything()]);
    assert_eq!(outcome(&out, "SeesTwoRows"), Outcome::Pass, "{out:?}");
    // A harness suite whose oracle was recorded through BcContainerHelper with its own runner runs as recorded, also
    // when the harness asks for the TestRunnerHub of the main app (BC 28 and later), which a suite with a runner never
    // takes.
    for required_isolation in [false, true] {
        let suite = SuiteRun {
            name: "own".into(),
            app: Some(0),
            runner: Some(50790),
            driver: Some(TestDriver::BcContainerHelper),
        };
        let opts = RunOptions {
            required_isolation,
            ..Default::default()
        };
        let out = run_apps(&apps, opts, &[suite]);
        assert_eq!(
            outcome(&out, "SeesTheDiscoveryRowOnly"),
            Outcome::Pass,
            "required {required_isolation}: {out:?}"
        );
    }
}

/// The discovery pass over the test codeunits of several apps in one session is not pinned: refused.
#[test]
fn bccontainerhelper_refuses_the_test_codeunits_of_several_apps() {
    let b = r#"codeunit 50741 "B Tests" { Subtype = Test; trigger OnRun() begin end; [Test] procedure BTest() begin end; }"#;
    let out = run_apps(&[("a", LOG), ("b", b)], bcch(false), &[SuiteRun::everything()]);
    for t in ["SeesTwoRows", "BTest"] {
        assert!(not_run(&out, t, "several apps"), "{out:?}");
    }
}

/// Which codeunits each test suite's discovery pass runs is not pinned: `--test-suites` is refused.
#[test]
fn bccontainerhelper_refuses_test_suites() {
    let opts = RunOptions {
        test_suites: Some(std::sync::Arc::new(
            al_testrun::test_suites::TestSuites::parse("one: 50701\ntwo: 50702").unwrap(),
        )),
        ..bcch(false)
    };
    let out = run_apps(&[("a", LOG)], opts, &[SuiteRun::everything()]);
    for t in ["SeesTwoRows", "SeesTheDiscoveryRowOnly"] {
        assert!(not_run(&out, t, "--test-suites"), "{out:?}");
    }
}

/// A subscriber outside the Test Runner app to the events codeunit 130452 raises in the discovery pass is not pinned.
#[test]
fn bccontainerhelper_refuses_a_foreign_subscriber_to_the_get_methods_events() {
    let runner = r#"
codeunit 130452 "Test Runner - Get Methods"
{
    [IntegrationEvent(false, false)]
    procedure OnGetTestMethods(CodeunitID: Integer)
    begin
    end;
}
"#;
    let a = r#"
codeunit 50750 "Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Test Runner - Get Methods", 'OnGetTestMethods', '', false, false)]
    local procedure OnGet(CodeunitID: Integer)
    begin
    end;
}
codeunit 50751 "Sub Tests"
{
    Subtype = Test;
    [Test]
    procedure SubTest()
    var
        G: Codeunit "Test Runner - Get Methods";
    begin
        // Names codeunit 130452, so the demand compiles it, as the Test Runner app's own code does in a real run.
        G.OnGetTestMethods(0);
    end;
}
"#;
    let out = run_apps(
        &[("dep:runner", runner), ("a", a)],
        bcch(false),
        &[SuiteRun::everything()],
    );
    assert!(not_run(&out, "SubTest", "130452"), "{out:?}");
}

/// An OnRun that commits a write and then raises in the discovery pass: what the pass keeps is not pinned, so the tests
/// that could see it do not run.
#[test]
fn bccontainerhelper_refuses_what_an_onrun_that_raised_after_a_committed_write_can_reach() {
    let src = r#"
table 50700 "Log" { fields { field(1; N; Integer) { } } keys { key(PK; N) { Clustered = true; } } }
codeunit 50710 "State"
{
    SingleInstance = true;
    var
        Calls: Integer;
    procedure Bump(): Integer begin Calls += 1; exit(Calls); end;
}
codeunit 50711 "Raises After Write"
{
    Subtype = Test;
    var
        S: Codeunit "State";
    trigger OnRun()
    var
        L: Record "Log";
    begin
        L.N := L.Count() + 1;
        L.Insert();
        Commit();
        if S.Bump() = 1 then
            Error('first call raises after a committed write');
    end;

    [Test]
    procedure RaiserMethod()
    begin
    end;
}
codeunit 50712 "Reader Static"
{
    Subtype = Test;
    [Test]
    procedure ReaderStatic()
    var
        L: Record "Log";
    begin
        Error('static sees %1 rows', L.Count());
    end;
}
codeunit 50713 "Reader RecRef"
{
    Subtype = Test;
    [Test]
    procedure ReaderRecRef()
    var
        R: RecordRef;
    begin
        R.Open(50700);
        Error('recref sees %1 rows', R.Count());
    end;
}
"#;
    for strict in [false, true] {
        let out = run_apps(&[("a", src)], bcch(strict), &[SuiteRun::everything()]);
        for t in ["ReaderStatic", "ReaderRecRef"] {
            assert!(
                matches!(outcome(&out, t), Outcome::NotRun(_)),
                "strict {strict}: {out:?}"
            );
        }
    }
}

/// The session language and WorkDate the pass sets do not reach the run's sessions: every test of the session is
/// refused.
#[test]
fn bccontainerhelper_refuses_a_session_whose_discovery_pass_changed_workdate() {
    let src = r#"
codeunit 50760 "Sets WorkDate" { Subtype = Test; trigger OnRun() begin WorkDate(20200101D); end; [Test] procedure T1() begin end; }
codeunit 50761 "Other" { Subtype = Test; [Test] procedure T2() begin end; }
"#;
    let out = run_apps(&[("a", src)], bcch(true), &[SuiteRun::everything()]);
    for t in ["T1", "T2"] {
        assert!(not_run(&out, t, "WorkDate"), "{out:?}");
    }
}

/// Review finding (round 2, LOW): only the rows and the SingleInstance state the discovery pass leaves are pinned
/// (SwOnRunRunner.al:19-26). A number sequence or isolated storage it changed is refused for every codeunit of the
/// session whose reach can read them: the earlier one, the writer itself and the later one.
#[test]
fn bccontainerhelper_refuses_every_reader_of_a_number_sequence_or_isolated_storage_the_pass_changed() {
    let src = r#"
codeunit 50721 "Seq Writer"
{
    Subtype = Test;
    trigger OnRun()
    begin
        if not NumberSequence.Exists('REVSEQ') then
            NumberSequence.Insert('REVSEQ', 1, 1);
        if NumberSequence.Next('REVSEQ') = 0 then;
        if IsolatedStorage.Set('revkey', 'x') then;
    end;

    [Test]
    procedure SeqWriterMethod()
    begin
        Error('writer current %1', NumberSequence.Current('REVSEQ'));
    end;
}
codeunit 50722 "Seq Reader"
{
    Subtype = Test;
    [Test]
    procedure SeqReader()
    begin
        Error('reader current %1 storage %2', NumberSequence.Current('REVSEQ'), IsolatedStorage.Contains('revkey'));
    end;
}
codeunit 50720 "Seq Earlier"
{
    Subtype = Test;
    [Test]
    procedure SeqEarlier()
    begin
        Error('earlier exists %1 storage %2', NumberSequence.Exists('REVSEQ'), IsolatedStorage.Contains('revkey'));
    end;
}
"#;
    for strict in [false, true] {
        let out = run_apps(&[("a", src)], bcch(strict), &[SuiteRun::everything()]);
        for t in ["SeqEarlier", "SeqWriterMethod", "SeqReader"] {
            assert!(
                not_run(&out, t, "number sequence or isolated storage"),
                "strict {strict} {t}: {out:?}"
            );
        }
    }
}

/// The core shape of `LOG`, with System Application's "Server Setting" (a SingleInstance cache of the service tier's
/// settings) read in the writer's OnRun and in both tests, as Microsoft's code reached from a plain Insert reads it in a
/// run with Base Application.
const LOG_SETTINGS: &str = r#"
table 50700 "Log" { fields { field(1; N; Integer) { } } keys { key(PK; N) { Clustered = true; } } }
codeunit 50701 "Writes"
{
    Subtype = Test;
    trigger OnRun()
    var
        L: Record "Log";
        S: Codeunit "Server Setting";
    begin
        if S.GetTestAutomationEnabled() then;
        L.N := L.Count() + 1;
        L.Insert();
    end;

    [Test]
    procedure SeesTwoRows()
    var
        L: Record "Log";
        S: Codeunit "Server Setting";
    begin
        if S.GetTestAutomationEnabled() then;
        if L.Count() <> 2 then
            Error('rows %1', L.Count());
    end;
}
codeunit 50702 "Later"
{
    Subtype = Test;
    [Test]
    procedure SeesTheDiscoveryRowOnly()
    var
        L: Record "Log";
        S: Codeunit "Server Setting";
    begin
        if S.GetTestAutomationEnabled() then;
        if L.Count() <> 1 then
            Error('rows %1', L.Count());
    end;
}
"#;

/// Review finding (round 2, MED 2): with Microsoft's apps loaded, the discovery pass changes SingleInstance state of
/// Microsoft codeunits (System Application's "Server Setting Impl." cache). BC keeps it into the run, in the one session
/// of page 130455 (SwOnRunRunner.al:18-20), so al-runner and serve model BcContainerHelper in BC's order
/// (`TestDriver::one_session`): there the pinned shape runs. In the default order (fresh SingleInstance state per test
/// codeunit) a test that reads such state is refused, which is what al-runner did before.
#[test]
fn with_microsofts_apps_the_pinned_shape_runs_in_the_order_bccontainerhelper_implies() {
    assert!(TestDriver::BcContainerHelper.one_session() && !TestDriver::BcLinux.one_session());
    let system = al_deps::package::read_package(&std::fs::read(al_testfiles::system_app()).unwrap())
        .expect("System.app reads")
        .1;
    let sysapp = al_testfiles::sources("System Application.Source.zip");
    let platform = al_sema::workspace::PLATFORM_APP_ID;
    let run = |strict: bool| {
        let apps = vec![
            AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec!["sysapp".into(), platform.into()],
                files: vec![("main.al".into(), LOG_SETTINGS.to_string())],
            },
            AppSource {
                id: "sysapp".into(),
                name: "System Application".into(),
                dependencies: vec![platform.into()],
                files: sysapp.clone(),
            },
            AppSource {
                id: platform.into(),
                name: "System".into(),
                dependencies: vec![],
                files: system.clone(),
            },
        ];
        let rt = CorpusRuntime::run_suites_with(
            apps,
            vec![
                al_sema::AppKind::Workspace,
                al_sema::AppKind::Dependency,
                al_sema::AppKind::Dependency,
            ],
            &[],
            &Default::default(),
            None,
            Default::default(),
            &[SuiteRun::everything()],
            &bcch(strict),
        );
        let mut out: Vec<_> = rt
            .results()
            .iter()
            .map(|r| (r.codeunit, r.test.clone(), r.outcome.clone()))
            .collect();
        out.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
        out
    };
    let strict = run(true);
    assert_eq!(outcome(&strict, "SeesTwoRows"), Outcome::Pass, "{strict:?}");
    assert_eq!(outcome(&strict, "SeesTheDiscoveryRowOnly"), Outcome::Pass, "{strict:?}");
    let default = run(false);
    assert!(not_run(&default, "SeesTwoRows", "Server Setting Impl."), "{default:?}");
}
