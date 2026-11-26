// The reporter's base table (al-runner issue 8986), plus a procedure that reads xRec.
table 50300 "NX Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Serial No."; Integer) { }
        field(3; "xRec Serial In Table OnInsert"; Integer) { }
        field(4; Mirror; Integer)
        {
            trigger OnValidate()
            var
                BaseLog: Codeunit "NX Base Log";
            begin
                BaseLog.Set(xRec."Serial No.");
            end;
        }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }

    trigger OnInsert()
    begin
        "xRec Serial In Table OnInsert" := xRec."Serial No.";
    end;

    trigger OnDelete()
    var
        BaseLog: Codeunit "NX Base Log";
    begin
        BaseLog.Set(xRec."Serial No.");
    end;

    procedure NxValidateMirror(V: Integer)
    begin
        Validate(Mirror, V);
    end;

    procedure NxBaseXSerial(): Integer
    begin
        exit(xRec."Serial No.");
    end;
}

// What xRec."Serial No." held in the table's own OnDelete.
codeunit 50301 "NX Base Log"
{
    SingleInstance = true;

    var
        Serial: Integer;

    procedure Set(NewSerial: Integer)
    begin
        Serial := NewSerial;
    end;

    procedure Get(): Integer
    begin
        exit(Serial);
    end;
}
