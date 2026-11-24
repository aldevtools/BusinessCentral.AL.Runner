// The review of #8934 at 03:29Z, just past the cells of corpus PR 1538 :2804 and :2576, where the base page states
// ApplicationArea = All: a page extension's field with ApplicationArea = All on a Card that states none, and a part a page
// extension adds with ApplicationArea = All, each by its extension's id. The runner refuses each. ApplicationArea on a page
// extension object does not compile (AL0246: the property cannot be customized), so no neighbor holds it.
page 50119 "Repro ExtField Plain Card"
{
    PageType = Card;
    SourceTable = "Repro ExtField Row";
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
            }
        }
    }
}

pageextension 50122 "Repro ExtField Plain Ext" extends "Repro ExtField Plain Card"
{
    layout
    {
        addlast(General)
        {
            field(PlainAreaNote; Rec.Note) { ApplicationArea = All; }
        }
    }
}

pageextension 50124 "Repro ExtField Part Ext" extends "Repro ExtField Card"
{
    layout
    {
        addlast(Content)
        {
            part(ExtAreaPart; "Repro ExtField List") { ApplicationArea = All; }
        }
    }
}

codeunit 50135 "Repro ExtField Area Neighbors"
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

    local procedure Outcome(Ok: Boolean; Shown: Text): Text
    begin
        if Ok then
            exit('found ' + Shown);
        exit('error ' + GetLastErrorCode() + ': ' + GetLastErrorText());
    end;

    [TryFunction]
    local procedure TryPlain(var Pg: TestPage "Repro ExtField Plain Card"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    [TryFunction]
    local procedure TryCard(var Pg: TestPage "Repro ExtField Card"; Id: Integer; var Shown: Text)
    begin
        Shown := Pg.GetField(Id).Value();
    end;

    // PlainAreaNote (ApplicationArea = All) of page extension 50122 on the Card 50119, which states no ApplicationArea.
    [Test]
    procedure PlainBaseCardExtensionFieldWithArea()
    var
        Pg: TestPage "Repro ExtField Plain Card";
        Shown: Text;
        Ok: Boolean;
    begin
        Seed();
        Pg.OpenView();
        ClearLastError();
        Ok := TryPlain(Pg, 141332332, Shown);
        Pg.Close();
        Check(Outcome(Ok, Shown), 'found Gamma');
    end;

    // The part ExtAreaPart (ApplicationArea = All) of page extension 50124, by the extension's id.
    [Test]
    procedure ExtensionPartWithArea()
    var
        Pg: TestPage "Repro ExtField Card";
        Shown: Text;
        Ok: Boolean;
    begin
        Seed();
        Pg.OpenView();
        ClearLastError();
        Ok := TryCard(Pg, 1390269562, Shown);
        Pg.Close();
        Check(Outcome(Ok, Shown), 'found ');
    end;
}
