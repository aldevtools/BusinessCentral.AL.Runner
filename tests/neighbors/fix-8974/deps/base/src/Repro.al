table 50400 "Repro XRec Line"
{
    DataClassification = CustomerContent;
    InherentPermissions = RIMD;
    InherentEntitlements = RIMD;

    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "No."; Code[20])
        {
            trigger OnValidate()
            begin
                "Profile Code" := 'NEW';
            end;
        }
        field(3; "Profile Code"; Code[20]) { }
        field(4; Route; Integer) { }
        field(5; "Seen xRec Profile"; Code[20]) { }
        field(6; "Proc Ran"; Boolean) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }

    procedure OwnProc()
    begin
        "Proc Ran" := true;
        "Seen xRec Profile" := xRec."Profile Code";
    end;
}
