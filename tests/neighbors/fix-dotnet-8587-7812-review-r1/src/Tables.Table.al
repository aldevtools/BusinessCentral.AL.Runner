table 50320 "Review R1 Buffer"
{
    TableType = Temporary;

    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Data; Blob) { }
        field(3; At; DateTime) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}

codeunit 50320 "Review R1 Help"
{
    procedure Ch(Code: Integer): Text
    var
        C: Char;
    begin
        C := Code;
        exit(Format(C));
    end;

    procedure Stream(Json: Text; var Buf: Record "Review R1 Buffer" temporary; var InS: InStream)
    var
        OutS: OutStream;
    begin
        Buf.Data.CreateOutStream(OutS);
        OutS.WriteText(Json);
        Buf.Data.CreateInStream(InS);
    end;
}
