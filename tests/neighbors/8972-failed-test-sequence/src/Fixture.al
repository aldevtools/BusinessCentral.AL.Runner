table 52700 "N72 Row"
{
    fields { field(1; "Key"; Code[30]) { } }
    keys { key(PK; "Key") { Clustered = true; } }
}

codeunit 52701 "N72 Access"
{
    procedure Put(RowKey: Code[30])
    var
        Row: Record "N72 Row";
    begin
        Row."Key" := RowKey;
        Row.Insert();
    end;

    procedure ReadFirst()
    var
        Row: Record "N72 Row";
    begin
        if Row.FindFirst() then;
    end;

    procedure Seq(Name: Text): Text
    begin
        if not NumberSequence.Exists(Name) then
            exit('missing');
        exit('next=' + Format(NumberSequence.Next(Name)));
    end;
}

codeunit 52702 "N72 Run Writes"
{
    trigger OnRun()
    var
        Access: Codeunit "N72 Access";
    begin
        Access.Put('RUN-W');
    end;
}

codeunit 52703 "N72 Run Reads"
{
    trigger OnRun()
    var
        Access: Codeunit "N72 Access";
    begin
        Access.ReadFirst();
    end;
}
