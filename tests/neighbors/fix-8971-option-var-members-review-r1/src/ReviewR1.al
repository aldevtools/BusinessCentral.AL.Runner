// Review round 1 of the fix for al-runner issue 8971 (assumption var-option-carried-members, corpus PR 1569). Each test
// below flips one fact the round-1 gates did not read:
// - the callee changes its by-value text before Evaluate reads it;
// - an [EventSubscriber] procedure called directly, as a plain hop;
// - a value whose list is shorter than the callee's;
// - the binding of a `var` parameter with members that hands the value on to a memberless one.
// Review round 2 adds: a subscriber called directly from a hop below the raise's subscriber, and a store into one character
// of the by-value text (by `:=` and by `var`).
table 62520 "R1 Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Opt; Option) { OptionMembers = First,Second,Third; }
        field(3; OptCap; Option) { OptionMembers = First,Second,Third; OptionCaption = 'One,Two,Three'; }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }

    // A table procedure as the direct caller of the hand-on (the cells' direct callers are codeunit procedures).
    procedure HandOnFromTable(): Text
    var
        Ops: Codeunit "R1 Ops";
        Choice: Option " ",Skip,Update;
    begin
        Choice := Choice::Skip;
        exit(Ops.HandOnFormat(Choice));
    end;
}

codeunit 62521 "R1 Ops"
{
    procedure EvaluateReassigned(var Choice: Option " ",Skip,Update; T: Text): Text
    begin
        T := 'second';
        exit(Format(Evaluate(Choice, T)) + '>' + Format(Choice) + '|' + Format(Choice, 0, 2));
    end;

    procedure EvaluateReassignedBlank(var Choice: Option " ",Skip,Update; T: Text): Text
    begin
        T := '';
        exit(Format(Evaluate(Choice, T)) + '>' + Format(Choice) + '|' + Format(Choice, 0, 2));
    end;

    procedure EvaluateReassignedStatement(var Choice: Option " ",Skip,Update; T: Text)
    begin
        T := '';
        Evaluate(Choice, T);
    end;

    procedure EvaluateChangedByVar(var Choice: Option " ",Skip,Update; T: Text): Text
    begin
        SetSecond(T);
        exit(Format(Evaluate(Choice, T)) + '>' + Format(Choice) + '|' + Format(Choice, 0, 2));
    end;

    // The text is written only after Evaluate read it: the scan does not order the uses, so it refuses too.
    procedure EvaluateThenWrite(var Choice: Option " ",Skip,Update; T: Text): Text
    begin
        Evaluate(Choice, T);
        T := '';
        exit(Format(Choice) + '|' + Format(Choice, 0, 2));
    end;

    local procedure SetSecond(var T: Text)
    begin
        T := 'second';
    end;

    procedure EvaluateCharStore(var Choice: Option " ",Skip,Update; T: Text): Text
    begin
        T[1] := 's';
        exit(Format(Evaluate(Choice, T)) + '>' + Format(Choice) + '|' + Format(Choice, 0, 2));
    end;

    procedure EvaluateCharByVar(var Choice: Option " ",Skip,Update; T: Text): Text
    begin
        SetChar(T[1]);
        exit(Format(Evaluate(Choice, T)) + '>' + Format(Choice) + '|' + Format(Choice, 0, 2));
    end;

    local procedure SetChar(var C: Char)
    begin
        C := 's';
    end;

    procedure WriteUpdate(var Choice: Option " ",Skip,Update): Text
    var
        R: Text;
    begin
        case Choice of
            Choice::" ":
                R := 'blank';
            Choice::Skip:
                R := 'skip';
            Choice::Update:
                R := 'update';
            else
                R := 'else';
        end;
        Choice := Choice::Update;
        exit(R + '>' + Format(Choice) + '|' + Format(Choice, 0, 2));
    end;

    procedure HandOnFormat(var Choice: Option " ",Skip,Update): Text
    begin
        exit(FormatMemberless(Choice));
    end;

    // A `var` parameter with members handed on to another one, which hands it to the memberless routine.
    procedure HandOnThroughMembers(var Choice: Option " ",Skip,Update): Text
    begin
        exit(HandOnFormat(Choice));
    end;

    local procedure FormatMemberless(var V: Option): Text
    begin
        exit(Format(V) + '|' + Format(V, 0, 2));
    end;
}

codeunit 62522 "R1 Subscribers"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"R1 Events", 'OnWithSender', '', false, false)]
    procedure HandleDirectHop(var V: Option)
    begin
        PassWrite(V);
    end;

    local procedure PassWrite(var V: Option)
    var
        Ops: Codeunit "R1 Ops";
        T: Text;
    begin
        T := Ops.WriteUpdate(V);
    end;

    // The raise's subscriber, then a hop with no attribute, then `HandleDirectHop` called directly.
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"R1 Events", 'OnRaise', '', false, false)]
    local procedure HandleRaise(var V: Option)
    begin
        HopToDirect(V);
    end;

    local procedure HopToDirect(var V: Option)
    begin
        HandleDirectHop(V);
    end;
}

