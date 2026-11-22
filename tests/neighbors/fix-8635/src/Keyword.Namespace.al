// A namespace segment that is a keyword compiles in the namespace declaration (al-runner issue 8635).
namespace NeighborsKM.If;

table 50000 "NB KM Row"
{
    fields
    {
        field(1; Id; Integer)
        {
        }
        field(2; "Key"; Integer)
        {
        }
        field(3; "Area"; Integer)
        {
        }
        field(4; Local; Integer)
        {
        }
        field(5; Value; Integer)
        {
        }
        field(6; "If"; Integer)
        {
        }
        field(7; "This"; Integer)
        {
        }
        field(8; "Event"; Integer)
        {
        }
    }
    keys
    {
        key(PK; Id)
        {
            Clustered = true;
        }
    }
}
