// The shapes next to the cells corpus PR 1486 measured (record/TransferFieldsRemovedField.al). Expected values are BC's where
// the probe measured them; elsewhere they say what the runner answers (main's skip or copy into a removed target field).
codeunit 50120 "NB TransferFields Removed"
{
    Subtype = Test;

    local procedure Source(var Src: Record "NB Src")
    begin
        Src."No." := 'A';
        Src.F2 := true;
        Src.F3 := 'ABCDEFGHIJ';
        Src.F4 := '12';
        Src.F5 := 4;
        Src.ExtBool := true;
    end;

    local procedure Expect(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('Expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure RemovedTarget_BoolToCode_Default()
    var
        Src: Record "NB Src";
        Tgt: Record "NB T BoolToCode";
    begin
        Source(Src);
        Tgt.TransferFields(Src);
        Expect(Tgt."No.", 'A');
    end;

    [Test]
    procedure RemovedTarget_BoolToCode_InitPkFalse()
    var
        Src: Record "NB Src";
        Tgt: Record "NB T BoolToCode";
    begin
        Source(Src);
        Tgt."No." := 'T';
        Tgt.TransferFields(Src, false);
        Expect(Tgt."No.", 'T');
    end;

    [Test]
    procedure RemovedTarget_IntFromText_Skip()
    var
        Src: Record "NB Src";
        Tgt: Record "NB T IntFromText";
    begin
        Source(Src);
        Tgt.TransferFields(Src, true, true);
        Expect(Tgt."No.", 'A');
    end;

    [Test]
    procedure RemovedTarget_LongCode_Default()
    var
        Src: Record "NB Src";
        Tgt: Record "NB T LongCode";
    begin
        Source(Src);
        Tgt.TransferFields(Src);
        Expect(Tgt."No.", 'A');
    end;

    [Test]
    procedure RemovedTarget_IntSame_Default()
    var
        Src: Record "NB Src";
        Tgt: Record "NB T IntSame";
    begin
        Source(Src);
        Tgt.TransferFields(Src);
        Expect(Tgt."No.", 'A');
    end;

    [Test]
    procedure RemovedTarget_IntSame_Skip_Unmeasured()
    var
        Src: Record "NB Src";
        Tgt: Record "NB T IntSame";
    begin
        Source(Src);
        Tgt.TransferFields(Src, true, true);
        Expect(Tgt."No.", 'A');
    end;

    [Test]
    procedure RemovedTarget_TextSame_Unmeasured()
    var
        Src: Record "NB Src";
        Tgt: Record "NB T TextSame";
    begin
        Source(Src);
        Tgt.TransferFields(Src);
        Expect(Tgt."No.", 'A');
    end;

    [Test]
    procedure RemovedTarget_DecFromInt_Unmeasured()
    var
        Src: Record "NB Src";
        Tgt: Record "NB T DecFromInt";
    begin
        Source(Src);
        Tgt.TransferFields(Src);
        Expect(Tgt."No.", 'A');
    end;

    [Test]
    procedure RemovedTarget_ExtensionField_Default()
    var
        Src: Record "NB Src";
        Tgt: Record "NB T Plain";
    begin
        Source(Src);
        Tgt.TransferFields(Src);
        Expect(Tgt."No.", 'A');
    end;

    [Test]
    procedure RemovedSource_IntToInt_Default()
    var
        Src: Record "NB S Removed";
        Tgt: Record "NB T LiveInt";
    begin
        Src."No." := 'A';
        Tgt.F2 := 8;
        Tgt.F5 := 7;
        Tgt.TransferFields(Src);
        Expect(Format(Tgt.F5), '7');
        Expect(Format(Tgt.F2), '8');
    end;

    [Test]
    procedure RemovedSource_BoolToInt_Skip()
    var
        Src: Record "NB S Removed";
        Tgt: Record "NB T LiveInt";
    begin
        Src."No." := 'A';
        Tgt.F2 := 8;
        Tgt.F5 := 7;
        Tgt.TransferFields(Src, true, true);
        Expect(Format(Tgt.F2), '8');
        Expect(Format(Tgt.F5), '7');
    end;

    [Test]
    procedure RemovedSource_DecToDec_Unmeasured()
    var
        Src: Record "NB S Removed";
        Tgt: Record "NB T LiveDec";
    begin
        Src."No." := 'A';
        Tgt.F6 := 7;
        Tgt.TransferFields(Src);
        Expect(Format(Tgt.F6), '7');
    end;

    [Test]
    procedure RemovedSource_BoolToDate_Skip_Unmeasured()
    var
        Src: Record "NB S Removed";
        Tgt: Record "NB T LiveDate";
    begin
        Src."No." := 'A';
        Tgt.F7 := 20240131D;
        Tgt.TransferFields(Src, true, true);
        Expect(Format(Tgt.F7, 0, 9), '2024-01-31');
    end;

    [Test]
    procedure LiveControl_BoolToCode_Default()
    var
        Src: Record "NB Src";
        Tgt: Record "NB T LiveCode";
    begin
        Source(Src);
        asserterror Tgt.TransferFields(Src);
        Expect(GetLastErrorCode(), 'DB:TypeMisMatch');
    end;
}
