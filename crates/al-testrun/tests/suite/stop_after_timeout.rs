//! Al-runner issue 9213: `RunOptions::stop_after_timeout` ends a run at its first row the time limit stopped. The later
//! tests of that codeunit and every later entry (or, with `stop_group`s, every later entry of its group) are not run and
//! end as not run, with a reason that names the stopped test. Default off: everything runs.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, EntryRun, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

const SRC: &str = r#"
dotnet { assembly("MyAssembly") { type("MyCompany.Settings"; "MySettings") { } } }
codeunit 50720 "Stops At Timeout"
{
    Subtype = Test;
    [Test]
    procedure A_Passes()
    begin
    end;
    [Test]
    procedure B_TimesOut()
    begin
        Sleep(5000);
    end;
    [Test]
    procedure C_Passes()
    begin
    end;
}
codeunit 50721 "Passing Codeunit"
{
    Subtype = Test;
    [Test]
    procedure D_Passes()
    begin
    end;
}
codeunit 50722 "Failing Codeunit"
{
    Subtype = Test;
    [Test]
    procedure F_Fails()
    begin
        Error('fails');
    end;
    [Test]
    procedure G_Passes()
    begin
    end;
}
codeunit 50723 "OnRun Times Out"
{
    Subtype = Test;
    trigger OnRun()
    begin
        Sleep(5000);
    end;
    [Test]
    procedure H_Passes()
    begin
    end;
    [Test]
    procedure I_Passes()
    begin
    end;
}
codeunit 50724 "Spins"
{
    Subtype = Test;
    [Test]
    procedure J_Spins()
    var
        i: Integer;
    begin
        repeat
            i := (i + 1) mod 7919;
        until false;
    end;
    [Test]
    procedure K_Passes()
    begin
    end;
}
codeunit 50725 "Refused"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('H')]
    procedure L_Refused()
    begin
    end;
    [FilterPageHandler]
    procedure H(var Builder: FilterPageBuilder): Boolean
    begin
    end;
}
table 50727 "Hang Row" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { } } }
codeunit 50727 "Untrusted Timeout"
{
    Subtype = Test;
    [Test]
    procedure A_RefusedMidRunWrites()
    var
        W: Record "Hang Row";
    begin
        Sleep(-1);
        W.Insert();
    end;
    [Test]
    procedure B_ReadsThenTimesOut()
    var
        W: Record "Hang Row";
    begin
        if W.IsEmpty() then
            Sleep(5000);
    end;
    [Test]
    procedure C_Passes()
    begin
    end;
}
codeunit 50726 "Waits In DotNet"
{
    Subtype = Test;
    [Test]
    procedure M_Waits()
    var
        S: DotNet MySettings;
        T: Text;
        i: Integer;
    begin
        T := S.Value();
        repeat
            i := (i + 1) mod 7919;
        until false;
    end;
}
"#;

fn run(opts: RunOptions) -> CorpusRuntime {
    run_with(opts, Default::default())
}

fn run_with(opts: RunOptions, dotnet: al_vm::dotnet::DotNetConfig) -> CorpusRuntime {
    CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), SRC.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        dotnet,
        &[SuiteRun::everything()],
        &opts,
    )
}

fn entry(id: u32) -> EntryRun {
    EntryRun {
        only: Some(vec![(0, id)]),
        ..EntryRun::default()
    }
}

fn grouped(id: u32, g: &str) -> EntryRun {
    EntryRun {
        stop_group: Some(g.into()),
        ..entry(id)
    }
}

/// The stop after a timeout, with a time limit of one second, over `entries`.
fn stop_on(entries: Vec<EntryRun>) -> RunOptions {
    RunOptions {
        stop_after_timeout: true,
        test_time_limit: Some(Duration::from_secs(1)),
        entries,
        ..RunOptions::default()
    }
}

fn outcome(rt: &CorpusRuntime, codeunit: u32, name: &str) -> Outcome {
    rt.first_outcome(&CaseId {
        codeunit,
        name: name.into(),
    })
}

/// The rows of each entry, in the order of the entries: (test, outcome, marked as left out by an option).
fn entry_rows(rt: &CorpusRuntime) -> Vec<Vec<(String, Outcome, bool)>> {
    rt.entries
        .iter()
        .map(|r| {
            rt.results()[r.clone()]
                .iter()
                .map(|t| (t.test.clone(), t.outcome.clone(), t.skipped_by_option))
                .collect()
        })
        .collect()
}

