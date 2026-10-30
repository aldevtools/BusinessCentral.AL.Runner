//! al-runner issue 6248: while a `[Test]` procedure runs, `NavApp.GetCallerCallstackModuleInfos()` lists Microsoft's Test Runner
//! app below the test's own apps, as `SessionInformation.Callstack` lists the Test Runner's frames below the test's
//! (session/TestSessionInformationCallstackContracts.al:89-97).

use al_testrun::Outcome;

const SRC: &str = r#"
codeunit 50100 "CM Test Detector"
{
    procedure IsCalledFromTestRunner(): Boolean
    var
        Callers: List of [ModuleInfo];
        Caller: ModuleInfo;
    begin
        Callers := NavApp.GetCallerCallstackModuleInfos();
        foreach Caller in Callers do
            if Caller.Id = '23de40a6-dfe8-4f80-80db-d70f83ce8caf' then
                exit(Caller.Name = 'Test Runner');
        exit(false);
    end;

    procedure LastCaller(): Text
    var
        Callers: List of [ModuleInfo];
    begin
        Callers := NavApp.GetCallerCallstackModuleInfos();
        exit(Callers.Get(Callers.Count()).Name);
    end;
}
codeunit 50101 "CM Tests"
{
    Subtype = Test;

    [Test]
    procedure TestRunnerAppIsOnTheCallerStack()
    var
        Detector: Codeunit "CM Test Detector";
    begin
        if not Detector.IsCalledFromTestRunner() then
            Error('Test Runner app not among the caller modules');
        if Detector.LastCaller() <> 'Test Runner' then
            Error('last caller %1', Detector.LastCaller());
    end;
}
"#;

#[test]
fn the_test_runner_app_is_on_the_caller_stack_of_a_test() {
    for (n, o) in al_testkit::outcomes_in(SRC, "CM Tests") {
        assert_eq!(o, Outcome::Pass, "{n}");
    }
}
