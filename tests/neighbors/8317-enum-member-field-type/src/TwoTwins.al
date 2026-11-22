// Usings that find two twins, never the field's enum.
namespace NX.TwoTwins;

using NX.Data;
using NX.Twin;
using NX.TwinB;

codeunit 50104 "NX Two Twins Reader"
{
    procedure LocalOrdinal(): Integer
    var
        Line: Record "NX Line";
    begin
        Line.Kind := Line.Kind::Production;
        exit(Line.Kind.AsInteger());
    end;
}