codeunit 62523 "R1 Events"
{
    procedure Raise(var V: Option)
    begin
        OnRaise(V);
    end;

    [IntegrationEvent(true, false)]
    local procedure OnRaise(var V: Option)
    begin
    end;

    [IntegrationEvent(true, false)]
    local procedure OnWithSender(var V: Option)
    begin
    end;
}

codeunit 62524 "R1 Ops Tests"
{
    Subtype = Test;

    var
        GlobalChoice: Option A,B,C;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    local procedure HopReassigned(var V: Option; T: Text): Text
    var
        Ops: Codeunit "R1 Ops";
    begin
        exit(Ops.EvaluateReassigned(V, T));
    end;

    local procedure HopReassignedBlank(var V: Option; T: Text): Text
    var
        Ops: Codeunit "R1 Ops";
    begin
        exit(Ops.EvaluateReassignedBlank(V, T));
    end;

    local procedure HopReassignedStatement(var V: Option; T: Text)
    var
        Ops: Codeunit "R1 Ops";
    begin
        Ops.EvaluateReassignedStatement(V, T);
    end;

    local procedure HopCharStore(var V: Option; T: Text): Text
    var
        Ops: Codeunit "R1 Ops";
    begin
        exit(Ops.EvaluateCharStore(V, T));
    end;

    local procedure HopCharByVar(var V: Option; T: Text): Text
    var
        Ops: Codeunit "R1 Ops";
    begin
        exit(Ops.EvaluateCharByVar(V, T));
    end;

    local procedure HopChangedByVar(var V: Option; T: Text): Text
    var
        Ops: Codeunit "R1 Ops";
    begin
        exit(Ops.EvaluateChangedByVar(V, T));
    end;

    local procedure HopThenWrite(var V: Option; T: Text): Text
    var
        Ops: Codeunit "R1 Ops";
    begin
        exit(Ops.EvaluateThenWrite(V, T));
    end;

    local procedure ByValueHandOn(Choice: Option A,B,C): Text
    var
        Ops: Codeunit "R1 Ops";
    begin
        exit(Ops.HandOnFormat(Choice));
    end;

    local procedure MemberlessHopHandOn(var V: Option): Text
    var
        Ops: Codeunit "R1 Ops";
    begin
        exit(Ops.HandOnFormat(V));
    end;

    // MED 1: the text at the call is `Nope`; the callee sets `second` (another case of a member) before Evaluate.
    [Test]
    procedure OtherMembers_EvaluateReassignedText()
    var
        Choice: Option First,Second,Third;
    begin
        Choice := Choice::Third;
        Check(HopReassigned(Choice, 'Nope') + '>' + Format(Choice), 'Yes>Second|1>Second');
    end;

    [Test]
    procedure OtherMembers_EvaluateReassignedBlank()
    var
        Choice: Option First,Second,Third;
    begin
        Choice := Choice::Third;
        Check(HopReassignedBlank(Choice, 'Nope') + '>' + Format(Choice), 'No>Third|2>Third');
    end;

    [Test]
    procedure OtherMembers_EvaluateReassignedBlankStatement()
    var
        Choice: Option First,Second,Third;
    begin
        Choice := Choice::Third;
        HopReassignedStatement(Choice, 'Nope');
        Check(Format(Choice), 'Third');
    end;

    [Test]
    procedure OtherMembers_EvaluateTextChangedByVar()
    var
        Choice: Option First,Second,Third;
    begin
        Choice := Choice::Third;
        Check(HopChangedByVar(Choice, 'Nope') + '>' + Format(Choice), 'Yes>Second|1>Second');
    end;

    // Review round 2: one character of the text stored, or handed on by `var`, before Evaluate reads it ('xecond' becomes
    // 'second', another case of a member of the value's list).
    [Test]
    procedure OtherMembers_EvaluateCharStore()
    var
        Choice: Option First,Second,Third;
    begin
        Choice := Choice::Third;
        Check(HopCharStore(Choice, 'xecond') + '>' + Format(Choice), 'Yes>Second|1>Second');
    end;

    [Test]
    procedure OtherMembers_EvaluateCharByVar()
    var
        Choice: Option First,Second,Third;
    begin
        Choice := Choice::Third;
        Check(HopCharByVar(Choice, 'xecond') + '>' + Format(Choice), 'Yes>Second|1>Second');
    end;

    [Test]
    procedure OtherMembers_EvaluateThenWriteText()
    var
        Choice: Option First,Second,Third;
    begin
        Choice := Choice::Third;
        Check(HopThenWrite(Choice, 'Second') + '>' + Format(Choice), 'Second|1>Second');
    end;

    // MED 4: the binding of the `var` parameter with members that hands the value on.
    [Test]
    procedure HandOn_CaptionField()
    var
        Row: Record "R1 Row";
        Ops: Codeunit "R1 Ops";
    begin
        Row.OptCap := Row.OptCap::Second;
        Check(Ops.HandOnFormat(Row.OptCap), 'Two|1');
    end;

    [Test]
    procedure HandOn_PlainField()
    var
        Row: Record "R1 Row";
        Ops: Codeunit "R1 Ops";
    begin
        Row.Opt := Row.Opt::Second;
        Check(Ops.HandOnFormat(Row.Opt), 'Second|1');
    end;

    [Test]
    procedure HandOn_GlobalOtherList()
    var
        Ops: Codeunit "R1 Ops";
    begin
        GlobalChoice := GlobalChoice::B;
        Check(Ops.HandOnFormat(GlobalChoice), 'B|1');
    end;

    [Test]
    procedure HandOn_ByValueOtherList()
    begin
        Check(ByValueHandOn(1), 'B|1');
    end;

    [Test]
    procedure HandOn_ThroughMembersParameter()
    var
        Ops: Codeunit "R1 Ops";
        Choice: Option " ",Skip,Update;
    begin
        Choice := Choice::Skip;
        Check(Ops.HandOnThroughMembers(Choice), 'Skip|1');
    end;

    [Test]
    procedure HandOn_TableLocal()
    var
        Row: Record "R1 Row";
    begin
        Check(Row.HandOnFromTable(), 'Skip|1');
    end;

    // Measured bindings, kept running: a memberless local (CalleeOps_MemberlessLocal `m Skip|1`), a local with another list
    // (OptionVarMemberlessWrites.al:372: the memberless callee sees the parameter's member) and with the same list, and a
    // value a memberless hop carries (CalleeOps_VarHop_Other `m Fifth|4`).
    [Test]
    procedure HandOn_MemberlessLocal()
    var
        Ops: Codeunit "R1 Ops";
        Choice: Option;
    begin
        Choice := 1;
        Check(Ops.HandOnFormat(Choice), 'Skip|1');
    end;

    [Test]
    procedure HandOn_OtherListLocal()
    var
        Ops: Codeunit "R1 Ops";
        Choice: Option A,B,C;
    begin
        Choice := Choice::B;
        Check(Ops.HandOnFormat(Choice), 'Skip|1');
    end;

    [Test]
    procedure HandOn_SameListLocal()
    var
        Ops: Codeunit "R1 Ops";
        Choice: Option " ",Skip,Update;
    begin
        Choice := Choice::Skip;
        Check(Ops.HandOnFormat(Choice), 'Skip|1');
    end;

    [Test]
    procedure HandOn_CarriedThroughHop()
    var
        Choice: Option First,Second,Third;
    begin
        Choice := Choice::Third;
        Check(MemberlessHopHandOn(Choice), 'Third|2');
    end;
}