/// The reason of a test the stop did not run after the time limit stopped `stopped` (`Codeunit<id>.<test>` or, for a
/// codeunit's OnRun, `Codeunit<id>.OnRun`), with `later` the entries it names.
fn stop_text(stopped: &str, later: &str) -> Outcome {
    let what = match stopped.ends_with(".OnRun") {
        true => format!("the trigger {stopped}"),
        false => format!("the test {stopped}"),
    };
    Outcome::NotRun(format!(
        "{what} was stopped at the time limit before it, and stopAfterTimeout (--stop-after-timeout) runs no test of its codeunit or of {later} after a timeout"
    ))
}

fn is_timeout(o: &Outcome) -> bool {
    matches!(o, Outcome::NotRun(m) if m.starts_with("the test used more than the time limit of "))
}

/// Every row of `rows` is the stop's, for `stopped` and `later`, and marked.
fn all_stopped(rows: &[(String, Outcome, bool)], stopped: &str, later: &str) -> bool {
    let want = stop_text(stopped, later);
    !rows.is_empty() && rows.iter().all(|(_, o, marked)| *o == want && *marked)
}

fn all_pass(rows: &[(String, Outcome, bool)]) -> bool {
    !rows.is_empty() && rows.iter().all(|(_, o, marked)| *o == Outcome::Pass && !marked)
}

#[test]
fn by_default_a_timeout_stops_nothing() {
    let rt = run(RunOptions {
        stop_after_timeout: false,
        ..stop_on(vec![entry(50720), entry(50721)])
    });
    let rows = entry_rows(&rt);
    assert!(is_timeout(&rows[0][1].1), "{rows:?}");
    assert_eq!(rows[0][2].1, Outcome::Pass, "{rows:?}");
    assert!(all_pass(&rows[1]), "{rows:?}");
}

/// Issue 9213: "After a timeout row, the rest of its codeunit and the later entries are not run."
#[test]
fn a_timeout_stops_the_rest_of_its_codeunit_and_the_later_entries() {
    let rt = run(stop_on(vec![entry(50720), entry(50721), entry(50720)]));
    let rows = entry_rows(&rt);
    assert_eq!(rows.len(), 3, "{rows:?}");
    assert_eq!(rows[0][0].1, Outcome::Pass, "{rows:?}");
    assert!(
        is_timeout(&rows[0][1].1) && !rows[0][1].2,
        "the timeout row stays a timeout, unmarked: {rows:?}"
    );
    let named = "Codeunit50720.B_TimesOut";
    assert!(all_stopped(&rows[0][2..], named, "a later entry"), "{rows:?}");
    assert!(all_stopped(&rows[1], named, "a later entry"), "{rows:?}");
    assert!(all_stopped(&rows[2], named, "a later entry"), "{rows:?}");
}

/// Without entries the stop ends the codeunit; the other codeunits still run, as for `stop_after_first_failure`.
#[test]
fn without_entries_the_other_codeunits_still_run() {
    let rt = run(RunOptions {
        only: Some(vec![(0, 50720), (0, 50721)]),
        ..stop_on(Vec::new())
    });
    assert_eq!(
        outcome(&rt, 50720, "C_Passes"),
        stop_text("Codeunit50720.B_TimesOut", "a later entry")
    );
    assert_eq!(outcome(&rt, 50721, "D_Passes"), Outcome::Pass);
}

/// Issue 9213: "With `stopGroup`s, only the later entries of its own group."
#[test]
fn with_groups_a_timeout_stops_only_the_later_entries_of_its_group() {
    let rt = run(stop_on(vec![
        grouped(50720, "m1"),
        grouped(50721, "m2"),
        grouped(50721, "m1"),
    ]));
    let rows = entry_rows(&rt);
    let later = "a later entry of its stopGroup `m1`";
    assert!(
        all_stopped(&rows[0][2..], "Codeunit50720.B_TimesOut", later),
        "{rows:?}"
    );
    assert!(all_pass(&rows[1]), "{rows:?}");
    assert!(all_stopped(&rows[2], "Codeunit50720.B_TimesOut", later), "{rows:?}");
}

