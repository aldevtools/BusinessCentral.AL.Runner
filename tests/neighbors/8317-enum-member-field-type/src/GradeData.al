namespace NX.GradeData;

using NX.Grades;

table 50102 "NX Graded Line"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Grade; Enum "NX Grade") { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}
