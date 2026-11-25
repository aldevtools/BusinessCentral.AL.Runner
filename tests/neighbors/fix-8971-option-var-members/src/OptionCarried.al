// al-runner issue 8971: a memberless `var` Option parameter handed on to a `var` Option parameter with members. The run
// lets it through for a value from a measured origin (assumption var-option-carried-members, corpus PR 1569): one that
// carries exactly the callee's list, or another list or none at a callee whose every use of the parameter the cells measure
// (`=` and `case` on own members, `:=` of an own member, Format). Every neighbor below flips one fact of that condition.
table 62500 "OC Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Opt; Option) { OptionMembers = " ",Skip,Update; }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }

    procedure Confirm(var Choice: Option " ",Skip,Update): Text
    var
        Seen: Text;
    begin
        Seen := Format(Choice) + '|' + Format(Choice, 0, 2);
        Choice := Choice::Update;
        exit(Seen);
    end;

    [TryFunction]
    procedure TryConfirm(var Choice: Option " ",Skip,Update)
    begin
        Choice := Choice::Update;
    end;

    procedure ConfirmAbc(var Choice: Option A,B,C): Text
    var
        Seen: Text;
    begin
        Seen := Format(Choice) + '|' + Format(Choice, 0, 2);
        Choice := Choice::C;
        exit(Seen);
    end;

    // The callee of the queued batch path (OptionVarMembersBatch.al `ConfirmOnce`).
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
        end;
    end;

    // Evaluate of a text no member of the value's list names (CalleeOps_VarHop_Other, _Memberless `Evaluate Skip`).
    procedure EvaluateIt(var Choice: Option " ",Skip,Update): Text
    begin
        exit(Format(Evaluate(Choice, 'Skip')) + '>' + Format(Choice, 0, 2));
    end;

    // Callees that use the parameter in a way no cell measures for a value with another list or none.

    procedure CopyIt(var Choice: Option " ",Skip,Update): Text
    var
        Other: Option " ",Skip,Update;
    begin
        Other := Choice;
        exit(Format(Other, 0, 2));
    end;

    procedure ClearIt(var Choice: Option " ",Skip,Update): Text
    begin
        Clear(Choice);
        exit(Format(Choice, 0, 2));
    end;

    procedure NotEqualIt(var Choice: Option " ",Skip,Update): Text
    begin
        exit(Format(Choice <> Choice::Skip));
    end;

    procedure ByValueOnIt(var Choice: Option " ",Skip,Update): Text
    begin
        exit(ReadByValue(Choice));
    end;

    local procedure ReadByValue(Choice: Option " ",Skip,Update): Text
    begin
        exit(Format(Choice, 0, 2));
    end;

    procedure FormatOtherIt(var Choice: Option " ",Skip,Update): Text
    begin
        exit(Format(Choice, 0, 1));
    end;

    // An origin in a table procedure: no cell has one.
    procedure TableOrigin(): Text
    var
        Choice: Option " ",Skip,Update;
        Seen: Text;
    begin
        Choice := Choice::Skip;
        Seen := HopInTable(Choice);
        exit(Seen + '>' + Format(Choice));
    end;

    local procedure HopInTable(var V: Option): Text
    begin
        exit(Confirm(V));
    end;
}

interface "OC Hop"
{
    procedure Hop(var V: Option; var Row: Record "OC Row"): Text;
}

codeunit 62501 "OC Hop Impl" implements "OC Hop"
{
    procedure Hop(var V: Option; var Row: Record "OC Row"): Text
    begin
        exit(Row.Confirm(V));
    end;
}

codeunit 62502 "OC Events"
{
    procedure RaiseNoSender(var V: Option; var Row: Record "OC Row")
    begin
        OnNoSender(V, Row);
    end;

    procedure RaiseBusiness(var V: Option; var Row: Record "OC Row")
    begin
        OnBusiness(V, Row);
    end;

    procedure RaiseWithSender(var V: Option; var Row: Record "OC Row")
    begin
        OnWithSender(V, Row);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnNoSender(var V: Option; var Row: Record "OC Row")
    begin
    end;

    [BusinessEvent(true)]
    local procedure OnBusiness(var V: Option; var Row: Record "OC Row")
    begin
    end;

    [IntegrationEvent(true, false)]
    local procedure OnWithSender(var V: Option; var Row: Record "OC Row")
    begin
    end;
}

