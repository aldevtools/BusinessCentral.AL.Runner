//! `ClearAll()` in a codeunit whose global holds a bound manual subscriber (al-runner issue 6974). Whether the binding
//! survives, and whether the variable is the bound instance afterwards, is not pinned: the corpus probe
//! (events/binding-senders/BindSubscriptionClearAll.al) is pending, so the call is refused. Clear of a local bound
//! variable ends the binding (events/TestProbeEventContracts.al BSK_Bind_AfterClear_And_OnCopy) and stays as it is.

use al_builtins::native::Signal;

const OBJECTS: &str = "
codeunit 50720 Pub { procedure Ping(): Text var Hits: Text; begin OnPing(Hits); exit(Hits); end;
    [IntegrationEvent(false, false)] local procedure OnPing(var Hits: Text) begin end; }
codeunit 50721 Sub { EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnPing', '', false, false)]
    local procedure OnPing(var Hits: Text) begin Hits += 'M'; end; }
codeunit 50722 Plain { procedure Get(): Integer begin exit(1); end; }
";

#[test]
fn clear_all_of_a_global_that_holds_a_bound_subscriber_is_refused() {
    let src = format!(
        "{OBJECTS} codeunit 50723 Holder {{ var S: Codeunit Sub; P: Codeunit Pub;
            procedure F(): Text begin BindSubscription(S); ClearAll(); exit(P.Ping()); end; }}"
    );
    match al_testkit::run_in(&src, "Holder", "F") {
        Err(Signal::Unsupported(m)) => assert!(m.contains("ClearAll"), "{m}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn clear_all_after_unbind_still_runs() {
    let src = format!(
        "{OBJECTS} codeunit 50724 Holder {{ var S: Codeunit Sub; P: Codeunit Pub; N: Integer;
            procedure F(): Text begin N := 4; BindSubscription(S); UnbindSubscription(S); ClearAll(); exit(P.Ping() + Format(N)); end; }}"
    );
    assert_eq!(al_testkit::text(al_testkit::run_in(&src, "Holder", "F")), "0");
}

#[test]
fn clear_all_with_an_unbound_codeunit_global_still_runs() {
    let src = format!(
        "{OBJECTS} codeunit 50725 Holder {{ var C: Codeunit Plain; N: Integer;
            procedure F(): Integer begin N := 3; ClearAll(); exit(N + C.Get()); end; }}"
    );
    assert_eq!(al_testkit::int(al_testkit::run_in(&src, "Holder", "F")), 1);
}
