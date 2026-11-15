//! The install code of an app with no dependency app runs once per run when what it left is only what a codeunit's
//! session can start from (al-runner issue 8144, al-testrun `install_once.rs`, al-vm `Session::install_left`). Install
//! code that leaves any other session state keeps running in each test codeunit's session, as before: every case below
//! gives that run's answer and counts one install run per codeunit.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::memory::MemoryBudget;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};
use std::sync::Arc;

fn manifest(target: &str) -> Manifest {
    Manifest {
        publisher: "Me".into(),
        version: "1.0.0.0".into(),
        resources: vec![],
        include_source: false,
        target: target.into(),
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

/// The app alone (no dependency app, so no install baseline runs) under `opts`.
fn run_with(src: &str, target: &str, opts: &RunOptions) -> CorpusRuntime {
    CorpusRuntime::try_run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), src.to_string())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[manifest(target)],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        opts,
    )
    .expect("the run starts")
}

/// Every test's failure text, by codeunit.
fn failures(rt: &CorpusRuntime) -> Vec<(u32, String)> {
    let mut v: Vec<(u32, String)> = rt
        .results()
        .iter()
        .map(|r| match &r.outcome {
            Outcome::Fail(m) => (r.codeunit, m.clone()),
            other => (r.codeunit, format!("not a failure: {other:?}")),
        })
        .collect();
    v.sort();
    v
}

const ROW: &str = "table 60500 \"Io Row\" { fields { field(1; Code; Code[20]) { } field(2; Value; Text[2048]) { } field(3; Pic; Media) { } field(4; Pics; MediaSet) { } field(5; B; Blob) { } field(6; G; Guid) { } } keys { key(PK; Code) { } } }";

/// Install code `install` (the body of OnInstallAppPerCompany, with locals `locals`), then two test codeunits whose test
/// fails with `Error(<read>)`; `on_run`: the codeunit's OnRun sets the global `Saw` first, which `read` may use.
fn app(locals: &str, install: &str, on_run: &str, read: &str) -> String {
    let tests: String = [60510, 60511]
        .iter()
        .map(|id| {
            format!(
                "codeunit {id} \"Io T{id}\" {{ Subtype = Test; var Saw: Text;
                    trigger OnRun() begin {on_run} end;
                    [Test] procedure Reads() var S: Record \"Io Row\"; T2: Record \"Io Row\" temporary; OS: OutStream; Ok: Boolean;
                    begin Error({read}); end; }}\n"
            )
        })
        .collect();
    format!(
        "{ROW}
        codeunit 60501 \"Io Installer\" {{ Subtype = Install;
            trigger OnInstallAppPerCompany() var S: Record \"Io Row\"; T: Record \"Io Row\" temporary; OS: OutStream; IStr: InStream; {locals}
            begin {install} end; }}
        codeunit 60502 \"Io Try\" {{ [TryFunction] procedure Fails() begin Error('install-try-error'); end; }}
        codeunit 60503 \"Io Task\" {{ trigger OnRun() begin end; }}
        {tests}"
    )
}

/// Runs `src` and checks that both test codeunits fail with `want` and that the install code ran once per codeunit, after
/// the one run in a session of its own whose verdict was that a codeunit's session cannot start from it.
fn keeps_install_per_codeunit(src: &str, target: &str, want: &str) {
    let rt = run_with(src, target, &RunOptions::default());
    let got = failures(&rt);
    assert_eq!(
        got,
        vec![(60510, want.to_string()), (60511, want.to_string())],
        "{got:?}"
    );
    assert_eq!(rt.phases.install_runs, 1 + 2, "{got:?}");
}

const MEDIA_IN: &str = "T.B.CreateOutStream(OS); OS.WriteText('picture-bytes'); T.B.CreateInStream(IStr);";

#[test]
fn install_code_that_imports_a_media_set_runs_per_codeunit() {
    let install = format!("{MEDIA_IN} S.Code := 'SET'; S.Pics.ImportStream(IStr, 'set'); S.Insert();");
    let src = app("", &install, "", "'x'").replace(
        "begin Error('x'); end; }",
        "begin S.Get('SET'); Error('count=%1', S.Pics.Count()); end; }",
    );
    keeps_install_per_codeunit(&src, "", "count=1");
}

#[test]
fn install_code_that_imports_a_media_runs_per_codeunit() {
    let install = format!("{MEDIA_IN} S.Code := 'PIC'; S.Pic.ImportStream(IStr, 'pic'); S.Insert();");
    let src = app("", &install, "", "'x'").replace(
        "begin Error('x'); end; }",
        "begin S.Get('PIC'); T2.B.CreateOutStream(OS); Ok := S.Pic.ExportStream(OS); Error('exported=%1 len=%2', Ok, T2.B.Length()); end; }",
    );
    keeps_install_per_codeunit(&src, "", "exported=Yes len=13");
}

