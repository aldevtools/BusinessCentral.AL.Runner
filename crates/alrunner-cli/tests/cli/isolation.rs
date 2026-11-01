//! Test isolation, outbound requests, partial runs and profiles.

use super::*;

/// Issues 5609 and 5607: a test that calls Commit() runs under `--isolation disabled` and `--isolation test`, as under a
/// test runner codeunit with that TestIsolation (corpus PR 735, IsolProbeCommit.Codeunit.al and SdIsolCommitError.Codeunit.al
/// under runners 61300 and 61304).
#[test]
fn a_commit_runs_under_every_isolation() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path().join("commit");
    write(
        &dir,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000005607", "Commit", &[]),
    );
    write(
        &dir,
        "src/Tests.al",
        "codeunit 50660 \"Commit Tests\"\n{\n    Subtype = Test;\n\n    [Test]\n    procedure CommitCall()\n    begin\n        Commit();\n    end;\n}\n",
    );
    let a = dir.to_str().unwrap();
    for isolation in ["disabled", "test", "codeunit"] {
        let o = alrunner(&["test", a, "--no-download", "--isolation", isolation]);
        assert_eq!(code(&o), 0, "{isolation}: {}", text(&o));
    }
}

/// Issue 5609: under `--isolation disabled` StartSession starts the session, it ends, and the test sees its committed row
/// (tests/al-language-isolation-probe/src/SdIsolStartSession.Codeunit.al:22-30, runner 61304). Under `--isolation test` it
/// fails with BC's text (:20-21, runner 61300).
#[test]
fn start_session_runs_under_isolation_disabled() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path().join("session");
    write(
        &dir,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000005609", "Session", &[]),
    );
    write(
        &dir,
        "src/Tests.al",
        "table 50680 \"Session Row\" { fields { field(1; K; Code[20]) { } } keys { key(PK; K) { Clustered = true; } } }\n\
         codeunit 50681 \"Session Writer\" { trigger OnRun() var R: Record \"Session Row\"; begin R.K := 'SESSION'; R.Insert(); end; }\n\
         codeunit 50682 \"Session Tests\"\n{\n    Subtype = Test;\n\n    [Test]\n    procedure Starts()\n    var\n        R: Record \"Session Row\";\n        Id: Integer;\n    begin\n\
                 Session.StartSession(Id, Codeunit::\"Session Writer\");\n        if Session.IsSessionActive(Id) then Error('active');\n\
                 if not R.Get('SESSION') then Error('no row');\n    end;\n}\n",
    );
    let a = dir.to_str().unwrap();
    let o = alrunner(&["test", a, "--no-download", "--isolation", "disabled"]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    let o = alrunner(&["test", a, "--no-download", "--isolation", "test"]);
    assert_eq!(code(&o), 1, "{}", text(&o));
    assert!(
        text(&o).contains("Sessions can only be started in tests"),
        "{}",
        text(&o)
    );
}

/// Issues 5607 and 5609 with RequiredTestIsolation on the test codeunit: `--isolation required` runs each test codeunit
/// under its own RequiredTestIsolation, as BC's TestRunnerHub does on BC 28 and later (corpus
/// codeunit/lt-qu-commit-isolation/QuCommitIsolation.al:10-16, 25-39).
const REQUIRED: &str = r#"
table 50670 "Req Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 50670 "Req Function"
{
    Subtype = Test;
    RequiredTestIsolation = Function;

    [Test]
    procedure A_CommitCall()
    var
        R: Record "Req Row";
    begin
        R.Id := 1;
        R.Insert();
        Commit();
    end;

    [Test]
    procedure B_NextTestSeesNothing()
    var
        R: Record "Req Row";
    begin
        if R.Count() <> 0 then
            Error('kept');
    end;
}
codeunit 50671 "Req Disabled A"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure Passes()
    var
        R: Record "Req Row";
    begin
        R.Id := 2;
        R.Insert();
    end;
}
codeunit 50672 "Req Disabled B"
{
    Subtype = Test;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure SeesTheRow()
    var
        R: Record "Req Row";
    begin
        if not R.Get(2) then
            Error('gone');
    end;
}
"#;

