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
    procedure Key_SaveRecord_ThenTestPageFilter_ReadsLine()
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
    procedure OptionKey_OneFilter()
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
    procedure OptionKey_OneFilter_ThenTestPageFilter()
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

codeunit 97513 "N9121 Measured Cell Tests"
{
    Subtype = Test;

    [Test]
    procedure Cell_EnumKey_OneFilter()
    var
        P: TestPage "N9121 Enum Cell";
        K: Enum "N9121 Kind";
    begin
        P.OpenEdit();
        P.KindFilter.SetValue(Format(K::C));
        P.Kind.AssertEquals(Format(K::C));
    end;

    [Test]
    procedure EnumKey_TwoFilters()
    var
        P: TestPage "N9121 Enum Cell";
    begin
        P.OpenEdit();
        P.TwoFilters.SetValue(true);
        P."Item No.".AssertEquals('X');
    end;

    [Test]
    procedure Cell_TwoKeyFilters_SecondChanges()
    var
        P: TestPage "N9121 Two Cell";
    begin
        P.OpenEdit();
        P.SalesTypeFilter.SetValue(2);
        P."Sales Type".AssertEquals(2);
        P.Type.AssertEquals(Format(Enum::"N9121 Line Type"::Item));
    end;

    [Test]
    procedure OptionKey_TwoKeyFilters_SecondChanges()
    var
        P: TestPage "N9121 Option Two Cell";
    begin
        P.OpenEdit();
        P.SalesTypeFilter.SetValue(2);
        P."Sales Type".AssertEquals(2);
    end;

    [Test]
    procedure TwoKeyFilters_BothChange()
    var
        P: TestPage "N9121 Two Cell";
    begin
        P.OpenEdit();
        P.BothFilter.SetValue(true);
        P."Sales Type".AssertEquals(2);
        P.Type.AssertEquals(Format(Enum::"N9121 Line Type"::Group));
    end;

    [Test]
    procedure Cell_RecFieldAssignedInValidate()
    var
        P: TestPage "N9121 Two Cell";
    begin
        P.OpenEdit();
        P.AssignFilter.SetValue(true);
        P."Sales Type".AssertEquals(2);
        P."Code".AssertEquals('');
    end;

    [Test]
    procedure EnumKey_TwoKeyFilters_SecondChanges()
    var
        P: TestPage "N9121 Enum Two Cell";
        K: Enum "N9121 Kind";
    begin
        P.OpenEdit();
        P.SalesTypeFilter.SetValue(Format(K::C));
        P."Sales Type".AssertEquals(Format(K::C));
    end;

    [Test]
    procedure Cell_OnAfterGetCurrRecordWritesLog()
    var
        P: TestPage "N9121 Two Cell Log";
        Log: Record "N9121 Log";
    begin
        P.OpenEdit();
        P.SalesTypeFilter.SetValue(2);
        P."Sales Type".AssertEquals(2);
        if Log.Count() < 1 then
            Error('no log row');
    end;

    [Test]
    procedure Cell_OnAfterGetCurrRecordCountsRuns()
    var
        P: TestPage "N9121 Two Cell Counts";
    begin
        P.OpenEdit();
        P.SalesTypeFilter.SetValue(2);
        P."Sales Type".AssertEquals(2);
        P.Runs.AssertEquals(P.Runs.AsInteger());
    end;
}
