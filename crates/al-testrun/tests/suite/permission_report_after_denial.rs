//! ADR-062 stage 2: a test that makes a denied read and later runs a report the runtime cannot run is refused at the
//! denied read, before it changes any state a later test of the codeunit reads (the report in this shape has
//! `InherentPermissions`, which is not supported: Report_RequestPagePermissions_AndInherentPermissions in
//! `report/lt-u-report-props/UPropertyTests.al`, whose later test Query_InherentPermissions_UnderTheMock passes on BC).

use super::permission_writes::run_main;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions};

const MAIN: &str = r#"
table 51500 "RD Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

report 51500 "RD Report"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    InherentPermissions = X;
}

report 51501 "RD Plain Report"
{
    ProcessingOnly = true;
    UseRequestPage = false;
}

codeunit 51500 "RD Log"
{
    SingleInstance = true;
    var Marks: Integer;
    procedure Mark() begin Marks += 1; end;
    procedure Count(): Integer begin exit(Marks); end;
}

permissionset 51500 "RD None" { Assignable = true; Permissions = tabledata "RD Row" = X; }

codeunit 51501 "RD Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mock: Codeunit "Permissions Mock";
        Log: Codeunit "RD Log";

    local procedure Seed()
    var
        Row: Record "RD Row";
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Insert();
    end;

    [Test]
    procedure A_DeniedReadThenUnsupportedReport()
    var
        Row: Record "RD Row";
    begin
        Seed();
        Mock.Start();
        Mock.Set('RD None');
        asserterror Row.FindFirst();
        Log.Mark();
        Report.Run(Report::"RD Report", false, false);
        Mock.Stop();
    end;

    [Test]
    procedure B_ReadsWhatATestBeforeItLeft()
    begin
        if Log.Count() <> 0 then
            Error('left by an earlier test: %1', Log.Count());
    end;

    // A report the runtime can run does not stop the test at the denied read.
    [Test]
    procedure C_DeniedReadThenSupportedReport()
    var
        Row: Record "RD Row";
    begin
        Seed();
        Mock.Start();
        Mock.Set('RD None');
        asserterror Row.FindFirst();
        Report.Run(Report::"RD Plain Report", false, false);
        Mock.Stop();
    end;
}
"#;

fn results() -> &'static CorpusRuntime {
    static RT: std::sync::OnceLock<CorpusRuntime> = std::sync::OnceLock::new();
    RT.get_or_init(|| run_main(MAIN, &RunOptions::default()))
}

fn outcome(test: &str) -> Outcome {
    results()
        .results()
        .iter()
        .find(|r| r.test == test)
        .unwrap_or_else(|| panic!("{test} did not run"))
        .outcome
        .clone()
}

#[test]
fn a_denied_read_before_an_unsupported_report_refuses_the_test_at_the_read() {
    match outcome("A_DeniedReadThenUnsupportedReport") {
        Outcome::NotRun(why) => assert!(why.contains("not run") || why.contains("report"), "{why}"),
        o => panic!("{o:?}"),
    }
    assert_eq!(outcome("B_ReadsWhatATestBeforeItLeft"), Outcome::Pass);
}

#[test]
fn a_supported_report_after_a_denied_read_runs() {
    assert_eq!(outcome("C_DeniedReadThenSupportedReport"), Outcome::Pass);
}