codeunit 62503 "OC Subscribers"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"OC Events", 'OnNoSender', '', false, false)]
    local procedure HandleNoSender(var V: Option; var Row: Record "OC Row")
    begin
        Pass(V, Row);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"OC Events", 'OnBusiness', '', false, false)]
    local procedure HandleBusiness(var V: Option; var Row: Record "OC Row")
    begin
        Pass(V, Row);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"OC Events", 'OnWithSender', '', false, false)]
    local procedure HandleWithSender(var V: Option; var Row: Record "OC Row")
    begin
        Pass(V, Row);
    end;

    local procedure Pass(var V: Option; var Row: Record "OC Row")
    begin
        Row.Confirm(V);
    end;
}

codeunit 62504 "OC Carried Tests"
{
    Subtype = Test;

    var
        GlobalChoice: Option " ",Skip,Update;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    local procedure Hop(var V: Option): Text
    var
        Row: Record "OC Row";
    begin
        exit(Row.Confirm(V));
    end;

    local procedure HopOnce(var V: Option): Text
    var
        Row: Record "OC Row";
    begin
        exit(Row.ConfirmOnce(V));
    end;

    local procedure HopEvaluate(var V: Option): Text
    var
        Row: Record "OC Row";
    begin
        exit(Row.EvaluateIt(V));
    end;

    local procedure HopCopy(var V: Option): Text
    var
        Row: Record "OC Row";
    begin
        exit(Row.CopyIt(V));
    end;

    local procedure HopClear(var V: Option): Text
    var
        Row: Record "OC Row";
    begin
        exit(Row.ClearIt(V));
    end;

    local procedure HopNotEqual(var V: Option): Text
    var
        Row: Record "OC Row";
    begin
        exit(Row.NotEqualIt(V));
    end;

    local procedure HopByValueOn(var V: Option): Text
    var
        Row: Record "OC Row";
    begin
        exit(Row.ByValueOnIt(V));
    end;

    local procedure HopFormatOther(var V: Option): Text
    var
        Row: Record "OC Row";
    begin
        exit(Row.FormatOtherIt(V));
    end;

    local procedure HopAbc(var V: Option): Text
    var
        Row: Record "OC Row";
    begin
        exit(Row.ConfirmAbc(V));
    end;

    local procedure HopToTry(var V: Option): Boolean
    var
        Row: Record "OC Row";
    begin
        exit(Row.TryConfirm(V));
    end;

    [TryFunction]
    local procedure TryHop(var V: Option)
    var
        Row: Record "OC Row";
    begin
        Row.Confirm(V);
    end;

    local procedure ByValueOrigin(Choice: Option " ",Skip,Update): Text
    var
        Seen: Text;
    begin
        Seen := Hop(Choice);
        exit(Seen + '>' + Format(Choice));
    end;

    // The measured shape: a local of the `var` section with the callee's own list (VarHop_Same_*).
    [Test]
    procedure Same_LocalOrigin()
    var
        Choice: Option " ",Skip,Update;
    begin
        Choice := Choice::Skip;
        Check(Hop(Choice) + '>' + Format(Choice), 'Skip|1>Update');
    end;

    // Another list, the same in origin and callee: the list's content is not read.
    [Test]
    procedure SameOtherList_LocalOrigin()
    var
        Choice: Option A,B,C;
    begin
        Choice := Choice::B;
        Check(HopAbc(Choice) + '>' + Format(Choice), 'B|1>C');
    end;

    // A memberless origin: the callee reads the value's (no) list (VarHop_Memberless_*).
    [Test]
    procedure MemberlessOrigin()
    var
        Choice: Option;
    begin
        Choice := 1;
        Check(Hop(Choice) + '>' + Format(Choice), '1|1>2');
    end;

    // Other members in the origin: the callee reads the value's list (VarHop_Other_*).
    [Test]
    procedure OtherMembersOrigin()
    var
        Choice: Option First,Second,Third;
    begin
        Choice := Choice::Second;
        Check(Hop(Choice) + '>' + Format(Choice), 'Second|1>Third');
    end;

    // The queued batch path: a memberless local kept across calls (BatchQueued_*).
    [Test]
    procedure MemberlessOrigin_Queued()
    var
        Choice: Option;
        Log: Text;
        I: Integer;
    begin
        for I := 1 to 3 do
            Log += HopOnce(Choice) + '/' + Format(Choice) + ';';
        Check(Log + Format(Choice, 0, 2), 'blank/2;update/2;update/2;2');
    end;

    // The callee reads, compares and writes; its use of the parameter decides (the values below are BC's where a cell
    // measures them, OptionVarMembersCallee.al CalleeOps_VarHop_Memberless; the rest are not measured).
    [Test]
    procedure Memberless_EvaluateCallee()
    var
        Choice: Option;
    begin
        Choice := 1;
        Check(HopEvaluate(Choice), 'No>1');
    end;

    [Test]
    procedure Memberless_CopyCallee()
    var
        Choice: Option;
    begin
        Choice := 1;
        Check(HopCopy(Choice), '1');
    end;

    [Test]
    procedure Memberless_ClearCallee()
    var
        Choice: Option;
    begin
        Choice := 1;
        Check(HopClear(Choice), '0');
    end;

    [Test]
    procedure Memberless_NotEqualCallee()
    var
        Choice: Option;
    begin
        Choice := 1;
        Check(HopNotEqual(Choice), 'No');
    end;

    [Test]
    procedure Memberless_ByValueHandOnCallee()
    var
        Choice: Option;
    begin
        Choice := 1;
        Check(HopByValueOn(Choice), '1');
    end;

    [Test]
    procedure Memberless_FormatOtherCallee()
    var
        Choice: Option;
    begin
        Choice := 1;
        Check(HopFormatOther(Choice), '1');
    end;

    [Test]
    procedure OtherMembers_EvaluateCallee()
    var
        Choice: Option First,Second,Third;
    begin
        Choice := Choice::Second;
        Check(HopEvaluate(Choice), 'No>1');
    end;

    // The same list: the callee's own list, any use (CalleeOps_VarHop_Same `Evaluate Skip inside`).
    [Test]
    procedure Same_EvaluateCallee()
    var
        Choice: Option " ",Skip,Update;
    begin
        Choice := Choice::Update;
        Check(HopEvaluate(Choice), 'Yes>1');
    end;

    [Test]
    procedure GlobalOrigin()
    begin
        GlobalChoice := GlobalChoice::Skip;
        Check(Hop(GlobalChoice) + '>' + Format(GlobalChoice), 'Skip|1>Update');
    end;

    [Test]
    procedure FieldOrigin()
    var
        Row: Record "OC Row";
    begin
        Row.Opt := Row.Opt::Skip;
        Check(Hop(Row.Opt) + '>' + Format(Row.Opt), 'Skip|1>Update');
    end;

    [Test]
    procedure ByValueParameterOrigin()
    begin
        Check(ByValueOrigin(1), 'Skip|1>Update');
    end;

    [Test]
    procedure TableProcedureOrigin()
    var
        Row: Record "OC Row";
    begin
        Check(Row.TableOrigin(), 'Skip|1>Update');
    end;

    // The callee is a TryFunction.
    [Test]
    procedure TryFunctionCallee()
    var
        Choice: Option " ",Skip,Update;
    begin
        Choice := Choice::Skip;
        HopToTry(Choice);
        Check(Format(Choice), 'Update');
    end;

    // The last hop is a TryFunction.
    [Test]
    procedure TryFunctionHop()
    var
        Choice: Option " ",Skip,Update;
    begin
        Choice := Choice::Skip;
        TryHop(Choice);
        Check(Format(Choice), 'Update');
    end;

    // A memberless `var` parameter of an interface method hands it on.
    [Test]
    procedure InterfaceMethodHop()
    var
        Choice: Option " ",Skip,Update;
        Impl: Codeunit "OC Hop Impl";
        H: Interface "OC Hop";
        Row: Record "OC Row";
    begin
        Choice := Choice::Skip;
        H := Impl;
        Check(H.Hop(Choice, Row) + '>' + Format(Choice), 'Skip|1>Update');
    end;

    // The event contract of the batch cells: IntegrationEvent(true, false).
    [Test]
    procedure Event_WithSender()
    var
        Choice: Option " ",Skip,Update;
        Events: Codeunit "OC Events";
        Row: Record "OC Row";
    begin
        Choice := Choice::Skip;
        Events.RaiseWithSender(Choice, Row);
        Check(Format(Choice), 'Update');
    end;

    // The batch event with a memberless or another list: no cell raises it with one.
    [Test]
    procedure Event_WithSender_Memberless()
    var
        Choice: Option;
        Events: Codeunit "OC Events";
        Row: Record "OC Row";
    begin
        Choice := 1;
        Events.RaiseWithSender(Choice, Row);
        Check(Format(Choice), '2');
    end;

    [Test]
    procedure Event_WithSender_OtherMembers()
    var
        Choice: Option First,Second,Third;
        Events: Codeunit "OC Events";
        Row: Record "OC Row";
    begin
        Choice := Choice::Second;
        Events.RaiseWithSender(Choice, Row);
        Check(Format(Choice), 'Third');
    end;

    [Test]
    procedure Event_NoSender()
    var
        Choice: Option " ",Skip,Update;
        Events: Codeunit "OC Events";
        Row: Record "OC Row";
    begin
        Choice := Choice::Skip;
        Events.RaiseNoSender(Choice, Row);
        Check(Format(Choice), 'Update');
    end;

    [Test]
    procedure Event_Business()
    var
        Choice: Option " ",Skip,Update;
        Events: Codeunit "OC Events";
        Row: Record "OC Row";
    begin
        Choice := Choice::Skip;
        Events.RaiseBusiness(Choice, Row);
        Check(Format(Choice), 'Update');
    end;

    // Main's paths, unchanged: a local with other members straight to the callee, and a chain of memberless parameters.
    [Test]
    procedure Main_DirectOtherMembers()
    var
        Choice: Option First,Second,Third;
        Row: Record "OC Row";
    begin
        Choice := Choice::Second;
        Check(Row.Confirm(Choice) + '>' + Format(Choice), 'Skip|1>Third');
    end;

    [Test]
    procedure Main_MemberlessChain()
    var
        Choice: Option " ",Skip,Update;
    begin
        Choice := Choice::Skip;
        Check(Inner(Choice), 'Skip|1');
    end;

    local procedure Inner(var V: Option): Text
    begin
        exit(Format(V) + '|' + Format(V, 0, 2));
    end;
}

