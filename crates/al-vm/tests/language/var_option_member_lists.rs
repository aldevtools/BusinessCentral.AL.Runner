//! A `var` Option argument whose declared member list differs from the parameter's (al-runner issues 7994 and 8291), through
//! assumption `var-option-caller-members` (corpus PR 1432, tests/al-language/types/enum-option/OptionVarMemberLists.al and
//! OptionVarMemberlessEvaluate.al, measured on BC 27.5, 28.5 and 29.0): a parameter declared without members reads the caller's
//! members, and the caller keeps its own after the call. The shapes the probe does not measure refuse; the neighbors keep
//! main's result.

use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69821 Row
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Opt; Option) { OptionMembers = First,Second,Third; }
        field(3; OptCap; Option) { OptionMembers = First,Second,Third; OptionCaption = 'One,Two,Three'; }
    }
}

codeunit 69820 C
{
    var
        GlobalV: Option First,Second,Third;
        GlobalArr: array[2] of Option First,Second,Third;
        GlobalRow: Record Row;

    procedure EmptyBodyLocal(): Text
    var V: Option First,Second,Third;
    begin
        V := V::Second;
        Nop(V);
        exit(Format(V) + '|' + Format(V, 0, 2));
    end;

    procedure Assign1Local(): Text
    var V: Option First,Second,Third;
    begin
        Assign1(V);
        exit(Format(V));
    end;

    procedure InsideLocal(): Text
    var V: Option First,Second,Third;
    begin
        V := V::Second;
        exit(FormatInside(V) + '|' + Format(V));
    end;

    procedure GlobalCaller(): Text
    begin
        GlobalV := GlobalV::Second;
        Nop(GlobalV);
        GlobalV := GlobalV::First;
        Assign1(GlobalV);
        exit(Format(GlobalV));
    end;

    procedure FieldCaller(): Text
    var R: Record Row;
    begin
        R.Opt := R.Opt::Second;
        exit(FormatInside(R.Opt) + '|' + Format(R.Opt));
    end;

    procedure ArrayElementCaller(): Text
    var A: array[2] of Option First,Second,Third;
    begin
        A[1] := A[1]::Second;
        exit(FormatInside(A[1]) + '|' + Format(A[1]));
    end;

    procedure TwoHops(): Text
    var V: Option First,Second,Third;
    begin
        HopAssign1(V);
        exit(Format(V));
    end;

    procedure OtherMembers(): Text
    var V: Option First,Second,Third;
        Inside: Text;
    begin
        V := V::Second;
        Inside := OtherFormat(V);
        OtherNop(V);
        Inside += '|' + Format(V);
        OtherAssignC(V);
        exit(Inside + '|' + Format(V));
    end;

    procedure MemberlessCaller(): Text
    var V0: Option;
    begin
        V0 := 1;
        exit(SameFormat(V0) + '|' + Format(V0));
    end;

    procedure VarArray(): Text
    var Arr: array[2] of Option First,Second,Third;
    begin
        ArrayAssign1(Arr);
        exit(Format(Arr[1]));
    end;

    procedure EventMemberless(): Text
    var V: Option First,Second,Third;
        Seen: Text;
    begin
        V := V::Second;
        OnMemberless(V, Seen);
        exit(Seen + '|' + Format(V));
    end;

    procedure EvaluateName(): Text
    var V: Option First,Second,Third;
    begin
        V := V::Third;
        EvalStatement(V, 'second');
        exit(Format(V));
    end;

    procedure EvaluateOrdinal7(): Text
    var V: Option First,Second,Third;
    begin
        EvalStatement(V, '7');
        exit(Format(V));
    end;

    procedure EvaluateUnknownResult(): Text
    var V: Option First,Second,Third;
    begin
        V := V::Third;
        exit(Format(EvalResult(V, 'Nope')) + '|' + Format(V));
    end;

    procedure EvaluateUnknownStatement(): Text
    var V: Option First,Second,Third;
    begin
        V := V::Third;
        asserterror EvalStatement(V, 'Nope');
        exit(GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(V));
    end;

    procedure EvaluateMemberlessCaller(): Text
    var V0: Option;
    begin
        asserterror EvalStatement(V0, 'Second');
        exit(GetLastErrorText());
    end;

    procedure EvaluateArrayElement(): Text
    var Arr: array[2] of Option First,Second,Third;
    begin
        exit(ArrayEvalResult(Arr, 'Second') + '|' + Format(Arr[1]));
    end;

    procedure CaptionFieldCaller(): Text
    var R: Record Row;
    begin
        Nop(R.OptCap);
        exit(Format(R.OptCap));
    end;

    procedure ByValueParameterCaller(): Text
    begin
        GlobalV := GlobalV::Second;
        exit(PassOn(GlobalV));
    end;

    procedure VarParameterWithMembersCaller(): Text
    begin
        PassOnVar(GlobalV);
        exit(Format(GlobalV));
    end;

    procedure GlobalToOtherMembers(): Text
    begin
        OtherNop(GlobalV);
        exit(Format(GlobalV));
    end;

    procedure GlobalToMemberlessPublisher(): Text
    var Seen: Text;
    begin
        GlobalV := GlobalV::Second;
        OnMemberless(GlobalV, Seen);
        exit(Seen + '|' + Format(GlobalV));
    end;

    procedure PublisherWithOtherMembers(): Text
    var V: Option A,B,C;
        Seen: Text;
    begin
        OnWithMembers(V, Seen);
        exit(Format(V));
    end;

    procedure GlobalRecordField(): Text
    begin
        Nop(GlobalRow.Opt);
        exit(Format(GlobalRow.Opt));
    end;

    procedure GlobalArrayElement(): Text
    begin
        Nop(GlobalArr[1]);
        exit(Format(GlobalArr[1]));
    end;

    procedure GlobalVarArray(): Text
    begin
        ArrayAssign1(GlobalArr);
        exit(Format(GlobalArr[1]));
    end;

    procedure EvaluateBlankStatement(): Text
    var V: Option First,Second,Third;
    begin
        EvalStatement(V, '');
        exit(Format(V));
    end;

    procedure EvaluateArrayElementUnknown(): Text
    var Arr: array[2] of Option First,Second,Third;
    begin
        exit(ArrayEvalResult(Arr, 'Nope'));
    end;

    procedure SubscriberWithMembers(): Text
    var V: Option First,Second,Third;
    begin
        OnMemberlessToMembers(V);
        exit(Format(V));
    end;

    procedure SameMembers(): Text
    var V: Option First,Second,Third;
    begin
        V := V::Second;
        exit(SameFormat(V) + '|' + Format(V));
    end;

    procedure ByValueMemberless(): Text
    var V: Option First,Second,Third;
    begin
        V := V::Second;
        exit(ByValueFormat(V) + '|' + Format(V));
    end;

    procedure FieldToOtherMembers(): Text
    var R: Record Row;
    begin
        R.Opt := R.Opt::Second;
        exit(OtherFormat(R.Opt) + '|' + Format(R.Opt));
    end;

    local procedure PassOn(V: Option First,Second,Third): Text
    begin
        Nop(V);
        exit(Format(V));
    end;

    local procedure PassOnVar(var V: Option First,Second,Third)
    begin
        Nop(V);
    end;

    local procedure Nop(var Value: Option)
    begin
    end;

    local procedure Assign1(var Value: Option)
    begin
        Value := 1;
    end;

    local procedure FormatInside(var Value: Option): Text
    begin
        exit(Format(Value) + '|' + Format(Value, 0, 2));
    end;

    local procedure ByValueFormat(Value: Option): Text
    begin
        exit(Format(Value));
    end;

    local procedure HopAssign1(var Value: Option)
    begin
        Assign1(Value);
    end;

    local procedure OtherNop(var Value: Option A,B,C)
    begin
    end;

    local procedure OtherFormat(var Value: Option A,B,C): Text
    begin
        exit(Format(Value));
    end;

    local procedure OtherAssignC(var Value: Option A,B,C)
    begin
        Value := Value::C;
    end;

    local procedure SameFormat(var Value: Option First,Second,Third): Text
    begin
        exit(Format(Value));
    end;

    local procedure ArrayAssign1(var A: array[2] of Option)
    begin
        A[1] := 1;
    end;

    local procedure EvalStatement(var Value: Option; T: Text)
    begin
        Evaluate(Value, T);
    end;

    local procedure EvalResult(var Value: Option; T: Text): Boolean
    begin
        exit(Evaluate(Value, T));
    end;

    local procedure ArrayEvalResult(var A: array[2] of Option; T: Text): Text
    begin
        exit(Format(Evaluate(A[1], T)) + '|' + Format(A[1]));
    end;

    [IntegrationEvent(false, false)]
    local procedure OnMemberless(var Value: Option; var Seen: Text)
    begin
    end;

    [IntegrationEvent(false, false)]
    local procedure OnWithMembers(var Value: Option First,Second,Third; var Seen: Text)
    begin
    end;

    [IntegrationEvent(false, false)]
    local procedure OnMemberlessToMembers(var Value: Option)
    begin
    end;
}