/// A codeunit the TestRunnerHub mode still refuses: RequiredTestIsolation = Function with handlers.
const REQUIRED_HANDLERS: &str = r#"
codeunit 50675 "Req Handlers"
{
    Subtype = Test;
    RequiredTestIsolation = Function;

    [Test]
    [HandlerFunctions('Msg')]
    procedure Shows()
    begin
        Message('x');
    end;

    [MessageHandler]
    procedure Msg(Text: Text[1024])
    begin
    end;
}
"#;

#[test]
fn isolation_required_runs_each_codeunit_under_its_required_test_isolation() {
    let d = tempfile::tempdir().unwrap();
    let app = d.path().join("req");
    write(
        &app,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000005670", "Req", &[]),
    );
    write(&app, "T.al", REQUIRED);
    let a = app.to_str().unwrap();
    let run = |extra: &[&str]| {
        let mut args = vec!["test", a, "--no-download", "--no-test-data", "--output-json"];
        args.extend_from_slice(extra);
        alrunner(&args)
    };
    // BC 28 and later: Function returns the next test to its start, the Commit() included (QuCommitIsolation.al:466-488);
    // Disabled keeps the row for the next codeunit (QuCommitIsolation.al:695-718).
    let o = run(&["--isolation", "required"]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert_eq!(json_of(&o)["passed"], 4, "{}", text(&o));
    // The default is unchanged: Codeunit isolation for every codeunit, so the second test sees the first one's row.
    let o = run(&[]);
    assert_eq!(code(&o), 1, "{}", text(&o));
    assert_eq!(
        row(&json_of(&o), "Codeunit50670.B_NextTestSeesNothing")["message"],
        "kept"
    );
    assert_eq!(row(&json_of(&o), "Codeunit50672.SeesTheRow")["message"], "gone");
    // BC 27: every codeunit runs under Codeunit isolation (QuCommitIsolation.al:36-40, the BC28PLUS guards), and the
    // report says so.
    let o = run(&["--isolation", "required", "--bc-version", "27.5.46862.48827"]);
    assert_eq!(code(&o), 1, "{}", text(&o));
    assert_eq!(
        row(&json_of(&o), "Codeunit50670.B_NextTestSeesNothing")["message"],
        "kept"
    );
    assert!(
        text(&o).contains("Business Central 27 runs every test codeunit under TestIsolation = Codeunit"),
        "{}",
        text(&o)
    );
    // A Commit() under --isolation test is rolled back with the test, as under a runner with TestIsolation = Function
    // (corpus PR 735, IsolProbeCommit.Codeunit.al C1-C3 under runner 61300; al-runner issue 5607).
    let o = run(&["--isolation", "test"]);
    assert_eq!(
        row(&json_of(&o), "Codeunit50670.A_CommitCall")["status"],
        "pass",
        "{}",
        text(&o)
    );
    assert_eq!(
        row(&json_of(&o), "Codeunit50670.B_NextTestSeesNothing")["status"],
        "pass",
        "{}",
        text(&o)
    );
    // RequiredTestIsolation = Disabled has no effect under it: the next codeunit does not see the row.
    assert_eq!(row(&json_of(&o), "Codeunit50672.SeesTheRow")["message"], "gone");
    // A codeunit with handlers runs under runner 130450, as Codeunit isolation, whatever its RequiredTestIsolation says
    // (corpus codeunit/lt-qu-commit-isolation/RqIsolationModes.al:673-839).
    let h = d.path().join("handlers");
    write(
        &h,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000005675", "ReqHandlers", &[]),
    );
    write(&h, "T.al", REQUIRED_HANDLERS);
    let o = alrunner(&[
        "test",
        h.to_str().unwrap(),
        "--no-download",
        "--no-test-data",
        "--output-json",
        "--isolation",
        "required",
    ]);
    assert_eq!(code(&o), 0, "{}", text(&o));
}
/// Issue 5608: an outbound request no handler answers fails with BC's pinned error where BC fails it without a
/// connection (an internal address: PyHttpPolicy.al:82-90) and under BlockOutboundRequests (AyHttpHandler.al:121); a
/// request BC sends for real is refused with a reason that says why and what to do instead.
#[test]
fn an_unhandled_outbound_request_fails_as_bc_does_or_says_why_it_is_refused() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path().join("http");
    write(
        &dir,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000005608", "Http", &[]),
    );
    let unit = |id: u32, policy: &str| {
        format!(
            "codeunit {id} \"Http Tests {id}\"\n{{\n    Subtype = Test;\n{policy}\n    [Test]\n    procedure Local()\n    var\n        Client: HttpClient;\n        Response: HttpResponseMessage;\n    begin\n        Client.Get('http://localhost:1/x', Response);\n    end;\n\n    [Test]\n    procedure Public()\n    var\n        Client: HttpClient;\n        Response: HttpResponseMessage;\n    begin\n        Client.Get('http://example.com/x', Response);\n    end;\n}}\n"
        )
    };
    write(&dir, "src/Default.al", &unit(50670, ""));
    write(
        &dir,
        "src/Block.al",
        &unit(50671, "    TestHttpRequestPolicy = BlockOutboundRequests;"),
    );
    let o = alrunner(&["test", dir.to_str().unwrap(), "--no-download", "--output-json"]);
    let doc = json_of(&o);
    let message = |name: &str| row(&doc, name)["message"].as_str().unwrap().to_string();
    assert!(
        message("Codeunit50670.Local").contains("violates security requirements"),
        "{doc}"
    );
    assert!(
        message("Codeunit50671.Public").starts_with("Unhandled outbound HTTP requests are not permitted"),
        "{doc}"
    );
    let m = message("Codeunit50670.Public");
    assert!(
        m.contains("[HttpClientHandler]") && m.contains("BlockOutboundRequests"),
        "{m}"
    );
}

