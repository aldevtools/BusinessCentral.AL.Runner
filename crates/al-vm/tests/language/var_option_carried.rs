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

    // The callee of the queued batch path (OptionVarMembersBatch.al `ConfirmOnce`): an `=` and a `case` on its own members,
    // `:=` of an own member.
    procedure ConfirmOnce(var Choice: Option " ",Skip,Update): Text
    begin
        if Choice = Choice::Skip then
            exit('skip-exit');
        case Choice of
            Choice::" ":
                begin
                    Choice := Choice::Update;
                    exit('blank');
                end;
            Choice::Update:
                exit('update');
            else
                exit('else');
        end;
    end;

    procedure WriteAndRead(var Choice: Option " ",Skip,Update): Text
    begin
        Choice := Choice::Update;
        exit(Format(Choice) + '|' + Format(Choice, 0, 2));
    end;

    // Uses no cell measures for a value that carries no list.
    procedure EvaluateIt(var Choice: Option " ",Skip,Update): Text
    begin
        exit(Format(Evaluate(Choice, 'Skip')));
    end;

    procedure CopyIt(var Choice: Option " ",Skip,Update): Text
    var
        Other: Option " ",Skip,Update;
    begin
        Other := Choice;
        exit(Format(Other));
    end;

    procedure ClearIt(var Choice: Option " ",Skip,Update): Text
    begin
        Clear(Choice);
        exit('');
    end;

    // The other uses of OptionVarMembersCallee.al `ConfirmOp`, `EvaluateStatement`.
    procedure ClearRead(var Choice: Option " ",Skip,Update): Text
    begin
        Clear(Choice);
        exit(Format(Choice) + '|' + Format(Choice, 0, 2));
    end;

    procedure EvaluateText(var Choice: Option " ",Skip,Update; Txt: Text): Text
    begin
        exit(Format(Evaluate(Choice, Txt)) + '>' + Format(Choice) + '|' + Format(Choice, 0, 2));
    end;

    // The by-value text changed before Evaluate reads it: the text at the call is not the one Evaluate gets.
    procedure EvaluateReassigned(var Choice: Option " ",Skip,Update; Txt: Text): Text
    begin
        Txt := 'b';
        exit(Format(Evaluate(Choice, Txt)) + '>' + Format(Choice));
    end;

    procedure EvaluateTextSetByVar(var Choice: Option " ",Skip,Update; Txt: Text): Text
    var
        Lib: Codeunit CLib;
    begin
        Lib.SetText(Txt);
        exit(Format(Evaluate(Choice, Txt)) + '>' + Format(Choice));
    end;

    procedure AssignOne(var Choice: Option " ",Skip,Update): Text
    begin
        Choice := 1;
        exit(Format(Choice) + '|' + Format(Choice, 0, 2));
    end;

    procedure HandOn(var Choice: Option " ",Skip,Update): Text
    var
        Lib: Codeunit CLib;
    begin
        exit(Lib.Show(Choice) + Lib.ShowByValue(Choice));
    end;

    procedure HandOnEvaluate(var Choice: Option " ",Skip,Update): Text
    var
        Lib: Codeunit CLib;
    begin
        exit(Lib.ShowEvaluate(Choice));
    end;
}

codeunit 69875 CLib
{
    procedure Show(var V: Option): Text
    begin
        exit('m ' + Format(V) + '|' + Format(V, 0, 2));
    end;

    procedure ShowByValue(V: Option): Text
    begin
        exit(' b ' + Format(V) + '|' + Format(V, 0, 2));
    end;

    procedure SetText(var T: Text)
    begin
        T := 'b';
    end;

    procedure ShowEvaluate(var V: Option): Text
    begin
        Evaluate(V, '1');
        exit(Format(V));
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

    // A subscriber that is also called directly, as a plain procedure.
    [EventSubscriber(ObjectType::Codeunit, Codeunit::CEvents, 'OnNoSender', '', false, false)]
    procedure CalledDirect(var V: Option; var Row: Record CRow)
    begin
        Pass(V, Row);
    end;

    local procedure Pass(var V: Option; var Row: Record CRow)
    begin
        Row.Confirm(V);
    end;
}

