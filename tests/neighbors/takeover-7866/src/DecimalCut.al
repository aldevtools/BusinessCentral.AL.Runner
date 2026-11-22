// The shapes next to the Decimal storage cells of open corpus pull request 1427 (record/crud/RcAGetFind.al:749-842):
// a Decimal field with one secondary key of its own, written by Record.Insert() and Record.Modify(), filtered by
// Record.SetRange and Record.SetFilter('<%1' / '>%1') and read by Record.Count(). Measured_ tests run through
// decimal-column-cuts-twenty-places; Refused_ tests refuse on purpose; Main_ tests keep main's result.
table 50760 "DC Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Decimal) { }
    }
    keys { key(PK; Id) { Clustered = true; } key(ByAmount; Amount) { } }
}
table 50761 "DC Plain"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Decimal) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
table 50762 "DC Key"
{
    fields
    {
        field(1; Amount; Decimal) { }
    }
    keys { key(PK; Amount) { Clustered = true; } }
}
table 50763 "DC Unique"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Decimal) { }
    }
    keys { key(PK; Id) { Clustered = true; } key(UniqueAmount; Amount) { Unique = true; } }
}
table 50764 "DC Two"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Decimal) { }
        field(3; Other; Integer) { }
    }
    keys { key(PK; Id) { Clustered = true; } key(ByBoth; Amount, Other) { } }
}
codeunit 50765 "DC Tests"
{
    Subtype = Test;

    local procedure Full(): Decimal
    begin
        exit(1000 / 3600000);
    end;

    local procedure Cut(): Decimal
    begin
        exit(0.00027777777777777777);
    end;

    local procedure Check(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('got %1, want %2', Got, Want);
    end;

    local procedure Seed(var R: Record "DC Row")
    begin
        R.DeleteAll();
        R.Id := 1;
        R.Amount := Full();
        R.Insert();
    end;

    [Test]
    procedure Measured_Insert_GetReadsCut()
    var R: Record "DC Row"; G: Record "DC Row";
    begin
        Seed(R);
        G.Get(1);
        Check(Format(G.Amount = Cut()) + Format(R.Amount = Full()), 'YesYes');
    end;

    [Test]
    procedure Measured_Modify_GetReadsCut()
    var R: Record "DC Row"; G: Record "DC Row";
    begin
        R.DeleteAll();
        R.Id := 1;
        R.Insert();
        R.Amount := Full();
        R.Modify();
        G.Get(1);
        Check(Format(G.Amount = Cut()), 'Yes');
    end;

    [Test]
    procedure Measured_Count_SetRangeFull()
    var R: Record "DC Row";
    begin
        Seed(R);
        R.SetRange(Amount, Full());
        Check(Format(R.Count()), '1');
    end;

    [Test]
    procedure Measured_Count_SetFilterLessFull()
    var R: Record "DC Row";
    begin
        Seed(R);
        R.SetFilter(Amount, '<%1', Full());
        Check(Format(R.Count()), '0');
    end;

    [Test]
    procedure Measured_Count_RangeCutFull()
    var R: Record "DC Row";
    begin
        Seed(R);
        R.SetRange(Amount, Cut(), Full());
        Check(Format(R.Count()), '1');
    end;

    [Test]
    procedure Assumed_Count_OnAssignedCopy()
    var R: Record "DC Row"; C: Record "DC Row";
    begin
        Seed(R);
        R.SetRange(Amount, Full());
        C := R;
        Check(Format(C.Count()), '1');
    end;

    [Test]
    procedure Main_Count_ValueTheColumnHolds()
    var R: Record "DC Row";
    begin
        Seed(R);
        R.SetRange(Amount, 0.5);
        Check(Format(R.Count()), '0');
    end;

    [Test]
    procedure Main_PlainField_KeepsValue()
    var R: Record "DC Plain"; G: Record "DC Plain";
    begin
        R.DeleteAll();
        R.Id := 1;
        R.Amount := Full();
        R.Insert();
        G.Get(1);
        G.SetRange(Amount, Full());
        Check(Format(G.Amount = Full()) + Format(G.Count()), 'Yes1');
    end;

    [Test]
    procedure Main_DecimalPrimaryKey_GetFull()
    var R: Record "DC Key"; G: Record "DC Key";
    begin
        R.DeleteAll();
        R.Amount := Full();
        R.Insert();
        Check(Format(G.Get(Full())), 'Yes');
    end;

    [Test]
    procedure Main_UniqueKey_SecondInsert()
    var R: Record "DC Unique";
    begin
        R.DeleteAll();
        R.Id := 1;
        R.Amount := 0.00000000000000000001;
        R.Insert();
        R.Id := 2;
        R.Amount := 0.000000000000000000019;
        Check(Format(R.Insert()), 'Yes');
    end;

    [Test]
    procedure Main_TwoFieldKey_KeepsValue()
    var R: Record "DC Two"; G: Record "DC Two";
    begin
        R.DeleteAll();
        R.Id := 1;
        R.Amount := Full();
        R.Insert();
        G.Get(1);
        Check(Format(G.Amount = Full()), 'Yes');
    end;

    [Test]
    procedure Main_Temporary_KeepsValue()
    var R: Record "DC Row" temporary;
    begin
        R.Id := 1;
        R.Amount := Full();
        R.Insert();
        R.Get(1);
        R.SetRange(Amount, Full());
        Check(Format(R.Amount = Full()) + Format(R.FindFirst()), 'YesYes');
    end;

    [Test]
    procedure Refused_InsertTrue()
    var R: Record "DC Row";
    begin
        R.DeleteAll();
        R.Id := 1;
        R.Amount := Full();
        R.Insert(true);
    end;

    [Test]
    procedure Refused_ModifyTrue()
    var R: Record "DC Row";
    begin
        R.DeleteAll();
        R.Id := 1;
        R.Insert();
        R.Amount := Full();
        R.Modify(true);
    end;

    [Test]
    procedure Refused_ModifyAll()
    var R: Record "DC Row";
    begin
        R.DeleteAll();
        R.Id := 1;
        R.Insert();
        R.ModifyAll(Amount, Full());
    end;

    [Test]
    procedure Refused_Rename()
    var R: Record "DC Row";
    begin
        R.DeleteAll();
        R.Id := 1;
        R.Insert();
        R.Amount := Full();
        R.Rename(2);
    end;

    [Test]
    procedure Refused_RecordRefInsert()
    var RR: RecordRef;
    begin
        RR.Open(Database::"DC Row");
        RR.DeleteAll();
        RR.Field(1).Value := 1;
        RR.Field(2).Value := Full();
        RR.Insert();
    end;

    [Test]
    procedure Refused_FindFirst()
    var R: Record "DC Row";
    begin
        Seed(R);
        R.SetRange(Amount, Full());
        Check(Format(R.FindFirst()), 'Yes');
    end;

    [Test]
    procedure Refused_IsEmpty()
    var R: Record "DC Row";
    begin
        Seed(R);
        R.SetRange(Amount, Full());
        Check(Format(R.IsEmpty()), 'No');
    end;

    [Test]
    procedure Refused_CalcSums()
    var R: Record "DC Row";
    begin
        Seed(R);
        R.SetRange(Amount, Full());
        R.CalcSums(Amount);
    end;

    [Test]
    procedure Refused_DeleteAll()
    var R: Record "DC Row";
    begin
        Seed(R);
        R.SetRange(Amount, Full());
        R.DeleteAll();
    end;

    [Test]
    procedure Refused_FieldRefSetRange()
    var R: Record "DC Row"; RR: RecordRef; FR: FieldRef;
    begin
        Seed(R);
        RR.Open(Database::"DC Row");
        FR := RR.Field(2);
        FR.SetRange(Full());
        Check(Format(RR.Count()), '1');
    end;

    [Test]
    procedure Refused_SetFilterEqualPlaceholder()
    var R: Record "DC Row";
    begin
        Seed(R);
        R.SetFilter(Amount, '%1', Full());
        Check(Format(R.Count()), '1');
    end;

    [Test]
    procedure Refused_SetFilterLessOrEqual()
    var R: Record "DC Row";
    begin
        Seed(R);
        R.SetFilter(Amount, '<=%1', Full());
        Check(Format(R.Count()), '1');
    end;

    [Test]
    procedure Refused_CopyFilters()
    var R: Record "DC Row"; C: Record "DC Row";
    begin
        Seed(R);
        R.SetRange(Amount, Full());
        C.CopyFilters(R);
        Check(Format(C.Count()), '1');
    end;

    [Test]
    procedure Refused_FilterGroup2()
    var R: Record "DC Row";
    begin
        Seed(R);
        R.FilterGroup(2);
        R.SetRange(Amount, Full());
        R.FilterGroup(0);
        Check(Format(R.Count()), '1');
    end;
}
