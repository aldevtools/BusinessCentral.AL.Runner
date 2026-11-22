// Neighbors of al-runner issue 8317: `Field::Member` on an Enum field where the file finds another enum by the enum's
// name. Expected values are Business Central's (corpus PR 1442, PR 1480): the field's enum, ordinal 1, in every shape.
// This file uses neither NX.Enums nor NX.Twin, so the search in every namespace finds "NX Kind" three times.
namespace NX.Tests;

using NX.Data;
using NX.GradeData;

// Main refuses a test of this file that names a member through the field, and then every later test of its codeunit:
// each such test has a codeunit of its own.
codeunit 50110 "NX Name Found Three Times"
{
    Subtype = Test;

    local procedure Expect(Actual: Integer; Expected: Integer)
    begin
        if Actual <> Expected then
            Error('Expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure NameFoundThreeTimes_LocalRecord()
    var
        Line: Record "NX Line";
    begin
        Line.Kind := Line.Kind::Production;
        Expect(Line.Kind.AsInteger(), 1);
    end;
}

codeunit 50112 "NX Extension Member"
{
    Subtype = Test;

    local procedure Expect(Actual: Integer; Expected: Integer)
    begin
        if Actual <> Expected then
            Error('Expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure NameFoundThreeTimes_EnumExtensionMember()
    var
        Line: Record "NX Line";
    begin
        Line.Kind := Line.Kind::Recall;
        Expect(Line.Kind.AsInteger(), 30);
    end;
}

codeunit 50111 "NX Field Enum Tests"
{
    Subtype = Test;

    local procedure Expect(Actual: Integer; Expected: Integer)
    begin
        if Actual <> Expected then
            Error('Expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure TwinOwnNamespace_LocalRecord()
    var
        Reader: Codeunit NX.Twin."NX Twin Reader";
    begin
        Expect(Reader.LocalOrdinal(), 1);
    end;

    [Test]
    procedure TwinOwnNamespace_VarParameter()
    var
        Reader: Codeunit NX.Twin."NX Twin Reader";
        Plain: Codeunit NX.Plain."NX Plain Reader";
        Line: Record "NX Line";
    begin
        Plain.SetProduction(Line);
        if not Reader.IsProduction(Line) then
            Error('Expected Production');
    end;

    [Test]
    procedure TwinOwnNamespace_TemporaryRecord()
    var
        Reader: Codeunit NX.Twin."NX Twin Reader";
    begin
        Expect(Reader.TemporaryOrdinal(), 1);
    end;

    [Test]
    procedure TwinOwnNamespace_GlobalRecord()
    var
        Reader: Codeunit NX.Twin."NX Twin Reader";
    begin
        Expect(Reader.GlobalOrdinal(), 1);
    end;

    [Test]
    procedure TwinOwnNamespace_GlobalRecordWithoutMember()
    var
        Reader: Codeunit NX.Twin."NX Twin Reader";
    begin
        Expect(Reader.GlobalReadWithoutMember(), 0);
    end;

    [Test]
    procedure TwinOwnNamespace_GlobalNamespaceFieldEnum_Refused()
    var
        Reader: Codeunit NX.Twin."NX Twin Reader";
    begin
        Expect(Reader.GlobalNamespaceFieldEnumOrdinal(), 1);
    end;

    [Test]
    procedure TwinOwnNamespace_TableExtensionRec()
    var
        Line: Record "NX Line";
    begin
        Expect(Line.RecOrdinal(), 1);
    end;

    [Test]
    procedure TwinOwnNamespace_TableExtensionBareField()
    var
        Line: Record "NX Line";
    begin
        Expect(Line.BareOrdinal(), 1);
    end;

    [Test]
    procedure TwinOwnNamespace_TableExtensionThis_Refused()
    var
        Line: Record "NX Line";
    begin
        Expect(Line.ThisOrdinal(), 1);
    end;

    [Test]
    procedure TwinOwnNamespace_PageRec()
    var
        TwinPage: Page NX.Twin."NX Twin Page";
    begin
        Expect(TwinPage.RecOrdinal(), 1);
    end;

    [Test]
    procedure TwinOwnNamespace_PageBareField()
    var
        TwinPage: Page NX.Twin."NX Twin Page";
    begin
        Expect(TwinPage.BareOrdinal(), 1);
    end;

    [Test]
    procedure TwinOwnNamespace_TableCode_Refused()
    var
        Holder: Record NX.Twin."NX Twin Holder";
    begin
        Expect(Holder.LocalOrdinal(), 1);
    end;

    [Test]
    procedure TwinThroughUsing()
    var
        Reader: Codeunit NX.Using."NX Using Reader";
    begin
        Expect(Reader.LocalOrdinal(), 1);
    end;

    [Test]
    procedure UsingsFindFieldsEnumAndTwin()
    var
        Reader: Codeunit NX.Both."NX Both Reader";
    begin
        Expect(Reader.LocalOrdinal(), 1);
    end;

    [Test]
    procedure UsingsFindTwoTwins()
    var
        Reader: Codeunit NX.TwoTwins."NX Two Twins Reader";
    begin
        Expect(Reader.LocalOrdinal(), 1);
    end;

    [Test]
    procedure FileFindsFieldsEnum_Unchanged()
    var
        Reader: Codeunit NX.Plain."NX Plain Reader";
    begin
        Expect(Reader.LocalOrdinal(), 1);
    end;
}

codeunit 50113 "NX Global Twin"
{
    Subtype = Test;

    local procedure Expect(Actual: Integer; Expected: Integer)
    begin
        if Actual <> Expected then
            Error('Expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure TwinInGlobalNamespace()
    var
        Line: Record "NX Graded Line";
    begin
        Line.Grade := Line.Grade::Production;
        Expect(Line.Grade.AsInteger(), 1);
    end;
}