codeunit 69876 CTrigger
{
    // A trigger's `var` section as the origin: no cell of corpus PR 1569 hands a value on from a trigger.
    trigger OnRun()
    var
        V: Option " ",Skip,Update;
        Row: Record CRow;
    begin
        V := V::Skip;
        Hop(V, Row);
    end;

    local procedure Hop(var V: Option; var Row: Record CRow)
    begin
        Row.Confirm(V);
    end;
}

codeunit 69874 C
{
    var
        G: Option " ",Skip,Update;
        GOther: Option A,B,C,D;

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
        Seen: Text;
    begin
        V := 1;
        Seen := Hop(V);
        exit(Seen + '>' + Format(V) + '|' + Format(V, 0, 2));
    end;

    procedure MemberlessWriteInside(): Text
    var
        Row: Record CRow;
        V: Option;
    begin
        exit(WriteHop(V, Row) + '>' + Format(V));
    end;

    local procedure WriteHop(var V: Option; var Row: Record CRow): Text
    begin
        exit(Row.WriteAndRead(V));
    end;

    local procedure QueuedHop(var V: Option; var Row: Record CRow): Text
    begin
        exit(Row.ConfirmOnce(V) + '/' + Format(V));
    end;

    procedure Queued(): Text
    var
        Row: Record CRow;
        V: Option;
        Log: Text;
        I: Integer;
    begin
        for I := 1 to 3 do
            Log += QueuedHop(V, Row) + ';';
        exit(Log + Format(V) + '|' + Format(V, 0, 2));
    end;

    local procedure EvaluateHop(var V: Option): Text
    var
        Row: Record CRow;
    begin
        exit(Row.EvaluateIt(V));
    end;

    local procedure CopyHop(var V: Option): Text
    var
        Row: Record CRow;
    begin
        exit(Row.CopyIt(V));
    end;

    local procedure ClearHop(var V: Option): Text
    var
        Row: Record CRow;
    begin
        exit(Row.ClearIt(V));
    end;

    procedure MemberlessEvaluate(): Text
    var
        V: Option;
    begin
        exit(EvaluateHop(V));
    end;

    procedure MemberlessCopy(): Text
    var
        V: Option;
    begin
        exit(CopyHop(V));
    end;

    local procedure ClearReadHop(var V: Option): Text
    var
        Row: Record CRow;
    begin
        exit(Row.ClearRead(V));
    end;

    local procedure EvaluateTextHop(var V: Option; Txt: Text): Text
    var
        Row: Record CRow;
    begin
        exit(Row.EvaluateText(V, Txt));
    end;

    local procedure AssignOneHop(var V: Option): Text
    var
        Row: Record CRow;
    begin
        exit(Row.AssignOne(V));
    end;

    local procedure HandOnHop(var V: Option): Text
    var
        Row: Record CRow;
    begin
        exit(Row.HandOn(V));
    end;

    procedure OtherClear(): Text
    var
        V: Option A,B,C,D;
    begin
        V := 3;
        exit(ClearReadHop(V) + '>' + Format(V));
    end;

    procedure OtherEvaluate(Txt: Text): Text
    var
        V: Option A,B,C,D;
    begin
        V := 3;
        exit(EvaluateTextHop(V, Txt) + '>' + Format(V));
    end;

    procedure OtherEvaluateName(): Text
    begin
        exit(OtherEvaluate('B'));
    end;

    procedure OtherEvaluateDigits(): Text
    begin
        exit(OtherEvaluate('7'));
    end;

    procedure OtherEvaluateNoName(): Text
    begin
        exit(OtherEvaluate('Skip'));
    end;

    procedure OtherEvaluateOtherCase(): Text
    begin
        exit(OtherEvaluate('b'));
    end;

    procedure OtherEvaluateBlank(): Text
    begin
        exit(OtherEvaluate(''));
    end;

    procedure MemberlessEvaluateName(): Text
    var
        V: Option;
    begin
        V := 1;
        exit(EvaluateTextHop(V, 'Skip') + '>' + Format(V));
    end;

    procedure OtherAssignOne(): Text
    var
        V: Option A,B,C,D;
    begin
        exit(AssignOneHop(V) + '>' + Format(V));
    end;

    procedure OtherHandOn(): Text
    var
        V: Option A,B,C,D;
    begin
        V := 3;
        exit(HandOnHop(V) + '>' + Format(V));
    end;

    procedure DirectHandOn(): Text
    var
        Row: Record CRow;
        V: Option " ",Skip,Update;
    begin
        V := 1;
        exit(Row.HandOn(V));
    end;

    procedure DirectHandOnEvaluate(): Text
    var
        Row: Record CRow;
        V: Option " ",Skip,Update;
    begin
        exit(Row.HandOnEvaluate(V));
    end;

    procedure OtherEvaluateReassigned(): Text
    var
        Row: Record CRow;
        V: Option A,B,C,D;
    begin
        V := 3;
        exit(ReassignedHop(V, Row) + '>' + Format(V));
    end;

    local procedure ReassignedHop(var V: Option; var Row: Record CRow): Text
    begin
        exit(Row.EvaluateReassigned(V, 'Nope'));
    end;

    procedure OtherEvaluateSetByVar(): Text
    var
        Row: Record CRow;
        V: Option A,B,C,D;
    begin
        V := 3;
        exit(SetByVarHop(V, Row) + '>' + Format(V));
    end;

    local procedure SetByVarHop(var V: Option; var Row: Record CRow): Text
    begin
        exit(Row.EvaluateTextSetByVar(V, 'Nope'));
    end;

    // An [EventSubscriber] procedure called directly as the hop: its frame's mark did not come from the raise.
    procedure SubscriberCalledDirect(): Text
    var
        Subs: Codeunit CSubs;
        Row: Record CRow;
        V: Option A,B,C,D;
    begin
        V := 1;
        Subs.CalledDirect(V, Row);
        exit(Format(V));
    end;

    procedure SubscriberCalledDirectMemberless(): Text
    var
        Subs: Codeunit CSubs;
        Row: Record CRow;
        V: Option;
    begin
        V := 1;
        Subs.CalledDirect(V, Row);
        exit(Format(V));
    end;

    // A list shorter than the callee's: the callee's own members lie past it (no cell measures it).
    procedure ShorterOrigin(): Text
    var
        V: Option A,B;
    begin
        V := 1;
        exit(Hop(V) + '>' + Format(V));
    end;

    // The hand-on from a `var` parameter with members bound to a global with another list, a field, a by-value parameter,
    // or a memberless local (CalleeOps_MemberlessLocal `m Skip|1`).
    procedure GlobalOtherHandOn(): Text
    var
        Row: Record CRow;
    begin
        GOther := 1;
        exit(Row.HandOn(GOther));
    end;

    procedure FieldHandOn(): Text
    var
        Row: Record CRow;
    begin
        Row.Opt := Row.Opt::Skip;
        exit(Row.HandOn(Row.Opt));
    end;

    procedure ByValueHandOn(): Text
    begin
        exit(ByValueHandOnInner(1));
    end;

    local procedure ByValueHandOnInner(V: Option A,B,C,D): Text
    var
        Row: Record CRow;
    begin
        exit(Row.HandOn(V));
    end;

    procedure MemberlessHandOn(): Text
    var
        Row: Record CRow;
        V: Option;
    begin
        V := 1;
        exit(Row.HandOn(V));
    end;

    procedure MemberlessClear(): Text
    var
        V: Option;
    begin
        exit(ClearHop(V));
    end;

    // A memberless local through the event: no cell measures it.
    procedure MemberlessEventOrigin(): Text
    var
        V: Option;
        Events: Codeunit CEvents;
        Row: Record CRow;
    begin
        Events.Raise(V, Row);
        exit(Format(V));
    end;

    procedure OtherOrigin(): Text
    var
        V: Option A,B,C,D;
        Seen: Text;
    begin
        V := 1;
        Seen := Hop(V);
        exit(Seen + '>' + Format(V));
    end;

    procedure OtherEventOrigin(): Text
    var
        V: Option A,B,C,D;
        Events: Codeunit CEvents;
        Row: Record CRow;
    begin
        Events.Raise(V, Row);
        exit(Format(V));
    end;

    procedure TriggerOrigin(): Text
    begin
        Codeunit.Run(Codeunit::CTrigger);
        exit('ran');
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

/// The start of the hand-on refusal (`hand_on_args`).
const HAND_ON: &str = "a `var` parameter with members handed to a `var` Option parameter declared without members";

fn refused(routine: &str) {
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

// A memberless local of a codeunit procedure, through memberless hops: the callee reads the value's own (no) list, so Format
// gives the number, and its `:=` of an own member comes back as the number (OptionVarMembersCallee.al VarHop_Memberless_*,
// OptionVarMembersBatch.al BatchQueued_*). Runs, flagged.
#[test]
fn a_memberless_value_from_a_measured_origin_runs_flagged_and_keeps_no_list() {
    let (out, hits) = run("MemberlessOrigin");
    assert_eq!(out.unwrap(), "1|1>2|2");
    assert_eq!(hits, vec![Assumption::VarOptionCarriedMembers]);
    // The callee's own `:=` reads back as the number (VarHop_Memberless_Table `0 write inside`).
    let (out, hits) = run("MemberlessWriteInside");
    assert_eq!(out.unwrap(), "2|2>2");
    assert_eq!(hits, vec![Assumption::VarOptionCarriedMembers]);
    let (out, hits) = run("Queued");
    assert_eq!(out.unwrap(), "blank/2;update/2;update/2;2|2");
    assert_eq!(hits, vec![Assumption::VarOptionCarriedMembers]);
}

// A local with another list, through memberless hops: the callee reads the value's list, its `:=` of an own member stores the
// number in that list (OptionVarMembersCallee.al VarHop_Other_*). Runs, flagged.
#[test]
fn a_value_carrying_another_list_from_a_measured_origin_runs_flagged_and_keeps_its_list() {
    let (out, hits) = run("OtherOrigin");
    assert_eq!(out.unwrap(), "B|1>C");
    assert_eq!(hits, vec![Assumption::VarOptionCarriedMembers]);
}

// A memberless value at a callee that uses the parameter in a way no cell measures for it (a copy, Clear), and a
// memberless origin through the event, refuse.
#[test]
fn a_memberless_value_at_an_unmeasured_use_or_through_the_event_refuses() {
    refused("MemberlessCopy");
    refused("MemberlessClear");
    refused("MemberlessEventOrigin");
    refused("OtherEventOrigin");
}

// An origin no cell has and another event contract refuse.
#[test]
fn other_origins_and_event_contracts_refuse() {
    refused("GlobalOrigin");
    refused("EventNoSenderOrigin");
    refused("TriggerOrigin");
}

// The callee's Evaluate reads the value's list (OptionVarMembersCallee.al CalleeOps_VarHop_Other: `Second` Yes, `5` Yes, `Skip`
// No; CalleeOps_VarHop_Memberless: `Skip` No), Clear gives its first member (`>First|0`), `:= 1` its second member (`>Second|1`).
// Runs, flagged.
#[test]
fn evaluate_clear_and_an_integer_read_the_list_the_value_carries() {
    for (routine, want) in [
        ("OtherEvaluateName", "Yes>B|1>B"),
        ("OtherEvaluateDigits", "Yes>7|7>7"),
        ("OtherEvaluateNoName", "No>D|3>D"),
        ("MemberlessEvaluateName", "No>1|1>1"),
        ("OtherClear", "A|0>A"),
        ("OtherAssignOne", "B|1>B"),
    ] {
        let (out, hits) = run(routine);
        assert_eq!(out.unwrap(), want, "{routine}");
        assert_eq!(hits, vec![Assumption::VarOptionCarriedMembers], "{routine}");
    }
}

// Texts no cell measures for a value that carries another list (a blank text, another spelling of a member of its list) and
// Clear of a value that carries no list (BC leaves the callee's list in the memberless callers) refuse.
#[test]
fn unmeasured_evaluate_texts_and_clear_of_no_list_refuse() {
    refused("OtherEvaluateOtherCase");
    refused("OtherEvaluateBlank");
    refused("MemberlessClear");
}

// The parameter handed on to a memberless `var` parameter and by value to a memberless one of a routine that only formats it
// shows the value's list, and the number by value (CalleeOps_VarHop_Other `m Fifth|4`, `b 4|4`); from a direct caller the same
// (CalleeOps_MemberlessLocal `m Skip|1`). Runs, flagged. A target that does more with it refuses.
#[test]
fn a_hand_on_to_a_routine_that_only_formats_runs_and_any_other_refuses() {
    let (out, hits) = run("OtherHandOn");
    assert_eq!(out.unwrap(), "m D|3 b 3|3>D");
    assert_eq!(hits, vec![Assumption::VarOptionCarriedMembers]);
    let (out, hits) = run("DirectHandOn");
    assert_eq!(out.unwrap(), "m Skip|1 b 1|1");
    assert_eq!(hits, vec![Assumption::VarOptionCarriedMembers]);
    match run("DirectHandOnEvaluate").0 {
        Err(Signal::Unsupported(why)) => assert!(why.starts_with(HAND_ON), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// MED 1 of review round 1: the callee changes its by-value text before Evaluate reads it (by `:=` or by handing it on by
// `var`): the text at the call is not the text Evaluate gets, so the value check at the call does not cover it. Refused.
#[test]
fn a_text_parameter_the_callee_writes_before_evaluate_refuses() {
    refused("OtherEvaluateReassigned");
    refused("OtherEvaluateSetByVar");
}

// MED 2 of review round 1: an [EventSubscriber] procedure called directly takes no mark from its frame; only the raise
// gives a subscriber one (`Carried::Event`). Refused.
#[test]
fn a_subscriber_called_directly_as_a_hop_refuses() {
    refused("SubscriberCalledDirect");
    refused("SubscriberCalledDirectMemberless");
}

// MED 3 of review round 1: a value whose list is shorter than the callee's; only a longer one is measured (VarHop_Other_*:
// First,...,Sixth against " ",Skip,Update). Refused.
#[test]
fn a_value_carrying_a_shorter_list_refuses() {
    refused("ShorterOrigin");
}

// MED 4 of review round 1: the hand-on from a `var` parameter with members runs only for the bindings the cells measure: a
// local of the `var` section of the direct caller (CalleeOps_MemberlessLocal `m Skip|1`), or a value `carried_args` let
// through. A global with another list, a field and a by-value parameter refuse.
#[test]
fn a_hand_on_from_an_unmeasured_binding_refuses() {
    let (out, hits) = run("MemberlessHandOn");
    assert_eq!(out.unwrap(), "m Skip|1 b 1|1");
    assert_eq!(hits, vec![Assumption::VarOptionCarriedMembers]);
    for routine in ["GlobalOtherHandOn", "FieldHandOn", "ByValueHandOn"] {
        match run(routine).0 {
            Err(Signal::Unsupported(why)) => assert!(why.starts_with(HAND_ON), "{routine}: {why}"),
            other => panic!("{routine}: expected a refusal, got {other:?}"),
        }
    }
}