// Round 4: the callee's Evaluate, Clear, `:= 1` and hand-ons on a value that carries another list or none, and a `var`
// parameter with members handed to a memberless one (OptionVarMembersCallee.al ConfirmOp, HandOnMemberless, HandOnByValue).
codeunit 62505 "OC Ops"
{
    procedure EvaluateText(var Choice: Option " ",Skip,Update; T: Text): Text
    begin
        exit(Format(Evaluate(Choice, T)) + '>' + Format(Choice, 0, 2));
    end;

    procedure EvaluateBlank(var Choice: Option " ",Skip,Update): Text
    begin
        exit(Format(Evaluate(Choice, '')) + '>' + Format(Choice, 0, 2));
    end;

    procedure ClearOnly(var Choice: Option " ",Skip,Update)
    begin
        Clear(Choice);
    end;

    procedure AssignOne(var Choice: Option " ",Skip,Update)
    begin
        Choice := 1;
    end;

    procedure HandOnFormat(var Choice: Option " ",Skip,Update): Text
    begin
        exit(FormatMemberless(Choice));
    end;

    procedure HandOnByValue(var Choice: Option " ",Skip,Update): Text
    begin
        exit(FormatByValue(Choice));
    end;

    procedure HandOnEvaluate(var Choice: Option " ",Skip,Update): Text
    begin
        exit(EvaluateMemberless(Choice));
    end;

    procedure HandOnEmpty(var Choice: Option " ",Skip,Update): Text
    begin
        NeverReads(Choice);
        exit('');
    end;

    local procedure FormatMemberless(var V: Option): Text
    begin
        exit(Format(V) + '|' + Format(V, 0, 2));
    end;

    local procedure FormatByValue(V: Option): Text
    begin
        exit(Format(V) + '|' + Format(V, 0, 2));
    end;

    local procedure EvaluateMemberless(var V: Option): Text
    begin
        exit(Format(Evaluate(V, 'Nope')) + '|' + Format(V, 0, 2));
    end;

    local procedure NeverReads(var V: Option)
    begin
    end;
}

