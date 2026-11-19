//! The shapes the probes of corpus PRs 1432 and 1439 do not measure for a `var` Option parameter with another member list
//! than the value (assumption `var-option-caller-members`): they refuse. Their neighbors keep running: the measured ones
//! flagged, the ones main answers as main does. OVMC = OptionVarMemberlessCallers.al, OVMW = OptionVarMemberlessWrites.al
//! (corpus PR 1439).

use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69851 VRow
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Opt; Option) { OptionMembers = First,Second,Third; }
        field(3; OptCap; Option) { OptionMembers = First,Second,Third; OptionCaption = 'One,Two,Three'; }
        field(4; OptNum; Option) { OptionMembers = "0","1","2"; OptionCaption = '0,1,2'; }
        field(5; Abc; Option) { OptionMembers = A,B,C; }
    }
}

table 69856 VBase
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
    protected var
        Mode: Option A,B,C;
}

tableextension 69857 VBaseExt extends VBase
{
    procedure UnheardBaseVar(): Text
    var Pub: Codeunit VPub;
    begin
        Mode := Mode::B;
        Pub.OnUnheardWithMembers(Mode);
        exit(Format(Mode));
    end;
}

codeunit 69852 VPub
{
    procedure RaiseWithMembers(var Value: Option First,Second,Third; var Seen: Text)
    begin
        OnWithMembers(Value, Seen);
    end;

    procedure RaiseTwoHops(var Value: Option First,Second,Third; var Seen: Text)
    begin
        RaiseWithMembers(Value, Seen);
    end;

    [IntegrationEvent(false, false)]
    procedure OnWithMembers(var Value: Option First,Second,Third; var Seen: Text)
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnMemberless(var Value: Option; var Seen: Text)
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnUnheardMemberless(var Value: Option)
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnUnheardWithMembers(var Value: Option First,Second,Third)
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnUnheardArray(var Value: array[2] of Option First,Second,Third)
    begin
    end;
}

codeunit 69853 VSub
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::VPub, 'OnWithMembers', '', false, false)]
    local procedure HandleWithMembers(var Value: Option; var Seen: Text)
    begin
        Seen := Format(Value) + '|' + Format(Value, 0, 2);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::VPub, 'OnMemberless', '', false, false)]
    local procedure HandleMemberless(var Value: Option; var Seen: Text)
    begin
        Seen := Format(Value) + '|' + Format(Value, 0, 2);
    end;
}

interface VFmt
{
    procedure FormatInside(var Value: Option): Text;
    procedure FormatArray(var A: array[2] of Option): Text;
}

codeunit 69854 VFmtImpl implements VFmt
{
    procedure FormatInside(var Value: Option): Text
    begin
        exit(Format(Value));
    end;

    procedure FormatArray(var A: array[2] of Option): Text
    begin
        exit(Format(A[1]));
    end;
}

