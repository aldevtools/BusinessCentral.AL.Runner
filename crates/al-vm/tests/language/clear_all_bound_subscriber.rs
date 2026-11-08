//! `ClearAll()` in a codeunit whose global holds a bound manual subscriber (al-runner issue 6974). Measured on BC 29.0
//! (corpus branch al-runtime-rs-staging-6974, events/binding-senders/BindSubscriptionClearAll.al
//! ClearAll_HolderWithBoundGlobal_Binding, ClearAll_TestCodeunitWithBoundGlobal_Binding): the variable keeps the bound
//! instance and the binding stays, but the instance's own globals are reset. A second BindSubscription of the variable
//! raises "already been bound"; UnbindSubscription of it ends the binding. Clear of the global gives a fresh, unbound
//! instance (Clear_HolderWithBoundGlobal_Binding). Only a bound instance whose globals are all Text (as the probe's
//! subscriber) is pinned; an array of codeunits, a TableNo record or a global of another kind is refused.

use al_builtins::native::Signal;

const OBJECTS: &str = "
codeunit 50720 Pub { procedure Ping(): Text var Hits: Text; begin OnPing(Hits); exit(Hits); end;
    [IntegrationEvent(false, false)] local procedure OnPing(var Hits: Text) begin end; }
codeunit 50721 Sub { EventSubscriberInstance = Manual; var Tag: Text;
    procedure SetTag(T: Text) begin Tag := T; end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnPing', '', false, false)]
    local procedure OnPing(var Hits: Text) begin Hits += 'M' + Tag; end; }
codeunit 50722 Plain { procedure Get(): Integer begin exit(1); end; }
";

#[test]
fn clear_all_keeps_the_bound_instance_and_resets_its_globals() {
    let src = format!(
        "{OBJECTS} codeunit 50723 Holder {{ var S: Codeunit Sub; P: Codeunit Pub; N: Integer;
            [TryFunction] local procedure TryBind() begin BindSubscription(S); end;
            procedure F(): Text var R: Text; begin
                N := 5; S.SetTag('1'); BindSubscription(S); R := P.Ping();
                ClearAll();
                R += '|' + Format(N) + '|' + P.Ping();
                S.SetTag('C'); R += '|' + P.Ping();
                if TryBind() then R += '|ok' else R += '|' + GetLastErrorText();
                R += '|' + Format(UnbindSubscription(S)) + '|' + P.Ping();
                exit(R); end; }}"
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_in(&src, "Holder", "F")),
        "M1|0|M|MC|The binding of codeunit 50721 was unsuccessful. The codeunit has already been bound.|Yes|"
    );
}

#[test]
fn clear_of_a_bound_global_ends_the_binding() {
    let src = format!(
        "{OBJECTS} codeunit 50726 Holder {{ var S: Codeunit Sub; P: Codeunit Pub;
            procedure F(): Text var R: Text; begin
                S.SetTag('2'); BindSubscription(S); R := P.Ping(); Clear(S); S.SetTag('D'); R += '|' + P.Ping();
                BindSubscription(S); R += '|' + P.Ping(); UnbindSubscription(S); exit(R); end; }}"
    );
    assert_eq!(al_testkit::text(al_testkit::run_in(&src, "Holder", "F")), "M2||MD");
}

#[test]
fn clear_all_of_an_array_that_holds_a_bound_subscriber_is_refused() {
    let src = format!(
        "{OBJECTS} codeunit 50727 Holder {{ var S: array[2] of Codeunit Sub; P: Codeunit Pub;
            procedure F(): Text begin BindSubscription(S[1]); ClearAll(); exit(P.Ping()); end; }}"
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

/// The probe pins a bound instance whose only global is a Text (`Tag` of "LT BSK Manual"). A bound instance with a TableNo
/// record, or with a global of another kind, is not pinned: ClearAll refuses it.
fn refused(holder_src: &str) {
    let src = format!("{OBJECTS} {holder_src}");
    match al_testkit::run_in(&src, "Holder", "F") {
        Err(Signal::Unsupported(m)) => assert!(m.contains("ClearAll"), "{m}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn clear_all_of_a_bound_instance_with_a_tableno_record_is_refused() {
    refused(
        "table 50730 T { fields { field(1; K; Integer) { } } }
        codeunit 50731 SubT { EventSubscriberInstance = Manual; TableNo = T;
            [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnPing', '', false, false)]
            local procedure OnPing(var Hits: Text) begin Hits += 'T'; end; }
        codeunit 50732 Holder { var S: Codeunit SubT; P: Codeunit Pub;
            procedure F(): Text begin BindSubscription(S); ClearAll(); exit(P.Ping()); end; }",
    );
}

#[test]
fn clear_all_of_a_bound_instance_with_an_integer_global_is_refused() {
    refused(
        "codeunit 50733 SubI { EventSubscriberInstance = Manual; var N: Integer;
            [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnPing', '', false, false)]
            local procedure OnPing(var Hits: Text) begin Hits += Format(N); end; }
        codeunit 50734 Holder { var S: Codeunit SubI; P: Codeunit Pub;
            procedure F(): Text begin BindSubscription(S); ClearAll(); exit(P.Ping()); end; }",
    );
}

#[test]
fn clear_all_of_a_bound_instance_with_a_record_global_is_refused() {
    refused(
        "table 50735 T2 { fields { field(1; K; Integer) { } } }
        codeunit 50736 SubR { EventSubscriberInstance = Manual; var R: Record T2;
            [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnPing', '', false, false)]
            local procedure OnPing(var Hits: Text) begin Hits += 'R'; end; }
        codeunit 50737 Holder { var S: Codeunit SubR; P: Codeunit Pub;
            procedure F(): Text begin BindSubscription(S); ClearAll(); exit(P.Ping()); end; }",
    );
}

#[test]
fn clear_all_of_a_bound_instance_with_a_codeunit_global_is_refused() {
    refused(
        "codeunit 50738 SubC { EventSubscriberInstance = Manual; var C: Codeunit Plain;
            [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnPing', '', false, false)]
            local procedure OnPing(var Hits: Text) begin Hits += 'C'; end; }
        codeunit 50739 Holder { var S: Codeunit SubC; P: Codeunit Pub;
            procedure F(): Text begin BindSubscription(S); ClearAll(); exit(P.Ping()); end; }",
    );
}
