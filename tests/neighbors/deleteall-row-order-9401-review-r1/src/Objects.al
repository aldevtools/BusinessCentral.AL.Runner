// Neighbors of the DeleteAll row order (al-runner issue 9401, corpus PR 1659), review round 1: row code that reads a
// stored row without a record read (a Blob through a FieldRef, a scan of a temporary table, RecordRef.IsDirty, the load
// of a partial record's fields, an XmlPort import, the temporary rows StartSession hands over), that writes the table in
// another company, that writes another table other than by an Insert in the session's company, and a record that called
// ChangeCompany with an empty name. Corpus PR 1659 measures none of these: each refuses with main's text.

table 94111 "DRR Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; Pic; Blob) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }

    trigger OnDelete()
    var
        Ctl: Codeunit "DRR Ctl";
    begin
        Ctl.RowCode(Rec);
    end;
}

// One set of rows for all companies.
table 94112 "DRR Shared"
{
    DataClassification = SystemMetadata;
    DataPerCompany = false;

    fields
    {
        field(1; Id; Integer) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }

    trigger OnDelete()
    var
        Ctl: Codeunit "DRR Ctl";
        Shared: Record "DRR Shared";
    begin
        Ctl.Add('T' + Format(Id));
        if Id = 3 then begin
            Shared.ChangeCompany('OTHER');
            Shared.Id := 0;
            Shared.Insert(false);
        end;
    end;
}

// Another table the row code writes or reads.
table 94113 "DRR Other"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

xmlport 94115 "DRR Export Temp"
{
    Direction = Export;

    schema
    {
        textelement(Root)
        {
            tableelement(Other; "DRR Other")
            {
                UseTemporary = true;
                fieldelement(Id; Other.Id) { }
            }
        }
    }
}

xmlport 94116 "DRR Import"
{
    Direction = Import;

    schema
    {
        textelement(Root)
        {
            tableelement(Other; "DRR Other")
            {
                fieldelement(Id; Other.Id) { }
            }
        }
    }
}

codeunit 94118 "DRR Session"
{
    TableNo = "DRR Other";

    trigger OnRun()
    begin
    end;
}

codeunit 94117 "DRR Ctl"
{
    SingleInstance = true;

    var
        Seen: Text;
        Mode: Text;
        Held: Record "DRR Row";

    procedure SetMode(NewMode: Text)
    begin
        Mode := NewMode;
    end;

    procedure Add(Entry: Text)
    begin
        if Seen <> '' then
            Seen += ',';
        Seen += Entry;
    end;

    procedure Take() Result: Text
    begin
        Result := Seen;
        Seen := '';
        Mode := '';
    end;

    // A partial record of row Id, read before the DeleteAll, that the row code reads on.
    procedure Hold(Id: Integer)
    begin
        Held.SetLoadFields(Id);
        Held.Get(Id);
    end;

    // The OnDelete of "DRR Row": logs the row, then does what the mode asks at the first row deleted (Id 3).
    procedure RowCode(var Row: Record "DRR Row")
    var
        Same: Record "DRR Row";
        Buffer: Record "DRR Row" temporary;
        Other: Record "DRR Other";
        TempOther: Record "DRR Other" temporary;
        RecRef: RecordRef;
        FldRef: FieldRef;
        OutStr: OutStream;
        InStr: InStream;
        SessionId: Integer;
    begin
        Add('T' + Format(Row.Id));
        if Row.Id <> 3 then
            exit;
        case Mode of
            'blob-fieldref':
                begin
                    RecRef.GetTable(Row);
                    FldRef := RecRef.Field(3);
                    FldRef.CalcField();
                end;
            'xmlport-export-temp':
                begin
                    Buffer.Pic.CreateOutStream(OutStr);
                    Xmlport.Export(Xmlport::"DRR Export Temp", OutStr);
                end;
            'xmlport-import':
                begin
                    Buffer.Pic.CreateOutStream(OutStr);
                    OutStr.WriteText('<Root><Other><Id>5</Id></Other></Root>');
                    Buffer.Pic.CreateInStream(InStr);
                    Xmlport.Import(Xmlport::"DRR Import", InStr);
                end;
            'recref-isdirty':
                begin
                    RecRef.GetTable(Row);
                    Add(Format(RecRef.IsDirty()));
                end;
            'held-unloaded-field':
                Add(Held.Name);
            'held-validate':
                Held.Validate(Name, 'x');
            'start-session-temp':
                begin
                    TempOther.Id := 1;
                    StartSession(SessionId, Codeunit::"DRR Session", CompanyName(), TempOther);
                end;
            'insert-same-other-company':
                begin
                    Same.ChangeCompany('OTHER');
                    Same.Id := 0;
                    Same.Insert(false);
                end;
            'delete-other-by-key':
                begin
                    Other.Id := 1;
                    Other.Delete(false);
                end;
            'modify-other-by-key':
                begin
                    Other.Id := 1;
                    Other.Name := 'changed';
                    Other.Modify(false);
                end;
            'rename-other-by-key':
                begin
                    Other.Id := 1;
                    Other.Rename(7);
                end;
            'modifyall-other':
                Other.ModifyAll(Name, 'changed', false);
            'insert-other-other-company':
                begin
                    Other.ChangeCompany('OTHER');
                    Other.Id := 5;
                    Other.Insert(false);
                end;
            'insert-other':
                begin
                    Other.Id := 5;
                    Other.Insert(false);
                end;
            'insert-temp-other':
                begin
                    TempOther.Id := 5;
                    TempOther.Insert(false);
                end;
        end;
    end;
}
