//! A manual subscriber instance that unbinds itself in its first subscriber method does not run its second method in
//! the same raise (handlers/ProbeEventSubscriberOrder.al:253 ManualSubscriber_UnbindsItselfInFirstMethod_SecondMethodInSameRaise).
//! An instance unbound by another subscriber is not pinned and is refused.

use al_builtins::native::Signal;

const PUB: &str = "
codeunit 50707 Pub { procedure Ping(Tag: Text) begin OnPing(Tag); end;
    [IntegrationEvent(false, false)] local procedure OnPing(Tag: Text) begin end; }
codeunit 50709 Log { SingleInstance = true; var Entries: Text;
    procedure Add(E: Text) begin if Entries <> '' then Entries += ';'; Entries += E; end;
    procedure Get(): Text begin exit(Entries); end; }
codeunit 50708 Unbinder { EventSubscriberInstance = Manual; var L: Codeunit Log;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnPing', '', false, false)]
    local procedure A_OnPing(Tag: Text) begin L.Add('A:' + Tag); UnbindSubscription(this); L.Add('after-unbind'); end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnPing', '', false, false)]
    local procedure B_OnPing(Tag: Text) begin L.Add('B:' + Tag); end; }
";

#[test]
fn second_method_of_a_self_unbound_instance_is_not_called() {
    let src = format!(
        "{PUB} codeunit 50710 C {{ procedure F(): Text var U: Codeunit Unbinder; P: Codeunit Pub; L: Codeunit Log; begin
            BindSubscription(U); P.Ping('stop'); P.Ping('again'); exit(L.Get()); end; }}"
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_in(&src, "C", "F")),
        "A:stop;after-unbind"
    );
}

#[test]
fn an_instance_unbound_by_another_subscriber_is_refused() {
    let src = format!(
        "{PUB} codeunit 50700 Other {{ EventSubscriberInstance = Manual; var U: Codeunit Unbinder;
            procedure SetU(var X: Codeunit Unbinder) begin U := X; end;
            [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnPing', '', false, false)]
            local procedure O_OnPing(Tag: Text) begin UnbindSubscription(U); end; }}
        codeunit 50712 C {{ procedure F(): Text var U: Codeunit Unbinder; O: Codeunit Other; P: Codeunit Pub; L: Codeunit Log; begin
            O.SetU(U); BindSubscription(O); BindSubscription(U); P.Ping('x'); exit(L.Get()); end; }}"
    );
    assert!(matches!(
        al_testkit::run_in(&src, "C", "F"),
        Err(Signal::Unsupported(_))
    ));
}
