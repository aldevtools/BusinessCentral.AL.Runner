// Review round 1: the facts of AtSignAloneMethods.al:507 around group 1 of a temporary record.
table 97600 "AS Review Group1 Row"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; Kind; Code[20]) { }
        field(3; Note; Text[50]) { }
        field(4; Kind Lookup; Code[20])
        {
            FieldClass = FlowField;
            CalcFormula = lookup("AS Review Group1 Row".Kind where("No." = field("No.")));
        }
        field(5; Tag; Integer) { }
    }
    keys { key(PK; "No.") { } }
}

table 97607 "AS Review Group1 Database Row"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; Kind; Code[20]) { }
    }
    keys { key(PK; "No.") { } }
}

table 97608 "AS Review G1 Other Row"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; Kind; Code[20]) { }
        field(3; Tag; Integer) { }
    }
    keys { key(PK; "No.") { } }
}

codeunit 97600 "AS Review Group1 Tests"
{
    Subtype = Test;
    TestIsolation = Function;

    local procedure Fill(var Row: Record "AS Review Group1 Row" temporary)
    begin
        Row."No." := 1;
        Row.Kind := 'K';
        Row.Insert();
    end;

    local procedure ExactSequence(var Row: Record "AS Review Group1 Row" temporary): Text
    var
        Seen: Text;
    begin
        Fill(Row);
        Row.FilterGroup(1);
        Row.SetFilter(Kind, '<>@');
        Row.FilterGroup(0);
        Seen := Format(Row.Count());
        if Row.FindSet() then
            Seen += ':' + Format(Row."No.");
        Row.FilterGroup(1);
        Row.SetRange(Kind);
        Row.SetRange(Note);
        exit(Seen);
    end;

    [Test]
    procedure Group1Temp_ExactSequence()
    var
        Row: Record "AS Review Group1 Row" temporary;
        Assert: Codeunit Assert;
    begin
        Assert.AreEqual('0', ExactSequence(Row), 'temporary <>@ entry, own Count and FindSet, own clear');
    end;

    [Test]
    procedure Group1Temp_FreshClear()
    var
        Row: Record "AS Review Group1 Row" temporary;
    begin
        Fill(Row);
        Row.FilterGroup(1);
        Row.SetRange(Kind);
    end;

    [Test]
    procedure Group1Temp_MainIntegerSequence()
    var
        Row: Record "AS Review Group1 Row" temporary;
    begin
        Row.FilterGroup(1);
        Row.SetRange(Tag, 1);
        Row.SetRange(Tag);
    end;

    [Test]
    procedure Group1Temp_MainIntegerPrimaryKeySequence()
    var
        Row: Record "AS Review Group1 Row" temporary;
    begin
        Row.FilterGroup(1);
        Row.SetRange("No.", 1);
        Row.SetRange("No.");
    end;

    [Test]
    procedure Group1Temp_DatabaseIntegerPrimaryKeyCopy()
    var
        Db: Record "AS Review Group1 Row";
        Temp: Record "AS Review Group1 Row" temporary;
    begin
        Db."No." := 1;
        Db.Insert();
        Db.FilterGroup(1);
        Db.SetRange("No.", 1);
        Temp.Copy(Db);
        Temp.FilterGroup(1);
        Temp.SetRange("No.");
    end;

    [Test]
    procedure Group1Temp_FieldRefSetter()
    var
        Temp: Record "AS Review Group1 Row" temporary;
        R: RecordRef;
        F: FieldRef;
    begin
        Fill(Temp);
        R.GetTable(Temp);
        R.FilterGroup(1);
        F := R.Field(Temp.FieldNo(Kind));
        F.SetFilter('<>@');
    end;

    [Test]
    procedure Group1Temp_FlowFieldClear()
    var
        Temp: Record "AS Review Group1 Row" temporary;
    begin
        Fill(Temp);
        Temp.FilterGroup(1);
        Temp.SetFilter("Kind Lookup", '<>@');
        Temp.SetRange("Kind Lookup");
    end;

    [Test]
    procedure Group1Temp_DatabaseSetterFindSet()
    var
        Db: Record "AS Review Group1 Database Row";
        Temp: Record "AS Review Group1 Database Row" temporary;
    begin
        Db."No." := 1;
        Db.Kind := 'K';
        Db.Insert();
        Temp."No." := 2;
        Temp.Insert();
        Db.FilterGroup(1);
        Db.SetFilter(Kind, '<>@');
        Temp := Db;
        Temp.FilterGroup(0);
        Temp.FindSet();
    end;

    [Test]
    procedure Group1Temp_DifferentFieldClear()
    var Temp: Record "AS Review G1 Other Row" temporary;
    begin
        Temp.FilterGroup(1);
        Temp.SetFilter(Kind, '<>@');
        Temp.SetRange(Tag);
    end;
}
