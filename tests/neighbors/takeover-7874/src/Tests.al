// RecordRef calls on a table whose filter group 1 another Record set. The metadata calls run through the assumption
// `recordref-metadata-global-filter` (corpus PR 1525); the others keep main's refusal.
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
        Once: Codeunit "NB Glob Once";
        i: Integer;
    begin
        // The filter stands for the whole session; a second SetRange would refuse, so only the first test sets it.
        if Once.Done() then
            exit;
        Once.MarkDone();
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
    procedure RefCount_Reader()
    var
        RR: RecordRef;
    begin
        SetGlobal();
        RR.Open(50100);
        Expect(Format(RR.Count()), '2');
    end;

    [Test]
    procedure RefNumber()
    var
        RR: RecordRef;
    begin
        SetGlobal();
        RR.Open(50100);
        Expect(Format(RR.Number()), '50100');
    end;

    [Test]
    procedure RefName()
    var
        RR: RecordRef;
    begin
        SetGlobal();
        RR.Open(50100);
        Expect(RR.Name(), 'NB Glob Row');
    end;

    [Test]
    procedure RefCaption()
    var
        RR: RecordRef;
    begin
        SetGlobal();
        RR.Open(50100);
        Expect(RR.Caption(), 'NB Glob Caption');
    end;

    [Test]
    procedure RefFieldCount()
    var
        RR: RecordRef;
    begin
        SetGlobal();
        RR.Open(50100);
        Expect(Format(RR.FieldCount()), '2');
    end;

    [Test]
    procedure RefFieldExist()
    var
        RR: RecordRef;
    begin
        SetGlobal();
        RR.Open(50100);
        Expect(Format(RR.FieldExist(2)), 'Yes');
    end;

    [Test]
    procedure RefIsTemporary()
    var
        RR: RecordRef;
    begin
        SetGlobal();
        RR.Open(50100);
        Expect(Format(RR.IsTemporary()), 'No');
    end;

    [Test]
    procedure RefKeyCount()
    var
        RR: RecordRef;
    begin
        SetGlobal();
        RR.Open(50100);
        if RR.KeyCount() < 1 then
            Error('no keys');
    end;

    [Test]
    procedure RefClose()
    var
        RR: RecordRef;
    begin
        SetGlobal();
        RR.Open(50100);
        RR.Close();
    end;

    [Test]
    procedure RefFindFirst_Reader()
    var
        RR: RecordRef;
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
    begin
        SetGlobal();
        RR.Open(50100);
        Expect(Format(RR.IsEmpty()), 'No');
    end;

    [Test]
    procedure RefRecordId_BufferOnly()
    var
        RR: RecordRef;
    begin
        SetGlobal();
        RR.Open(50100);
        Expect(Format(RR.RecordId()), 'NB Glob Row: 0');
    end;

    [Test]
    procedure RefFieldExist_NumberNotMeasured()
    var
        RR: RecordRef;
    begin
        SetGlobal();
        RR.Open(50100);
        Expect(Format(RR.FieldExist(1)), 'Yes');
    end;

    [Test]
    procedure RefCurrentCompany_NotMeasured()
    var
        RR: RecordRef;
    begin
        SetGlobal();
        RR.Open(50100);
        Expect(RR.CurrentCompany(), CompanyName());
    end;

    [Test]
    procedure RefName_InGroupTwo()
    var
        RR: RecordRef;
    begin
        SetGlobal();
        RR.Open(50100);
        RR.FilterGroup(2);
        Expect(RR.Name(), 'NB Glob Row');
    end;

    [Test]
    procedure RefName_TemporaryRecordRef()
    var
        RR: RecordRef;
    begin
        SetGlobal();
        RR.Open(50100, true);
        Expect(RR.Name(), 'NB Glob Row');
    end;

    [Test]
    procedure RefName_WithFilterOfItsOwn()
    var
        RR: RecordRef;
    begin
        SetGlobal();
        RR.Open(50100);
        RR.Field(1).SetRange(1);
        Expect(RR.Name(), 'NB Glob Row');
    end;

    [Test]
    procedure RefKeyIndexAndField_Measured()
    var
        RR: RecordRef;
    begin
        SetGlobal();
        RR.Open(50100);
        Expect(Format(RR.KeyIndex(1).FieldCount()), '1');
        Expect(Format(RR.FieldIndex(1).Number()), '1');
        Expect(Format(RR.CurrentKeyIndex()), '1');
    end;

    [Test]
    procedure RefReadPermission_Measured()
    var
        RR: RecordRef;
    begin
        SetGlobal();
        RR.Open(50100);
        Expect(Format(RR.ReadPermission()), 'Yes');
        Expect(Format(RR.SecurityFiltering()), 'Validated');
    end;

    [Test]
    procedure RefRecordId_AfterFindFirst()
    var
        RR: RecordRef;
    begin
        SetGlobal();
        RR.Open(50100);
        RR.FindFirst();
        Expect(Format(RR.RecordId()), 'NB Glob Row: 1');
    end;

    [Test]
    procedure RefName_AfterClose()
    var
        RR: RecordRef;
        s: Text;
    begin
        SetGlobal();
        RR.Open(50100);
        RR.Close();
        s := RR.Name();
    end;
}
