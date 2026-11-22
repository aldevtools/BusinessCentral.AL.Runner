// Usings that find the field's enum and the twin.
namespace NX.Both;

using NX.Data;
using NX.Enums;
using NX.Twin;

codeunit 50103 "NX Both Reader"
{
    procedure LocalOrdinal(): Integer
    var
        Line: Record "NX Line";
    begin
        Line.Kind := Line.Kind::Production;
        exit(Line.Kind.AsInteger());
    end;
}
