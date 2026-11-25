codeunit 97510 "N9121 Tests"
{
    Subtype = Test;

    [Test]
    procedure NonKey_FilterSetInValidate()
    var
        P: TestPage "N9121 Lines";
    begin
        P.OpenEdit();
        P.KindFilter.SetValue(2);
        P.Kind.AssertEquals(2);
    end;

    [Test]
    procedure NonKey_TestPageFilter()
    var
        P: TestPage "N9121 Lines";
        L: Record "N9121 Line";
    begin
        P.OpenEdit();
        P.Filter.SetFilter(Kind, Format(L.Kind::D));
        P.Kind.AssertEquals(Format(L.Kind::D));
    end;

    [Test]
    procedure Key_SaveRecordThenSetRange_InValidate()
    var
        P: TestPage "N9121 Keyed Save";
    begin
        P.OpenEdit();
        P.KindFilter.SetValue(2);
        P.Kind.AssertEquals(2);
    end;

    [Test]
    procedure Key_SetRange_InValidate()
    var
        P: TestPage "N9121 Keyed Plain";
    begin
        P.OpenEdit();
        P.KindFilter.SetValue(2);
        P.Kind.AssertEquals(2);
    end;

    [Test]
    procedure Key_RangeFilter_InValidate()
    var
        P: TestPage "N9121 Keyed Plain";
    begin
        P.OpenEdit();
        P.RangeFilter.SetValue(true);
        P.Kind.AssertEquals(0);
    end;

    [Test]
    procedure Key_TwoValueFilter_InValidate()
    var
        P: TestPage "N9121 Keyed Plain";
    begin
        P.OpenEdit();
        P.TwoValues.SetValue(true);
        P.Kind.AssertEquals(0);
    end;

    [Test]
    procedure Key_TestPageFilter()
    var
        P: TestPage "N9121 Keyed Plain";
        K: Record "N9121 Keyed";
    begin
        P.OpenEdit();
        P.Filter.SetFilter(Kind, Format(K.Kind::D));
        P.Kind.AssertEquals(Format(K.Kind::D));
    end;

    [Test]
    procedure Key_SaveRecord_ThenTestPageFilter_PageVariable()
    var
        P: TestPage "N9121 Keyed Save";
        K: Record "N9121 Keyed";
    begin
        P.OpenEdit();
        P.KindFilter.SetValue(1);
        P.Filter.SetFilter(Kind, Format(K.Kind::D));
        P.Kind.AssertEquals(Format(K.Kind::D));
    end;
}

codeunit 97511 "N9121 Cell Tests"
{
    Subtype = Test;

    [Test]
    procedure Cell_SaveRecordThenSetRange_InValidate()
    var
        P: TestPage "N9121 Keyed Cell";
    begin
        P.OpenEdit();
        P.KindFilter.SetValue(2);
        P.Kind.AssertEquals(2);
    end;

    [Test]
    procedure Cell_SameValue_InValidate()
    var
        P: TestPage "N9121 Keyed Cell";
    begin
        P.OpenEdit();
        P.KindFilter.SetValue(0);
        P.Kind.AssertEquals(0);
    end;

    [Test]
    procedure Cell_SetValueThenTestPageFilter()
    var
        P: TestPage "N9121 Keyed Cell";
        K: Record "N9121 Keyed";
    begin
        P.OpenEdit();
        P.KindFilter.SetValue(1);
        P.Filter.SetFilter(Kind, Format(K.Kind::D));
        P.Kind.AssertEquals(Format(K.Kind::D));
    end;
}

codeunit 97512 "N9121 Function Control Tests"
{
    Subtype = Test;

    [Test]
    procedure HiddenFunctionControl_AfterTestPageFilter()
    var
        P: TestPage "N9121 Keyed Plain";
        K: Record "N9121 Keyed";
    begin
        P.OpenEdit();
        P.Filter.SetFilter(Kind, Format(K.Kind::D));
        P.KindFilter.AssertEquals(Format(K.Kind::D));
    end;

}