codeunit 69855 C
{
    var
        GRow: Record VRow;
        GArr: array[2] of Option First,Second,Third;
        GArr0: array[2] of Option;
        GlobalPqr: Option P,Q,R;
        GAbc: Option A,B,C;

    procedure LocalHop(): Text
    var Pub: Codeunit VPub; V: Option First,Second,Third; Seen: Text;
    begin
        V := V::Second;
        Pub.RaiseWithMembers(V, Seen);
        exit(Seen);
    end;

    procedure LocalFieldHop(): Text
    var Pub: Codeunit VPub; Row: Record VRow; Seen: Text;
    begin
        Row.Opt := Row.Opt::Second;
        Pub.RaiseWithMembers(Row.Opt, Seen);
        exit(Seen);
    end;

    procedure CaptionFieldHop(): Text
    var Pub: Codeunit VPub; Row: Record VRow; Seen: Text;
    begin
        Row.OptCap := Row.OptCap::Second;
        Pub.RaiseWithMembers(Row.OptCap, Seen);
        exit(Seen);
    end;

    procedure GlobalCaptionFieldHop(): Text
    var Pub: Codeunit VPub; Seen: Text;
    begin
        GRow.OptCap := GRow.OptCap::Second;
        Pub.RaiseWithMembers(GRow.OptCap, Seen);
        exit(Seen);
    end;

    procedure CaptionFieldTwoHops(): Text
    var Pub: Codeunit VPub; Row: Record VRow; Seen: Text;
    begin
        Row.OptCap := Row.OptCap::Second;
        Pub.RaiseTwoHops(Row.OptCap, Seen);
        exit(Seen);
    end;

    procedure LocalTwoHops(): Text
    var Pub: Codeunit VPub; V: Option First,Second,Third; Seen: Text;
    begin
        V := V::Second;
        Pub.RaiseTwoHops(V, Seen);
        exit(Seen);
    end;

    procedure GlobalRecordFieldHop(): Text
    var Pub: Codeunit VPub; Seen: Text;
    begin
        GRow.Opt := GRow.Opt::Second;
        Pub.RaiseWithMembers(GRow.Opt, Seen);
        exit(Seen);
    end;

    procedure MemberlessLocalHop(): Text
    var Pub: Codeunit VPub; M: Option; Seen: Text;
    begin
        M := 1;
        Pub.RaiseWithMembers(M, Seen);
        exit(Seen);
    end;

    procedure DirectLocalToMembersPublisher(): Text
    var Pub: Codeunit VPub; V: Option First,Second,Third; Seen: Text;
    begin
        V := V::Second;
        Pub.OnWithMembers(V, Seen);
        exit(Seen);
    end;

    procedure DirectMemberlessLocalToMembersPublisher(): Text
    var Pub: Codeunit VPub; M: Option; Seen: Text;
    begin
        M := 1;
        Pub.OnWithMembers(M, Seen);
        exit(Seen);
    end;

    procedure DirectLocalToMemberlessPublisher(): Text
    var Pub: Codeunit VPub; V: Option First,Second,Third; Seen: Text;
    begin
        V := V::Second;
        Pub.OnMemberless(V, Seen);
        exit(Seen);
    end;

    procedure UnheardMemberlessPublisher(): Text
    var Pub: Codeunit VPub; V: Option First,Second,Third; Row: Record VRow;
    begin
        V := V::Second;
        Pub.OnUnheardMemberless(V);
        Row.OptCap := Row.OptCap::Third;
        Pub.OnUnheardMemberless(Row.OptCap);
        exit(Format(V) + '|' + Format(V, 0, 2) + '@' + Format(Row.OptCap));
    end;

    procedure UnheardMembersPublisher(): Text
    var Pub: Codeunit VPub; M: Option; P: Option P,Q,R;
    begin
        M := 1;
        Pub.OnUnheardWithMembers(M);
        P := P::R;
        Pub.OnUnheardWithMembers(P);
        exit(Format(M) + '@' + Format(P));
    end;

    procedure DirectByValueToMemberlessPublisher(): Text
    begin
        exit(ByValueToMemberlessPublisher(1));
    end;

    local procedure ByValueToMemberlessPublisher(Value: Option ,PrimaryKey,NonKeyFields): Text
    var Pub: Codeunit VPub; Seen: Text;
    begin
        Pub.OnMemberless(Value, Seen);
        exit(Seen);
    end;

    procedure InterfaceByValue(): Text
    begin
        exit(ByValueToInterface(1));
    end;

    local procedure ByValueToInterface(Value: Option First,Second,Third): Text
    var Impl: Codeunit VFmtImpl; Fmt: Interface VFmt;
    begin
        Fmt := Impl;
        exit(Fmt.FormatInside(Value));
    end;

    procedure InterfaceArray(): Text
    var Impl: Codeunit VFmtImpl; Fmt: Interface VFmt; Arr: array[2] of Option First,Second,Third;
    begin
        Arr[1] := Arr[1]::Second;
        Fmt := Impl;
        exit(Fmt.FormatArray(Arr));
    end;

    procedure ProcedureGlobalArrayElement(): Text
    begin
        GArr[1] := GArr[1]::Second;
        exit(FormatInsideVar(GArr[1]));
    end;

    procedure ProcedureLocalArrayElement(): Text
    var Arr: array[2] of Option First,Second,Third;
    begin
        Arr[1] := Arr[1]::Second;
        exit(FormatInsideVar(Arr[1]) + '@' + Format(Arr[1]));
    end;

    procedure MemberlessVarParamToOtherMembers(): Text
    var V: Option First,Second,Third;
    begin
        V := V::Second;
        exit(MemberlessToOtherMembers(V) + '@' + Format(V));
    end;

    local procedure MemberlessToOtherMembers(var Value: Option): Text
    begin
        exit(OtherMembersFormat(Value));
    end;

    procedure CaptionNamesMembersToProcedure(): Text
    var Row: Record VRow;
    begin
        Row.OptNum := Row.OptNum::"1";
        exit(FormatInsideVar(Row.OptNum));
    end;

    procedure FieldToOtherMembers(): Text
    var Row: Record VRow;
    begin
        Row.Opt := Row.Opt::Second;
        exit(OtherMembersFormat(Row.Opt) + '@' + Format(Row.Opt));
    end;

    procedure VarArrayAssignOtherElement(): Text
    var Arr: array[2] of Option First,Second,Third;
    begin
        Arr[1] := Arr[1]::First;
        Arr[2] := Arr[2]::Third;
        exit(StoreFromOtherElement(Arr) + '@' + Format(Arr[1]) + '|' + Format(Arr[1], 0, 2));
    end;

    procedure VarArrayAssignSameElement(): Text
    var Arr: array[2] of Option First,Second,Third;
    begin
        Arr[1] := Arr[1]::Third;
        exit(StoreFromSameElement(Arr) + '@' + Format(Arr[1]) + '|' + Format(Arr[1], 0, 2));
    end;

    procedure ByValueArrayEvaluate(): Text
    var Arr: array[2] of Option First,Second,Third;
    begin
        Arr[1] := Arr[1]::Second;
        exit(ByValueEvaluate(Arr));
    end;

    procedure ByValueArrayClear(): Text
    var Arr: array[2] of Option First,Second,Third;
    begin
        Arr[1] := Arr[1]::Second;
        exit(ByValueClear(Arr));
    end;

    procedure ByValueArrayAssignElement(): Text
    var Arr: array[2] of Option First,Second,Third;
    begin
        Arr[1] := Arr[1]::Second;
        Arr[2] := Arr[2]::Third;
        exit(ByValueAssignElement(Arr));
    end;

    procedure CopyArrayEvaluate(): Text
    var Arr: array[2] of Option First,Second,Third; Arr0: array[2] of Option;
    begin
        Arr[1] := Arr[1]::Second;
        CopyArray(Arr0, Arr, 1);
        Evaluate(Arr0[1], 'Third');
        exit(Format(Arr0[1]));
    end;

    procedure CopyArrayAssignOption(): Text
    var Arr: array[2] of Option First,Second,Third; Arr0: array[2] of Option; P: Option P,Q,R;
    begin
        Arr[1] := Arr[1]::Second;
        CopyArray(Arr0, Arr, 1);
        P := P::R;
        Arr0[1] := P;
        exit(Format(Arr0[1]));
    end;

    procedure CopyArrayAssignElement(): Text
    var Arr: array[2] of Option First,Second,Third; Arr0: array[2] of Option;
    begin
        Arr[1] := Arr[1]::Second;
        Arr[2] := Arr[2]::Third;
        CopyArray(Arr0, Arr, 1);
        Arr0[1] := Arr0[2];
        exit(Format(Arr0[1]));
    end;

    procedure GlobalCopyArrayInteger(): Text
    begin
        GArr[1] := GArr[1]::Second;
        CopyArray(GArr0, GArr, 1);
        GArr0[1] := 2;
        exit(Format(GArr0[1]));
    end;

    procedure ElementToProcedureWriteBack(): Text
    var Arr: array[2] of Option First,Second,Third;
    begin
        Arr[1] := Arr[1]::Second;
        exit(ElementHandOn(Arr) + '@' + Format(Arr[1]));
    end;

    local procedure ElementHandOn(var A: array[2] of Option): Text
    begin
        exit(FormatInsideVar(A[1]));
    end;

    local procedure StoreFromOtherElement(var A: array[2] of Option): Text
    begin
        A[1] := A[2];
        exit(Format(A[1]) + '|' + Format(A[1], 0, 2));
    end;

    local procedure StoreFromSameElement(var A: array[2] of Option): Text
    begin
        A[1] := A[1];
        exit(Format(A[1]) + '|' + Format(A[1], 0, 2));
    end;

    procedure VarArrayAssignOtherArrayElement(): Text
    var Arr: array[2] of Option First,Second,Third; Other: array[2] of Option P,Q,R;
    begin
        Arr[1] := Arr[1]::First;
        Other[2] := Other[2]::R;
        exit(StoreFromOtherArray(Arr, Other));
    end;

    local procedure StoreFromOtherArray(var A: array[2] of Option; var B: array[2] of Option): Text
    begin
        A[1] := B[2];
        exit(Format(A[1]) + '|' + Format(A[1], 0, 2));
    end;

    procedure VarArrayAssignVarParameter(): Text
    var Arr: array[2] of Option First,Second,Third; V: Option P,Q,R;
    begin
        Arr[1] := Arr[1]::First;
        V := V::R;
        exit(StoreFromVarParameter(Arr, V));
    end;

    local procedure StoreFromVarParameter(var A: array[2] of Option; var Value: Option): Text
    begin
        A[1] := Value;
        exit(Format(A[1]) + '|' + Format(A[1], 0, 2));
    end;

    procedure VarArrayAssignGlobal(): Text
    var Arr: array[2] of Option First,Second,Third;
    begin
        Arr[1] := Arr[1]::First;
        GlobalPqr := GlobalPqr::R;
        exit(StoreFromGlobal(Arr));
    end;

    local procedure StoreFromGlobal(var A: array[2] of Option): Text
    begin
        A[1] := GlobalPqr;
        exit(Format(A[1]) + '|' + Format(A[1], 0, 2));
    end;

    procedure VarArrayAssignLocal(): Text
    var Arr: array[2] of Option First,Second,Third;
    begin
        Arr[1] := Arr[1]::First;
        exit(StoreFromLocal(Arr));
    end;

    local procedure StoreFromLocal(var A: array[2] of Option): Text
    var L: Option P,Q,R;
    begin
        L := L::R;
        A[1] := L;
        exit(Format(A[1]) + '|' + Format(A[1], 0, 2));
    end;

    procedure LocalArrayAssignVarParameter(): Text
    var V: Option P,Q,R;
    begin
        V := V::R;
        exit(LocalElementFromVarParameter(V));
    end;

    local procedure LocalElementFromVarParameter(var Value: Option): Text
    var A: array[2] of Option;
    begin
        A[1] := Value;
        exit(Format(A[1]) + '|' + Format(A[1], 0, 2));
    end;

    procedure UnheardFromParameters(): Text
    var V: Option First,Second,Third; Seen: Text;
    begin
        V := V::Second;
        UnheardNamedReturn(Seen);
        exit(UnheardByValue(V) + '@' + Seen);
    end;

    local procedure UnheardByValue(Value: Option First,Second,Third): Text
    var Pub: Codeunit VPub;
    begin
        Pub.OnUnheardMemberless(Value);
        exit(Format(Value) + '|' + Format(Value, 0, 2));
    end;

    local procedure UnheardNamedReturn(var Seen: Text) Value: Option First,Second,Third
    var Pub: Codeunit VPub;
    begin
        Value := Value::Second;
        Pub.OnUnheardMemberless(Value);
        Seen := Format(Value) + '|' + Format(Value, 0, 2);
    end;

    procedure UnheardFromMemberlessVarParam(): Text
    var V: Option First,Second,Third; Inside: Text;
    begin
        V := V::Second;
        Inside := HandOnToUnheard(V);
        exit(Inside + '@' + Format(V) + '|' + Format(V, 0, 2));
    end;

    procedure UnheardThroughVarParamSameMembers(): Text
    var V: Option A,B,C; Inside: Text;
    begin
        V := V::B;
        Inside := HandOnSameMembersToUnheard(V);
        exit(Inside + '@' + Format(V) + '|' + Format(V, 0, 2));
    end;

    local procedure HandOnSameMembersToUnheard(var Value: Option A,B,C): Text
    var Pub: Codeunit VPub;
    begin
        Pub.OnUnheardWithMembers(Value);
        exit(Format(Value) + '|' + Format(Value, 0, 2));
    end;

    procedure UnheardThroughMemberlessVarParamMemberless(): Text
    var V: Option; Inside: Text;
    begin
        V := 1;
        Inside := HandOnMemberlessToUnheardMembers(V);
        exit(Inside + '@' + Format(V) + '|' + Format(V, 0, 2));
    end;

    local procedure HandOnMemberlessToUnheardMembers(var Value: Option): Text
    var Pub: Codeunit VPub;
    begin
        Pub.OnUnheardWithMembers(Value);
        exit(Format(Value) + '|' + Format(Value, 0, 2));
    end;

    local procedure HandOnToUnheard(var Value: Option): Text
    var Pub: Codeunit VPub;
    begin
        Pub.OnUnheardMemberless(Value);
        exit(Format(Value) + '|' + Format(Value, 0, 2));
    end;

    procedure UnheardFromVarArrayElement(): Text
    var Arr: array[2] of Option First,Second,Third;
    begin
        Arr[2] := Arr[2]::Third;
        exit(ElementToUnheard(Arr));
    end;

    local procedure ElementToUnheard(var A: array[2] of Option): Text
    var Pub: Codeunit VPub;
    begin
        Pub.OnUnheardMemberless(A[2]);
        exit(Format(A[2]) + '|' + Format(A[2], 0, 2));
    end;

    procedure UnheardThenMemberlessVarParam(): Text
    var Pub: Codeunit VPub; V: Option A,B,C;
    begin
        V := V::B;
        Pub.OnUnheardWithMembers(V);
        exit(FormatInsideVar(V) + '|' + Format(V, 0, 2));
    end;

    procedure UnheardFieldThenMemberless(): Text
    var Pub: Codeunit VPub; Row: Record VRow;
    begin
        Row.Abc := Row.Abc::B;
        Pub.OnUnheardWithMembers(Row.Abc);
        exit(FormatInsideVar(Row.Abc) + '|' + Format(Row.Abc, 0, 2));
    end;

    procedure UnheardGlobalRecordField(): Text
    var Pub: Codeunit VPub;
    begin
        GRow.Abc := GRow.Abc::B;
        Pub.OnUnheardWithMembers(GRow.Abc);
        exit(Format(GRow.Abc) + '|' + Format(GRow.Abc, 0, 2));
    end;

    procedure UnheardByValueRecordField(): Text
    var Row: Record VRow;
    begin
        Row.Abc := Row.Abc::C;
        exit(ByValueRecordToUnheard(Row));
    end;

    local procedure ByValueRecordToUnheard(Row: Record VRow): Text
    var Pub: Codeunit VPub;
    begin
        Pub.OnUnheardWithMembers(Row.Abc);
        exit(Format(Row.Abc) + '|' + Format(Row.Abc, 0, 2));
    end;

    procedure UnheardElementThenMemberless(): Text
    var Pub: Codeunit VPub; Arr: array[2] of Option A,B,C;
    begin
        Arr[1] := Arr[1]::B;
        Pub.OnUnheardWithMembers(Arr[1]);
        exit(FormatInsideVar(Arr[1]) + '|' + Format(Arr[1], 0, 2));
    end;

    procedure UnheardGlobalThenMemberless(): Text
    var Pub: Codeunit VPub;
    begin
        GAbc := GAbc::B;
        Pub.OnUnheardWithMembers(GAbc);
        exit(FormatInsideVar(GAbc) + '|' + Format(GAbc, 0, 2));
    end;

    procedure UnheardArrayOtherMembers(): Text
    var Pub: Codeunit VPub; Arr: array[2] of Option A,B,C;
    begin
        Arr[1] := Arr[1]::B;
        Pub.OnUnheardArray(Arr);
        exit(FormatInsideVar(Arr[1]) + '|' + Format(Arr[1], 0, 2));
    end;

    procedure UnheardBaseTableVar(): Text
    var R: Record VBase;
    begin
        exit(R.UnheardBaseVar());
    end;

    procedure UnheardThenEvaluateInside(): Text
    var Pub: Codeunit VPub; V: Option A,B,C;
    begin
        V := V::B;
        Pub.OnUnheardWithMembers(V);
        EvaluateInsideVar(V, 'C');
        exit(Format(V) + '|' + Format(V, 0, 2));
    end;

    local procedure EvaluateInsideVar(var Value: Option; T: Text)
    begin
        Evaluate(Value, T);
    end;

    procedure UnheardThenElementStore(): Text
    var Arr: array[2] of Option A,B,C;
    begin
        exit(ElementFromUnheardLocal(Arr));
    end;

    local procedure ElementFromUnheardLocal(var A: array[2] of Option): Text
    var Pub: Codeunit VPub; L: Option A,B,C;
    begin
        L := L::C;
        Pub.OnUnheardWithMembers(L);
        A[1] := L;
        exit(Format(A[1]) + '|' + Format(A[1], 0, 2));
    end;

    local procedure ByValueEvaluate(A: array[2] of Option): Text
    begin
        Evaluate(A[1], 'Third');
        exit(Format(A[1]));
    end;

    local procedure ByValueClear(A: array[2] of Option): Text
    begin
        Clear(A[1]);
        exit(Format(A[1]));
    end;

    local procedure ByValueAssignElement(A: array[2] of Option): Text
    begin
        A[1] := A[2];
        exit(Format(A[1]));
    end;

    local procedure FormatInsideVar(var Value: Option): Text
    begin
        exit(Format(Value));
    end;

    local procedure OtherMembersFormat(var Value: Option A,B,C): Text
    begin
        exit(Format(Value));
    end;

    procedure UnheardFromCopyArrayElement(): Text
    var Pub: Codeunit VPub; Arr: array[2] of Option A,B,C; Arr0: array[2] of Option;
    begin
        Arr[1] := Arr[1]::B;
        CopyArray(Arr0, Arr, 1);
        Pub.OnUnheardWithMembers(Arr0[1]);
        exit(Format(Arr0[1]) + '|' + Format(Arr0[1], 0, 2));
    end;

    procedure UnheardFromByValueArrayElement(): Text
    var Arr: array[2] of Option A,B,C;
    begin
        Arr[2] := Arr[2]::C;
        exit(ByValueElementToUnheard(Arr) + '>' + Format(Arr[2]) + '|' + Format(Arr[2], 0, 2));
    end;

    local procedure ByValueElementToUnheard(A: array[2] of Option): Text
    var Pub: Codeunit VPub;
    begin
        Pub.OnUnheardWithMembers(A[2]);
        exit(Format(A[2]) + '|' + Format(A[2], 0, 2));
    end;

    procedure UnheardFromPlainMemberlessElement(): Text
    var Pub: Codeunit VPub; Arr0: array[2] of Option;
    begin
        Arr0[1] := 2;
        Pub.OnUnheardMemberless(Arr0[1]);
        exit(Format(Arr0[1]) + '|' + Format(Arr0[1], 0, 2));
    end;

    procedure HeardMemberlessArrayElement(): Text
    var Pub: Codeunit VPub; Arr0: array[2] of Option; Seen: Text;
    begin
        Arr0[1] := 2;
        Pub.OnMemberless(Arr0[1], Seen);
        exit(Seen + '>' + Format(Arr0[1]));
    end;

    procedure GlobalToOtherMembersProcedure(): Text
    var T: Text;
    begin
        GAbc := GAbc::B;
        T := OtherMembersOrdinal(GAbc);
        exit(T + '>' + Format(GAbc) + '|' + Format(GAbc, 0, 2));
    end;

    local procedure OtherMembersOrdinal(var Value: Option X,Y,Z): Text
    begin
        exit(Format(Value) + '|' + Format(Value, 0, 2));
    end;
}
"#;

