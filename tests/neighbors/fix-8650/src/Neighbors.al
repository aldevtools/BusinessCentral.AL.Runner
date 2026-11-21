// Expressions next to al-runner issue 8650 (one of 32 keywords unquoted after `.` or `::` in a property or source expression
// does not compile) that the AL compiler takes: keywords quoted, Key and Local unquoted, and Exit unquoted in TableRelation,
// CalcFormula and an xmlport fieldelement source, which reject only some of the words.
table 50000 "NB KP Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; "If"; Integer) { }
        field(3; "Exit"; Integer) { }
        field(4; "Key"; Integer) { }
        field(5; Local; Integer) { }
        field(6; Opt; Option) { OptionMembers = Zero,"Do"; }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 50001 "NB KP Ref"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Ref; Integer)
        {
            TableRelation = "NB KP Row".Exit;
        }
        field(3; Total; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = sum("NB KP Row".Exit);
        }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

page 50002 "NB KP Card"
{
    PageType = Card;
    SourceTable = "NB KP Row";
    DataCaptionExpression = Format(Rec."If");

    layout
    {
        area(Content)
        {
            group(General)
            {
                Visible = Rec.Key >= 0;

                field(IfShown; Rec."If" + Rec.Key)
                {
                    Visible = Rec."Exit" > 0;
                    Editable = Rec.Local > 0;
                }
                field(OptShown; Rec.Opt)
                {
                    Visible = Rec.Opt = Rec.Opt::"Do";
                    CaptionClass = Format(Rec."Exit");
                }
            }
        }
    }
    actions
    {
        area(Processing)
        {
            action(Act)
            {
                Visible = Rec."If" > 0;
                Enabled = Rec.Local > 0;

                trigger OnAction()
                begin
                end;
            }
        }
    }
}

report 50003 "NB KP Report"
{
    ProcessingOnly = true;

    dataset
    {
        dataitem(Row; "NB KP Row")
        {
            column(IfValue; Row."If")
            {
            }
            column(KeyValue; Row.Key)
            {
            }
        }
    }
}

xmlport 50004 "NB KP Port"
{
    schema
    {
        textelement(Root)
        {
            tableelement(Row; "NB KP Row")
            {
                fieldelement(ExitValue; Row.Exit)
                {
                }
            }
        }
    }
}

codeunit 50005 "NB KP Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure PageExpressions_QuotedKeywordsAndPlainWords_ReadTheRow()
    var
        Row: Record "NB KP Row";
        Card: TestPage "NB KP Card";
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row."If" := 4;
        Row."Exit" := 1;
        Row.Key := 2;
        Row.Local := 3;
        Row.Opt := Row.Opt::"Do";
        Row.Insert();
        Card.OpenEdit();
        Card.GoToRecord(Row);
        Check(Card.IfShown.Value, '6');
        Check(Card.OptShown.Value, 'Do');
        Card.Close();
    end;

    [Test]
    procedure CalcFormula_UnquotedExit_SumsTheRows()
    var
        Row: Record "NB KP Row";
        Ref: Record "NB KP Ref";
    begin
        Row.DeleteAll();
        Ref.DeleteAll();
        Row.Id := 1;
        Row."Exit" := 5;
        Row.Insert();
        Row.Id := 2;
        Row."Exit" := 7;
        Row.Insert();
        Ref.Id := 1;
        Ref.Insert();
        Ref.CalcFields(Total);
        Check(Format(Ref.Total), '12');
    end;

    [Test]
    procedure ReportAndXmlPort_KeywordMembers_Compile()
    begin
        Check(Format(Report::"NB KP Report"), '50003');
        Check(Format(XmlPort::"NB KP Port"), '50004');
    end;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('Expected %1, got %2.', Expected, Actual);
    end;
}
