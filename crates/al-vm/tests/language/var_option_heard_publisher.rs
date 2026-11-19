//! An Option variable handed directly to an event publisher's `var` Option parameter with members P,Q,R, which a subscriber
//! hears (al-runner issue 8384, assumption `var-option-caller-members`): the subscriber reads the publisher's members and
//! the caller's variable takes back the ordinal with its own members. Measured on a local BC of the newest supported major
//! by the corpus probe tests/al-language/types/enum-option/OptionVarMemberlessHeard.al (OVH). A `var` parameter of the
//! running routine handed on and a global are measured to show the subscriber another list than the publisher's, and an
//! error after the subscriber's write to keep the write; the run does not carry these: refused, with the shapes the probe
//! does not measure.

use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69860 HRow
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Opt; Option) { OptionMembers = First,Second,Third; }
        field(3; OptCap; Option) { OptionMembers = First,Second,Third; OptionCaption = 'One,Two,Three'; }
    }
}

codeunit 69861 HPub
{
    [IntegrationEvent(false, false)]
    procedure OnIgnored(var Value: Option P,Q,R)
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnRead(var Value: Option P,Q,R; var Seen: Text)
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnWritten(var Value: Option P,Q,R; var Seen: Text)
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnWrittenInteger(var Value: Option P,Q,R; var Seen: Text)
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnTwice(var Value: Option P,Q,R; var Seen: Text)
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnWrittenThenError(var Value: Option P,Q,R)
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnMemberlessRead(var Value: Option; var Seen: Text)
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnOtherList(var Value: Option P,Q,R; var Seen: Text)
    begin
    end;

    [IntegrationEvent(false, false, true)]
    procedure OnIsolated(var Value: Option P,Q,R; var Seen: Text)
    begin
    end;

    [BusinessEvent(false)]
    procedure OnBusiness(var Value: Option P,Q,R; var Seen: Text)
    begin
    end;

    [IntegrationEvent(true, false)]
    procedure OnWithSender(var Value: Option P,Q,R; var Seen: Text)
    begin
    end;

    [IntegrationEvent(false, true)]
    procedure OnGlobalAccess(var Value: Option P,Q,R; var Seen: Text)
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnManual(var Value: Option P,Q,R; var Seen: Text)
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnByValueSubscriber(var Value: Option P,Q,R; var Seen: Text)
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnTwoOptions(var Value: Option P,Q,R; var Other: Option P,Q,R)
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnNest(var Seen: Text)
    begin
    end;
}

codeunit 69864 HManualSub
{
    EventSubscriberInstance = Manual;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::HPub, 'OnManual', '', false, false)]
    local procedure Manual(var Value: Option P,Q,R; var Seen: Text)
    begin
        Seen := Format(Value);
    end;
}

codeunit 69865 HRunner
{
    trigger OnRun()
    var Pub: Codeunit HPub; V0: Option; Seen: Text;
    begin
        V0 := 1;
        Pub.OnRead(V0, Seen);
    end;
}

