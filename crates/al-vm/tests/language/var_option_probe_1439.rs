//! The shapes corpus PR 1439 measured (tests/al-language/types/enum-option/OptionVarMemberlessWrites.al and
//! OptionVarMemberlessCallers.al, the same on BC 28.5 and 29.0) through assumption `var-option-caller-members`: a `var`
//! parameter declared without members stands for the caller's variable. It reads the caller's members while the call runs,
//! keeps them when the routine stores into it or clears it, and the caller has its own members after the call. Each AL
//! procedure below cites the probe test it stands in for; the expected texts are the probe's, joined with `@`.

use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69841 Row
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Opt; Option) { OptionMembers = First,Second,Third; }
        field(3; OptCap; Option) { OptionMembers = First,Second,Third; OptionCaption = 'One,Two,Three'; }
        field(4; OptNum; Option) { OptionMembers = "0","1","2"; OptionCaption = '0,1,2'; }
    }

    procedure FieldsViaLib(): Text
    var
        Lib: Codeunit Lib;
        Seen: Text;
    begin
        Seen := Lib.FormatInside(Opt) + '#' + Lib.FormatInside(Rec.Opt);
        Opt := Opt::First;
        Lib.Assign1(Opt);
        exit(Seen + '#' + Format(Opt));
    end;

    procedure FieldsViaOwnProcedure(): Text
    var
        Seen: Text;
    begin
        Seen := FormatInsideOwn(Opt) + '#' + FormatInsideOwn(Rec.Opt);
        Opt := Opt::First;
        Assign1Own(Opt);
        exit(Seen + '#' + Format(Opt));
    end;

    procedure CaptionViaOwnProcedure(): Text
    begin
        exit(FormatInsideOwn(OptCap));
    end;

    procedure NumericCaptionToPublisher(): Text
    var
        Seen: Text;
    begin
        OptNum := OptNum::"1";
        OnNumeric(OptNum, Seen);
        exit(Seen + '#' + Format(OptNum) + '|' + Format(OptNum, 0, 2));
    end;

    [IntegrationEvent(false, false)]
    local procedure OnNumeric(var Value: Option; var Seen: Text)
    begin
    end;

    local procedure FormatInsideOwn(var Value: Option): Text
    begin
        exit(Format(Value) + '|' + StrSubstNo('%1', Value) + '|' + Format(Value, 0, 2));
    end;

    local procedure Assign1Own(var Value: Option)
    begin
        Value := 1;
    end;
}

codeunit 69842 Lib
{
    procedure FormatInside(var Value: Option): Text
    begin
        exit(Format(Value) + '|' + StrSubstNo('%1', Value) + '|' + Format(Value, 0, 2));
    end;

    procedure Assign1(var Value: Option)
    begin
        Value := 1;
    end;
}

codeunit 69843 Publisher
{
    procedure RaiseMemberless(var Value: Option; Write: Boolean; var Seen: Text)
    begin
        OnMemberless(Value, Write, Seen);
    end;

    procedure RaiseWithMembers(var Value: Option First,Second,Third; Write: Boolean; var Seen: Text)
    begin
        OnWithMembers(Value, Write, Seen);
    end;

    [IntegrationEvent(false, false)]
    procedure OnMemberless(var Value: Option; Write: Boolean; var Seen: Text)
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnWithMembers(var Value: Option First,Second,Third; Write: Boolean; var Seen: Text)
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnMembersOnly(var Value: Option First,Second,Third; var Seen: Text)
    begin
    end;
}

codeunit 69844 Subscriber
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Publisher, 'OnMemberless', '', false, false)]
    local procedure HandleMemberless(var Value: Option; Write: Boolean; var Seen: Text)
    begin
        Seen := Format(Value) + '|' + Format(Value, 0, 2);
        if Write then
            Value := 2;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::Publisher, 'OnWithMembers', '', false, false)]
    local procedure HandleWithMembers(var Value: Option; Write: Boolean; var Seen: Text)
    begin
        Seen := Format(Value) + '|' + Format(Value, 0, 2);
        if Write then
            Value := 2;
    end;

    [EventSubscriber(ObjectType::Table, Database::Row, 'OnNumeric', '', false, false)]
    local procedure HandleNumeric(var Value: Option; var Seen: Text)
    begin
        Seen := Format(Value) + '|' + Format(Value, 0, 2);
        Value := 2;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::Publisher, 'OnMembersOnly', '', false, false)]
    local procedure HandleMembersOnly(var Value: Option First,Second,Third; var Seen: Text)
    begin
        Seen := Format(Value) + '|' + Format(Value, 0, 2);
        Value := Value::Third;
    end;
}

interface IFmt
{
    procedure FormatInside(var Value: Option): Text;
    procedure Assign1(var Value: Option);
    procedure Assign1ThenFormat(var Value: Option): Text;
    procedure EvalResult(var Value: Option; T: Text): Text;
}

codeunit 69845 Impl implements IFmt
{
    procedure FormatInside(var Value: Option): Text
    begin
        exit(Format(Value) + '|' + StrSubstNo('%1', Value) + '|' + Format(Value, 0, 2));
    end;

    procedure Assign1(var Value: Option)
    begin
        Value := 1;
    end;

    procedure Assign1ThenFormat(var Value: Option): Text
    begin
        Value := 1;
        exit(Format(Value) + '|' + StrSubstNo('%1', Value) + '|' + Format(Value, 0, 2));
    end;

    procedure EvalResult(var Value: Option; T: Text): Text
    var
        Ok: Boolean;
    begin
        Ok := Evaluate(Value, T);
        exit(Format(Ok) + '|' + Format(Value) + '|' + Format(Value, 0, 2));
    end;
}

