// Page controls whose source is a Record method without parentheses (reporter's PR: `Rec.Count`).
codeunit 50100 "NB Cnt Tests"
{
    Subtype = Test;

    local procedure Expect(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('Expected %1, got %2', Expected, Actual);
    end;

    local procedure Seed()
    var
        R: Record "NB Cnt Row";
        i: Integer;
    begin
        R.DeleteAll();
        for i := 1 to 3 do begin
            R.Id := i;
            R.Insert();
        end;
    end;

    [Test]
    procedure RecCount_Control()
    var
        P: TestPage "NB Cnt Count";
    begin
        Seed();
        P.OpenView();
        P.First();
        Expect(P.Total.Value(), '3');
    end;

    [Test]
    procedure RecCountApprox_Control()
    var
        P: TestPage "NB Cnt CountApprox";
    begin
        Seed();
        P.OpenView();
        P.First();
        Expect(P.Total.Value(), '3');
    end;

    [Test]
    procedure RecGetFilters_Control()
    var
        P: TestPage "NB Cnt GetFilters";
    begin
        Seed();
        P.OpenView();
        P.First();
        Expect(P.Fl.Value(), '');
    end;

    [Test]
    procedure RecCountParens_Control()
    var
        P: TestPage "NB Cnt Count Parens";
    begin
        Seed();
        P.OpenView();
        P.First();
        Expect(P.Total.Value(), '3');
    end;

    [Test]
    procedure RecCount_TableHasFieldNamedCount()
    var
        R: Record "NB Cnt Field Row";
        P: TestPage "NB Cnt Field Count";
    begin
        R.DeleteAll();
        R.Id := 1;
        R."Count" := 7;
        R.Insert();
        P.OpenView();
        P.First();
        Expect(P.Total.Value(), '7');
    end;

    [Test]
    procedure RecCount_Control_AfterLast()
    var
        P: TestPage "NB Cnt Count";
    begin
        Seed();
        P.OpenView();
        P.Last();
        Expect(P.Total.Value(), '3');
    end;

    [Test]
    procedure RecCount_Control_AfterNextLast()
    var
        P: TestPage "NB Cnt Count";
    begin
        Seed();
        P.OpenView();
        P.Next();
        P.Last();
        Expect(P.Total.Value(), '3');
    end;

    [Test]
    procedure RecCount_CardControl()
    var
        P: TestPage "NB Cnt Card";
    begin
        Seed();
        P.OpenView();
        P.First();
        Expect(P.Total.Value(), '3');
    end;

    [Test]
    procedure RecCount_WorksheetControl()
    var
        P: TestPage "NB Cnt Worksheet";
    begin
        Seed();
        P.OpenView();
        P.First();
        Expect(P.Total.Value(), '3');
    end;

    [Test]
    procedure RecHasFilter_Control()
    var
        P: TestPage "NB Cnt HasFilter";
    begin
        Seed();
        P.OpenView();
        P.First();
        Expect(P.Total.Value(), 'No');
    end;

    [Test]
    procedure BareCount_Control()
    var
        P: TestPage "NB Cnt Bare Count";
    begin
        Seed();
        P.OpenView();
        P.First();
        Expect(P.Total.Value(), '3');
    end;

    [Test]
    procedure RecCount_TableProcedureCount()
    var
        P: TestPage "NB Cnt Proc Count";
    begin
        Seed();
        P.OpenView();
        P.First();
        Expect(P.Total.Value(), '42');
    end;

    [Test]
    procedure RecCount_PageExtensionControl()
    var
        P: TestPage "NB Cnt Plain";
    begin
        Seed();
        P.OpenView();
        P.First();
        Expect(P.ExtTotal.Value(), '3');
    end;

    [Test]
    procedure RecCount_OtherExpressionControlAfterLast()
    var
        P: TestPage "NB Cnt Other Expr";
    begin
        Seed();
        P.OpenView();
        P.Last();
        Expect(P.Text.Value(), '3');
    end;

    [Test]
    procedure RecCount_OtherExpressionControlAfterFirst()
    var
        P: TestPage "NB Cnt Other Expr";
    begin
        Seed();
        P.OpenView();
        P.First();
        Expect(P.Text.Value(), '1');
    end;
}
