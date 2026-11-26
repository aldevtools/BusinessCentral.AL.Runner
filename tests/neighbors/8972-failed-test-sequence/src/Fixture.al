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

codeunit 52704 "N72 Helper"
{
    // The RecordRef calls of GetLastEntryNoFromTable (shapes only): Lock sets UpdLock, else ReadUncommitted.
    procedure LastNo(Lock: Boolean): Integer
    var
        RR: RecordRef;
        KR: KeyRef;
        FR: FieldRef;
    begin
        RR.Open(Database::"N72 Row");
        if Lock then
            RR.ReadIsolation(IsolationLevel::UpdLock)
        else
            RR.ReadIsolation(IsolationLevel::ReadUncommitted);
        KR := RR.KeyIndex(1);
        RR.SetLoadFields(KR.FieldIndex(KR.FieldCount).Number);
        if RR.FindLast() then begin
            FR := KR.FieldIndex(KR.FieldCount);
            exit(StrLen(Format(FR.Value)));
        end;
        exit(0);
    end;
}

page 52705 "N72 Row Card"
{
    SourceTable = "N72 Row";
    layout { area(Content) { field(RowKey; Rec."Key") { } } }
}
