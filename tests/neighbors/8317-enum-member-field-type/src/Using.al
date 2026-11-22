// A `using` of the twin's namespace, none of the field's enum's.
namespace NX.Using;

using NX.Data;
using NX.Twin;

codeunit 50102 "NX Using Reader"
{
    procedure LocalOrdinal(): Integer
    var
        Line: Record "NX Line";
    begin
        Line.Kind := Line.Kind::Production;
        exit(Line.Kind.AsInteger());
    end;
}