fn run(routine: &str) -> (Result<String, Signal>, bool) {
    assumptions::clear_hits();
    let out = al_testkit::run_in(SRC, "C", routine).map(|v| text(Ok(v)));
    (
        out,
        assumptions::take_hits().contains(&Assumption::VarOptionCallerMembers),
    )
}

fn assumed(routine: &str) -> String {
    match run(routine) {
        (Ok(out), true) => out,
        other => panic!("{routine}: expected a flagged result, got {other:?}"),
    }
}

fn clean(routine: &str) -> String {
    match run(routine) {
        (Ok(out), false) => out,
        other => panic!("{routine}: expected a result that is not flagged, got {other:?}"),
    }
}

fn refused(routine: &str, reason: &str) {
    match run(routine).0 {
        Err(Signal::Unsupported(why)) => assert!(why.contains(reason), "{routine}: {why}"),
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    }
}

const SUBSCRIBER: &str = "an event subscriber's `var` Option parameter declared without members";
const CALLER: &str = "from a caller the probe does not measure";
const PUBLISHER: &str = "handed directly to an event publisher's";
const ELEMENT: &str = "a store the probe does not measure into an element";
const UNHEARD: &str = "which no subscriber hears, from a place the probe does not measure";

// A memberless subscriber under a publisher with members runs only through the measured hop: a procedure's `var` parameter
// with the publisher's members, handed a local or a field without OptionCaption of a local record
// (OptionVarMemberLists.al:531; OVMC:311).
#[test]
fn the_measured_hop_to_a_memberless_subscriber_runs() {
    assert_eq!(assumed("LocalHop"), "Second|1");
    assert_eq!(assumed("LocalFieldHop"), "Second|1");
}

