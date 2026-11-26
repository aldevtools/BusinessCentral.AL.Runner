// Neighbors of the lone surrogate half readers: TextBuilder Remove, Length (set) and Capacity, BigText.AddText on a
// text that holds one half, the comparison of a Code that holds one, and the order of a Text that holds one against a first character or a shape the probe does not measure, and `<=` and `>=` on it. The probe measures none of them. Each must
// refuse on the head binary (no assumption covers it).
codeunit 75795 "LT SH Refusals"
{
    Subtype = Test;

    local procedure Hi(): Text
    var
        C: Char;
    begin
        C := 55357;
        exit(Format(C));
    end;

    [Test]
    procedure TextBuilder_Remove_LoneHalf()
    var
        TB: TextBuilder;
    begin
        TB.Append('a');
        TB.Append(Hi());
        TB.Append('b');
        TB.Remove(1, 1);
        if TB.Length() <> 2 then
            Error('length %1', TB.Length());
    end;

    [Test]
    procedure TextBuilder_SetLength_LoneHalf()
    var
        TB: TextBuilder;
    begin
        TB.Append('ab');
        TB.Append(Hi());
        TB.Length(2);
        if TB.ToText() <> 'ab' then
            Error('text %1', TB.ToText());
    end;

    [Test]
    procedure BigText_AddText_LoneHalf()
    var
        BT: BigText;
    begin
        BT.AddText(Hi());
        if BT.Length() <> 1 then
            Error('length %1', BT.Length());
    end;

    [Test]
    procedure Code_Compare_LoneHalf()
    var
        C: Code[10];
    begin
        C := Hi();
        if C < 'A' then
            Error('less');
    end;

    [Test]
    procedure TextBuilder_Capacity_LoneHalf()
    var
        TB: TextBuilder;
    begin
        TB.Append(Hi());
        if TB.Capacity() <> 16 then
            Error('capacity %1', TB.Capacity());
    end;

    [Test]
    procedure Text_Order_LoneHalfVsUnmeasuredChar()
    var
        T: Text;
    begin
        T := Hi();
        if T < 'A' then
            Error('less');
    end;

    [Test]
    procedure Text_Order_SharedPrefixThenLoneHalf()
    var
        T: Text;
    begin
        T := 'a' + Hi();
        if T < 'ab' then
            Error('less');
    end;

    [Test]
    procedure Text_LessOrEqual_LoneHalf()
    var
        T: Text;
    begin
        T := Hi();
        if T <= 'a' then
            Error('less or equal');
    end;

    [Test]
    procedure Text_GreaterOrEqual_LoneHalf()
    var
        T: Text;
    begin
        T := Hi();
        if not (T >= 'a') then
            Error('not greater or equal');
    end;
}
