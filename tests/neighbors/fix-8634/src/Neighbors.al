// Section names next to al-runner issue 8634 (a keyword as the unquoted name of a page control, action, enum value, column,
// data item, xmlport element, key or field group does not compile) that the AL compiler takes: keywords quoted, and words
// that are no keyword in that place (If, Exit, Local, Keys) unquoted.
table 50000 "NB KS Row"
{
    fields
    {
        field(1; Id; Integer)
        {
        }
        field(2; Amount; Integer)
        {
        }
        field(3; Quantity; Integer)
        {
        }
    }
    keys
    {
        key(PK; Id)
        {
            Clustered = true;
        }
        key("Key"; Amount)
        {
        }
    }
    fieldgroups
    {
        fieldgroup("Var"; Id, Amount)
        {
        }
    }
}

page 50001 "NB KS Card"
{
    PageType = Card;
    SourceTable = "NB KS Row";

    layout
    {
        area(Content)
        {
            field(If; Rec.Amount)
            {
            }
            field(Exit; Rec.Quantity)
            {
            }
            field(Local; Rec.Id)
            {
            }
            field(Keys; Rec.Amount)
            {
            }
            field("Key"; Rec.Quantity)
            {
            }
            group("Var")
            {
                field("Area"; Rec.Id)
                {
                }
            }
        }
    }
    actions
    {
        area(Processing)
        {
            action("Event")
            {
                trigger OnAction()
                begin
                    Rec.Amount += 1;
                end;
            }
        }
    }
}

pageextension 50002 "NB KS Card Ext" extends "NB KS Card"
{
    layout
    {
        addafter("Key")
        {
            field("End"; Rec.Amount)
            {
            }
        }
    }
    actions
    {
        addafter("Event")
        {
            action("True")
            {
                trigger OnAction()
                begin
                    Rec.Quantity += 1;
                end;
            }
        }
    }
}

enum 50003 "NB KS Value"
{
    Extensible = true;

    value(0; "Key")
    {
    }
    value(1; Exit)
    {
    }
}

enumextension 50004 "NB KS Value Ext" extends "NB KS Value"
{
    value(50000; "Area")
    {
    }
}

query 50005 "NB KS Query"
{
    elements
    {
        dataitem("Area"; "NB KS Row")
        {
            column("Key"; Amount)
            {
            }
            column(If; Id)
            {
            }
            filter("End"; Quantity)
            {
            }
        }
    }
}

report 50006 "NB KS Report"
{
    ProcessingOnly = true;

    dataset
    {
        dataitem("Begin"; "NB KS Row")
        {
            column("Key"; Amount)
            {
            }
        }
    }
}

xmlport 50007 "NB KS Port"
{
    schema
    {
        textelement(Root)
        {
            tableelement("Key"; "NB KS Row")
            {
                textattribute("Of")
                {
                }
                fieldelement("Area"; "Key".Amount)
                {
                }
            }
        }
    }
}

codeunit 50008 "NB KS Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure PageControls_QuotedKeywordsAndPlainWords_ReadTheRow()
    var
        Row: Record "NB KS Row";
        Card: TestPage "NB KS Card";
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Amount := 7;
        Row.Quantity := 9;
        Row.Insert();
        Card.OpenView();
        Card.GoToRecord(Row);
        Check(Card."If".Value, '7');
        Check(Card."Exit".Value, '9');
        Check(Card."Local".Value, '1');
        Check(Card."Keys".Value, '7');
        Check(Card."Key".Value, '9');
        Check(Card."Area".Value, '1');
        Card.Close();
    end;

    [Test]
    procedure PageExtension_QuotedKeywordControlAndAnchor_ReadTheRow()
    var
        Row: Record "NB KS Row";
        Card: TestPage "NB KS Card";
    begin
        Row.DeleteAll();
        Row.Id := 2;
        Row.Amount := 11;
        Row.Insert();
        Card.OpenView();
        Card.GoToRecord(Row);
        Check(Card."End".Value, '11');
        Card.Close();
    end;

    [Test]
    procedure Enum_QuotedKeywordValues_FormatAndOrdinal()
    var
        Value: Enum "NB KS Value";
    begin
        Value := Value::"Key";
        Check(Format(Value), 'Key');
        Value := Value::"Exit";
        Check(Format(Value.AsInteger()), '1');
        Value := Value::"Area";
        Check(Format(Value), 'Area');
        Check(Format(Value.AsInteger()), '50000');
    end;

    [Test]
    procedure Query_QuotedKeywordColumnsAndFilter_Read()
    var
        Row: Record "NB KS Row";
        Q: Query "NB KS Query";
        Total: Integer;
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Amount := 5;
        Row.Quantity := 1;
        Row.Insert();
        Row.Id := 2;
        Row.Amount := 6;
        Row.Quantity := 2;
        Row.Insert();
        Q.SetRange("End", 2);
        Q.Open();
        while Q.Read() do
            Total += Q."Key" + Q."If";
        Q.Close();
        Check(Format(Total), '8');
    end;

    [Test]
    procedure TableKey_QuotedKeywordName_SortsByItsField()
    var
        Row: Record "NB KS Row";
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Amount := 30;
        Row.Insert();
        Row.Id := 2;
        Row.Amount := 10;
        Row.Insert();
        Row.SetCurrentKey(Amount);
        Row.FindFirst();
        Check(Format(Row.Id), '2');
    end;

    [Test]
    procedure ReportAndXmlPort_QuotedKeywordNames_Compile()
    begin
        Check(Format(Report::"NB KS Report"), '50006');
        Check(Format(XmlPort::"NB KS Port"), '50007');
    end;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('Expected %1, got %2.', Expected, Actual);
    end;
}