codeunit 62506 "OC Ops Tests"
{
    Subtype = Test;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    local procedure Hop(var V: Option; Op: Integer; T: Text): Text
    var
        Ops: Codeunit "OC Ops";
    begin
        case Op of
            1:
                exit(Ops.EvaluateText(V, T));
            2:
                exit(Ops.EvaluateBlank(V));
            3:
                Ops.ClearOnly(V);
            4:
                Ops.AssignOne(V);
            5:
                exit(Ops.HandOnFormat(V));
            6:
                exit(Ops.HandOnByValue(V));
            7:
                exit(Ops.HandOnEvaluate(V));
            8:
                exit(Ops.HandOnEmpty(V));
        end;
    end;

    // Evaluate of a member of the value's list as it is spelled (CalleeErrors_VarHop_Other `Evaluate Second`).
    [Test]
    procedure OtherMembers_EvaluateMember()
    var
        Choice: Option First,Second,Third;
    begin
        Choice := Choice::Third;
        Check(Hop(Choice, 1, 'Second') + '>' + Format(Choice), 'Yes>1>Second');
    end;

    // Digits (ConfirmOp op 3, `Evaluate 5`).
    [Test]
    procedure Memberless_EvaluateDigits()
    var
        Choice: Option;
    begin
        Choice := 1;
        Check(Hop(Choice, 1, '5') + '>' + Format(Choice), 'Yes>5>5');
    end;

    // Not measured, refused: a member of the value's list in another case, a blank text (a parameter and a literal).
    [Test]
    procedure OtherMembers_EvaluateOtherCase()
    var
        Choice: Option First,Second,Third;
    begin
        Choice := Choice::Third;
        Check(Hop(Choice, 1, 'second'), 'Yes>1');
    end;

    [Test]
    procedure OtherMembers_EvaluateBlankParam()
    var
        Choice: Option First,Second,Third;
    begin
        Choice := Choice::Third;
        Check(Hop(Choice, 1, ''), 'Yes>0');
    end;

    [Test]
    procedure OtherMembers_EvaluateBlankLiteral()
    var
        Choice: Option First,Second,Third;
    begin
        Choice := Choice::Third;
        Check(Hop(Choice, 2, ''), 'Yes>0');
    end;

    // Clear of a value that carries a list (CalleeOps_VarHop_Other `Clear`); of one that carries none, refused.
    [Test]
    procedure OtherMembers_Clear()
    var
        Choice: Option First,Second,Third;
    begin
        Choice := Choice::Third;
        Hop(Choice, 3, '');
        Check(Format(Choice) + '|' + Format(Choice, 0, 2), 'First|0');
    end;

    [Test]
    procedure Memberless_Clear()
    var
        Choice: Option;
    begin
        Choice := 2;
        Hop(Choice, 3, '');
        Check(Format(Choice, 0, 2), '0');
    end;

    // `:= 1` stores the number in the value's list (ConfirmOp op 5).
    [Test]
    procedure OtherMembers_AssignOne()
    var
        Choice: Option First,Second,Third;
    begin
        Choice := Choice::Third;
        Hop(Choice, 4, '');
        Check(Format(Choice), 'Second');
    end;

    // The callee hands the value on to a memberless routine that only formats it (ConfirmOp ops 6 and 7: by value the list
    // is lost, `b 4|4`).
    [Test]
    procedure OtherMembers_HandOnFormat()
    var
        Choice: Option First,Second,Third;
    begin
        Choice := Choice::Third;
        Check(Hop(Choice, 5, ''), 'Third|2');
    end;

    [Test]
    procedure OtherMembers_HandOnByValue()
    var
        Choice: Option First,Second,Third;
    begin
        Choice := Choice::Third;
        Check(Hop(Choice, 6, ''), '2|2');
    end;

    // Refused: the target does more than format it, or never reads it.
    [Test]
    procedure OtherMembers_HandOnEvaluate()
    var
        Choice: Option First,Second,Third;
    begin
        Choice := Choice::Third;
        Check(Hop(Choice, 7, ''), 'No|2');
    end;

    [Test]
    procedure OtherMembers_HandOnEmpty()
    var
        Choice: Option First,Second,Third;
    begin
        Choice := Choice::Third;
        Check(Hop(Choice, 8, ''), '');
    end;

    // A `var` parameter with members handed to a memberless one, called straight from the test (CalleeOps_VarHop_Same
    // `HandOnMemberless`): runs when the target only formats it, refused otherwise.
    [Test]
    procedure Same_HandOnFormat_Direct()
    var
        Choice: Option " ",Skip,Update;
        Ops: Codeunit "OC Ops";
    begin
        Choice := Choice::Skip;
        Check(Ops.HandOnFormat(Choice), 'Skip|1');
    end;

    [Test]
    procedure Same_HandOnEvaluate_Direct()
    var
        Choice: Option " ",Skip,Update;
        Ops: Codeunit "OC Ops";
    begin
        Choice := Choice::Skip;
        Check(Ops.HandOnEvaluate(Choice), 'No|1');
    end;

    [Test]
    procedure Same_HandOnEmpty_Direct()
    var
        Choice: Option " ",Skip,Update;
        Ops: Codeunit "OC Ops";
    begin
        Choice := Choice::Skip;
        Check(Ops.HandOnEmpty(Choice), '');
    end;
}
