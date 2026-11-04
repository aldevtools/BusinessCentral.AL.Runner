//! A failed test that did not reach its `UnbindSubscription` of a global leaves the binding for the next test of the codeunit
//! (codeunits/TestEventManualBinding.al:161-200, Contract 9). On BC that test may pass and unbind, so a later failure while
//! that binding is still live is not trusted: the test is reported as not run (al-runner issue 6924, the second test's
//! "already been bound").

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
codeunit 50500 "LB Pub"
{
    procedure Ping()
    begin
        OnPing();
    end;

    [IntegrationEvent(false, false)]
    local procedure OnPing()
    begin
    end;
}

codeunit 50501 "LB Sub"
{
    EventSubscriberInstance = Manual;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LB Pub", 'OnPing', '', false, false)]
    local procedure OnPing()
    begin
    end;
}

codeunit 50502 "LB Tests"
{
    Subtype = Test;

    var
        Sub: Codeunit "LB Sub";

    [Test]
    procedure A_FailsBeforeUnbind()
    begin
        BindSubscription(Sub);
        Error('a fails');
        UnbindSubscription(Sub);
    end;

    [Test]
    procedure B_BindsAgain()
    begin
        BindSubscription(Sub);
        UnbindSubscription(Sub);
    end;

    [Test]
    procedure C_FailsWithoutBinding()
    begin
        Error('c fails');
    end;
}

codeunit 50503 "LB Unbound Tests"
{
    Subtype = Test;

    var
        Sub: Codeunit "LB Sub";

    [Test]
    procedure A_FailsWithLocalBinding()
    var
        LocalSub: Codeunit "LB Sub";
    begin
        BindSubscription(LocalSub);
        Error('a fails');
    end;

    [Test]
    procedure B_FailsAfterwards()
    begin
        Error('b fails');
    end;
}
"#;

fn outcome<'a>(all: &'a [(String, Outcome)], name: &str) -> &'a Outcome {
    &all.iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no test {name}: {all:?}"))
        .1
}

#[test]
fn a_failure_while_an_earlier_failed_test_left_a_binding_is_not_run() {
    let all = outcomes_in(SRC, "LB Tests");
    assert!(
        matches!(outcome(&all, "A_FailsBeforeUnbind"), Outcome::Fail(m) if m.contains("a fails")),
        "{all:?}"
    );
    match outcome(&all, "B_BindsAgain") {
        Outcome::NotRun(why) => {
            assert!(why.contains("left a manual event subscription bound"), "{why}");
            assert!(why.contains("already been bound"), "{why}");
        }
        other => panic!("B_BindsAgain: {other:?}"),
    }
    // B's failure was not trusted; it ended at its first statement, so A's binding is still live for C.
    assert!(
        matches!(outcome(&all, "C_FailsWithoutBinding"), Outcome::NotRun(w) if w.contains("left a manual event subscription bound")),
        "{all:?}"
    );
}

#[test]
fn a_binding_of_a_local_variable_ends_with_its_test() {
    let all = outcomes_in(SRC, "LB Unbound Tests");
    assert!(
        matches!(outcome(&all, "B_FailsAfterwards"), Outcome::Fail(m) if m.contains("b fails")),
        "{all:?}"
    );
}