codeunit 69822 Subs
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::C, 'OnMemberless', '', false, false)]
    local procedure SeeMemberless(var Value: Option; var Seen: Text)
    begin
        Seen := Format(Value);
        Value := 2;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::C, 'OnMemberlessToMembers', '', false, false)]
    local procedure SeeWithMembers(var Value: Option A,B,C)
    begin
        Value := Value::C;
    end;
}
"#;

/// Runs `routine` and answers its text, asserting that it ran through the assumption.
fn assumed(routine: &str) -> String {
    assumptions::clear_hits();
    let out = text(al_testkit::run_in(SRC, "C", routine));
    assert!(
        assumptions::take_hits().contains(&Assumption::VarOptionCallerMembers),
        "{routine}: not flagged"
    );
    out
}

/// Runs `routine` and answers its text, asserting that it did not run through the assumption.
fn plain(routine: &str) -> String {
    assumptions::clear_hits();
    let out = text(al_testkit::run_in(SRC, "C", routine));
    assert!(
        !assumptions::take_hits().contains(&Assumption::VarOptionCallerMembers),
        "{routine}: flagged"
    );
    out
}

fn refused(routine: &str, reason: &str) {
    match al_testkit::run_in(SRC, "C", routine) {
        Err(Signal::Unsupported(why)) => assert!(why.contains(reason), "{routine}: {why}"),
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    }
}