// A caption field through the hop: BC's subscriber shows the caption, Two (OVMC:290), which the value does not carry. Through
// a global record or two hops it is not measured either.
#[test]
fn a_caption_field_handed_on_to_a_memberless_subscriber_refuses() {
    refused("CaptionFieldHop", SUBSCRIBER);
    refused("GlobalCaptionFieldHop", SUBSCRIBER);
    refused("CaptionFieldTwoHops", SUBSCRIBER);
}

// Not measured: two hops, a global record's field, a caller without members, the publisher called directly.
#[test]
fn unmeasured_callers_of_a_memberless_subscriber_refuse() {
    refused("LocalTwoHops", SUBSCRIBER);
    refused("GlobalRecordFieldHop", SUBSCRIBER);
    refused("MemberlessLocalHop", SUBSCRIBER);
    refused("DirectLocalToMembersPublisher", SUBSCRIBER);
    refused("DirectMemberlessLocalToMembersPublisher", PUBLISHER);
}

// A publisher called directly with a variable of another member list that a subscriber hears refuses when its parameter
// has no members (only a parameter with members is measured, OptionVarMemberlessHeard.al). One no subscriber hears changes
// nothing, flagged: the caller's variable keeps its value and members, also an
// element of a `var` memberless array that holds its caller's members (corpus PR 1444, OptionVarMemberlessUnheard.al:115,
// :281, :327, :347).
#[test]
fn a_publisher_called_directly_refuses_only_when_a_subscriber_hears_it() {
    refused("DirectLocalToMemberlessPublisher", PUBLISHER);
    refused("DirectByValueToMemberlessPublisher", PUBLISHER);
    assert_eq!(assumed("UnheardMemberlessPublisher"), "Second|1@Three");
    assert_eq!(assumed("UnheardMembersPublisher"), "1@R");
    assert_eq!(assumed("UnheardFromParameters"), "Second|1@Second|1");
    assert_eq!(assumed("UnheardFromMemberlessVarParam"), "Second|1@Second|1");
    assert_eq!(assumed("UnheardFromVarArrayElement"), "Third|2");
    assert_eq!(assumed("UnheardThroughVarParamSameMembers"), "B|1@B|1");
    assert_eq!(assumed("UnheardThroughMemberlessVarParamMemberless"), "1|1@1|1");
}

