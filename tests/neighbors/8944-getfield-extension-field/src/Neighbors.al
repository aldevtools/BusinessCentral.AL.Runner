// Neighbors just past the cells of corpus PR 1538 :2804 (a Card's extension field with ApplicationArea = All, found by the
// extension's id) and :2576 (the same on a Worksheet): the same field on a List, one with another ApplicationArea, a hidden one,
// and Caption through GetField, on the repro's Card (al-runner issue 8944). The runner refuses each.
page 50118 "Repro ExtField List"
{
    PageType = List;
    SourceTable = "Repro ExtField Row";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { }
                field(Name; Rec.Name) { }
            }
        }
    }
}

pageextension 50120 "Repro ExtField List Ext" extends "Repro ExtField List"
{
    layout
    {
        addlast(Lines)
        {
            field(AreaNote; Rec.Note) { ApplicationArea = All; }
        }
    }
}

pageextension 50121 "Repro ExtField Card Ext2" extends "Repro ExtField Card"
{
    layout
    {
        addlast(General)
        {
            field(SuiteNote; Rec.Note) { ApplicationArea = Suite; }
            field(HiddenNote; Rec.Name)
            {
                ApplicationArea = All;
                Visible = false;
            }
        }
    }
}

codeunit 50132 "Repro ExtField Neighbors"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure Seed()
    var
        Row: Record "Repro ExtField Row";
    begin
        Row.DeleteAll();
        Row."No." := 'R1';
        Row.Name := 'Alpha';
        Row.Note := 'Gamma';
        Row.Insert();
    end;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('<%1>, expected <%2>', Actual, Expected);
    end;

    [TryFunction]
    local procedure TryListValue(var Pg: TestPage "Repro ExtField List"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    [TryFunction]
    local procedure TryCardValue(var Pg: TestPage "Repro ExtField Card"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    [TryFunction]
    local procedure TryCardCaption(var Pg: TestPage "Repro ExtField Card"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Caption();
    end;

    local procedure Outcome(Ok: Boolean; Shown: Text): Text
    begin
        if Ok then
            exit('found ' + Shown);
        exit('error ' + GetLastErrorCode() + ': ' + GetLastErrorText());
    end;

    // AreaNote of page extension 50120 on the List, by the extension's id.
    [Test]
    procedure ListExtensionFieldWithArea()
    var
        Pg: TestPage "Repro ExtField List";
        Shown: Text;
        Ok: Boolean;
    begin
        Seed();
        Pg.OpenView();
        Pg.First();
        ClearLastError();
        Ok := TryListValue(Pg, 1924745308, Shown);
        Pg.Close();
        Check(Outcome(Ok, Shown), 'found Gamma');
    end;

    // SuiteNote of page extension 50121 (ApplicationArea = Suite) on the Card, by the extension's id.
    [Test]
    procedure CardExtensionFieldSuiteArea()
    var
        Pg: TestPage "Repro ExtField Card";
        Shown: Text;
        Ok: Boolean;
    begin
        Seed();
        Pg.OpenView();
        ClearLastError();
        Ok := TryCardValue(Pg, 1005793524, Shown);
        Pg.Close();
        Check(Outcome(Ok, Shown), 'found Gamma');
    end;

    // HiddenNote of page extension 50121 (ApplicationArea = All, Visible = false) on the Card, by the extension's id.
    [Test]
    procedure CardExtensionFieldHidden()
    var
        Pg: TestPage "Repro ExtField Card";
        Shown: Text;
        Ok: Boolean;
    begin
        Seed();
        Pg.OpenView();
        ClearLastError();
        Ok := TryCardValue(Pg, 1019479344, Shown);
        Pg.Close();
        Check(Outcome(Ok, Shown), 'error TestFieldNotFound: The field with ID = 1019479344 is not found on the page.');
    end;

    // Caption of the repro's ExtNote (page extension 50117) through GetField by the extension's id.
    [Test]
    procedure CardExtensionFieldCaption()
    var
        Pg: TestPage "Repro ExtField Card";
        Shown: Text;
        Ok: Boolean;
    begin
        Seed();
        Pg.OpenView();
        ClearLastError();
        Ok := TryCardCaption(Pg, 962137182, Shown);
        Pg.Close();
        Check(Outcome(Ok, Shown), 'found Note');
    end;
}
