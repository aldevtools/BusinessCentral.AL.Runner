//! A memberless `var` Option parameter carried through `IntegrationEvent(true, false)` to a manual subscriber
//! (`EventSubscriberInstance = Manual`) that hands it on to a `var` Option parameter with members (al-runner issue 8971).
//! One instance bound by `BindSubscription` outside any subscriber gives what the static subscriber gives (assumption
//! var-option-carried-members, corpus PR 1569, OptionVarMembersBatchManual.al ManualRun_*, ManualBound_*,
//! ManualTestBound_*), flagged. Two instances bound at once, or one bound inside a subscriber: no cell, so the hand-on
//! refuses as for any unmeasured origin.

use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69881 MRow
{
    fields
    {
        field(1; Id; Integer) { }
    }

    procedure Confirm(var Choice: Option " ",Skip,Update): Text
    var
        Seen: Text;
    begin
        Seen := Format(Choice) + '|' + Format(Choice, 0, 2);
        Choice := Choice::Update;
        exit(Seen);
    end;
}

codeunit 69882 MEvents
{
    procedure Raise(var V: Option; var Row: Record MRow)
    begin
        OnWithSender(V, Row);
    end;

    procedure Setup(var Bound: Codeunit MManual)
    begin
        OnSetup(Bound);
    end;

    [IntegrationEvent(true, false)]
    local procedure OnWithSender(var V: Option; var Row: Record MRow)
    begin
    end;

    [IntegrationEvent(false, false)]
    local procedure OnSetup(var Bound: Codeunit MManual)
    begin
    end;
}

codeunit 69883 MManual
{
    EventSubscriberInstance = Manual;

    trigger OnRun()
    var
        Bound: Codeunit MManual;
        C: Codeunit MCaller;
    begin
        BindSubscription(Bound);
        RunLog := C.Batch();
        UnbindSubscription(Bound);
    end;

    var
        RunLog: Text;

    procedure GetRunLog(): Text
    begin
        exit(RunLog);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::MEvents, 'OnWithSender', '', false, false)]
    local procedure WithSender(var V: Option; var Row: Record MRow)
    begin
        Pass(V, Row);
    end;

    local procedure Pass(var V: Option; var Row: Record MRow)
    begin
        Row.Confirm(V);
    end;
}

codeunit 69884 MBinder
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::MEvents, 'OnSetup', '', false, false)]
    local procedure BindOnSetup(var Bound: Codeunit MManual)
    begin
        BindSubscription(Bound);
    end;
}

codeunit 69885 MCaller
{
    procedure Batch(): Text
    var
        V: Option " ",Skip,Update;
        Events: Codeunit MEvents;
        Row: Record MRow;
    begin
        V := V::Skip;
        Events.Raise(V, Row);
        exit(Format(V));
    end;

    // Codeunit.Run on a fresh instance whose OnRun binds another fresh one (ManualRun_*).
    procedure ViaRun(): Text
    var
        M: Codeunit MManual;
    begin
        M.Run();
        exit(M.GetRunLog());
    end;

    // Bound in a procedure, as the test itself binds (ManualBound_*, ManualTestBound_*).
    procedure Bound(): Text
    var
        M: Codeunit MManual;
        Log: Text;
    begin
        BindSubscription(M);
        Log := Batch();
        UnbindSubscription(M);
        exit(Log);
    end;

    // Bound and unbound before the raise (ManualUnbound_*): no subscriber runs.
    procedure Unbound(): Text
    var
        M: Codeunit MManual;
    begin
        BindSubscription(M);
        UnbindSubscription(M);
        exit(Batch());
    end;

    // Two instances bound at once: no cell.
    procedure TwoBound(): Text
    var
        M1: Codeunit MManual;
        M2: Codeunit MManual;
    begin
        BindSubscription(M1);
        BindSubscription(M2);
        exit(Batch());
    end;

    // Bound by a subscriber of another event: no cell.
    procedure BoundInSubscriber(): Text
    var
        M: Codeunit MManual;
        Events: Codeunit MEvents;
    begin
        Events.Setup(M);
        exit(Batch());
    end;
}
"#;

fn run(routine: &str) -> (Result<String, Signal>, Vec<Assumption>) {
    assumptions::clear_hits();
    let out = al_testkit::run_in(SRC, "MCaller", routine).map(|v| text(Ok(v)));
    (out, assumptions::take_hits().into_iter().collect())
}

// One bound instance, bound in OnRun of Codeunit.Run or in a procedure: the static subscriber's answer, flagged.
#[test]
fn one_instance_bound_outside_a_subscriber_runs_flagged() {
    for routine in ["ViaRun", "Bound"] {
        let (out, hits) = run(routine);
        assert_eq!(out.unwrap(), "Update", "{routine}");
        assert_eq!(hits, vec![Assumption::VarOptionCarriedMembers], "{routine}");
    }
}

// Unbound before the raise: no subscriber runs, the local keeps its value, nothing flagged.
#[test]
fn an_instance_unbound_before_the_raise_receives_nothing() {
    let (out, hits) = run("Unbound");
    assert_eq!(out.unwrap(), "Skip");
    assert!(hits.is_empty(), "{hits:?}");
}

// Two instances bound at once, or one bound inside a subscriber: no cell measures it, the hand-on refuses.
#[test]
fn two_bound_instances_or_a_binding_inside_a_subscriber_refuse() {
    for routine in ["TwoBound", "BoundInSubscriber"] {
        match run(routine).0 {
            Err(Signal::Unsupported(why)) => assert!(
                why.contains(
                    "an Option that a memberless `var` parameter hands on to a `var` Option parameter with members"
                ),
                "{routine}: {why}"
            ),
            other => panic!("{routine}: expected a refusal, got {other:?}"),
        }
    }
}
