// Namespace NX.Twin declares enums named like the enums of NX Line.Kind and NX Line."G Kind", with Production at 5.
// Every reader here finds the twin by the name; Business Central reads the field's enum (corpus PR 1442, PR 1480).
namespace NX.Twin;

using NX.Data;

enum 50100 "NX Kind"
{
    value(0; " ") { }
    value(5; Production) { }
}

enum 50101 "NX GKind"
{
    value(0; " ") { }
    value(5; Production) { }
}

codeunit 50100 "NX Twin Reader"
{
    var
        GlobalLine: Record "NX Line";

    procedure LocalOrdinal(): Integer
    var
        Line: Record "NX Line";
    begin
        Line.Kind := Line.Kind::Production;
        exit(Line.Kind.AsInteger());
    end;

    procedure IsProduction(var Line: Record "NX Line"): Boolean
    begin
        exit(Line.Kind = Line.Kind::Production);
    end;

    procedure TemporaryOrdinal(): Integer
    var
        Line: Record "NX Line" temporary;
    begin
        Line.Kind := Line.Kind::Production;
        exit(Line.Kind.AsInteger());
    end;

    procedure GlobalOrdinal(): Integer
    begin
        GlobalLine.Kind := GlobalLine.Kind::Production;
        exit(GlobalLine.Kind.AsInteger());
    end;

    procedure GlobalReadWithoutMember(): Integer
    begin
        GlobalLine.Init();
        exit(GlobalLine.Kind.AsInteger());
    end;

    procedure GlobalNamespaceFieldEnumOrdinal(): Integer
    var
        Line: Record "NX Line";
    begin
        Line."G Kind" := Line."G Kind"::Production;
        exit(Line."G Kind".AsInteger());
    end;
}

tableextension 50100 "NX Twin Line Ext" extends "NX Line"
{
    procedure RecOrdinal(): Integer
    begin
        Rec.Kind := Rec.Kind::Production;
        exit(Rec.Kind.AsInteger());
    end;

    procedure BareOrdinal(): Integer
    begin
        Kind := Kind::Production;
        exit(Kind.AsInteger());
    end;

    procedure ThisOrdinal(): Integer
    begin
        this.Kind := this.Kind::Production;
        exit(this.Kind.AsInteger());
    end;
}

page 50100 "NX Twin Page"
{
    PageType = Card;
    SourceTable = "NX Line";

    layout
    {
        area(Content)
        {
            field(Id; Rec.Id) { ApplicationArea = All; }
        }
    }

    procedure RecOrdinal(): Integer
    begin
        Rec.Kind := Rec.Kind::Production;
        exit(Rec.Kind.AsInteger());
    end;

    procedure BareOrdinal(): Integer
    begin
        Kind := Kind::Production;
        exit(Kind.AsInteger());
    end;
}

table 50101 "NX Twin Holder"
{
    fields
    {
        field(1; Id; Integer) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }

    procedure LocalOrdinal(): Integer
    var
        Line: Record "NX Line";
    begin
        Line.Kind := Line.Kind::Production;
        exit(Line.Kind.AsInteger());
    end;
}