/// Issue 9213: the trigger is a time-limit row of the CPU limit too, not only the Sleep budget.
#[test]
fn a_test_stopped_by_its_cpu_time_stops() {
    let rt = run(stop_on(vec![entry(50724), entry(50721)]));
    let rows = entry_rows(&rt);
    assert!(is_timeout(&rows[0][0].1), "{rows:?}");
    assert!(
        all_stopped(&rows[0][1..], "Codeunit50724.J_Spins", "a later entry"),
        "{rows:?}"
    );
    assert!(
        all_stopped(&rows[1], "Codeunit50724.J_Spins", "a later entry"),
        "{rows:?}"
    );
}

/// Issue 9213: an OnRun cut stops; the rows name the trigger, not a test that did not cause the timeout.
#[test]
fn an_onrun_cut_stops_naming_the_trigger() {
    let rt = run(stop_on(vec![entry(50723), entry(50721)]));
    let rows = entry_rows(&rt);
    assert!(rows[0].iter().all(|(_, o, _)| is_timeout(o)), "{rows:?}");
    assert!(
        all_stopped(&rows[1], "Codeunit50723.OnRun", "a later entry"),
        "{rows:?}"
    );
}

/// Issue 9213: "Never a failure, a refusal, a row a stop skipped". Each alone keeps its behavior.
#[test]
fn a_failure_or_a_refusal_does_not_stop_and_the_failure_stop_ignores_a_timeout() {
    let rt = run(stop_on(vec![entry(50722), entry(50725), entry(50721)]));
    let rows = entry_rows(&rt);
    assert!(matches!(rows[0][0].1, Outcome::Fail(_)), "{rows:?}");
    assert_eq!(rows[0][1].1, Outcome::Pass, "{rows:?}");
    assert!(
        matches!(&rows[1][0].1, Outcome::NotRun(m) if !m.contains("time limit")),
        "{rows:?}"
    );
    assert!(all_pass(&rows[2]), "{rows:?}");
    let failure_only = run(RunOptions {
        stop_after_timeout: false,
        stop_after_first_failure: true,
        ..stop_on(vec![entry(50720), entry(50721)])
    });
    let rows = entry_rows(&failure_only);
    assert_eq!(rows[0][2].1, Outcome::Pass, "{rows:?}");
    assert!(all_pass(&rows[1]), "{rows:?}");
}

/// Issue 9213: "With `stopAfterFirstFailure` too, either trigger stops, and the first triggering row is named."
#[test]
fn with_both_stops_the_first_triggering_row_is_named() {
    let both = |entries| RunOptions {
        stop_after_first_failure: true,
        ..stop_on(entries)
    };
    let rows = entry_rows(&run(both(vec![entry(50720), entry(50722)])));
    assert!(
        all_stopped(&rows[1], "Codeunit50720.B_TimesOut", "a later entry"),
        "{rows:?}"
    );
    let rows = entry_rows(&run(both(vec![entry(50722), entry(50720)])));
    let failure = Outcome::NotRun("the test Codeunit50722.F_Fails failed before it, and stopAfterFirstFailure (--stop-after-first-failure) runs no test of its codeunit or of a later entry after a failure".into());
    assert!(
        rows[1].iter().all(|(_, o, marked)| *o == failure && *marked),
        "{rows:?}"
    );
}

/// Issue 9213: "With `skipAfterTimeout` too, `stopAfterTimeout` includes it, and the codeunit's rows carry the one stop
/// text."
#[test]
fn with_skip_after_timeout_too_the_codeunit_carries_the_stop_text() {
    let rt = run(RunOptions {
        skip_after_timeout: true,
        ..stop_on(vec![entry(50720), entry(50721)])
    });
    let rows = entry_rows(&rt);
    assert!(
        all_stopped(&rows[0][2..], "Codeunit50720.B_TimesOut", "a later entry"),
        "{rows:?}"
    );
    assert!(
        all_stopped(&rows[1], "Codeunit50720.B_TimesOut", "a later entry"),
        "{rows:?}"
    );
}

