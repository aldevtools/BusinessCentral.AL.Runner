// Members next to al-runner issue 8635 (one of 32 keywords unquoted after `.` in code does not compile) that the AL compiler
// takes: words that are no keyword there unquoted (Key, Area, Local, Value), keywords quoted, and a namespace segment that is a
// keyword in `using` and in a variable's type.
namespace NeighborsKM.Tests;

using NeighborsKM.If;

codeunit 50001 "NB KM Callee"
{
    procedure "Exit"(): Integer
    begin
        exit(41);
    end;

    procedure "Do"(Value: Integer): Integer
    begin
        exit(Value + 1);
    end;
}

page 50002 "NB KM Card"
{
    PageType = Card;
    SourceTable = "NB KM Row";

    layout
    {
        area(Content)
        {
            field("If"; Rec."If")
            {
                Visible = Rec."This" >= 0;
            }
            field("Key"; Rec.Key)
            {
            }
        }
    }
}

codeunit 50003 "NB KM Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Members_UnquotedNonKeywords_WriteAndRead()
    var
        Row: Record "NB KM Row";
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Key := 2;
        Row.Area := 3;
        Row.Local := 4;
        Row.Value := 5;
        Row.Insert();
        Clear(Row);
        Row.Get(1);
        Check(Format(Row.Key + Row.Area + Row.Local + Row.Value), '14');
    end;

    [Test]
    procedure Members_QuotedKeywords_WriteAndRead()
    var
        Row: Record "NB KM Row";
    begin
        Row.DeleteAll();
        Row.Id := 2;
        Row."If" := 6;
        Row."This" := 7;
        Row."Event" := 8;
        Row.Insert();
        Clear(Row);
        Row.Get(2);
        Check(Format(Row."If" + Row."This" + Row."Event"), '21');
    end;

    [Test]
    procedure Methods_QuotedKeywordNames_Called()
    var
        Callee: Codeunit "NB KM Callee";
    begin
        Check(Format(Callee."Exit"()), '41');
        Check(Format(Callee."Do"(1)), '2');
    end;

    [Test]
    procedure TestPage_QuotedKeywordControl_ReadsTheRow()
    var
        Row: Record "NB KM Row";
        Card: TestPage "NB KM Card";
    begin
        Row.DeleteAll();
        Row.Id := 3;
        Row."If" := 9;
        Row.Key := 10;
        Row.Insert();
        Card.OpenView();
        Card.GoToRecord(Row);
        Check(Card."If".Value, '9');
        Check(Card.Key.Value, '10');
        Card.Close();
    end;

    [Test]
    procedure Namespace_KeywordSegment_InUsingAndType()
    var
        Row: Record NeighborsKM.If."NB KM Row";
    begin
        Row.DeleteAll();
        Row.Id := 4;
        Row.Insert();
        Check(Format(Row.Count()), '1');
    end;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('Expected %1, got %2.', Expected, Actual);
    end;
}
