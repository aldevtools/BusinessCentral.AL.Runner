//! Unbounded recursion through an event subscriber stops with the error code `RecursiveEventCall:StackOverflow` and a text
//! that names the subscriber, the event and its publisher (al-runner issue 5944; pinned on corpus branch
//! al-runtime-rs-staging-sh, codeunit/lt-call-depth-direct/CallDepthDirectAndEvent.al:101-137; assumption
//! `recursive-event-call-code` until the corpus bump).

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};

const SRC: &str = "
table 50711 Row { fields { field(1; Code; Code[20]) { } field(4; Qty; Integer) { } } keys { key(PK; Code) { } } }
codeunit 50710 Recursor { EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Table, Database::Row, 'OnAfterModifyEvent', '', false, false)]
    local procedure OnAfterModifyRow(var Rec: Record Row; var xRec: Record Row; RunTrigger: Boolean)
    begin Rec.Qty += 1; Rec.Modify(true); end; }
codeunit 50767 C {
    local procedure Go() var Row: Record Row; R: Codeunit Recursor; begin
        Row.Code := 'R1'; Row.Insert(); BindSubscription(R); Row.Qty := 1; asserterror Row.Modify(true); UnbindSubscription(R); end;
    procedure Code(): Text begin Go(); exit(GetLastErrorCode()); end;
    procedure Txt(): Text begin Go(); exit(GetLastErrorText()); end;
    procedure Plain(N: Integer): Integer begin exit(Plain(N + 1)); end;
    procedure PlainCode(): Text begin asserterror Plain(0); exit(GetLastErrorCode()); end;
    procedure ViaEvent(): Text var W: Codeunit Worker; begin asserterror W.Go(); exit(GetLastErrorText()); end; }
codeunit 72884 Worker {
    procedure Go() begin OnDeeper(); end;
    [IntegrationEvent(false, false)] local procedure OnDeeper() begin end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Worker, 'OnDeeper', '', false, false)]
    local procedure HandleDeeper() begin Go(); end; }";

#[test]
fn recursion_through_a_subscriber_has_the_recursive_event_code() {
    // Same stack the CLI gives the VM (al-test-cli main).
    std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(body)
        .unwrap()
        .join()
        .unwrap();
}

fn body() {
    clear_hits();
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Code")),
        "RecursiveEventCall:StackOverflow"
    );
    assert!(take_hits().contains(&Assumption::RecursiveEventCallCode));
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Txt")),
        "Excessive recursion has been detected in event subscriber function OnAfterModifyRow for event OnAfterModifyEvent that is raised in Table 50711."
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "ViaEvent")),
        "Excessive recursion has been detected in event subscriber function HandleDeeper for event None that is raised in CodeUnit 72884."
    );
    // Plain recursion keeps its code.
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "PlainCode")),
        "StackOverflow"
    );
}