/// Issue 9213 (review of the pull request): a timeout the judging does not trust, because an earlier test of the codeunit
/// was refused while it ran and wrote what it reads, still hit the time limit. It stops as a trusted one does, alone and
/// with the failure stop too: the trigger is the row before the judging, as for `skip_after_timeout`.
#[test]
fn an_untrusted_timeout_stops_the_codeunit_and_the_later_entries() {
    for failure in [false, true] {
        let rt = run(RunOptions {
            stop_after_first_failure: failure,
            ..stop_on(vec![entry(50727), entry(50721)])
        });
        let rows = entry_rows(&rt);
        assert!(
            matches!(&rows[0][1].1, Outcome::NotRun(m) if m.contains("this timeout is not trusted: the test used more than the time limit")),
            "failure stop {failure}: {rows:?}"
        );
        let named = "Codeunit50727.B_ReadsThenTimesOut";
        assert!(
            all_stopped(&rows[0][2..], named, "a later entry"),
            "failure stop {failure}: {rows:?}"
        );
        assert!(
            all_stopped(&rows[1], named, "a later entry"),
            "failure stop {failure}: {rows:?}"
        );
    }
}

/// Test codeunits whose OnRun writes isolated storage: under BcContainerHelper's discovery pass every codeunit of the
/// session whose reach can read it is refused after its test loop, its failure or its timeout included. 50732 reads
/// nothing of it and runs.
const REFUSED_AFTER_THE_LOOP: &str = r#"
codeunit 50730 "Fails Then Refused"
{
    Subtype = Test;
    trigger OnRun()
    begin
        if IsolatedStorage.Set('k9213', 'x') then;
    end;
    [Test]
    procedure F_Fails()
    begin
        Error('fails');
    end;
}
codeunit 50731 "Times Out Then Refused"
{
    Subtype = Test;
    trigger OnRun()
    begin
        if IsolatedStorage.Set('k9213', 'x') then;
    end;
    [Test]
    procedure T_TimesOut()
    begin
        Sleep(5000);
    end;
}
codeunit 50732 "Runs After"
{
    Subtype = Test;
    [Test]
    procedure N_Passes()
    begin
    end;
}
"#;

/// Review of the pull request for issue 9213, round 2: a codeunit whose rows are all refused after its loop (here the
/// discovery pass of `--test-driver bccontainerhelper` changed isolated storage it can read) has no row that stops. Its
/// failure, or its timeout, was replaced by the refusal, so the next entry runs, as on main for the failure stop.
#[test]
fn a_codeunit_refused_after_its_loop_stops_no_later_entry() {
    let src_run = |opts: RunOptions| {
        CorpusRuntime::run_suites_with(
            vec![AppSource {
                id: "main".into(),
                name: "main".into(),
                dependencies: vec![],
                files: vec![("main.al".into(), REFUSED_AFTER_THE_LOOP.into())],
            }],
            vec![al_sema::AppKind::Workspace],
            &[],
            &Default::default(),
            None,
            Default::default(),
            &[SuiteRun::everything()],
            &RunOptions {
                test_driver: al_testrun::runtime::TestDriver::BcContainerHelper,
                strict_order: true,
                ..opts
            },
        )
    };
    let failure = src_run(RunOptions {
        stop_after_first_failure: true,
        entries: vec![entry(50730), entry(50732)],
        ..RunOptions::default()
    });
    let timeout = src_run(stop_on(vec![entry(50731), entry(50732)]));
    // Both stops are checked before the assertion, so a red run shows each.
    let wrong: Vec<String> = [("failure", failure), ("timeout", timeout)]
        .into_iter()
        .map(|(stop, rt)| (stop, entry_rows(&rt)))
        .filter(|(_, rows)| {
            !(matches!(&rows[0][0].1, Outcome::NotRun(m) if m.contains("isolated storage")) && all_pass(&rows[1]))
        })
        .map(|(stop, rows)| format!("{stop} stop: {rows:?}"))
        .collect();
    assert!(wrong.is_empty(), "the next entry must run: {wrong:#?}");
}

/// A .NET host whose call `Value()` waits `wait` before it answers, past the wall backstop of the test's time limit:
/// with `deadline` it answers as a sidecar whose call deadline passed (it was killed), else with a text. It counts the
/// sessions it opened and closed.
#[derive(Clone)]
struct Waiting {
    wait: Duration,
    deadline: bool,
    opened: Arc<AtomicUsize>,
    closed: Arc<AtomicUsize>,
}

