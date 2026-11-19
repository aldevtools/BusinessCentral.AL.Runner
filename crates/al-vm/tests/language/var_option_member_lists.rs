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
        field(4; OptML; Option) { OptionMembers = First,Second,Third; OptionCaptionML = ENU = 'One,Two,Three'; }
    }

    procedure SelfFormat(): Text
    begin
        exit(Inside(Opt));
    end;

    local procedure Inside(var Value: Option): Text
    begin
        exit(Format(Value));
    end;
}

interface IOpt
{
    procedure FormatInside(var Value: Option): Text;
}

codeunit 69823 Impl implements IOpt
{
    procedure FormatInside(var Value: Option): Text
    begin
        exit(Format(Value) + '|' + Format(Value, 0, 2));
    end;
}

codeunit 69820 C
{
    var
        GlobalV: Option First,Second,Third;
        GlobalArr: array[2] of Option First,Second,Third;
        GlobalRow: Record Row;
        GlobalV0: Option;

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
        V := V::Third;
        asserterror EvalStatement(V, '');
        exit(GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(V));
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

    procedure AssignThenFormatInside(): Text
    var V: Option First,Second,Third;
    begin
        V := V::Second;
        exit(AssignFormat(V));
    end;

    procedure AssignThenEvaluate(): Text
    var V: Option First,Second,Third;
    begin
        AssignEvaluate(V);
        exit(Format(V));
    end;

    procedure AssignThenHop(): Text
    var V: Option First,Second,Third;
    begin
        exit(AssignHop(V));
    end;

    procedure AssignOtherThenFormat(): Text
    var V: Option First,Second,Third;
    begin
        exit(AssignOtherFormat(V));
    end;

    procedure AssignThenOrdinal(): Text
    var V: Option First,Second,Third;
    begin
        exit(AssignOrdinal(V) + '|' + Format(V));
    end;

    procedure ClearInside(): Text
    var V: Option First,Second,Third;
    begin
        V := V::Second;
        exit(ClearFormat(V));
    end;

    procedure ArrayAssignThenFormatInside(): Text
    var Arr: array[2] of Option First,Second,Third;
    begin
        exit(ArrayAssignFormat(Arr));
    end;

    procedure ArrayAssignOther(): Text
    var Arr: array[2] of Option First,Second,Third;
    begin
        ArrayAssignOtherOption(Arr);
        exit(Format(Arr[1]));
    end;

    procedure ArrayClear(): Text
    var Arr: array[2] of Option First,Second,Third;
    begin
        ArrayClearAll(Arr);
        exit(Format(Arr[1]));
    end;

    procedure LocalArrayCopy(): Text
    var
        Arr: array[2] of Option First,Second,Third;
        Arr0: array[2] of Option;
    begin
        Arr[1] := Arr[1]::Second;
        Arr0 := Arr;
        Arr0[1] := 1;
        exit(Format(Arr0[1]));
    end;

    procedure LocalCopyArray(): Text
    var
        Arr: array[2] of Option First,Second,Third;
        Arr0: array[2] of Option;
    begin
        Arr[1] := Arr[1]::Second;
        CopyArray(Arr0, Arr, 1);
        Arr0[1] := 1;
        exit(Format(Arr0[1]));
    end;

    procedure ByValueArrayParameter(): Text
    var Arr: array[2] of Option First,Second,Third;
    begin
        Arr[1] := Arr[1]::Second;
        exit(ByValueArrayAssign(Arr));
    end;

    procedure InterfaceLocal(): Text
    var
        V: Option First,Second,Third;
        Impl: Codeunit Impl;
        I: Interface IOpt;
    begin
        I := Impl;
        V := V::Second;
        exit(I.FormatInside(V) + '|' + Format(V));
    end;

    procedure InterfaceCaptionField(): Text
    var
        R: Record Row;
        Impl: Codeunit Impl;
        I: Interface IOpt;
    begin
        I := Impl;
        exit(I.FormatInside(R.OptCap));
    end;

    procedure InterfaceGlobalRecordField(): Text
    var
        Impl: Codeunit Impl;
        I: Interface IOpt;
    begin
        I := Impl;
        exit(I.FormatInside(GlobalRow.Opt));
    end;

    procedure InterfaceVarParameter(): Text
    begin
        exit(InterfacePassOnVar(GlobalV));
    end;

    procedure ByValueToPublisher(): Text
    begin
        GlobalV := GlobalV::Second;
        exit(PassToPublisher(GlobalV));
    end;

    procedure ByValueToOtherMembers(): Text
    begin
        exit(PassToOther(GlobalV));
    end;

    procedure ByValueArrayToVarArray(): Text
    begin
        exit(PassArrayOn(GlobalArr));
    end;

    procedure NamedReturnCaller(): Text
    begin
        exit(Format(NamedReturn()));
    end;

    procedure ByValueRecordField(): Text
    var R: Record Row;
    begin
        exit(PassRowField(R));
    end;

    procedure ByValueMemberlessToMembers(): Text
    var V0: Option;
    begin
        exit(PassMemberless(V0));
    end;

    procedure LocalFieldToPublisher(): Text
    var
        R: Record Row;
        Seen: Text;
    begin
        OnMemberless(R.Opt, Seen);
        exit(Seen);
    end;

    procedure LocalElementToPublisher(): Text
    var
        Arr: array[2] of Option First,Second,Third;
        Seen: Text;
    begin
        OnMemberless(Arr[1], Seen);
        exit(Seen);
    end;

    procedure LocalArrayToPublisher(): Text
    var Arr: array[2] of Option First,Second,Third;
    begin
        OnArray(Arr);
        exit(Format(Arr[1]));
    end;

    procedure CaptionMLFieldCaller(): Text
    var R: Record Row;
    begin
        exit(FormatInside(R.OptML));
    end;

    procedure VarRecordField(): Text
    var R: Record Row;
    begin
        exit(PassVarRowField(R));
    end;

    procedure TableOwnField(): Text
    var R: Record Row;
    begin
        exit(R.SelfFormat());
    end;

    procedure MemberlessGlobalToMembers(): Text
    begin
        exit(SameFormat(GlobalV0));
    end;

    procedure VarArrayHandedOn(): Text
    begin
        exit(PassVarArrayOn(GlobalArr));
    end;

    procedure MembersPublisherLocal(): Text
    var
        V: Option First,Second,Third;
        Seen: Text;
    begin
        V := V::Second;
        OnWithMembers(V, Seen);
        exit(Seen);
    end;

    procedure MembersPublisherCaptionField(): Text
    var
        R: Record Row;
        Seen: Text;
    begin
        OnWithMembers(R.OptCap, Seen);
        exit(Seen);
    end;

    procedure MembersPublisherGlobal(): Text
    var Seen: Text;
    begin
        OnWithMembers(GlobalV, Seen);
        exit(Seen);
    end;

    local procedure AssignFormat(var Value: Option): Text
    begin
        Value := 1;
        exit(Format(Value));
    end;

    local procedure AssignEvaluate(var Value: Option)
    begin
        Value := 1;
        Evaluate(Value, 'Third');
    end;

    local procedure AssignHop(var Value: Option): Text
    begin
        Value := 1;
        exit(FormatInside(Value));
    end;

    local procedure AssignOtherFormat(var Value: Option): Text
    var O: Option A,B,C;
    begin
        O := O::C;
        Value := O;
        exit(Format(Value));
    end;

    local procedure AssignOrdinal(var Value: Option): Text
    begin
        Value := 1;
        exit(Format(Value, 0, 2));
    end;

    local procedure ClearFormat(var Value: Option): Text
    begin
        Clear(Value);
        exit(Format(Value));
    end;

    local procedure ArrayAssignFormat(var A: array[2] of Option): Text
    begin
        A[1] := 1;
        exit(Format(A[1]));
    end;

    local procedure ArrayAssignOtherOption(var A: array[2] of Option)
    var O: Option A,B,C;
    begin
        O := O::B;
        A[1] := O;
    end;

    local procedure ArrayClearAll(var A: array[2] of Option)
    begin
        Clear(A);
    end;

    local procedure ByValueArrayAssign(A: array[2] of Option): Text
    begin
        A[1] := 1;
        exit(Format(A[1]));
    end;

    local procedure InterfacePassOnVar(var V: Option First,Second,Third): Text
    var
        Impl: Codeunit Impl;
        I: Interface IOpt;
    begin
        I := Impl;
        exit(I.FormatInside(V));
    end;

    local procedure PassToPublisher(V: Option First,Second,Third): Text
    var Seen: Text;
    begin
        OnMemberless(V, Seen);
        exit(Seen + '|' + Format(V));
    end;

    local procedure PassToOther(V: Option First,Second,Third): Text
    begin
        exit(OtherFormat(V));
    end;

    local procedure PassArrayOn(A: array[2] of Option First,Second,Third): Text
    begin
        ArrayAssign1(A);
        exit(Format(A[1]));
    end;

    local procedure NamedReturn() R: Option First,Second,Third
    begin
        Nop(R);
    end;

    local procedure PassRowField(R: Record Row): Text
    begin
        exit(FormatInside(R.Opt));
    end;

    local procedure PassMemberless(V: Option): Text
    begin
        exit(SameFormat(V));
    end;

    local procedure PassVarRowField(var R: Record Row): Text
    begin
        exit(FormatInside(R.Opt));
    end;

    local procedure PassVarArrayOn(var A: array[2] of Option First,Second,Third): Text
    begin
        ArrayAssign1(A);
        exit(Format(A[1]));
    end;

    [IntegrationEvent(false, false)]
    local procedure OnArray(var A: array[2] of Option)
    begin
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

    [EventSubscriber(ObjectType::Codeunit, Codeunit::C, 'OnWithMembers', '', false, false)]
    local procedure SeeWithMembersMemberless(var Value: Option; var Seen: Text)
    begin
        Seen := Format(Value);
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
}

