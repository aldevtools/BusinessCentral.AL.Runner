// Field names next to al-runner issue 8630 (a keyword as an unquoted field name does not compile) that the AL compiler takes:
// keywords quoted, and words that are no keyword in that place, unquoted, in a table and in a table extension.
table 50000 "NB KF Row"
{
    fields
    {
        field(1; "No."; Code[20])
        {
        }
        field(2; "Area"; Integer)
        {
        }
        field(3; "Key"; Integer)
        {
        }
        field(4; "Begin"; Integer)
        {
        }
        field(5; Local; Integer)
        {
        }
        field(6; If; Integer)
        {
        }
        field(7; Exit; Integer)
        {
        }
        field(8; Keys; Integer)
        {
        }
        field(9; Fields; Integer)
        {
        }
        field(10; Layout; Integer)
        {
        }
    }
    keys
    {
        key(PK; "No.")
        {
            Clustered = true;
        }
    }
}

tableextension 50001 "NB KF Ext" extends "NB KF Row"
{
    fields
    {
        field(50000; "Event"; Integer)
        {
        }
        field(50001; Value; Integer)
        {
        }
    }
}

codeunit 50002 "NB KF Tests"
{
    Subtype = Test;

    [Test]
    procedure QuotedKeywordFieldNames()
    var
        Row: Record "NB KF Row";
    begin
        Row."No." := 'A';
        Row."Area" := 1;
        Row."Key" := 2;
        Row."Begin" := 3;
        Row.Insert();
        Row.Get('A');
        if Row."Area" + Row."Key" + Row."Begin" <> 6 then
            Error('got %1', Row."Area" + Row."Key" + Row."Begin");
    end;

    [Test]
    procedure OtherWordsAsUnquotedFieldNames()
    var
        Row: Record "NB KF Row";
    begin
        Row."No." := 'B';
        Row.Local := 1;
        Row."If" := 2;
        Row."Exit" := 3;
        Row.Keys := 4;
        Row.Fields := 5;
        Row.Layout := 6;
        Row.Insert();
        Row.Get('B');
        if Row.Local + Row."If" + Row."Exit" + Row.Keys + Row.Fields + Row.Layout <> 21 then
            Error('got %1', Row.Local + Row."If" + Row."Exit" + Row.Keys + Row.Fields + Row.Layout);
    end;

    [Test]
    procedure TableExtensionFieldNames()
    var
        Row: Record "NB KF Row";
    begin
        Row."No." := 'C';
        Row."Event" := 7;
        Row.Value := 8;
        Row.Insert();
        Row.Get('C');
        if Row."Event" + Row.Value <> 15 then
            Error('got %1', Row."Event" + Row.Value);
    end;
}