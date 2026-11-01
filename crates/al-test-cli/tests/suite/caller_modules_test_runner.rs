//! al-runner issue 6248: while a `[Test]` procedure runs, `NavApp.GetCallerCallstackModuleInfos()` lists Microsoft's Test Runner
//! app below the test's own apps, as `SessionInformation.Callstack` lists the Test Runner's frames below the test's
//! (session/TestSessionInformationCallstackContracts.al:89-97). Pinned by lt-ss1030/SsCallerModulesTestRunner.al (corpus
//! branch al-runtime-rs-staging-ss1030 at c36d14ea, not yet on master): called in the test method the list is the Test Runner
//! alone (:25-43); called in a helper of the test app it is the test app, then the Test Runner (:45-52). The probe is pinned
//! only for the corpus CI leg that runs tests through the Test Runner app; the newer legs run it through the server's test
//! runner hub, which has no AL test runner on the stack (see docs/owner-review.md, al-runner issue 6248).

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
codeunit 50102 "CM Probe Shape"
{
    Subtype = Test;

    local procedure Describe(Infos: List of [ModuleInfo]): Text
    var
        Info: ModuleInfo;
        Result: Text;
    begin
        foreach Info in Infos do
            Result += Format(Info.Id) + '=' + Info.Name + ' | ';
        exit(Result);
    end;

    // lt-ss1030/SsCallerModulesTestRunner.al:25-43
    [Test]
    procedure InTest()
    var
        Infos: List of [ModuleInfo];
    begin
        Infos := NavApp.GetCallerCallstackModuleInfos();
        if Describe(Infos) <> '{23DE40A6-DFE8-4F80-80DB-D70F83CE8CAF}=Test Runner | ' then
            Error('got %1', Describe(Infos));
    end;

    // lt-ss1030/SsCallerModulesTestRunner.al:45-52
    [Test]
    procedure ThroughHelper()
    begin
        if Describe(Ask()) <> '{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}=CM App | {23DE40A6-DFE8-4F80-80DB-D70F83CE8CAF}=Test Runner | ' then
            Error('got %1', Describe(Ask()));
    end;

    local procedure Ask(): List of [ModuleInfo]
    begin
        exit(NavApp.GetCallerCallstackModuleInfos());
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
    for cu in ["CM Tests", "CM Probe Shape"] {
        let app = al_testkit::AppSource {
            id: "a1b2c3d4-e5f6-7890-abcd-ef1234567890".into(),
            ..al_testkit::app_named("CM App", SRC)
        };
        for (n, o) in al_testkit::outcomes_in_app(app, cu) {
            assert_eq!(o, Outcome::Pass, "{n}");
        }
    }
}