#[test]
fn install_code_that_encrypts_runs_per_codeunit() {
    let src = app(
        "",
        "S.Code := 'ENC'; S.Value := Encrypt('secret'); S.Insert();",
        "",
        "'x'",
    )
    .replace(
        "begin Error('x'); end; }",
        "begin S.Get('ENC'); Error('plain=%1', Decrypt(S.Value)); end; }",
    );
    keeps_install_per_codeunit(&src, "", "plain=secret");
}

#[test]
fn install_code_that_deletes_the_encryption_key_runs_per_codeunit() {
    let src = app(
        "",
        "if EncryptionKeyExists() then DeleteEncryptionKey();",
        "",
        "StrSubstNo('keyexists=%1 enabled=%2', EncryptionKeyExists(), EncryptionEnabled())",
    );
    keeps_install_per_codeunit(&src, "", "keyexists=No enabled=No");
}

#[test]
fn install_code_that_creates_a_file_runs_per_codeunit() {
    let src = app(
        "F: File;",
        "F.TextMode(true); F.Create(TemporaryPath() + 'io8153.txt'); F.Write('x'); F.Close();",
        "",
        "StrSubstNo('exists=%1', Exists(TemporaryPath() + 'io8153.txt'))",
    );
    keeps_install_per_codeunit(&src, "OnPrem", "exists=Yes");
}

#[test]
fn install_code_that_schedules_a_task_runs_per_codeunit() {
    let src = app(
        "",
        "S.Code := 'TASK'; S.G := TaskScheduler.CreateTask(Codeunit::\"Io Task\", 0, false, CompanyName(), CurrentDateTime() + 3600000); S.Insert();",
        "",
        "'x'",
    )
    .replace(
        "begin Error('x'); end; }",
        "begin S.Get('TASK'); Error('task=%1', TaskScheduler.TaskExists(S.G)); end; }",
    );
    keeps_install_per_codeunit(&src, "", "task=Yes");
}

#[test]
fn install_code_that_registers_a_table_connection_runs_per_codeunit() {
    let src = app(
        "",
        "RegisterTableConnection(TableConnectionType::CRM, 'ioconn', '@@test@@');",
        "",
        "StrSubstNo('conn=%1', HasTableConnection(TableConnectionType::CRM, 'ioconn'))",
    );
    keeps_install_per_codeunit(&src, "", "conn=Yes");
}

#[test]
fn install_code_whose_try_function_failed_runs_per_codeunit() {
    let src = app(
        "Try: Codeunit \"Io Try\";",
        "if Try.Fails() then;",
        "Saw := GetLastErrorText();",
        "StrSubstNo('last=[%1]', Saw)",
    );
    keeps_install_per_codeunit(&src, "", "last=[install-try-error]");
}

#[test]
fn install_code_that_sets_the_application_area_runs_per_codeunit() {
    let src = app(
        "",
        "ApplicationArea('#IoArea');",
        "Saw := ApplicationArea();",
        "StrSubstNo('area=[%1]', Saw)",
    );
    keeps_install_per_codeunit(&src, "", "area=[#IoArea]");
}

#[test]
fn install_code_that_reseeds_random_runs_per_codeunit() {
    let src = app(
        "",
        "Randomize(42);",
        "Saw := Format(Random(100000));",
        "StrSubstNo('rnd=%1', Saw)",
    );
    keeps_install_per_codeunit(&src, "", "rnd=66811");
}

/// Install code that writes rows, takes a NumberSequence number and an AutoIncrement value.
const ROWS_ONLY: &str = "table 60520 \"Io Seq\" { fields { field(1; No; Integer) { AutoIncrement = true; } field(2; Code; Code[20]) { } } keys { key(PK; No) { } } }
    table 60521 \"Io Seeded\" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { } } }
    codeunit 60522 \"Io Rows Installer\" { Subtype = Install;
        trigger OnInstallAppPerCompany() var S: Record \"Io Seeded\"; Q: Record \"Io Seq\"; N: BigInteger;
        begin S.Code := 'A'; S.Insert(); S.Code := 'B'; S.Insert(); Q.Code := 'I'; Q.Insert();
            NumberSequence.Insert('IOSEQ'); N := NumberSequence.Next('IOSEQ'); end; }";

