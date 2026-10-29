//! al-runner issue 6112: an install routine reaches a report whose OnInitReport calls, through a TryFunction, a routine that
//! cannot be lowered (`Image Impl..Resize`, issue 5590). Every test was refused before the run with the install routine,
//! although neither the install routine nor the tests run the report. A report's triggers and a TryFunction are refused
//! when they run (`Session::call` checks `Engine::blocked` on every call), not before, as a page action's OnAction is
//! (issues 6014, 6031). When the path is taken, the refusal still comes: an Unsupported inside a TryFunction whose result
//! is used is not caught as an error.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50460 "LZR Setup" { fields { field(1; Code; Code[20]) { } field(2; Run; Boolean) { } }
    keys { key(PK; Code) { Clustered = true; } } }
codeunit 50461 "LZR Mgt"
{
    procedure Resize()
    begin
        if not TryResize() then
            exit;
    end;
    [TryFunction]
    procedure TryResize()
    begin
        CodeCoverageLoad();
    end;
}
report 50462 "LZR Report"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    trigger OnInitReport()
    begin
        InitCaptions();
    end;
    local procedure InitCaptions()
    var M: Codeunit "LZR Mgt";
    begin
        M.Resize();
    end;
}
codeunit 50463 "LZR Outbound"
{
    procedure Book(Send: Boolean)
    var Rep: Report "LZR Report";
    begin
        if Send then
            Rep.Run();
    end;
    procedure BookStatic(Send: Boolean)
    begin
        if Send then
            Report.Run(Report::"LZR Report", false);
    end;
    procedure TryOnly(Go: Boolean)
    var M: Codeunit "LZR Mgt";
    begin
        if Go then
            M.TryResize();
    end;
}
codeunit 50464 "LZR Install"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var S: Record "LZR Setup"; O: Codeunit "LZR Outbound";
    begin
        S.Code := 'SEEDED';
        S.Insert();
        O.Book(S.Run);
        O.BookStatic(S.Run);
    end;
}
codeunit 50465 "LZR Tests"
{
    Subtype = Test;
    [Test]
    procedure InstallRanAndNoReportRuns()
    var S: Record "LZR Setup"; O: Codeunit "LZR Outbound";
    begin
        S.Get('SEEDED');
        O.Book(false);
        O.BookStatic(false);
        O.TryOnly(false);
    end;
    [Test]
    procedure RunningTheReportIsRefused()
    var O: Codeunit "LZR Outbound";
    begin
        O.Book(true);
    end;
    [Test]
    procedure RunningTheReportStaticallyIsRefused()
    var O: Codeunit "LZR Outbound";
    begin
        O.BookStatic(true);
    end;
    [Test]
    procedure TryFunctionResultUsedIsRefusedNotCaught()
    var M: Codeunit "LZR Mgt";
    begin
        M.Resize();
    end;
    [Test]
    procedure TryFunctionCalledIsRefused()
    var O: Codeunit "LZR Outbound";
    begin
        O.TryOnly(true);
    end;
}
"#;

fn outcomes() -> Vec<(&'static str, Outcome)> {
    let rt = CorpusRuntime::run_suites_with(
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
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    );
    [
        "InstallRanAndNoReportRuns",
        "RunningTheReportIsRefused",
        "RunningTheReportStaticallyIsRefused",
        "TryFunctionResultUsedIsRefusedNotCaught",
        "TryFunctionCalledIsRefused",
    ]
    .into_iter()
    .map(|name| {
        (
            name,
            rt.first_outcome(&CaseId {
                codeunit: 50465,
                name: name.into(),
            }),
        )
    })
    .collect()
}

#[test]
fn a_refusal_reached_through_a_report_trigger_or_a_try_function_is_raised_only_when_it_runs() {
    let outcomes = outcomes();
    let get = |n: &str| outcomes.iter().find(|(k, _)| *k == n).map(|(_, o)| o.clone()).unwrap();
    // The install routine runs (it never runs the report), and the test that runs no report passes.
    assert_eq!(get("InstallRanAndNoReportRuns"), Outcome::Pass, "{outcomes:?}");
    // Each test that takes the path is not run, with the reason of the routine that holds it; inside a TryFunction whose
    // result is used the refusal is not caught as an error.
    for n in [
        "RunningTheReportIsRefused",
        "RunningTheReportStaticallyIsRefused",
        "TryFunctionResultUsedIsRefusedNotCaught",
        "TryFunctionCalledIsRefused",
    ] {
        match get(n) {
            Outcome::NotRun(why) => assert!(why.contains("CodeCoverageLoad"), "{n}: {why}"),
            o => panic!("{n}: {o:?}"),
        }
    }
}