/// Issue 5611: --run-selected-only runs only the selected tests of a codeunit, as BC's test tool runs only the test
/// methods that are enabled: a sibling that is not selected does not run, so its global does not carry over.
#[test]
fn run_selected_only_leaves_the_other_tests_of_the_codeunit_out() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path().join("persist");
    write(
        &dir,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000005611", "Persist", &[]),
    );
    write(
        &dir,
        "src/Tests.al",
        "codeunit 50680 \"Persist Tests\"\n{\n    Subtype = Test;\n\n    var\n        Counter: Integer;\n\n    [Test]\n    procedure PersistA()\n    begin\n        Counter += 1;\n    end;\n\n    [Test]\n    procedure PersistB()\n    begin\n        if Counter <> 0 then\n            Error('counter=%1', Counter);\n    end;\n}\n",
    );
    let a = dir.to_str().unwrap();
    let o = alrunner(&["test", a, "--no-download", "--test-exact", "Codeunit50680.PersistB"]);
    assert_eq!(code(&o), 1, "the whole codeunit runs by default: {}", text(&o));
    assert!(text(&o).contains("counter=1"), "{}", text(&o));
    let o = alrunner(&[
        "test",
        a,
        "--no-download",
        "--test-exact",
        "Codeunit50680.PersistB",
        "--run-selected-only",
    ]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(text(&o).contains("Tests: 1 total, 1 passed"), "{}", text(&o));
}

/// Issue 6133 (finding 3): `AL_PROFILE` writes its file when the run ends, also for a run shorter than the profiler's
/// periodic write (5 s), such as one test with `--run-selected-only`.
#[test]
fn al_profile_writes_its_file_at_the_end_of_a_short_run() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path().join("prof");
    write(
        &dir,
        "app.json",
        &app_json("11111111-0000-0000-0000-000000006133", "Prof", &[]),
    );
    write(
        &dir,
        "src/Tests.al",
        "codeunit 50682 \"Prof Tests\"\n{\n    Subtype = Test;\n\n    [Test]\n    procedure Loop()\n    var\n        i: Integer;\n        n: Integer;\n    begin\n        for i := 1 to 20000 do\n            n += i;\n    end;\n}\n",
    );
    let file = d.path().join("profile.txt");
    let o = Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .args([
            "test",
            dir.to_str().unwrap(),
            "--no-download",
            "--test-exact",
            "Codeunit50682.Loop",
            "--run-selected-only",
        ])
        .env("AL_CHECK_THREADS", "2")
        .env("AL_LOWER_THREADS", "2")
        .env("AL_TEST_THREADS", "2")
        .env("ALRUNNER_NO_PATH_NOTE", "1")
        .env("AL_PROFILE", &file)
        .env("AL_PROFILE_US", "50")
        .output()
        .unwrap();
    assert_eq!(code(&o), 0, "{}", text(&o));
    let profile = std::fs::read_to_string(&file).unwrap_or_default();
    assert!(profile.starts_with("samples "), "no profile written: {profile:?}");
}
