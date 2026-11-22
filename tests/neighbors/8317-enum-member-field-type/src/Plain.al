// A `using` of the field's enum's namespace only: the file finds the field's enum (main's path).
namespace NX.Plain;

using NX.Data;
using NX.Enums;

codeunit 50105 "NX Plain Reader"
{
    procedure LocalOrdinal(): Integer
    var
        Line: Record "NX Line";
    begin
        Line.Kind := Line.Kind::Production;
        exit(Line.Kind.AsInteger());
    end;

    procedure SetProduction(var Line: Record "NX Line")
    begin
        Line.Kind := Line.Kind::Production;
    end;
}
