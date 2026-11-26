// Written per company by X; copied with its company by Y.
table 53850 "IO9039S Setup"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[30]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

// Written by Y before its copy, in both orders; X does not touch it.
table 53851 "IO9039S Copied Alike"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[30]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}