// No later read meets a member list an unheard publisher left: a memberless `var` parameter reads the caller's own members
// (OptionVarMemberlessUnheard.al:555), Evaluate takes the caller's member names (:635), and an element store from a
// variable declared with the held members keeps them with the ordinal (:1020).
#[test]
fn no_read_after_an_unheard_publisher_sees_its_member_list() {
    assert_eq!(assumed("UnheardThenMemberlessVarParam"), "B|1");
    assert_eq!(assumed("UnheardThenEvaluateInside"), "C|2");
    assert_eq!(assumed("UnheardThenElementStore"), "C|2");
}

// After an unheard publisher a global, a field of a local record, an element of an array with members, an element of a
// CopyArray filled or by-value memberless array and a whole array keep their value and members (OptionVarMemberlessUnheard.al
// :176, :216, :244, :281). A field of a global or by-value record has no cell: main's write-back. A global of a base table is
// not measured: refused.
#[test]
fn other_places_handed_to_an_unheard_publisher_keep_main_or_refuse() {
    assert_eq!(assumed("UnheardFieldThenMemberless"), "B|1");
    // A field of a global record or of a record passed by value: no cell; main's write-back, which leaves the value as it was.
    assert_eq!(clean("UnheardGlobalRecordField"), "B|1");
    assert_eq!(clean("UnheardByValueRecordField"), "C|2");
    assert_eq!(assumed("UnheardElementThenMemberless"), "B|1");
    assert_eq!(assumed("UnheardArrayOtherMembers"), "B|1");
    assert_eq!(assumed("UnheardGlobalThenMemberless"), "B|1");
    assert_eq!(assumed("UnheardFromCopyArrayElement"), "B|1");
    assert_eq!(assumed("UnheardFromByValueArrayElement"), "C|2>C|2");
    refused("UnheardBaseTableVar", UNHEARD);
}

