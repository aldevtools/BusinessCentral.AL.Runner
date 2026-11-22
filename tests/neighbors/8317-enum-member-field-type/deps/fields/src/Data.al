namespace NX.Data;

using NX.Enums;

table 50120 "NX Line"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Kind; Enum "NX Kind") { }
        field(3; "G Kind"; Enum "NX GKind") { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}
