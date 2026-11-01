//! A test codeunit with SingleInstance = true runs its test methods on the session's single instance: a variable of its own
//! type and its manual event subscriber see and set the globals the test methods use, and the second test method sees what
//! the first one left (codeunit/QiSingleInstanceTestCodeunit.al:57 QI_SI_1_OwnTypeVariableAndManualSubscriber, :98
//! QI_SI_2_SecondTestSeesTheFirst).

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
codeunit 50600 "SI Publisher"
{
    procedure Ping(Value: Text) begin OnPing(Value); end;
    [IntegrationEvent(false, false)]
    local procedure OnPing(Value: Text) begin end;
}
codeunit 50601 "SI Manual Test"
{
    Subtype = Test;
    SingleInstance = true;
    EventSubscriberInstance = Manual;
    var
        Marker: Text;
        Calls: Integer;
    procedure SetMarker(Value: Text) begin Marker := Value; end;
    procedure GetMarker(): Text begin exit(Marker); end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"SI Publisher", 'OnPing', '', false, false)]
    local procedure HandlePing(Value: Text) begin Calls += 1; Marker := 'sub saw [' + Marker + '] got ' + Value; end;
    [Test]
    procedure A_First()
    var
        Other: Codeunit "SI Manual Test";
        P: Codeunit "SI Publisher";
    begin
        Marker := 'm1';
        if Other.GetMarker() <> 'm1' then Error('other-get %1', Other.GetMarker());
        Other.SetMarker('m2');
        if Marker <> 'm2' then Error('own-after-other-set %1', Marker);
        P.Ping('unbound');
        if Calls <> 0 then Error('unbound calls %1', Calls);
        BindSubscription(Other);
        P.Ping('bound');
        if Marker <> 'sub saw [m2] got bound' then Error('own-after-bound %1', Marker);
        UnbindSubscription(Other);
        Marker := 'left by test 1';
    end;
    [Test]
    procedure B_Second()
    var
        Other: Codeunit "SI Manual Test";
    begin
        if Marker <> 'left by test 1' then Error('start-marker %1', Marker);
        if Calls <> 1 then Error('start-calls %1', Calls);
        if Other.GetMarker() <> 'left by test 1' then Error('other-get %1', Other.GetMarker());
    end;
}
"#;

fn run() -> CorpusRuntime {
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
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    )
}

#[test]
fn a_single_instance_test_codeunit_runs_its_tests_on_the_sessions_single_instance() {
    let rt = run();
    for name in ["A_First", "B_Second"] {
        let o = rt.first_outcome(&CaseId {
            codeunit: 50601,
            name: name.into(),
        });
        assert!(matches!(o, Outcome::Pass), "{name}: {o:?}");
    }
}