// No cell hands an interface method a by-value parameter or an array, or a procedure an element of a global array; a
// memberless `var` parameter handed to other members and a field whose OptionCaption names its members are not measured
// either.
#[test]
fn unmeasured_pairs_of_call_and_caller_refuse() {
    refused("InterfaceByValue", CALLER);
    refused("InterfaceArray", CALLER);
    refused("ProcedureGlobalArrayElement", CALLER);
    refused("MemberlessVarParamToOtherMembers", CALLER);
    refused("CaptionNamesMembersToProcedure", CALLER);
}

// Neighbors: an element of a local array to a procedure is measured (OptionVarMemberLists.al:491); a field to a parameter
// with other members converts as on main; a global with members to a procedure's parameter with other members keeps its
// own (OptionVarMemberlessUnheard.al:415, :423).
#[test]
fn measured_and_main_neighbors_keep_running() {
    assert_eq!(assumed("ProcedureLocalArrayElement"), "Second@Second");
    assert_eq!(clean("FieldToOtherMembers"), "B@Second");
    assert_eq!(assumed("GlobalToOtherMembersProcedure"), "Y|1>B|1");
    // An element of an array of memberless Options to a publisher a subscriber hears: main's binding and write-back.
    assert_eq!(clean("HeardMemberlessArrayElement"), "2|2>2");
    // One that holds no caller's members, to a publisher no subscriber hears: main's path.
    assert_eq!(clean("UnheardFromPlainMemberlessElement"), "2|2");
}