// OptionVarMemberLists.al: MemberlessVar_EmptyBody_CallerFormat, MemberlessVar_Assign1_CallerFormat,
// MemberlessVar_CalleeFormat_CallerLocal, MemberlessVar_EmptyBody_Global, MemberlessVar_CalleeFormat_RecordField,
// MemberlessVar_ArrayElementCaller, MemberlessVar_TwoHops.
#[test]
fn a_parameter_without_members_reads_the_caller_s_members_and_the_caller_keeps_them() {
    assert_eq!(assumed("EmptyBodyLocal"), "Second|1");
    assert_eq!(assumed("Assign1Local"), "Second");
    assert_eq!(assumed("InsideLocal"), "Second|1|Second");
    assert_eq!(assumed("GlobalCaller"), "Second");
    assert_eq!(assumed("FieldCaller"), "Second|1|Second");
    assert_eq!(assumed("ArrayElementCaller"), "Second|1|Second");
    assert_eq!(assumed("TwoHops"), "Second");
    // A parameter passed by value is a local variable of its routine (Base Application's "Config. Package Management"
    // hands one to an event).
    assert_eq!(assumed("ByValueParameterCaller"), "Second");
}

// OptionVarMemberLists.al: OtherMembersVar_EmptyBody, OtherMembersVar_CalleeFormatAndAssign, MemberlessCaller_MembersVar.
#[test]
fn a_parameter_with_other_members_reads_its_own_and_the_caller_keeps_its_own() {
    assert_eq!(assumed("OtherMembers"), "B|Second|Third");
    assert_eq!(assumed("MemberlessCaller"), "Second|1");
}

// OptionVarMemberLists.al: MemberlessVarArray_Callee, Event_MemberlessPublisher.
#[test]
fn a_var_array_and_an_event_subscriber_keep_the_caller_s_members() {
    assert_eq!(assumed("VarArray"), "Second");
    assert_eq!(assumed("EventMemberless"), "Second|Third");
    // Base Application's "Config. Package Management" hands a global to a publisher's memberless `var` parameter (reached
    // by the pinned corpus case table-objects Y76_Package_AutoIncrementField_NotInPackage).
    assert_eq!(assumed("GlobalToMemberlessPublisher"), "Second|Third");
}

// OptionVarMemberlessEvaluate.al: MemberlessVar_EvaluateStatement_NameLowerCase, _Ordinal7, MemberlessVar_EvaluateResult_Others
// (Nope), MemberlessVar_EvaluateStatement_Unknown, MemberlessCaller_EvaluateStatement_Name, MemberlessVarArray_EvaluateResult.
#[test]
fn evaluate_into_a_parameter_without_members_reads_the_caller_s_members() {
    assert_eq!(assumed("EvaluateName"), "Second");
    assert_eq!(assumed("EvaluateOrdinal7"), "7");
    assert_eq!(assumed("EvaluateUnknownResult"), "No|Third");
    assert_eq!(
        assumed("EvaluateUnknownStatement"),
        "'Nope' is not an option. The existing options are: First,Second,Third|InvalidOptionString|Third"
    );
    assert_eq!(
        assumed("EvaluateMemberlessCaller"),
        "'Second' is not an option. The existing options are: "
    );
    assert_eq!(assumed("EvaluateArrayElement"), "Yes|Second|Second");
}

#[test]
fn a_record_field_with_an_option_caption_refuses() {
    refused("CaptionFieldCaller", "OptionCaption");
}

#[test]
fn callers_the_probe_does_not_measure_refuse() {
    for routine in [
        "VarParameterWithMembersCaller",
        "GlobalToOtherMembers",
        "PublisherWithOtherMembers",
        "GlobalRecordField",
        "GlobalArrayElement",
        "GlobalVarArray",
    ] {
        refused(routine, "another member list");
    }
}

#[test]
fn evaluate_shapes_the_probe_does_not_measure_refuse() {
    refused("EvaluateBlankStatement", "declared without members");
    refused("EvaluateArrayElementUnknown", "declared without members");
}

#[test]
fn a_subscriber_with_members_under_a_publisher_without_members_refuses() {
    refused("SubscriberWithMembers", "another member list");
}

#[test]
fn neighbors_keep_main_s_result() {
    assert_eq!(plain("SameMembers"), "Second|Second");
    assert_eq!(plain("ByValueMemberless"), "1|Second");
    assert_eq!(plain("FieldToOtherMembers"), "B|Second");
}
