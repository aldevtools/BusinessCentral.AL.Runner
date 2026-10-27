//! Unbounded recursion through an event subscriber stops with the error code `RecursiveEventCall:StackOverflow`
//! (al-runner issue 5944, the reporter's repro on real BC; registered assumption `recursive-event-call-code`). Only the
//! start of the text is known, so reading the text is refused.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
use al_builtins::native::Signal;

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
    procedure PlainCode(): Text begin asserterror Plain(0); exit(GetLastErrorCode()); end; }";

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
    assert!(matches!(
        al_testkit::run_in(SRC, "C", "Txt"),
        Err(Signal::Unsupported(_))
    ));
    // Plain recursion keeps its code.
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "PlainCode")),
        "StackOverflow"
    );
}