/// `n` test codeunits that read what [`ROWS_ONLY`] left: the rows, the next AutoIncrement value and the next number.
fn rows_readers(n: u32) -> String {
    (0..n)
        .map(|i| {
            let id = 60530 + i;
            format!(
                "codeunit {id} \"Io Reader {id}\" {{ Subtype = Test;
                    [Test] procedure Reads() var S: Record \"Io Seeded\"; Q: Record \"Io Seq\"; I: Integer;
                    begin for I := 1 to 20000 do; Q.Code := 'T'; Q.Insert();
                        Error('rows=%1 auto=%2 next=%3', S.Count(), Q.No, NumberSequence.Next('IOSEQ')); end; }}\n"
            )
        })
        .collect()
}

/// What each of `n` readers sees; `carried`: the number sequence carries from one codeunit to the next (`--strict-order`).
fn rows_only_wants(rt: &CorpusRuntime, n: u32, carried: bool) {
    let got = failures(rt);
    let want: Vec<(u32, String)> = (0..n)
        .map(|i| {
            let next = if carried { i + 1 } else { 1 };
            (60530 + i, format!("rows=2 auto=2 next={next}"))
        })
        .collect();
    assert_eq!(got, want, "{got:?}");
}

/// The neighbor of every case above: install code that writes only rows, an AutoIncrement value and a number runs once
/// for the run, and each codeunit's session starts from what it left, as when it ran in that session.
#[test]
fn install_code_that_writes_only_rows_runs_once_per_run() {
    let src = format!("{ROWS_ONLY}\n{}", rows_readers(2));
    let rt = run_with(&src, "", &RunOptions::default());
    rows_only_wants(&rt, 2, false);
    assert_eq!(rt.phases.install_runs, 1);
}

/// A budget with room for many test threads.
fn roomy() -> Option<MemoryBudget> {
    Some(MemoryBudget {
        limit: 64 << 30,
        source: "--max-memory".into(),
        rss: Arc::new(|| Some(1 << 20)),
        hard: false,
    })
}

/// Codeunits on several test threads: every thread starts each codeunit from the one install run of the run.
#[test]
fn install_code_runs_once_for_all_test_threads() {
    let src = format!("{ROWS_ONLY}\n{}", rows_readers(8));
    let rt = run_with(
        &src,
        "",
        &RunOptions {
            memory: roomy(),
            ..Default::default()
        },
    );
    rows_only_wants(&rt, 8, false);
    assert!(rt.phases.memory.most >= 2, "{:?}", rt.phases.memory);
    assert_eq!(rt.phases.install_runs, 1);
}

/// `--strict-order`: the codeunits run one after the other, the number sequence carries from one to the next (as when the
/// install code ran in each codeunit's session), and the install code runs once.
#[test]
fn install_code_runs_once_under_strict_order() {
    let src = format!("{ROWS_ONLY}\n{}", rows_readers(3));
    let rt = run_with(
        &src,
        "",
        &RunOptions {
            strict_order: true,
            ..Default::default()
        },
    );
    rows_only_wants(&rt, 3, true);
    assert_eq!(rt.phases.install_runs, 1);
}

/// A test runner's TestIsolation = Disabled: each later codeunit starts from what the codeunit before it left (a carried
/// baseline), which holds what the install code left once.
#[test]
fn a_codeunit_that_starts_from_a_carried_baseline_sees_the_install_rows_once() {
    let readers: String = (0..3)
        .map(|i| {
            let id = 60530 + i;
            let end = if i == 2 {
                "Error('rows=%1 auto=%2', S.Count(), Q.No);"
            } else {
                ""
            };
            format!(
                "codeunit {id} \"Io Reader {id}\" {{ Subtype = Test;
                    [Test] procedure Reads() var S: Record \"Io Seeded\"; Q: Record \"Io Seq\";
                    begin Q.Code := 'T'; Q.Insert(); {end} end; }}\n"
            )
        })
        .collect();
    let rt = run_with(
        &format!("{ROWS_ONLY}\n{readers}"),
        "",
        &RunOptions {
            isolation: Some(al_testrun::isolation::TestIsolation::Disabled),
            ..Default::default()
        },
    );
    let got: Vec<(u32, String)> = rt
        .results()
        .iter()
        .map(|r| (r.codeunit, format!("{:?}", r.outcome)))
        .collect();
    assert_eq!(
        got,
        vec![
            (60530, "Pass".to_string()),
            (60531, "Pass".to_string()),
            (60532, "Fail(\"rows=2 auto=4\")".to_string())
        ],
        "{got:?}"
    );
    assert_eq!(rt.phases.install_runs, 1, "{got:?}");
}