// OptionVarMemberLists.al: OtherMembersVar_EmptyBody, OtherMembersVar_CalleeFormatAndAssign, MemberlessCaller_MembersVar.
#[test]
fn a_parameter_with_other_members_reads_its_own_and_the_caller_keeps_its_own() {
    assert_eq!(assumed("OtherMembers"), "B|Second|Third");
    assert_eq!(assumed("MemberlessCaller"), "Second|1");
}

// OptionVarMemberLists.al: MemberlessVarArray_Callee.
#[test]
fn a_var_array_and_an_event_subscriber_keep_the_caller_s_members() {
    assert_eq!(assumed("VarArray"), "Second");
}

// An event publisher called directly with another member list that a subscriber hears is measured only for a parameter with
// members and subscribers that take it with the publisher's list (OptionVarMemberlessHeard.al); these refuse. Base Application's "Config. Package Management" calls one directly that no
// subscriber hears (the pinned corpus case table-objects Y76_Package_AutoIncrementField_NotInPackage, a global): that runs no
// code, and the global keeps its own members (`var_option::unheard_publisher_caller`).
#[test]
fn a_publisher_called_directly_refuses() {
    for routine in [
        "EventMemberless",
        "GlobalToMemberlessPublisher",
        "ByValueToPublisher",
        "PublisherWithOtherMembers",
        "LocalFieldToPublisher",
        "LocalElementToPublisher",
    ] {
        refused(routine, "handed directly to an event publisher's");
    }
    // An element of a global array is measured only handed to an interface method (OptionVarMemberlessCallers.al:573).
    refused("GlobalArrayElement", "caller the probe does not measure");
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
    // OptionVarMemberlessEvaluate.al:170.
    assert_eq!(
        assumed("EvaluateBlankStatement"),
        "'' is not an option. The existing options are: First,Second,Third|InvalidOptionString|Third"
    );
}