// `A[1] := A[2]` and `A[2] := A[2]` in a `var` array parameter of memberless Options keep the held members on a local BC
// of the newest supported major (corpus PR 1439 extension, local commit da4c8406, OptionVarMemberlessElementStores.al:77,
// not pushed yet).
// An element is no variable of the `var` section: refused.
#[test]
fn an_element_assigned_from_an_element_refuses() {
    refused("VarArrayAssignOtherElement", ELEMENT);
    refused("VarArrayAssignSameElement", ELEMENT);
}

// An Option stored into an element that holds a caller's members runs only from a variable of the routine's `var` section
// (OVMW:762, :771). From an element of another array, a memberless `var` parameter that holds another list, or a
// global: refused. Into an element that holds no members the store converts as on main.
#[test]
fn an_element_assigned_from_an_unmeasured_place_refuses() {
    refused("VarArrayAssignOtherArrayElement", ELEMENT);
    refused("VarArrayAssignVarParameter", ELEMENT);
    refused("VarArrayAssignGlobal", ELEMENT);
    assert_eq!(assumed("VarArrayAssignLocal"), "Third|2");
    assert_eq!(assumed("LocalArrayAssignVarParameter"), "2|2");
}

// Into a by-value array parameter or a local array CopyArray filled only an Integer store is measured (OVMW:833, :560); into a
// global array CopyArray filled none. A procedure's write-back into an element is not measured.
#[test]
fn unmeasured_element_stores_refuse() {
    for routine in [
        "ByValueArrayEvaluate",
        "ByValueArrayClear",
        "ByValueArrayAssignElement",
        "CopyArrayEvaluate",
        "CopyArrayAssignOption",
        "CopyArrayAssignElement",
        "GlobalCopyArrayInteger",
        "ElementToProcedureWriteBack",
    ] {
        refused(routine, ELEMENT);
    }
}