codeunit 69862 HSub
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::HPub, 'OnIgnored', '', false, false)]
    local procedure Ignored()
    begin
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::HPub, 'OnRead', '', false, false)]
    local procedure Read(var Value: Option P,Q,R; var Seen: Text)
    begin
        Seen := Format(Value) + '|' + Format(Value, 0, 2);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::HPub, 'OnWritten', '', false, false)]
    local procedure Written(var Value: Option P,Q,R; var Seen: Text)
    begin
        Seen := Format(Value) + '|' + Format(Value, 0, 2);
        Value := Value::R;
        Seen += '~' + Format(Value) + '|' + Format(Value, 0, 2);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::HPub, 'OnWrittenInteger', '', false, false)]
    local procedure WrittenInteger(var Value: Option P,Q,R; var Seen: Text)
    begin
        Value := 0;
        Seen := Format(Value) + '|' + Format(Value, 0, 2);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::HPub, 'OnTwice', '', false, false)]
    local procedure TwiceIgnored()
    begin
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::HPub, 'OnTwice', '', false, false)]
    local procedure TwiceWritten(var Value: Option P,Q,R; var Seen: Text)
    begin
        Value := Value::R;
        Seen := Format(Value) + '|' + Format(Value, 0, 2);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::HPub, 'OnWrittenThenError', '', false, false)]
    local procedure WrittenThenError(var Value: Option P,Q,R)
    begin
        Value := Value::R;
        Error('OVH subscriber error');
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::HPub, 'OnMemberlessRead', '', false, false)]
    local procedure MemberlessRead(var Value: Option; var Seen: Text)
    begin
        Seen := Format(Value) + '|' + Format(Value, 0, 2);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::HPub, 'OnOtherList', '', false, false)]
    local procedure OtherList(var Value: Option X,Y,Z; var Seen: Text)
    begin
        Seen := Format(Value) + '|' + Format(Value, 0, 2);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::HPub, 'OnIsolated', '', false, false)]
    local procedure Isolated(var Value: Option P,Q,R; var Seen: Text)
    begin
        Seen := Format(Value);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::HPub, 'OnBusiness', '', false, false)]
    local procedure Business(var Value: Option P,Q,R; var Seen: Text)
    begin
        Seen := Format(Value);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::HPub, 'OnWithSender', '', false, false)]
    local procedure WithSender(var Value: Option P,Q,R; var Seen: Text)
    begin
        Seen := Format(Value);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::HPub, 'OnGlobalAccess', '', false, false)]
    local procedure GlobalAccess(var Value: Option P,Q,R; var Seen: Text)
    begin
        Seen := Format(Value);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::HPub, 'OnByValueSubscriber', '', false, false)]
    local procedure ByValueSubscriber(Value: Option P,Q,R; var Seen: Text)
    begin
        Seen := Format(Value);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::HPub, 'OnTwoOptions', '', false, false)]
    local procedure TwoOptions(var Value: Option P,Q,R; var Other: Option P,Q,R)
    begin
        Value := Value::R;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::HPub, 'OnNest', '', false, false)]
    local procedure RaiseFromSubscriber(var Seen: Text)
    var Pub: Codeunit HPub; V0: Option;
    begin
        V0 := 1;
        Pub.OnRead(V0, Seen);
    end;
}

