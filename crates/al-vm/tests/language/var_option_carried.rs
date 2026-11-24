//! A memberless `var` Option parameter handed on to a `var` Option parameter with members (al-runner issue 8971). The run
//! lets it through only for a value that carries the callee's own list from a measured origin (assumption
//! var-option-carried-members, corpus PR 1569, OptionVarMembersCallee.al VarHop_Same_*, OptionVarMembersBatch.al
//! BatchEvent_*), flagged; every other such call refuses where it is made.

use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69871 CRow
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Opt; Option) { OptionMembers = " ",Skip,Update; }
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

codeunit 69872 CEvents
{
    procedure Raise(var V: Option; var Row: Record CRow)
    begin
        OnWithSender(V, Row);
    end;

    procedure RaiseNoSender(var V: Option; var Row: Record CRow)
    begin
        OnNoSender(V, Row);
    end;

    [IntegrationEvent(true, false)]
    local procedure OnWithSender(var V: Option; var Row: Record CRow)
    begin
    end;

    [IntegrationEvent(false, false)]
    local procedure OnNoSender(var V: Option; var Row: Record CRow)
    begin
    end;
}

codeunit 69873 CSubs
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::CEvents, 'OnWithSender', '', false, false)]
    local procedure WithSender(var V: Option; var Row: Record CRow)
    begin
        Pass(V, Row);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::CEvents, 'OnNoSender', '', false, false)]
    local procedure NoSender(var V: Option; var Row: Record CRow)
    begin
        Pass(V, Row);
    end;

    local procedure Pass(var V: Option; var Row: Record CRow)
    begin
        Row.Confirm(V);
    end;
}

codeunit 69874 C
{
    var
        G: Option " ",Skip,Update;

    local procedure Hop(var V: Option): Text
    var
        Row: Record CRow;
    begin
        exit(Row.Confirm(V));
    end;

    local procedure TwoHops(var V: Option): Text
    begin
        exit(Hop(V));
    end;

    procedure LocalOrigin(): Text
    var
        V: Option " ",Skip,Update;
        Seen: Text;
    begin
        V := V::Skip;
        Seen := TwoHops(V);
        exit(Seen + '>' + Format(V));
    end;

    procedure MemberlessOrigin(): Text
    var
        V: Option;
    begin
        V := 1;
        exit(Hop(V));
    end;

    procedure GlobalOrigin(): Text
    begin
        G := G::Skip;
        exit(Hop(G));
    end;

    procedure EventOrigin(): Text
    var
        V: Option " ",Skip,Update;
        Events: Codeunit CEvents;
        Row: Record CRow;
    begin
        V := V::Skip;
        Events.Raise(V, Row);
        exit(Format(V));
    end;

    procedure EventNoSenderOrigin(): Text
    var
        V: Option " ",Skip,Update;
        Events: Codeunit CEvents;
        Row: Record CRow;
    begin
        V := V::Skip;
        Events.RaiseNoSender(V, Row);
        exit(Format(V));
    end;
}
"#;

fn run(routine: &str) -> (Result<String, Signal>, Vec<Assumption>) {
    assumptions::clear_hits();
    let out = al_testkit::run_in(SRC, "C", routine).map(|v| text(Ok(v)));
    (out, assumptions::take_hits().into_iter().collect())
}

fn refused(routine: &str) {
    match run(routine).0 {
        Err(Signal::Unsupported(why)) => assert!(
            why.contains(
                "a `var` parameter declared without members hands on to a `var` Option parameter with members"
            ),
            "{routine}: {why}"
        ),
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    }
}

// A local with the callee's list, through two memberless hops or through IntegrationEvent(true, false): runs, flagged.
#[test]
fn a_value_carrying_the_callee_list_from_a_measured_origin_runs_flagged() {
    let (out, hits) = run("LocalOrigin");
    assert_eq!(out.unwrap(), "Skip|1>Update");
    assert_eq!(hits, vec![Assumption::VarOptionCarriedMembers]);
    let (out, hits) = run("EventOrigin");
    assert_eq!(out.unwrap(), "Update");
    assert_eq!(hits, vec![Assumption::VarOptionCarriedMembers]);
}

// Another carried list (BC's callee reads it, OptionVarMembersCallee.al VarHop_Memberless_*), an origin no cell has, and
// another event contract refuse.
#[test]
fn other_lists_origins_and_event_contracts_refuse() {
    refused("MemberlessOrigin");
    refused("GlobalOrigin");
    refused("EventNoSenderOrigin");
}
