//! A publisher call whose `var` argument names a global of the caller, and that fails before the event is raised (here at
//! the call-depth limit inside `asserterror`), does not hand that global to the next event (al-runner issue 7615). The next
//! event's subscribers get that event's own arguments: an `OnAfterInsertEvent` subscriber sees the inserted record. The
//! recursion error itself is pinned in codeunits/call-depth/CallDepthDirectAndEvent.al:101-137; that a table event's
//! subscriber receives the record being inserted is pinned throughout the corpus (events/).

const SRC: &str = "
table 50731 Row { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { } } }
table 50732 Log { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { } } }
codeunit 50733 Worker {
    var G: Record Row;
    procedure Go() begin OnDeeper(G); end;
    [IntegrationEvent(false, false)] local procedure OnDeeper(var R: Record Row) begin end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Worker, 'OnDeeper', '', false, false)]
    local procedure HandleDeeper(var R: Record Row) begin OnDeeper(G); end; }
codeunit 50734 Logger {
    [EventSubscriber(ObjectType::Table, Database::Row, 'OnAfterInsertEvent', '', false, false)]
    local procedure OnAfterInsertRow(var Rec: Record Row; RunTrigger: Boolean) var L: Record Log;
    begin L.Code := Rec.Code; if L.Code = '' then L.Code := 'EMPTY'; L.Insert(); end; }
codeunit 50735 C {
    procedure F(): Text var W: Codeunit Worker; Row: Record Row; L: Record Log; begin
        asserterror W.Go();
        Row.Code := 'X1'; Row.Insert(true);
        L.FindFirst(); exit(L.Code); end; }";

#[test]
fn a_publisher_call_that_fails_before_publishing_does_not_leak_its_arguments() {
    // Same stack the CLI gives the VM (al-test-cli main).
    std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(|| assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "F")), "X1"))
        .unwrap()
        .join()
        .unwrap();
}