impl al_host::dotnet::DotNetSession for Waiting {
    fn construct(
        &mut self,
        _: &al_host::dotnet::DotNetType,
        _: Vec<al_host::dotnet::DotNetArg>,
    ) -> Result<al_host::dotnet::DotNetReply, al_host::dotnet::DotNetFailure> {
        Err(al_host::dotnet::DotNetFailure::Refused("not served".into()))
    }
    fn invoke(
        &mut self,
        _: al_host::dotnet::DotNetTarget,
        _: &str,
        _: Vec<al_host::dotnet::DotNetArg>,
    ) -> Result<al_host::dotnet::DotNetReply, al_host::dotnet::DotNetFailure> {
        std::thread::sleep(self.wait);
        match self.deadline {
            true => Err(al_host::dotnet::DotNetFailure::Refused(
                "the call timed out after 1 s; the sidecar was killed".into(),
            )),
            false => Ok(al_host::dotnet::DotNetReply::value(al_host::dotnet::DotNetValue::Str(
                "answered".into(),
            ))),
        }
    }
    fn get_property(
        &mut self,
        _: al_host::dotnet::DotNetTarget,
        _: &str,
    ) -> Result<al_host::dotnet::DotNetReply, al_host::dotnet::DotNetFailure> {
        Err(al_host::dotnet::DotNetFailure::Refused("not served".into()))
    }
    fn set_property(
        &mut self,
        _: al_host::dotnet::DotNetTarget,
        _: &str,
        _: al_host::dotnet::DotNetValue,
    ) -> Result<al_host::dotnet::DotNetReply, al_host::dotnet::DotNetFailure> {
        Err(al_host::dotnet::DotNetFailure::Refused("not served".into()))
    }
    fn release(&mut self, _: &[al_host::dotnet::DotNetHandle]) {}
    fn close(self: Box<Self>) {
        self.closed.fetch_add(1, Ordering::SeqCst);
    }
}

impl al_host::dotnet::DotNetHost for Waiting {
    fn availability(&self) -> Result<(), al_host::dotnet::DotNetUnavailable> {
        Ok(())
    }
    fn open_session(&self, _: &al_host::dotnet::DotNetSessionConfig) -> Box<dyn al_host::dotnet::DotNetSession> {
        self.opened.fetch_add(1, Ordering::SeqCst);
        Box::new(self.clone())
    }
}

/// Issue 9213, "Daemon cleanliness": a test is never stopped inside a helper call. A call that waits past the wall
/// backstop either returns, and the stop lands at the next check, or fails at the helper's deadline; either way the row
/// is a timeout of the wall clock, the stop ends the later entries of its group, and every helper session the run opened
/// is closed (the sidecar's `close` decides whether its process is reused).
#[test]
fn a_test_waiting_in_a_dotnet_call_stops_after_the_call_and_its_session_is_closed() {
    for deadline in [false, true] {
        let host = Waiting {
            wait: Duration::from_millis(1500),
            deadline,
            opened: Default::default(),
            closed: Default::default(),
        };
        let rt = run_with(
            RunOptions {
                test_time_limit: Some(Duration::from_millis(100)),
                ..stop_on(vec![grouped(50726, "m1"), grouped(50721, "m1"), grouped(50721, "m2")])
            },
            al_vm::dotnet::DotNetConfig {
                host: Arc::new(host.clone()),
                ..Default::default()
            },
        );
        let rows = entry_rows(&rt);
        assert!(
            matches!(&rows[0][0].1, Outcome::NotRun(m) if m.starts_with("the test used more than the time limit of ") && m.contains("wall")),
            "deadline {deadline}: {rows:?}"
        );
        let later = "a later entry of its stopGroup `m1`";
        assert!(
            all_stopped(&rows[1], "Codeunit50726.M_Waits", later),
            "deadline {deadline}: {rows:?}"
        );
        assert!(all_pass(&rows[2]), "deadline {deadline}: {rows:?}");
        let (opened, closed) = (host.opened.load(Ordering::SeqCst), host.closed.load(Ordering::SeqCst));
        assert!(opened >= 1, "deadline {deadline}: no session opened");
        assert_eq!(opened, closed, "deadline {deadline}: every session is closed");
    }
}
