// Review round 1 neighbors of the lone surrogate half readers. Each shape is one the probe (corpus PR 1605,
// LtSurrogateHalves.al) does not measure, so each must refuse on the head binary:
// - a Text with a lone half against a Char, for every operator;
// - a TextBuilder Insert position or a ToText range that cuts a whole pair;
// - an order of a lone half other than 55357 and 56832, or against a text the probe does not compare it with.
// A .NET ordering of a Text against a Char is not here: AL gives no DotNet String operand that reaches it.
codeunit 75800 "LT SH Review R1"
{
    Subtype = Test;

    local procedure Unit(U: Integer): Text
    var
        C: Char;
    begin
        C := U;
        exit(Format(C));
    end;

    local procedure Hi(): Text
    begin
        exit(Unit(55357));
    end;

    [Test]
    procedure Text_LessOrEqual_LoneHalfVsChar()
    var
        C: Char;
        Res: Text;
    begin
        C := 'a';
        Res := Format(Hi() <= C);
    end;

    [Test]
    procedure Text_GreaterOrEqual_LoneHalfVsChar()
    var
        C: Char;
        Res: Text;
    begin
        C := 'a';
        Res := Format(Hi() >= C);
    end;

    [Test]
    procedure Text_Less_LoneHalfVsChar()
    var
        C: Char;
        Res: Text;
    begin
        C := 'a';
        Res := Format(Hi() < C);
    end;

    [Test]
    procedure Text_Equal_LoneHalfVsChar()
    var
        C: Char;
        Res: Text;
    begin
        C := 'a';
        Res := Format(Hi() = C);
    end;

    [Test]
    procedure TextBuilder_Insert_InsidePair()
    var
        TB: TextBuilder;
    begin
        TB.Append('😀');
        TB.Insert(2, 'x');
    end;

    [Test]
    procedure TextBuilder_ToText_CutsPair()
    var
        TB: TextBuilder;
        Res: Text;
    begin
        TB.Append('😀');
        Res := TB.ToText(1, 1);
    end;

    [Test]
    procedure TextBuilder_Insert_InsidePair_BuilderHasLone()
    var
        TB: TextBuilder;
    begin
        TB.Append('a');
        TB.Append(Hi());
        TB.Append('😀');
        TB.Insert(4, 'x');
    end;

    [Test]
    procedure TextBuilder_ToText_CutsPair_BuilderHasLone()
    var
        TB: TextBuilder;
        Res: Text;
    begin
        TB.Append(Hi());
        TB.Append('😀');
        Res := TB.ToText(1, 2);
    end;

    [Test]
    procedure Text_Order_OtherHalfUnit()
    var
        Res: Text;
    begin
        Res := Format(Unit(55296) > 'a');
    end;

    [Test]
    procedure Text_Order_StartsWithA_Unmeasured()
    var
        Res: Text;
    begin
        Res := Format(Hi() > 'aZ');
    end;

    [Test]
    procedure Text_Order_LoneHalfVsAPlusPair()
    var
        Res: Text;
    begin
        Res := Format(Hi() > ('a' + '😀'));
    end;

    [Test]
    procedure Text_Order_LowHalfVsZ()
    var
        Res: Text;
    begin
        Res := Format(Unit(56832) < 'z');
    end;
}