#[test]
fn a_record_field_with_an_option_caption_refuses() {
    refused("CaptionFieldCaller", "OptionCaption");
    refused("CaptionMLFieldCaller", "OptionCaption");
    refused("InterfaceCaptionField", "OptionCaption");
}

// An interface method's `var` parameter is gated as a procedure's (corpus PR 1439, OptionVarMemberlessCallers.al:511, :604).
#[test]
fn an_interface_call_reads_the_caller_s_members_from_the_measured_callers_only() {
    assert_eq!(assumed("InterfaceLocal"), "Second|1|Second");
    assert_eq!(assumed("InterfaceGlobalRecordField"), "First|0");
    refused("InterfaceVarParameter", "a `var` parameter with members");
}

// A store into the memberless `var` parameter keeps the caller's members, inside the routine and after it (corpus PR 1439,
// OptionVarMemberlessWrites.al:112, :130, :250, :269, :472).
#[test]
fn a_name_read_after_the_routine_stored_into_the_parameter_reads_the_caller_s_members() {
    assert_eq!(assumed("AssignThenFormatInside"), "Second");
    assert_eq!(assumed("AssignThenEvaluate"), "Third");
    assert_eq!(assumed("AssignThenHop"), "Second|1");
    assert_eq!(assumed("AssignOtherThenFormat"), "Third");
    assert_eq!(assumed("ArrayAssignThenFormatInside"), "Second");
    assert_eq!(assumed("AssignThenOrdinal"), "1|Second");
}

