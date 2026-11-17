table 50300 "Repro Parent"
{
    fields
    {
        field(1; Code; Code[10]) { }
        field(2; "Primary Child No."; Code[10]) { }
    }
    keys { key(PK; Code) { Clustered = true; } }

    trigger OnDelete()
    var
        ReproOther: Record "Repro Other";
    begin
        // no row of Repro Other points to this record
        ReproOther.SetRange("Parent Code", Code);
        ReproOther.ModifyAll("Parent Code", '');
    end;
}

table 50301 "Repro Other"
{
    fields
    {
        field(1; Code; Code[10]) { }
        field(2; "Parent Code"; Code[10]) { }
    }
    keys { key(PK; Code) { Clustered = true; } }
}

permissionset 50300 "Repro Parent Edit"
{
    Assignable = true;
    Permissions = tabledata "Repro Parent" = RIMD, tabledata "Repro Other" = R;
}
