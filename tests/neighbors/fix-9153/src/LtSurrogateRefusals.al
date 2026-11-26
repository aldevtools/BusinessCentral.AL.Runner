// Neighbors of the lone surrogate half readers: TextBuilder and BigText operations on a text that holds one half, which
// the probe does not measure. Each must refuse on the head binary (no assumption covers it).
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
}
