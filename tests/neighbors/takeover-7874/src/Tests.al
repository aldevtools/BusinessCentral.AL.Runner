// RecordRef calls on a table whose filter group 1 another Record set (reporter's PR: metadata answers regardless).
codeunit 50100 "NB Glob Tests"
{
    Subtype = Test;

    local procedure Expect(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('Expected %1, got %2', Expected, Actual);
    end;

    local procedure SetGlobal()
    var
        R: Record "NB Glob Row";
        i: Integer;
    begin
        R.DeleteAll();
        for i := 1 to 4 do begin
            R.Id := i;
            R.Qty := i mod 2;
            R.Insert();
        end;
        R.FilterGroup(1);
        R.SetRange(Qty, 1);
        R.FilterGroup(0);
    end;

    [Test]
    procedure Control_NoGlobalFilter_Number()
    var
        RR: RecordRef;
    begin
        RR.Open(50101);
        Expect(Format(RR.Number()), '50101');
    end;

    [Test]
    procedure RefNumberName()
    var
        RR: RecordRef;
        R: Record "NB Glob Row";
        s: Text;
    begin
        SetGlobal();
        RR.Open(50100);
        Expect(Format(RR.Number()) + ' ' + RR.Name(), '50100 NB Glob Row');
    end;

    [Test]
    procedure RefCaption()
    var
        RR: RecordRef;
        R: Record "NB Glob Row";
        s: Text;
    begin
        SetGlobal();
        RR.Open(50100);
        Expect(RR.Caption(), 'NB Glob Caption');
    end;

    [Test]
    procedure RefFieldCount()
    var
        RR: RecordRef;
        R: Record "NB Glob Row";
        s: Text;
    begin
        SetGlobal();
        RR.Open(50100);
        Expect(Format(RR.FieldCount()), '2');
    end;

    [Test]
    procedure RefFieldExist()
    var
        RR: RecordRef;
        R: Record "NB Glob Row";
        s: Text;
    begin
        SetGlobal();
        RR.Open(50100);
        Expect(Format(RR.FieldExist(2)), 'Yes');
    end;

    [Test]
    procedure RefIsTemporary()
    var
        RR: RecordRef;
        R: Record "NB Glob Row";
        s: Text;
    begin
        SetGlobal();
        RR.Open(50100);
        Expect(Format(RR.IsTemporary()), 'No');
    end;

    [Test]
    procedure RefKeyCount()
    var
        RR: RecordRef;
        R: Record "NB Glob Row";
        s: Text;
    begin
        SetGlobal();
        RR.Open(50100);
        Expect(Format(RR.KeyCount()), '1');
    end;

    [Test]
    procedure RefClose()
    var
        RR: RecordRef;
        R: Record "NB Glob Row";
        s: Text;
    begin
        SetGlobal();
        RR.Open(50100);
        RR.Close();
    end;

    [Test]
    procedure RefCount_Reader()
    var
        RR: RecordRef;
        R: Record "NB Glob Row";
        s: Text;
    begin
        SetGlobal();
        RR.Open(50100);
        Expect(Format(RR.Count()), '4');
    end;

    [Test]
    procedure RefFindFirst_Reader()
    var
        RR: RecordRef;
        R: Record "NB Glob Row";
        s: Text;
    begin
        SetGlobal();
        RR.Open(50100);
        if RR.FindFirst() then s := Format(RR.Field(1).Value());
        Expect(s, '1');
    end;

    [Test]
    procedure RefIsEmpty_Reader()
    var
        RR: RecordRef;
        R: Record "NB Glob Row";
        s: Text;
    begin
        SetGlobal();
        RR.Open(50100);
        Expect(Format(RR.IsEmpty()), 'No');
    end;

    [Test]
    procedure RefRecordId_BufferOnly()
    var
        RR: RecordRef;
        R: Record "NB Glob Row";
        s: Text;
    begin
        SetGlobal();
        RR.Open(50100);
        Expect(Format(RR.RecordId()), '');
    end;
}