codeunit 62525 "R1 Carried Tests"
{
    Subtype = Test;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    local procedure Hop(var V: Option): Text
    var
        Ops: Codeunit "R1 Ops";
    begin
        exit(Ops.WriteUpdate(V));
    end;

    local procedure PlainHop(var V: Option)
    var
        Ops: Codeunit "R1 Ops";
        T: Text;
    begin
        T := Ops.WriteUpdate(V);
    end;

    // MED 2: the subscriber procedure called directly, then a hop with no attribute, then the callee.
    [Test]
    procedure SubscriberCalledDirect_OtherList()
    var
        Choice: Option First,Second,Third;
        Subs: Codeunit "R1 Subscribers";
    begin
        Choice := Choice::Second;
        Subs.HandleDirectHop(Choice);
        Check(Format(Choice) + '|' + Format(Choice, 0, 2), 'Third|2');
    end;

    // Review round 2: the raise's subscriber, a hop, then the subscriber procedure of another event called directly.
    [Test]
    procedure SubscriberCalledDirect_AfterRaise()
    var
        Choice: Option " ",Skip,Update;
        Events: Codeunit "R1 Events";
    begin
        Choice := Choice::Skip;
        Events.Raise(Choice);
        Check(Format(Choice), 'Update');
    end;

    [Test]
    procedure SubscriberCalledDirect_Memberless()
    var
        Choice: Option;
        Subs: Codeunit "R1 Subscribers";
    begin
        Choice := 1;
        Subs.HandleDirectHop(Choice);
        Check(Format(Choice) + '|' + Format(Choice, 0, 2), '2|2');
    end;

    // The control: the same chain without the subscriber (VarHop_Other_*).
    [Test]
    procedure PlainHop_OtherList()
    var
        Choice: Option First,Second,Third;
    begin
        Choice := Choice::Second;
        PlainHop(Choice);
        Check(Format(Choice) + '|' + Format(Choice, 0, 2), 'Third|2');
    end;

    // MED 3: a list shorter than the callee's; the callee's case and its write name ordinals past it.
    [Test]
    procedure ShorterList_WriteBeyond()
    var
        Choice: Option A,B;
    begin
        Choice := Choice::B;
        Check(Hop(Choice) + '>' + Format(Choice) + '|' + Format(Choice, 0, 2), 'skip>2|2>2|2');
    end;

    // A list as long as the callee's: every own member is a member of the value's list, as in VarHop_Other_*.
    [Test]
    procedure EqualLengthList_Write()
    var
        Choice: Option A,B,C;
    begin
        Choice := Choice::B;
        Check(Hop(Choice) + '>' + Format(Choice), 'skip>C|2>C');
    end;
}
