// Controls next to al-runner issue 8660 (a field control bound to no field whose Visible names a field of the record reads
// Yes) that keep the value their Visible had when the page opened, before its first row: controls bound to a field of the
// source table, and Visible expressions that name no field. A BC 29.1 container gave these values.
table 50000 "NB TV Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Integer) { }
        field(3; Flag; Boolean) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

page 50001 "NB TV Card"
{
    PageType = Card;
    SourceTable = "NB TV Row";

    layout
    {
        area(Content)
        {
            field(FieldOwn; Rec.Amount) { Visible = Rec.Amount > 0; }
            field(FieldOther; Rec.Id) { Visible = Rec.Amount > 0; }
            field(FieldFlag; Rec.Flag) { Visible = Rec.Flag; }
            field(FieldOtherFlag; Rec.Amount) { Visible = Rec.Flag; }
            field(ExprVariable; Rec.Amount + 1) { Visible = ShowIt; }
            field(ExprNotVariable; Rec.Amount + 1) { Visible = not ShowIt; }
            field(ExprConstant; Rec.Amount + 1) { Visible = 1 = 0; }
            field(VarVariable; Doubled) { Visible = ShowIt; }
        }
    }

    var
        ShowIt: Boolean;
        Doubled: Integer;

    trigger OnAfterGetRecord()
    begin
        ShowIt := Rec.Amount > 0;
        Doubled := Rec.Amount * 2;
    end;
}

codeunit 50002 "NB TV Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure FieldSource_VisibleNamingItsField_OpenTimeNo()
    var
        Card: TestPage "NB TV Card";
    begin
        Open(Card);
        Check(Card.FieldOwn.Visible(), false);
    end;

    [Test]
    procedure FieldSource_VisibleNamingAnotherField_OpenTimeNo()
    var
        Card: TestPage "NB TV Card";
    begin
        Open(Card);
        Check(Card.FieldOther.Visible(), false);
    end;

    [Test]
    procedure BooleanFieldSource_VisibleIsTheField_OpenTimeNo()
    var
        Card: TestPage "NB TV Card";
    begin
        Open(Card);
        Check(Card.FieldFlag.Visible(), false);
        Check(Card.FieldOtherFlag.Visible(), false);
    end;

    [Test]
    procedure ExpressionSource_VisibleNamingAVariable_OpenTimeValue()
    var
        Card: TestPage "NB TV Card";
    begin
        Open(Card);
        Check(Card.ExprVariable.Visible(), false);
        Check(Card.ExprNotVariable.Visible(), true);
    end;

    [Test]
    procedure ExpressionSource_ConstantVisible_No()
    var
        Card: TestPage "NB TV Card";
    begin
        Open(Card);
        Check(Card.ExprConstant.Visible(), false);
    end;

    [Test]
    procedure VariableSource_VisibleNamingAVariable_OpenTimeNo()
    var
        Card: TestPage "NB TV Card";
    begin
        Open(Card);
        Check(Card.VarVariable.Visible(), false);
    end;

    local procedure Open(var Card: TestPage "NB TV Card")
    var
        Row: Record "NB TV Row";
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Amount := 1;
        Row.Flag := false;
        Row.Insert();
        Card.OpenEdit();
        Card.GoToRecord(Row);
    end;

    local procedure Check(Actual: Boolean; Expected: Boolean)
    begin
        if Actual <> Expected then
            Error('Visible is %1, expected %2.', Actual, Expected);
    end;
}