// Clear keeps the caller's members (OptionVarMemberlessWrites.al:226, :496); an Option with other members stored into an
// element of the array keeps them too (:472).
#[test]
fn clear_and_other_stores_into_a_memberless_var_parameter_keep_the_caller_s_members() {
    assert_eq!(assumed("ClearInside"), "First");
    assert_eq!(assumed("ArrayClear"), "First");
    assert_eq!(assumed("ArrayAssignOther"), "Second");
}

// A local array CopyArray filled and a by-value array parameter hold the caller's members too (OptionVarMemberlessWrites.al:551,
// :568); the assignment of the whole array is not measured and refuses.
#[test]
fn a_copied_array_and_a_by_value_array_hold_the_caller_s_members() {
    refused("LocalArrayCopy", "assignment of an array of Options with members");
    assert_eq!(assumed("LocalCopyArray"), "Second");
    assert_eq!(assumed("ByValueArrayParameter"), "Second");
}

// A publisher with members whose subscriber is declared without members: measured for a local (OptionVarMemberLists.al:531)
// and a global (corpus PR 1439, OptionVarMemberlessCallers.al:249); a field whose captions differ from its members refuses
// when such a subscriber binds (:290).
#[test]
fn a_memberless_subscriber_under_a_publisher_called_directly_refuses() {
    // The probes raise every event through a procedure (OptionVarMemberLists.al:531; OVMC:249); a direct call is not measured.
    for routine in [
        "MembersPublisherLocal",
        "MembersPublisherGlobal",
        "MembersPublisherCaptionField",
    ] {
        refused(routine, "caller the probe does not measure");
    }
}

// Caller kinds corpus PR 1439 measured handed to a procedure (OptionVarMemberlessCallers.al:117-215, :332, :353;
// OptionVarMemberlessWrites.al:411, :447).
#[test]
fn callers_corpus_pr_1439_measured_run() {
    for (routine, shown) in [
        ("GlobalRecordField", "First"),
        ("ByValueParameterCaller", "Second"),
        ("ByValueToOtherMembers", "A"),
        ("ByValueArrayToVarArray", "Second"),
        ("NamedReturnCaller", "First"),
        ("ByValueRecordField", "First|0"),
        ("ByValueMemberlessToMembers", "First"),
        ("VarRecordField", "First|0"),
        ("TableOwnField", "First"),
    ] {
        assert_eq!(assumed(routine), shown, "{routine}");
    }
}

#[test]
fn callers_the_probe_does_not_measure_refuse() {
    // An array handed directly to a publisher no subscriber hears: no code runs, the array keeps its value (main's result).
    assert_eq!(text(al_testkit::run_in(SRC, "C", "LocalArrayToPublisher")), "First");
    refused("VarParameterWithMembersCaller", "a `var` parameter with members");
    // A global with members to a procedure's parameter with other members keeps its own (corpus PR 1444,
    // OptionVarMemberlessUnheard.al:415, :423); a memberless global is not measured.
    assert_eq!(assumed("GlobalToOtherMembers"), "First");
    for routine in ["GlobalVarArray", "MemberlessGlobalToMembers", "VarArrayHandedOn"] {
        refused(routine, "another member list");
    }
}

#[test]
fn evaluate_shapes_the_probe_does_not_measure_refuse() {
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