codeunit 69863 C
{
    var
        Pub: Codeunit HPub;
        GlobalV: Option First,Second,Third;
        GlobalV0: Option;
        GlobalRow: Record HRow;
        GlobalArr: array[2] of Option First,Second,Third;

    local procedure F(V: Option First,Second,Third): Text
    begin
        exit(Format(V) + '|' + Format(V, 0, 2));
    end;

    local procedure F0(V: Option): Text
    begin
        exit(Format(V) + '|' + Format(V, 0, 2));
    end;

    procedure MemberlessLocal(): Text
    var V1: Option; V2: Option; V3: Option; V4: Option; V5: Option; V6: Option; V7: Option; Seen: Text; T: Text;
    begin
        V1 := 1;
        Pub.OnIgnored(V1);
        T := F0(V1);
        V2 := 1;
        Pub.OnRead(V2, Seen);
        T += ';' + Seen + '>' + F0(V2);
        V3 := 1;
        Pub.OnWritten(V3, Seen);
        T += ';' + Seen + '>' + F0(V3);
        V4 := 1;
        Pub.OnWrittenInteger(V4, Seen);
        T += ';' + Seen + '>' + F0(V4);
        V5 := 1;
        Pub.OnTwice(V5, Seen);
        T += ';' + Seen + '>' + F0(V5);
        V6 := 7;
        Pub.OnIgnored(V6);
        T += ';' + F0(V6);
        V7 := 7;
        Pub.OnRead(V7, Seen);
        exit(T + ';' + Seen + '>' + F0(V7));
    end;

    procedure LocalWithMembers(): Text
    var V1: Option First,Second,Third; V2: Option First,Second,Third; V3: Option First,Second,Third; V4: Option First,Second,Third; V5: Option First,Second,Third; V6: Option First,Second,Third; Seen: Text; T: Text;
    begin
        V1 := V1::Second;
        Pub.OnIgnored(V1);
        T := F(V1);
        V2 := V2::Second;
        Pub.OnRead(V2, Seen);
        T += ';' + Seen + '>' + F(V2);
        V3 := V3::Second;
        Pub.OnWritten(V3, Seen);
        T += ';' + Seen + '>' + F(V3);
        V4 := V4::Second;
        Pub.OnWrittenInteger(V4, Seen);
        T += ';' + Seen + '>' + F(V4);
        V5 := V5::Second;
        Pub.OnTwice(V5, Seen);
        T += ';' + Seen + '>' + F(V5);
        V6 := V6::Second;
        Pub.OnWritten(V6, Seen);
        V6 := V6::First;
        exit(T + ';' + F(V6));
    end;

    procedure Globals(): Text
    var Seen: Text; T: Text;
    begin
        GlobalV := GlobalV::Second;
        Pub.OnRead(GlobalV, Seen);
        T := Seen + '>' + F(GlobalV);
        GlobalV := GlobalV::Second;
        Pub.OnWritten(GlobalV, Seen);
        T += ';' + Seen + '>' + F(GlobalV);
        GlobalV := GlobalV::Second;
        Pub.OnIgnored(GlobalV);
        T += ';' + F(GlobalV);
        GlobalV0 := 1;
        Pub.OnRead(GlobalV0, Seen);
        T += ';' + Seen + '>' + F0(GlobalV0);
        GlobalV0 := 1;
        Pub.OnWritten(GlobalV0, Seen);
        exit(T + ';' + Seen + '>' + F0(GlobalV0));
    end;

    procedure MemberlessGlobal(): Text
    var Seen: Text;
    begin
        GlobalV0 := 1;
        Pub.OnRead(GlobalV0, Seen);
        exit(Seen);
    end;

    procedure ByValueAndNamedReturn(): Text
    var V: Option First,Second,Third; V0: Option; T: Text;
    begin
        V := V::Second;
        T := ByValueRaise(V) + ';' + F(V);
        V0 := 1;
        T += ';' + ByValueMemberlessRaise(V0);
        V := NamedReturnRaise(T);
        exit(T + ';' + F(V));
    end;

    local procedure ByValueRaise(V: Option First,Second,Third): Text
    var Seen: Text; T: Text;
    begin
        Pub.OnRead(V, Seen);
        T := Seen + '>' + F(V);
        Pub.OnWritten(V, Seen);
        exit(T + '>' + Seen + '>' + F(V));
    end;

    local procedure ByValueMemberlessRaise(V: Option): Text
    var Seen: Text; T: Text;
    begin
        Pub.OnRead(V, Seen);
        T := Seen + '>' + F0(V);
        Pub.OnWritten(V, Seen);
        exit(T + '>' + Seen + '>' + F0(V));
    end;

    local procedure NamedReturnRaise(var T: Text) V: Option First,Second,Third
    var Seen: Text;
    begin
        V := V::Second;
        Pub.OnRead(V, Seen);
        T += ';' + Seen + '>' + F(V);
        Pub.OnWritten(V, Seen);
        T += '>' + Seen + '>' + F(V);
    end;

    procedure Fields(): Text
    var Row: Record HRow; Seen: Text; T: Text;
    begin
        Row.Opt := Row.Opt::Second;
        Pub.OnRead(Row.Opt, Seen);
        T := Seen + '>' + Format(Row.Opt) + '|' + Format(Row.Opt, 0, 2);
        Row.Opt := Row.Opt::Second;
        Pub.OnWritten(Row.Opt, Seen);
        T += ';' + Seen + '>' + Format(Row.Opt) + '|' + Format(Row.Opt, 0, 2);
        Row.Opt := Row.Opt::Second;
        Pub.OnIgnored(Row.Opt);
        T += ';' + Format(Row.Opt) + '|' + Format(Row.Opt, 0, 2);
        Row.OptCap := Row.OptCap::Second;
        Pub.OnRead(Row.OptCap, Seen);
        T += ';' + Seen + '>' + Format(Row.OptCap) + '|' + Format(Row.OptCap, 0, 2);
        Row.OptCap := Row.OptCap::Second;
        Pub.OnWritten(Row.OptCap, Seen);
        T += ';' + Seen + '>' + Format(Row.OptCap) + '|' + Format(Row.OptCap, 0, 2);
        GlobalRow.Opt := GlobalRow.Opt::Second;
        Pub.OnWritten(GlobalRow.Opt, Seen);
        T += ';' + Seen + '>' + Format(GlobalRow.Opt) + '|' + Format(GlobalRow.Opt, 0, 2);
        Row.Opt := Row.Opt::Second;
        T += ';' + VarRecordRaise(Row);
        exit(T + ';' + Format(Row.Opt) + '|' + Format(Row.Opt, 0, 2));
    end;

    local procedure VarRecordRaise(var Row: Record HRow): Text
    var Seen: Text; T: Text;
    begin
        Pub.OnRead(Row.Opt, Seen);
        T := Seen + '>' + Format(Row.Opt) + '|' + Format(Row.Opt, 0, 2);
        Pub.OnWritten(Row.Opt, Seen);
        exit(T + '>' + Seen + '>' + Format(Row.Opt) + '|' + Format(Row.Opt, 0, 2));
    end;

    procedure Elements(): Text
    var Arr: array[2] of Option First,Second,Third; Arr0: array[2] of Option; Seen: Text; T: Text;
    begin
        Arr[1] := Arr[1]::Second;
        Pub.OnRead(Arr[1], Seen);
        T := Seen + '>' + F(Arr[1]);
        Arr[1] := Arr[1]::Second;
        Pub.OnWritten(Arr[1], Seen);
        T += ';' + Seen + '>' + F(Arr[1]) + '/' + F(Arr[2]);
        GlobalArr[1] := GlobalArr[1]::Second;
        Pub.OnWritten(GlobalArr[1], Seen);
        T += ';' + Seen + '>' + F(GlobalArr[1]);
        Arr0[1] := 1;
        Pub.OnRead(Arr0[1], Seen);
        T += ';' + Seen + '>' + F0(Arr0[1]);
        Arr0[1] := 1;
        Pub.OnWritten(Arr0[1], Seen);
        exit(T + ';' + Seen + '>' + F0(Arr0[1]));
    end;

    procedure ErrorAfterWrite(): Text
    var V: Option First,Second,Third; V0: Option; T: Text;
    begin
        V := V::Second;
        asserterror Pub.OnWrittenThenError(V);
        T := GetLastErrorText() + '>' + F(V);
        V0 := 1;
        asserterror Pub.OnWrittenThenError(V0);
        exit(T + ';' + GetLastErrorText() + '>' + F0(V0));
    end;

    procedure VarWithMembers(): Text
    var V: Option First,Second,Third;
    begin
        V := V::Second;
        exit(VarWithMembersRaise(V));
    end;

    local procedure VarWithMembersRaise(var V: Option First,Second,Third): Text
    var Seen: Text;
    begin
        Pub.OnRead(V, Seen);
        exit(Seen);
    end;

    procedure MemberlessVar(): Text
    var V: Option First,Second,Third;
    begin
        V := V::Second;
        exit(MemberlessVarRaise(V));
    end;

    local procedure MemberlessVarRaise(var V: Option): Text
    var Seen: Text;
    begin
        Pub.OnRead(V, Seen);
        exit(Seen);
    end;

    procedure MemberlessPublisher(): Text
    var V: Option First,Second,Third; Seen: Text;
    begin
        V := V::Second;
        Pub.OnMemberlessRead(V, Seen);
        exit(Seen);
    end;

    procedure SubscriberOtherList(): Text
    var V0: Option; Seen: Text;
    begin
        V0 := 1;
        Pub.OnOtherList(V0, Seen);
        exit(Seen);
    end;

    procedure ByValueRecordField(): Text
    var Row: Record HRow;
    begin
        Row.Opt := Row.Opt::Second;
        exit(ByValueRecordRaise(Row));
    end;

    local procedure ByValueRecordRaise(Row: Record HRow): Text
    var Seen: Text;
    begin
        Pub.OnRead(Row.Opt, Seen);
        exit(Seen);
    end;

    procedure VarArrayElement(): Text
    var Arr: array[2] of Option First,Second,Third;
    begin
        Arr[1] := Arr[1]::Second;
        exit(VarArrayRaise(Arr));
    end;

    local procedure VarArrayRaise(var A: array[2] of Option First,Second,Third): Text
    var Seen: Text;
    begin
        Pub.OnRead(A[1], Seen);
        exit(Seen);
    end;

    procedure MemberlessNamedReturn(): Text
    var T: Text;
    begin
        MemberlessNamedReturnRaise(T);
        exit(T);
    end;

    procedure IsolatedEvent(): Text
    var V0: Option; Seen: Text;
    begin
        V0 := 1;
        Pub.OnIsolated(V0, Seen);
        exit(Seen);
    end;

    procedure BusinessEvent(): Text
    var V0: Option; Seen: Text;
    begin
        V0 := 1;
        Pub.OnBusiness(V0, Seen);
        exit(Seen);
    end;

    procedure WithSender(): Text
    var V0: Option; Seen: Text;
    begin
        V0 := 1;
        Pub.OnWithSender(V0, Seen);
        exit(Seen);
    end;

    procedure GlobalAccess(): Text
    var V0: Option; Seen: Text;
    begin
        V0 := 1;
        Pub.OnGlobalAccess(V0, Seen);
        exit(Seen);
    end;

    procedure ManualSubscriber(): Text
    var V0: Option; Seen: Text; Manual: Codeunit HManualSub;
    begin
        BindSubscription(Manual);
        V0 := 1;
        Pub.OnManual(V0, Seen);
        exit(Seen);
    end;

    procedure ByValueSubscriber(): Text
    var V0: Option; Seen: Text;
    begin
        V0 := 1;
        Pub.OnByValueSubscriber(V0, Seen);
        exit(Seen);
    end;

    procedure TwoOptions(): Text
    var V0: Option; Other: Option P,Q,R;
    begin
        V0 := 1;
        Pub.OnTwoOptions(V0, Other);
        exit(F0(V0));
    end;

    procedure FromTrigger(): Text
    begin
        Codeunit.Run(Codeunit::HRunner);
        exit('ran');
    end;

    procedure FromSubscriber(): Text
    var Seen: Text;
    begin
        Pub.OnNest(Seen);
        exit(Seen);
    end;

    procedure TemporaryRecordField(): Text
    var Row: Record HRow temporary; Seen: Text;
    begin
        Row.Opt := Row.Opt::Second;
        Pub.OnRead(Row.Opt, Seen);
        exit(Seen);
    end;

    procedure TemporaryRecordThroughVarParam(): Text
    var Row: Record HRow temporary;
    begin
        Row.Opt := Row.Opt::Second;
        exit(VarRecordRaise(Row));
    end;

    local procedure MemberlessNamedReturnRaise(var T: Text) V: Option
    var Seen: Text;
    begin
        V := 1;
        Pub.OnRead(V, Seen);
        T := Seen;
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

fn refused(routine: &str, reason: &str) {
    match run(routine).0 {
        Err(Signal::Unsupported(why)) => assert!(why.contains(reason), "{routine}: {why}"),
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    }
}

const HEARD: &str = "which a subscriber hears, from a place the probe does not measure";

// Base Application's shape (codeunit 5407 hands its memberless NewStatusOption to OnAfterToProdOrderLineModify): a memberless
// local keeps its ordinal after a subscriber that ignores the parameter, the reading subscriber sees the publisher's member,
// a store of a member or an Integer comes back as the ordinal, also with two subscribers, and an ordinal past the publisher's
// members reads as the number (OVH MemberlessLocal_HeardPublishers).
#[test]
fn a_memberless_local_takes_the_subscriber_s_ordinal() {
    assert_eq!(
        assumed("MemberlessLocal"),
        "1|1;Q|1>1|1;Q|1~R|2>2|2;P|0>0|0;R|2>2|2;7|7;7|7>7|7"
    );
}

// A local and a field with members keep their own members and take the ordinal (OVH LocalWithMembers_,
// Fields_HeardPublishers), a caption field shows its caption after the call.
#[test]
fn a_caller_with_members_keeps_them_and_takes_the_ordinal() {
    assert_eq!(
        assumed("LocalWithMembers"),
        "Second|1;Q|1>Second|1;Q|1~R|2>Third|2;P|0>First|0;R|2>Third|2;First|0"
    );
    assert_eq!(
        assumed("Fields"),
        "Q|1>Second|1;Q|1~R|2>Third|2;Second|1;Q|1>Two|1;Q|1~R|2>Three|2;Q|1~R|2>Third|2;Q|1>Second|1>Q|1~R|2>Third|2;Third|2"
    );
}

// A parameter passed by value, with members or without, and a named return value (OVH Parameters_HeardPublishers); an
// element of a local or global array with members and of a local memberless array (OVH Elements_HeardPublishers).
#[test]
fn by_value_parameters_named_returns_and_elements_take_the_ordinal() {
    assert_eq!(
        assumed("ByValueAndNamedReturn"),
        "Q|1>Second|1>Q|1~R|2>Third|2;Second|1;Q|1>1|1>Q|1~R|2>2|2;Q|1>Second|1>Q|1~R|2>Third|2;Third|2"
    );
    assert_eq!(
        assumed("Elements"),
        "Q|1>Second|1;Q|1~R|2>Third|2/First|0;Q|1~R|2>Third|2;Q|1>1|1;Q|1~R|2>2|2"
    );
}

// Measured, and refused: an error the subscriber raises after its write leaves the write in the caller's variable (OVH
// ErrorAfterWrite_HeardPublisher: Third|2, 2|2), and the run keeps only what the subscribers before it wrote back.
#[test]
fn an_error_after_the_subscriber_s_write_refuses() {
    refused("ErrorAfterWrite", "BC keeps the write the subscriber made before it");
}

// Measured, and refused: a `var` parameter of the running routine handed on shows the subscriber the members of the value
// its caller handed (OVH Parameters_HeardPublishers: Second|1, not Q|1), and a global, which the subscriber aliases, would
// show its own list (OVH Globals_HeardPublishers: Q|1); the run carries neither. Not measured, and refused: a publisher
// declared without members, a field of a record passed by value, an element of a `var` array parameter, a memberless named
// return value. A subscriber with another list than the publisher refuses as before.
#[test]
fn unmeasured_callers_of_a_heard_publisher_refuse() {
    refused("VarWithMembers", HEARD);
    refused("MemberlessVar", HEARD);
    refused("Globals", HEARD);
    refused("MemberlessGlobal", HEARD);
    refused("MemberlessPublisher", HEARD);
    refused("ByValueRecordField", HEARD);
    refused("VarArrayElement", HEARD);
    refused("MemberlessNamedReturn", HEARD);
    refused("SubscriberOtherList", HEARD);
}

// Every cell raises an `IntegrationEvent(false, false)` of a codeunit with one `var` Option parameter, from a procedure of a
// codeunit, heard by static subscribers that take the parameter by `var` with the publisher's list or not at all, and hands
// fields of records that are not temporary. Each other event, routine or record refuses, one fact at a time.
#[test]
fn a_heard_publisher_outside_the_measured_contract_refuses() {
    for routine in [
        "IsolatedEvent",
        "BusinessEvent",
        "WithSender",
        "GlobalAccess",
        "ManualSubscriber",
        "ByValueSubscriber",
        "TwoOptions",
        "FromTrigger",
        "FromSubscriber",
        "TemporaryRecordField",
        "TemporaryRecordThroughVarParam",
    ] {
        refused(routine, HEARD);
    }
}