codeunit 69840 C
{
    var
        GlobalV: Option First,Second,Third;
        GlobalRow: Record Row;
        GlobalArr: array[2] of Option First,Second,Third;
        GSeen: Text;

    local procedure F(V: Option First,Second,Third): Text
    begin
        exit(Format(V) + '|' + Format(V, 0, 2));
    end;

    // OptionVarMemberlessCallers.al:117
    procedure ByValueParamToMemberlessVar(): Text
    var V: Option First,Second,Third;
    begin
        V := V::Second;
        exit(ByValueToMemberless(V) + '@' + F(V));
    end;

    // OptionVarMemberlessCallers.al:131
    procedure ByValueParamToOtherMembersVar(): Text
    var V: Option First,Second,Third;
    begin
        V := V::Second;
        exit(ByValueToOtherMembers(V) + '@' + F(V));
    end;

    // OptionVarMemberlessCallers.al:145
    procedure ByValueArrayElement(): Text
    var
        Arr: array[2] of Option First,Second,Third;
        Seen: Text;
    begin
        Arr[1] := Arr[1]::Second;
        Seen := ByValueArrayElementToMemberless(Arr) + '@' + F(Arr[1]);
        Arr[1] := Arr[1]::Second;
        exit(Seen + '@' + ByValueArrayToMemberlessArray(Arr) + '@' + F(Arr[1]));
    end;

    // OptionVarMemberlessCallers.al:164
    procedure NamedReturnValue(): Text
    var V: Option First,Second,Third;
    begin
        GSeen := '';
        V := ReturnNamed();
        exit(GSeen + '@' + F(V));
    end;

    // OptionVarMemberlessCallers.al:179 (the field without OptionCaption)
    procedure ByValueRecordField(): Text
    var Row: Record Row;
    begin
        Row.Opt := Row.Opt::Second;
        exit(ByValueRecordFieldInside(Row) + '@' + F(Row.Opt));
    end;

    // OptionVarMemberlessCallers.al:179 (the field with OptionCaption)
    procedure ByValueRecordCaptionField(): Text
    var Row: Record Row;
    begin
        exit(ByValueCaptionInside(Row));
    end;

    // OptionVarMemberlessCallers.al:196
    procedure ByValueParamToPublishers(): Text
    var V: Option First,Second,Third;
    begin
        V := V::Second;
        exit(ByValueToMemberlessPublisher(V) + '@' + F(V) + '@' + ByValueToMembersPublisher(V) + '@' + F(V));
    end;

    // OptionVarMemberlessCallers.al:215
    procedure ByValueMemberlessToMembersVar(): Text
    var V: Option First,Second,Third;
    begin
        V := V::Second;
        exit(ByValueMemberlessToMembers(V) + '@' + F(V));
    end;

    // OptionVarMemberlessCallers.al:229, :249
    procedure EventGlobal(Members: Boolean): Text
    var
        Publisher: Codeunit Publisher;
        Seen: Text;
        Result: Text;
    begin
        GlobalV := GlobalV::Second;
        if Members then Publisher.RaiseWithMembers(GlobalV, false, Seen) else Publisher.RaiseMemberless(GlobalV, false, Seen);
        Result := Seen + '@' + F(GlobalV);
        GlobalV := GlobalV::First;
        if Members then Publisher.RaiseWithMembers(GlobalV, true, Seen) else Publisher.RaiseMemberless(GlobalV, true, Seen);
        exit(Result + '@' + F(GlobalV));
    end;

    // OptionVarMemberlessCallers.al:311
    procedure EventLocalFieldMembersPublisher(): Text
    var
        Publisher: Codeunit Publisher;
        Seen: Text;
        Result: Text;
        Row: Record Row;
    begin
        Row.Opt := Row.Opt::Second;
        Publisher.RaiseWithMembers(Row.Opt, false, Seen);
        Result := Seen + '@' + F(Row.Opt);
        Row.Opt := Row.Opt::First;
        Publisher.RaiseWithMembers(Row.Opt, true, Seen);
        exit(Result + '@' + F(Row.Opt));
    end;

    // OptionVarMemberlessCallers.al:249, :311, :196 with the publisher called directly, as Base Application calls its events.
    procedure DirectCallersToMembersPublisher(): Text
    var
        Publisher: Codeunit Publisher;
        Row: Record Row;
        Seen: Text;
        Result: Text;
    begin
        GlobalV := GlobalV::Second;
        Publisher.OnWithMembers(GlobalV, true, Seen);
        Result := Seen + '#' + F(GlobalV);
        Row.Opt := Row.Opt::Second;
        Publisher.OnWithMembers(Row.Opt, true, Seen);
        Result += '@' + Seen + '#' + F(Row.Opt);
        exit(Result + '@' + DirectByValueToMembersPublisher(GlobalV::Second));
    end;

    local procedure DirectByValueToMembersPublisher(Value: Option First,Second,Third): Text
    var
        Publisher: Codeunit Publisher;
        Seen: Text;
    begin
        Publisher.OnWithMembers(Value, true, Seen);
        exit(Seen + '#' + F(Value));
    end;

    // OptionVarMemberlessCallers.al:290 with the publisher called directly.
    procedure DirectCaptionFieldToMembersPublisher(): Text
    var
        Publisher: Codeunit Publisher;
        Row: Record Row;
        Seen: Text;
    begin
        Publisher.OnWithMembers(Row.OptCap, false, Seen);
        exit(Seen);
    end;

    // OptionVarMemberlessCallers.al:269
    procedure EventCaptionFieldMemberlessPublisher(): Text
    var
        Publisher: Codeunit Publisher;
        Seen: Text;
        Row: Record Row;
    begin
        Row.OptCap := Row.OptCap::Second;
        Publisher.RaiseMemberless(Row.OptCap, false, Seen);
        exit(Seen);
    end;

    // OptionVarMemberlessCallers.al:332
    procedure EventGlobalRecordFieldMemberlessPublisher(): Text
    var
        Publisher: Codeunit Publisher;
        Seen: Text;
        Result: Text;
    begin
        Clear(GlobalRow);
        GlobalRow.Opt := GlobalRow.Opt::Second;
        Publisher.RaiseMemberless(GlobalRow.Opt, false, Seen);
        Result := Seen + '@' + F(GlobalRow.Opt);
        GlobalRow.Opt := GlobalRow.Opt::First;
        Publisher.RaiseMemberless(GlobalRow.Opt, true, Seen);
        exit(Result + '@' + F(GlobalRow.Opt));
    end;

    // OptionVarMemberlessCallers.al:353
    procedure EventLocalArrayElementMemberlessPublisher(): Text
    var
        Publisher: Codeunit Publisher;
        Seen: Text;
        Result: Text;
        Arr: array[2] of Option First,Second,Third;
    begin
        Arr[2] := Arr[2]::Second;
        Publisher.RaiseMemberless(Arr[2], false, Seen);
        Result := Seen + '@' + F(Arr[2]);
        Arr[2] := Arr[2]::First;
        Publisher.RaiseMemberless(Arr[2], true, Seen);
        exit(Result + '@' + F(Arr[2]));
    end;

    // OptionVarMemberlessCallers.al:374
    procedure EventMembersVarParamMemberlessPublisher(): Text
    var V: Option First,Second,Third;
    begin
        V := V::Second;
        exit(Level1ToMemberlessPublisher(V));
    end;

    // OptionVarMemberlessCallers.al:388 to :573, the callers without OptionCaption
    procedure InterfaceCaller(Kind: Integer): Text
    var
        Formatter: Interface IFmt;
        Impl: Codeunit Impl;
        V: Option First,Second,Third;
        Row: Record Row;
        Seen: Text;
    begin
        Formatter := Impl;
        Clear(GlobalRow);
        Clear(GlobalArr);
        case Kind of
            0:
                begin
                    V := V::Second;
                    Seen := Formatter.FormatInside(V) + '@' + F(V);
                    V := V::First;
                    Formatter.Assign1(V);
                    Seen += '@' + F(V);
                    V := V::First;
                    Seen += '@' + Formatter.Assign1ThenFormat(V) + '@' + F(V);
                    V := V::First;
                    Seen += '@' + Formatter.EvalResult(V, 'Third') + '@' + F(V);
                end;
            1:
                begin
                    Row.Opt := Row.Opt::Second;
                    Seen := Formatter.FormatInside(Row.Opt) + '@' + F(Row.Opt);
                    Row.Opt := Row.Opt::First;
                    Formatter.Assign1(Row.Opt);
                    Seen += '@' + F(Row.Opt);
                    Row.Opt := Row.Opt::First;
                    Seen += '@' + Formatter.Assign1ThenFormat(Row.Opt) + '@' + F(Row.Opt);
                    Row.Opt := Row.Opt::First;
                    Seen += '@' + Formatter.EvalResult(Row.Opt, 'Third') + '@' + F(Row.Opt);
                end;
            2:
                begin
                    GlobalV := GlobalV::Second;
                    Seen := Formatter.FormatInside(GlobalV) + '@' + F(GlobalV);
                    GlobalV := GlobalV::First;
                    Formatter.Assign1(GlobalV);
                    Seen += '@' + F(GlobalV);
                    GlobalV := GlobalV::First;
                    Seen += '@' + Formatter.Assign1ThenFormat(GlobalV) + '@' + F(GlobalV);
                    GlobalV := GlobalV::First;
                    Seen += '@' + Formatter.EvalResult(GlobalV, 'Third') + '@' + F(GlobalV);
                end;
            3:
                begin
                    GlobalRow.Opt := GlobalRow.Opt::Second;
                    Seen := Formatter.FormatInside(GlobalRow.Opt) + '@' + F(GlobalRow.Opt);
                    GlobalRow.Opt := GlobalRow.Opt::First;
                    Formatter.Assign1(GlobalRow.Opt);
                    Seen += '@' + F(GlobalRow.Opt);
                    GlobalRow.Opt := GlobalRow.Opt::First;
                    Seen += '@' + Formatter.Assign1ThenFormat(GlobalRow.Opt) + '@' + F(GlobalRow.Opt);
                    GlobalRow.Opt := GlobalRow.Opt::First;
                    Seen += '@' + Formatter.EvalResult(GlobalRow.Opt, 'Third') + '@' + F(GlobalRow.Opt);
                end;
            4:
                begin
                    GlobalArr[2] := GlobalArr[2]::Second;
                    Seen := Formatter.FormatInside(GlobalArr[2]) + '@' + F(GlobalArr[2]);
                    GlobalArr[2] := GlobalArr[2]::First;
                    Formatter.Assign1(GlobalArr[2]);
                    Seen += '@' + F(GlobalArr[2]);
                    GlobalArr[2] := GlobalArr[2]::First;
                    Seen += '@' + Formatter.Assign1ThenFormat(GlobalArr[2]) + '@' + F(GlobalArr[2]);
                    GlobalArr[2] := GlobalArr[2]::First;
                    Seen += '@' + Formatter.EvalResult(GlobalArr[2], 'Third') + '@' + F(GlobalArr[2]);
                end;
        end;
        exit(Seen);
    end;

    // OptionVarMemberlessCallers.al:450
    procedure InterfaceCaptionField(): Text
    var
        Formatter: Interface IFmt;
        Impl: Codeunit Impl;
        Row: Record Row;
    begin
        Formatter := Impl;
        exit(Formatter.FormatInside(Row.OptCap));
    end;

    // OptionVarMemberlessCallers.al:604
    procedure InterfaceMembersVarParam(): Text
    var
        Formatter: Interface IFmt;
        Impl: Codeunit Impl;
        V: Option First,Second,Third;
    begin
        Formatter := Impl;
        exit(Level1ToInterface(V, Formatter));
    end;

    // OptionVarMemberlessWrites.al:112
    procedure AssignThenFormatInside(): Text
    var V: Option First,Second,Third;
    begin
        V := V::First;
        exit(Assign1ThenFormat(V) + '@' + Format(V) + '|' + Format(V, 0, 2) + '|' + StrSubstNo('%1', V));
    end;

    // OptionVarMemberlessWrites.al:130
    procedure AssignThenEvaluateResult(T: Text): Text
    var V: Option First,Second,Third;
    begin
        V := V::First;
        ClearLastError();
        exit(Assign1ThenEvalResult(V, T) + '@' + Format(V) + '@' + GetLastErrorText());
    end;

    // OptionVarMemberlessWrites.al:162
    procedure AssignThenEvaluateTry(T: Text): Text
    var
        V: Option First,Second,Third;
        Seen: Text;
        Ok: Boolean;
    begin
        V := V::First;
        ClearLastError();
        Ok := TryAssign1ThenEval(V, T, Seen);
        exit(Format(Ok) + '@' + Seen + '@' + GetLastErrorText() + '@' + GetLastErrorCode() + '@' + F(V));
    end;

    // OptionVarMemberlessWrites.al:212
    procedure AssignThenEvaluateStatement(): Text
    var V: Option First,Second,Third;
    begin
        V := V::First;
        exit(Assign1ThenEvalStatement(V, 'Third') + '@' + F(V));
    end;

    // OptionVarMemberlessWrites.al:226
    procedure ClearInside(): Text
    var
        V: Option First,Second,Third;
        Seen: Text;
    begin
        V := V::Second;
        Seen := AssignThenClear(V) + '@' + F(V);
        V := V::Second;
        Seen += '@' + ClearOnly(V) + '@' + F(V);
        V := V::Third;
        exit(Seen + '@' + ClearThenAssign1(V) + '@' + F(V));
    end;

    // OptionVarMemberlessWrites.al:250
    procedure AssignThenHop(): Text
    var
        V: Option First,Second,Third;
        Seen: Text;
    begin
        V := V::First;
        Seen := Assign1ThenHopFormat(V) + '@' + Format(V);
        V := V::First;
        exit(Seen + '@' + Assign1ThenHopEval(V) + '@' + Format(V));
    end;

    // OptionVarMemberlessWrites.al:269
    procedure StoreOtherOption(): Text
    var
        V: Option First,Second,Third;
        Seen: Text;
    begin
        V := V::First;
        Seen := StoreOtherMembers(V) + '@' + F(V);
        V := V::First;
        Seen += '@' + StoreMemberlessLocal(V) + '@' + F(V);
        V := V::First;
        exit(Seen + '@' + StoreSameMembers(V) + '@' + F(V));
    end;

    // OptionVarMemberlessWrites.al:293
    procedure GlobalReadDuringCall(): Text
    var Seen: Text;
    begin
        GlobalV := GlobalV::Second;
        Seen := ReadGlobalInside(GlobalV);
        GlobalV := GlobalV::First;
        exit(Seen + '@' + Assign1ReadGlobalInside(GlobalV) + '@' + F(GlobalV));
    end;

    // OptionVarMemberlessWrites.al:309
    procedure CallerReaders(): Text
    var
        V: Option First,Second,Third;
        X: Variant;
    begin
        V := V::First;
        Assign1(V);
        X := V;
        exit(Format(V, 0, 1) + '|' + Format(V, 0, 0) + '|' + Format(X));
    end;

    // OptionVarMemberlessWrites.al:354, :373
    procedure HandOnSameMembers(): Text
    var V: Option First,Second,Third;
    begin
        V := V::Second;
        exit(Level1SameMembers(V));
    end;

    // OptionVarMemberlessWrites.al:411 (the field without OptionCaption)
    procedure VarRecordParamField(): Text
    var Row: Record Row;
    begin
        Row.Opt := Row.Opt::Second;
        exit(VarRecordField(Row) + '@' + F(Row.Opt));
    end;

    // OptionVarMemberlessWrites.al:447 (the field without OptionCaption)
    procedure RecFieldOwnTable(): Text
    var
        Row: Record Row;
        Seen: Text;
    begin
        Row.Opt := Row.Opt::Second;
        Seen := Row.FieldsViaLib() + '@' + F(Row.Opt);
        Row.Opt := Row.Opt::Second;
        exit(Seen + '@' + Row.FieldsViaOwnProcedure() + '@' + F(Row.Opt));
    end;

    // OptionVarMemberlessWrites.al:447 (the field with OptionCaption)
    procedure RecCaptionFieldOwnTable(): Text
    var Row: Record Row;
    begin
        exit(Row.CaptionViaOwnProcedure());
    end;

    // OptionVarMemberlessWrites.al:472
    procedure VarArrayStoreOtherOption(): Text
    var
        Arr: array[2] of Option First,Second,Third;
        Seen: Text;
    begin
        Arr[1] := Arr[1]::First;
        Seen := ArrayStoreOtherMembers(Arr) + '@' + F(Arr[1]);
        Arr[1] := Arr[1]::First;
        Seen += '@' + ArrayStoreMemberlessLocal(Arr) + '@' + Format(Arr[1]) + '|' + Format(Arr[1], 0, 2);
        Arr[1] := Arr[1]::First;
        exit(Seen + '@' + ArrayStoreSameMembers(Arr) + '@' + Format(Arr[1]) + '|' + Format(Arr[1], 0, 2));
    end;

    // OptionVarMemberlessWrites.al:472, without its last store
    procedure VarArrayStoreOtherThenMemberless(): Text
    var
        Arr: array[2] of Option First,Second,Third;
        Seen: Text;
    begin
        Arr[1] := Arr[1]::First;
        Seen := ArrayStoreOtherMembers(Arr) + '@' + F(Arr[1]);
        Arr[1] := Arr[1]::First;
        exit(Seen + '@' + ArrayStoreMemberlessLocal(Arr) + '@' + Format(Arr[1]) + '|' + Format(Arr[1], 0, 2));
    end;

    // Not measured: the caller assigns an element a memberless store left without members (OptionVarMemberlessWrites.al:483),
    // then the callee stores another list, an Integer or the held list into it.
    procedure DroppedThenStore(Kind: Integer): Text
    var
        Arr: array[2] of Option First,Second,Third;
        Seen: Text;
    begin
        Arr[1] := Arr[1]::First;
        Seen := ArrayStoreMemberlessLocal(Arr) + '@' + Format(Arr[1]) + '|' + Format(Arr[1], 0, 2);
        Arr[1] := Arr[1]::First;
        exit(Seen + '@' + StoreKind(Arr, Kind) + '@' + Format(Arr[1]) + '|' + Format(Arr[1], 0, 2));
    end;

    // The same after CopyArray from an array of memberless Options (OptionVarMemberlessWrites.al:539).
    procedure CopiedInThenStore(Kind: Integer): Text
    var
        Arr: array[2] of Option First,Second,Third;
        Seen: Text;
    begin
        Arr[1] := Arr[1]::First;
        Seen := ArrayCopyIn(Arr);
        Arr[1] := Arr[1]::First;
        exit(Seen + '@' + StoreKind(Arr, Kind) + '@' + Format(Arr[1]) + '|' + Format(Arr[1], 0, 2));
    end;

    // A memberless store, Clear of the whole array, then the held list (OptionVarMemberlessUnheard.al:988-1003, :1020-1021).
    procedure DroppedClearedThenSame(): Text
    var
        Arr: array[2] of Option First,Second,Third;
        Seen: Text;
    begin
        Arr[1] := Arr[1]::First;
        Seen := ArrayStoreMemberlessLocal(Arr);
        Clear(Arr);
        Arr[1] := Arr[1]::Second;
        exit(Seen + '@' + ArrayStoreSameMembers(Arr) + '@' + Format(Arr[1]) + '|' + Format(Arr[1], 0, 2));
    end;

    // An element of a fresh array declared with members holds its list: an ordinary store.
    procedure FreshElementStore(): Text
    var
        Arr: array[2] of Option First,Second,Third;
    begin
        Arr[1] := Arr[1]::Second;
        Arr[2] := 2;
        exit(Format(Arr[1]) + '|' + Format(Arr[2]));
    end;

    // Clear of a fresh array declared with members: an ordinary Clear.
    procedure FreshArrayClear(): Text
    var
        Arr: array[2] of Option First,Second,Third;
    begin
        Arr[1] := Arr[1]::Second;
        Clear(Arr);
        Arr[2] := Arr[2]::Third;
        exit(Format(Arr[1]) + '|' + Format(Arr[2]));
    end;

    local procedure StoreKind(var A: array[2] of Option; Kind: Integer): Text
    begin
        case Kind of
            0:
                exit(ArrayStoreOtherMembers(A));
            1:
                begin
                    A[1] := 2;
                    exit(Format(A[1]) + '|' + Format(A[1], 0, 2));
                end;
            2:
                exit(ArrayStoreSameMembers(A));
        end;
    end;

    // OptionVarMemberlessWrites.al:496, :513
    procedure VarArrayClear(Whole: Boolean): Text
    var
        Arr: array[2] of Option First,Second,Third;
        Seen: Text;
    begin
        Arr[1] := Arr[1]::Second;
        Arr[2] := Arr[2]::Third;
        if Whole then Seen := ArrayClearAll(Arr) else Seen := ArrayClearElement(Arr);
        exit(Seen + '@' + F(Arr[1]) + '@' + F(Arr[2]));
    end;

    // OptionVarMemberlessWrites.al:530
    procedure VarArrayCopyArray(): Text
    var
        Arr: array[2] of Option First,Second,Third;
        Seen: Text;
    begin
        Arr[1] := Arr[1]::Second;
        Arr[2] := Arr[2]::Third;
        Seen := ArrayCopyOut(Arr) + '@' + F(Arr[1]);
        Seen += '@' + ArrayCopyIn(Arr);
        exit(Seen + '@' + Format(Arr[1]) + '|' + Format(Arr[1], 0, 2) + '@' + Format(Arr[2]) + '|' + Format(Arr[2], 0, 2));
    end;

    // OptionVarMemberlessWrites.al:551
    procedure LocalCopyToMemberless(): Text
    var
        Arr: array[2] of Option First,Second,Third;
        Arr1: array[2] of Option;
        Seen: Text;
    begin
        Arr[1] := Arr[1]::Second;
        CopyArray(Arr1, Arr, 1);
        Seen := Format(Arr1[1]) + '|' + Format(Arr1[1], 0, 2);
        Arr1[1] := 1;
        exit(Seen + '@' + Format(Arr1[1]) + '|' + Format(Arr1[1], 0, 2));
    end;

    // OptionVarMemberlessWrites.al:568
    procedure ByValueMemberlessArray(): Text
    var Arr: array[2] of Option First,Second,Third;
    begin
        Arr[1] := Arr[1]::Second;
        exit(ByValueArrayStore(Arr) + '@' + F(Arr[1]));
    end;

    // Base Application's purchase order page hands a memberless local to an event publisher's parameter with members.
    procedure MemberlessLocalToMembersPublisher(): Text
    var
        Publisher: Codeunit Publisher;
        M: Option;
        Seen: Text;
    begin
        M := 1;
        Publisher.OnMembersOnly(M, Seen);
        exit(Seen + '@' + Format(M) + '|' + Format(M, 0, 2));
    end;

    // Base Application's batch posting hands a memberless var parameter to a procedure's parameter with members.
    procedure MemberlessVarParamToMembers(): Text
    var M: Option;
    begin
        M := 1;
        exit(MemberlessHandOn(M) + '@' + Format(M) + '|' + Format(M, 0, 2));
    end;

    // A field whose OptionCaption names its members (Base Application's "Item Entry Relation"."Source Subtype").
    procedure NumericCaptionFieldToPublisher(): Text
    var Row: Record Row;
    begin
        exit(Row.NumericCaptionToPublisher());
    end;

    local procedure MemberlessHandOn(var Value: Option): Text
    begin
        exit(SameMembersSet(Value));
    end;

    local procedure SameMembersSet(var Value: Option First,Second,Third): Text
    var Seen: Text;
    begin
        Seen := Format(Value) + '|' + Format(Value, 0, 2);
        Value := Value::Third;
        exit(Seen);
    end;

    procedure VarArrayStoreShorterOption(): Text
    var Arr: array[2] of Option First,Second,Third;
    begin
        exit(StoreShorterOption(Arr));
    end;

    local procedure StoreShorterOption(var A: array[2] of Option): Text
    var Two: Option X,Y;
    begin
        Two := Two::Y;
        A[1] := Two;
        exit(Format(A[1]));
    end;

    procedure RecordArrayElementFieldToMembersPublisher(): Text
    var
        Publisher: Codeunit Publisher;
        Rows: array[2] of Record Row;
        Seen: Text;
    begin
        Rows[1].Opt := Rows[1].Opt::Second;
        Publisher.OnWithMembers(Rows[1].Opt, false, Seen);
        exit(Seen);
    end;

    procedure RecordArrayElementField(): Text
    var Rows: array[2] of Record Row;
    begin
        Rows[1].Opt := Rows[1].Opt::Second;
        exit(FormatInsideVar(Rows[1].Opt));
    end;

    procedure NamedReturnRecordField(): Text
    begin
        NamedReturnRecord();
        exit(GSeen);
    end;

    local procedure NamedReturnRecord() R: Record Row
    begin
        R.Opt := R.Opt::Second;
        GSeen := FormatInsideVar(R.Opt);
    end;

    local procedure FormatInsideVar(var Value: Option): Text
    begin
        exit(Format(Value) + '|' + StrSubstNo('%1', Value) + '|' + Format(Value, 0, 2));
    end;

    local procedure Assign1(var Value: Option)
    begin
        Value := 1;
    end;

    local procedure EvalResultInside(var Value: Option; T: Text): Text
    var Ok: Boolean;
    begin
        Ok := Evaluate(Value, T);
        exit(Format(Ok) + '|' + Format(Value) + '|' + Format(Value, 0, 2));
    end;

    local procedure OtherMembersFormat(var Value: Option A,B,C): Text
    begin
        exit(Format(Value) + '|' + StrSubstNo('%1', Value) + '|' + Format(Value, 0, 2));
    end;

    local procedure OtherMembersAssignC(var Value: Option A,B,C)
    begin
        Value := Value::C;
    end;

    local procedure SameMembersFormat(var Value: Option First,Second,Third): Text
    begin
        exit(Format(Value) + '|' + StrSubstNo('%1', Value) + '|' + Format(Value, 0, 2));
    end;

    local procedure MemberlessArrayFormat(var A: array[2] of Option): Text
    var Seen: Text;
    begin
        Seen := Format(A[1]) + '|' + StrSubstNo('%1', A[1]) + '|' + Format(A[1], 0, 2);
        A[1] := 2;
        Seen += '#' + Format(A[1]) + '|' + Format(A[1], 0, 2);
        exit(Seen);
    end;

    local procedure ByValueToMemberless(Value: Option First,Second,Third): Text
    var Seen: Text;
    begin
        Seen := FormatInsideVar(Value) + '#' + Format(Value);
        Value := Value::First;
        Assign1(Value);
        Seen += '#' + Format(Value) + '|' + Format(Value, 0, 2);
        Value := Value::First;
        Seen += '#' + EvalResultInside(Value, 'Third') + '#' + Format(Value);
        exit(Seen);
    end;

    local procedure ByValueToOtherMembers(Value: Option First,Second,Third): Text
    var Seen: Text;
    begin
        Seen := OtherMembersFormat(Value) + '#' + Format(Value);
        OtherMembersAssignC(Value);
        Seen += '#' + Format(Value) + '|' + Format(Value, 0, 2);
        exit(Seen);
    end;

    local procedure ByValueArrayElementToMemberless(A: array[2] of Option First,Second,Third): Text
    var Seen: Text;
    begin
        Seen := FormatInsideVar(A[1]) + '#' + Format(A[1]);
        A[1] := A[1]::First;
        Assign1(A[1]);
        Seen += '#' + Format(A[1]) + '|' + Format(A[1], 0, 2);
        exit(Seen);
    end;

    local procedure ByValueArrayToMemberlessArray(A: array[2] of Option First,Second,Third): Text
    begin
        exit(MemberlessArrayFormat(A) + '#' + Format(A[1]) + '|' + Format(A[1], 0, 2));
    end;

    local procedure ReturnNamed() R: Option First,Second,Third
    begin
        R := R::Second;
        GSeen := FormatInsideVar(R) + '#' + Format(R);
        R := R::First;
        Assign1(R);
        GSeen += '#' + Format(R) + '|' + Format(R, 0, 2);
    end;

    local procedure ByValueRecordFieldInside(R: Record Row): Text
    var Seen: Text;
    begin
        Seen := FormatInsideVar(R.Opt);
        R.Opt := R.Opt::First;
        Assign1(R.Opt);
        exit(Seen + '#' + Format(R.Opt) + '|' + Format(R.Opt, 0, 2));
    end;

    local procedure ByValueCaptionInside(R: Record Row): Text
    begin
        exit(FormatInsideVar(R.OptCap));
    end;

    local procedure ByValueToMemberlessPublisher(Value: Option First,Second,Third): Text
    var
        Publisher: Codeunit Publisher;
        Seen: Text;
        Result: Text;
    begin
        Publisher.RaiseMemberless(Value, false, Seen);
        Result := Seen + '#' + Format(Value);
        Value := Value::First;
        Publisher.RaiseMemberless(Value, true, Seen);
        exit(Result + '#' + Format(Value) + '|' + Format(Value, 0, 2));
    end;

    local procedure ByValueToMembersPublisher(Value: Option First,Second,Third): Text
    var
        Publisher: Codeunit Publisher;
        Seen: Text;
        Result: Text;
    begin
        Publisher.RaiseWithMembers(Value, false, Seen);
        Result := Seen + '#' + Format(Value);
        Value := Value::First;
        Publisher.RaiseWithMembers(Value, true, Seen);
        exit(Result + '#' + Format(Value) + '|' + Format(Value, 0, 2));
    end;

    local procedure ByValueMemberlessToMembers(Value: Option): Text
    begin
        exit(Format(Value) + '#' + SameMembersFormat(Value) + '#' + Format(Value));
    end;

    local procedure Level1ToMemberlessPublisher(var Value: Option First,Second,Third): Text
    var
        Publisher: Codeunit Publisher;
        Seen: Text;
    begin
        Publisher.RaiseMemberless(Value, false, Seen);
        exit(Seen);
    end;

    local procedure Level1ToInterface(var Value: Option First,Second,Third; Formatter: Interface IFmt): Text
    begin
        exit(Formatter.FormatInside(Value));
    end;

    local procedure Assign1ThenFormat(var Value: Option): Text
    begin
        Value := 1;
        exit(Format(Value) + '|' + StrSubstNo('%1', Value) + '|' + Format(Value, 0, 2));
    end;

    local procedure Assign1ThenEvalResult(var Value: Option; T: Text): Text
    var Ok: Boolean;
    begin
        Value := 1;
        Ok := Evaluate(Value, T);
        exit(Format(Ok) + '|' + Format(Value) + '|' + Format(Value, 0, 2));
    end;

    [TryFunction]
    local procedure TryAssign1ThenEval(var Value: Option; T: Text; var Seen: Text)
    begin
        Seen := 'not reached';
        Value := 1;
        Evaluate(Value, T);
        Seen := Format(Value) + '|' + Format(Value, 0, 2);
    end;

    local procedure Assign1ThenEvalStatement(var Value: Option; T: Text): Text
    begin
        Value := 1;
        Evaluate(Value, T);
        exit(Format(Value) + '|' + Format(Value, 0, 2));
    end;

    local procedure AssignThenClear(var Value: Option): Text
    begin
        Value := 1;
        Clear(Value);
        exit(Format(Value) + '|' + Format(Value, 0, 2));
    end;

    local procedure ClearOnly(var Value: Option): Text
    begin
        Clear(Value);
        exit(Format(Value) + '|' + Format(Value, 0, 2));
    end;

    local procedure ClearThenAssign1(var Value: Option): Text
    begin
        Clear(Value);
        Value := 1;
        exit(Format(Value) + '|' + Format(Value, 0, 2));
    end;

    local procedure Assign1ThenHopFormat(var Value: Option): Text
    begin
        Value := 1;
        exit(FormatInsideVar(Value));
    end;

    local procedure Assign1ThenHopEval(var Value: Option): Text
    begin
        Value := 1;
        exit(EvalResultInside(Value, 'Third'));
    end;

    local procedure StoreOtherMembers(var Value: Option): Text
    var Other: Option P,Q,R;
    begin
        Other := Other::R;
        Value := Other;
        exit(Format(Value) + '|' + Format(Value, 0, 2));
    end;

    local procedure StoreMemberlessLocal(var Value: Option): Text
    var M: Option;
    begin
        M := 2;
        Value := M;
        exit(Format(Value) + '|' + Format(Value, 0, 2));
    end;

    local procedure StoreSameMembers(var Value: Option): Text
    var Same: Option First,Second,Third;
    begin
        Same := Same::Third;
        Value := Same;
        exit(Format(Value) + '|' + Format(Value, 0, 2));
    end;

    local procedure ReadGlobalInside(var Value: Option): Text
    begin
        exit(Format(Value) + '|' + Format(GlobalV) + '|' + Format(GlobalV, 0, 2));
    end;

    local procedure Assign1ReadGlobalInside(var Value: Option): Text
    begin
        Value := 1;
        exit(Format(Value) + '|' + Format(GlobalV) + '|' + Format(GlobalV, 0, 2));
    end;

    local procedure Level1SameMembers(var Value: Option First,Second,Third): Text
    begin
        exit(FormatInsideVar(Value));
    end;

    local procedure VarRecordField(var R: Record Row): Text
    var Seen: Text;
    begin
        Seen := FormatInsideVar(R.Opt);
        R.Opt := R.Opt::First;
        Assign1(R.Opt);
        Seen += '#' + Format(R.Opt);
        R.Opt := R.Opt::First;
        exit(Seen + '#' + Assign1ThenFormat(R.Opt));
    end;

    local procedure ArrayStoreOtherMembers(var A: array[2] of Option): Text
    var Other: Option P,Q,R;
    begin
        Other := Other::R;
        A[1] := Other;
        exit(Format(A[1]) + '|' + Format(A[1], 0, 2));
    end;

    local procedure ArrayStoreMemberlessLocal(var A: array[2] of Option): Text
    var M: Option;
    begin
        M := 2;
        A[1] := M;
        exit(Format(A[1]) + '|' + Format(A[1], 0, 2));
    end;

    local procedure ArrayStoreSameMembers(var A: array[2] of Option): Text
    var Same: Option First,Second,Third;
    begin
        Same := Same::Third;
        A[1] := Same;
        exit(Format(A[1]) + '|' + Format(A[1], 0, 2));
    end;

    local procedure ArrayClearAll(var A: array[2] of Option): Text
    var Seen: Text;
    begin
        Clear(A);
        Seen := Format(A[1]) + '|' + Format(A[1], 0, 2) + '#' + Format(A[2]) + '|' + Format(A[2], 0, 2);
        A[1] := 1;
        exit(Seen + '#' + Format(A[1]) + '|' + Format(A[1], 0, 2));
    end;

    local procedure ArrayClearElement(var A: array[2] of Option): Text
    var Seen: Text;
    begin
        Clear(A[1]);
        Seen := Format(A[1]) + '|' + Format(A[1], 0, 2) + '#' + Format(A[2]) + '|' + Format(A[2], 0, 2);
        A[1] := 1;
        exit(Seen + '#' + Format(A[1]) + '|' + Format(A[1], 0, 2));
    end;

    local procedure ArrayCopyOut(var A: array[2] of Option): Text
    var
        B: array[2] of Option;
        Seen: Text;
    begin
        CopyArray(B, A, 1);
        Seen := Format(B[1]) + '|' + Format(B[1], 0, 2);
        B[1] := 1;
        exit(Seen + '#' + Format(B[1]) + '|' + Format(B[1], 0, 2));
    end;

    local procedure ArrayCopyIn(var A: array[2] of Option): Text
    var Src: array[2] of Option;
    begin
        Src[1] := 2;
        Src[2] := 1;
        CopyArray(A, Src, 1);
        exit(Format(A[1]) + '|' + Format(A[1], 0, 2) + '#' + Format(A[2]) + '|' + Format(A[2], 0, 2));
    end;

    local procedure ByValueArrayStore(A: array[2] of Option): Text
    var Seen: Text;
    begin
        Seen := Format(A[1]) + '|' + Format(A[1], 0, 2);
        A[1] := 2;
        exit(Seen + '#' + Format(A[1]) + '|' + Format(A[1], 0, 2));
    end;

    procedure LocalMemberlessElementStores(): Text
    var
        Own: Option P,Q,R;
        A: array[2] of Option;
    begin
        Own := Own::R;
        A[1] := Own;
        A[2] := 1;
        exit(Format(A[1]) + '|' + Format(A[1], 0, 2) + '#' + Format(A[2]) + '|' + Format(A[2], 0, 2));
    end;

    procedure EventGlobalMemberless(): Text begin exit(EventGlobal(false)); end;
    procedure EventGlobalMembers(): Text begin exit(EventGlobal(true)); end;
    procedure InterfaceLocal(): Text begin exit(InterfaceCaller(0)); end;
    procedure InterfaceLocalField(): Text begin exit(InterfaceCaller(1)); end;
    procedure InterfaceGlobal(): Text begin exit(InterfaceCaller(2)); end;
    procedure InterfaceGlobalRecordField(): Text begin exit(InterfaceCaller(3)); end;
    procedure InterfaceGlobalArrayElement(): Text begin exit(InterfaceCaller(4)); end;
    procedure EvalResultThird(): Text begin exit(AssignThenEvaluateResult('Third')); end;
    procedure EvalResultThirdLower(): Text begin exit(AssignThenEvaluateResult('third')); end;
    procedure EvalResult2(): Text begin exit(AssignThenEvaluateResult('2')); end;
    procedure EvalResultNope(): Text begin exit(AssignThenEvaluateResult('Nope')); end;
    procedure EvalTryThird(): Text begin exit(AssignThenEvaluateTry('Third')); end;
    procedure EvalTryNope(): Text begin exit(AssignThenEvaluateTry('Nope')); end;
    procedure EvalTryEmpty(): Text begin exit(AssignThenEvaluateTry('')); end;
    procedure VarArrayClearAll(): Text begin exit(VarArrayClear(true)); end;
    procedure VarArrayClearElement(): Text begin exit(VarArrayClear(false)); end;
    procedure DroppedThenOther(): Text begin exit(DroppedThenStore(0)); end;
    procedure DroppedThenInteger(): Text begin exit(DroppedThenStore(1)); end;
    procedure DroppedThenSame(): Text begin exit(DroppedThenStore(2)); end;
    procedure CopiedInThenOther(): Text begin exit(CopiedInThenStore(0)); end;
    procedure CopiedInThenInteger(): Text begin exit(CopiedInThenStore(1)); end;
}
"#;

fn assumed(routine: &str) -> String {
    assumptions::clear_hits();
    let out = text(al_testkit::run_in(SRC, "C", routine));
    assert!(
        assumptions::take_hits().contains(&Assumption::VarOptionCallerMembers),
        "{routine}: not flagged"
    );
    out
}

fn clean(routine: &str) -> String {
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

// OptionVarMemberlessCallers.al:117, :131, :145, :164, :179, :215
#[test]
fn callers_passed_by_value_and_named_return_values() {
    assert_eq!(
        assumed("ByValueParamToMemberlessVar"),
        "Second|Second|1#Second#Second|1#Yes|Third|2#Third@Second|1"
    );
    assert_eq!(
        assumed("ByValueParamToOtherMembersVar"),
        "B|B|1#Second#Third|2@Second|1"
    );
    assert_eq!(
        assumed("ByValueArrayElement"),
        "Second|Second|1#Second#Second|1@Second|1@Second|Second|1#Third|2#Third|2@Second|1"
    );
    assert_eq!(assumed("NamedReturnValue"), "Second|Second|1#Second#Second|1@Second|1");
    assert_eq!(assumed("ByValueRecordField"), "Second|Second|1#Second|1@Second|1");
    assert_eq!(assumed("ByValueMemberlessToMembersVar"), "1#Second|Second|1#1@Second|1");
}

// OptionVarMemberlessCallers.al:196, :229, :249, :311, :332, :353
#[test]
fn event_subscribers_without_members_read_the_caller_s_members() {
    assert_eq!(
        assumed("ByValueParamToPublishers"),
        "Second|1#Second#Third|2@Second|1@Second|1#Second#Third|2@Second|1"
    );
    assert_eq!(assumed("EventGlobalMemberless"), "Second|1@Second|1@Third|2");
    assert_eq!(assumed("EventGlobalMembers"), "Second|1@Second|1@Third|2");
    assert_eq!(assumed("EventLocalFieldMembersPublisher"), "Second|1@Second|1@Third|2");
    assert_eq!(
        assumed("EventGlobalRecordFieldMemberlessPublisher"),
        "Second|1@Second|1@Third|2"
    );
    assert_eq!(
        assumed("EventLocalArrayElementMemberlessPublisher"),
        "Second|1@Second|1@Third|2"
    );
}

// OptionVarMemberlessCallers.al:388, :419, :481, :511, :573
#[test]
fn interface_methods_read_the_caller_s_members() {
    let measured = "Second|Second|1@Second|1@Second|1@Second|Second|1@Second|1@Yes|Third|2@Third|2";
    for routine in [
        "InterfaceLocal",
        "InterfaceLocalField",
        "InterfaceGlobal",
        "InterfaceGlobalRecordField",
        "InterfaceGlobalArrayElement",
    ] {
        assert_eq!(assumed(routine), measured, "{routine}");
    }
}

// OptionVarMemberlessWrites.al:112, :130, :162, :212, :226, :250, :269, :293, :309
#[test]
fn stores_into_the_parameter_keep_the_caller_s_members() {
    assert_eq!(assumed("AssignThenFormatInside"), "Second|Second|1@Second|1|Second");
    assert_eq!(assumed("EvalResultThird"), "Yes|Third|2@Third@");
    assert_eq!(assumed("EvalResultThirdLower"), "Yes|Third|2@Third@");
    assert_eq!(assumed("EvalResult2"), "Yes|Third|2@Third@");
    assert_eq!(assumed("EvalResultNope"), "No|Second|1@Second@");
    assert_eq!(assumed("EvalTryThird"), "Yes@Third|2@@@Third|2");
    assert_eq!(
        assumed("EvalTryNope"),
        "No@not reached@'Nope' is not an option. The existing options are: First,Second,Third@InvalidOptionString@Second|1"
    );
    assert_eq!(
        assumed("EvalTryEmpty"),
        "No@not reached@'' is not an option. The existing options are: First,Second,Third@InvalidOptionString@Second|1"
    );
    assert_eq!(assumed("AssignThenEvaluateStatement"), "Third|2@Third|2");
    assert_eq!(
        assumed("ClearInside"),
        "First|0@First|0@First|0@First|0@Second|1@Second|1"
    );
    assert_eq!(assumed("AssignThenHop"), "Second|Second|1@Second@Yes|Third|2@Third");
    assert_eq!(
        assumed("StoreOtherOption"),
        "Third|2@Third|2@Third|2@Third|2@Third|2@Third|2"
    );
    assert_eq!(
        assumed("GlobalReadDuringCall"),
        "Second|Second|1@Second|Second|1@Second|1"
    );
    assert_eq!(assumed("CallerReaders"), "Second|Second|Second");
}

// OptionVarMemberlessWrites.al:411, :447 (the fields without OptionCaption)
#[test]
fn fields_of_a_var_record_parameter_and_of_rec() {
    assert_eq!(
        assumed("VarRecordParamField"),
        "Second|Second|1#Second#Second|Second|1@Second|1"
    );
    assert_eq!(
        assumed("RecFieldOwnTable"),
        "Second|Second|1#Second|Second|1#Second@Second|1@Second|Second|1#Second|Second|1#Second@Second|1"
    );
}

// An element a memberless store or CopyArray from memberless Options left without members keeps none on BC when the caller
// assigns it (OptionVarMemberlessWrites.al:483-491); the run does not model that state, so the caller's store refuses, and no
// later store into the element meets it. Reads of the element stay (OVMW:488-489). Clear of the whole array gives the
// element its members back (OptionVarMemberlessUnheard.al:988-1003), and the held list then keeps them (:1020-1021).
#[test]
fn an_element_a_memberless_store_left_without_members() {
    const DROPPED: &str = "a store into an element of an array of Options declared with members that holds none";
    for routine in [
        "DroppedThenOther",
        "DroppedThenInteger",
        "DroppedThenSame",
        "CopiedInThenOther",
        "CopiedInThenInteger",
    ] {
        refused(routine, DROPPED);
    }
    assert_eq!(assumed("DroppedClearedThenSame"), "2|2@Third|2@Third|2");
    // An element of a fresh array declared with members holds its list: main's store, not flagged.
    assert_eq!(clean("FreshElementStore"), "Second|Third");
    assert_eq!(clean("FreshArrayClear"), "First|Third");
}

// OptionVarMemberlessWrites.al:472, :496, :513, :530, :551, :568
#[test]
fn arrays_of_options_without_members() {
    assert_eq!(assumed("VarArrayStoreOtherThenMemberless"), "Third|2@Third|2@2|2@2|2");
    // The caller assigns the element its memberless store left without members (OVMW:483): refused, see
    // `an_element_a_memberless_store_left_without_members`.
    refused(
        "VarArrayStoreOtherOption",
        "a store into an element of an array of Options declared with members that holds none",
    );
    assert_eq!(assumed("VarArrayClearAll"), "First|0#First|0#Second|1@Second|1@First|0");
    assert_eq!(
        assumed("VarArrayClearElement"),
        "First|0#Third|2#Second|1@Second|1@Third|2"
    );
    assert_eq!(
        assumed("VarArrayCopyArray"),
        "Second|1#Second|1@Second|1@2|2#1|1@2|2@1|1"
    );
    assert_eq!(assumed("LocalCopyToMemberless"), "Second|1@Second|1");
    assert_eq!(assumed("ByValueMemberlessArray"), "Second|1#Third|2@Second|1");
}

// An element of a memberless local array that holds no caller's members takes the number, as on main.
#[test]
fn stores_into_an_element_without_a_caller_s_members_keep_the_number() {
    assert_eq!(clean("LocalMemberlessElementStores"), "2|2#1|1");
}

// Shapes of Base Application that Microsoft's tests reach and no probe measures: a memberless `var` parameter handed on to a
// procedure's parameter with members (batch posting), and a field whose OptionCaption names its members ("Item Entry
// Relation"). They refuse; the probes measure a memberless caller handed to members only directly
// (OptionVarMemberLists.al:432), and only fields without OptionCaption. A memberless local handed directly to an event
// publisher's parameter with members that a subscriber hears (the purchase order page) is measured since
// (OptionVarMemberlessHeard.al:158, :161): the subscriber reads the publisher's member, the caller takes back its write.
#[test]
fn base_application_shapes_the_probes_do_not_measure_refuse() {
    assert_eq!(assumed("MemberlessLocalToMembersPublisher"), "Second|1@2|2");
    refused(
        "NumericCaptionFieldToPublisher",
        "handed directly to an event publisher's",
    );
    for routine in ["MemberlessVarParamToMembers", "DirectCallersToMembersPublisher"] {
        refused(routine, "caller the probe does not measure");
    }
}

// Not measured, or not carried: a field whose OptionCaption differs from its members (OptionVarMemberlessCallers.al:179,
// :269, :290 with the publisher called directly, :450; OptionVarMemberlessWrites.al:447: BC shows the caption, which the value does not carry), and a `var`
// parameter with members as the caller (OptionVarMemberlessCallers.al:374, :604; OptionVarMemberlessWrites.al:354: after a
// hop through other members a rejected Evaluate sets the first member, :387, which the run cannot tell from a direct caller).
#[test]
fn caption_fields_and_var_parameters_with_members_refuse() {
    refused("RecordArrayElementField", "caller the probe does not measure");
    refused("NamedReturnRecordField", "caller the probe does not measure");
    refused(
        "RecordArrayElementFieldToMembersPublisher",
        "from a caller the probe does not measure",
    );
    refused("VarArrayStoreShorterOption", "a store the probe does not measure");
    for routine in [
        "ByValueRecordCaptionField",
        "EventCaptionFieldMemberlessPublisher",
        "InterfaceCaptionField",
        "RecCaptionFieldOwnTable",
    ] {
        refused(routine, "OptionCaption");
    }
    refused(
        "DirectCaptionFieldToMembersPublisher",
        "caller the probe does not measure",
    );
    for routine in [
        "EventMembersVarParamMemberlessPublisher",
        "InterfaceMembersVarParam",
        "HandOnSameMembers",
    ] {
        refused(routine, "a `var` parameter with members");
    }
}
