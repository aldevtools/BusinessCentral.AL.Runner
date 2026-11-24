// Neighbors of al-runner issue 8986: xRec in the triggers of a table extension of a table in another app.
// Each test picks one trigger and one way to reach xRec (codeunit "NX Mode"); the trigger logs what it read.
codeunit 50350 "NX Mode"
{
    SingleInstance = true;

    var
        TriggerName: Text;
        How: Text;
        Log: Text;

    procedure Set(NewTrigger: Text; NewHow: Text)
    begin
        TriggerName := NewTrigger;
        How := NewHow;
        Log := '';
    end;

    procedure Active(Name: Text): Boolean
    begin
        exit(TriggerName = Name);
    end;

    procedure How(): Text
    begin
        exit(How);
    end;

    procedure Add(Line: Text)
    begin
        Log += Line;
    end;

    procedure Take(): Text
    begin
        exit(Log);
    end;
}

codeunit 50351 "NX Holder"
{
    SingleInstance = true;

    var
        Kept: Record "NX Line";
        KeptVariant: Variant;

    procedure Keep(Line: Record "NX Line")
    begin
        Kept := Line;
    end;

    procedure KeepVariant(V: Variant)
    begin
        KeptVariant := V;
    end;

    procedure KeptSerial(): Integer
    begin
        exit(Kept."Serial No.");
    end;

    procedure KeptVariantSerial(): Integer
    var
        Line: Record "NX Line";
    begin
        Line := KeptVariant;
        exit(Line."Serial No.");
    end;
}

codeunit 50352 "NX Var Caller"
{
    procedure ViaVarRec(var Line: Record "NX Line"): Integer
    begin
        exit(Line.NxBaseXSerial());
    end;
}

codeunit 50353 "NX Steps"
{
    var
        Mode: Codeunit "NX Mode";

    procedure Clean()
    var
        Line: Record "NX Line";
    begin
        Line.DeleteAll();
    end;

    procedure InsertAfterValidate(var Line: Record "NX Line")
    begin
        Clean();
        Clear(Line);
        Line."Entry No." := 1;
        Line.Validate("Serial No.", 42);
        Line.Insert(true);
    end;

    procedure InsertAfterAssign(var Line: Record "NX Line")
    begin
        Clean();
        Clear(Line);
        Line."Entry No." := 1;
        Line."Serial No." := 42;
        Line.Insert(true);
    end;

    procedure Expect(Expected: Text)
    begin
        if Mode.Take() <> Expected then
            Error('logged %1, expected %2', Mode.Take(), Expected);
    end;

    procedure Seed()
    var
        Line: Record "NX Line";
    begin
        Line.DeleteAll();
        Line."Entry No." := 1;
        Line."Serial No." := 42;
        Line.Insert(false);
    end;
}

tableextension 50350 "NX Line Ext" extends "NX Line"
{
    trigger OnBeforeInsert()
    begin
        Step('BeforeInsert');
    end;

    trigger OnInsert()
    begin
        Step('Insert');
    end;

    trigger OnAfterInsert()
    var
        Mode: Codeunit "NX Mode";
    begin
        // The reporter's read, straight in the trigger.
        if Mode.Active('AfterInsert') and (Mode.How() = 'direct') then begin
            Mode.Add(Format(xRec."Serial No."));
            exit;
        end;
        Step('AfterInsert');
    end;

    trigger OnBeforeDelete()
    begin
        Step('BeforeDelete');
    end;

    trigger OnDelete()
    begin
        Step('Delete');
    end;

    trigger OnAfterDelete()
    begin
        Step('AfterDelete');
    end;

    trigger OnModify()
    begin
        Step('Modify');
    end;

    trigger OnAfterModify()
    begin
        Step('AfterModify');
    end;

    procedure NxExtXSerial(): Integer
    begin
        exit(xRec."Serial No.");
    end;

    local procedure LocalXSerial(): Integer
    begin
        exit(xRec."Serial No.");
    end;

    local procedure SerialOf(Line: Record "NX Line"): Integer
    begin
        exit(Line."Serial No.");
    end;

    local procedure Step(Name: Text)
    var
        Mode: Codeunit "NX Mode";
        Holder: Codeunit "NX Holder";
        VarCaller: Codeunit "NX Var Caller";
        Other: Record "NX Line";
        RecRef: RecordRef;
    begin
        if not Mode.Active(Name) then
            exit;
        case Mode.How() of
            'direct':
                Mode.Add(Format(xRec."Serial No."));
            'none':
                Mode.Add(Format(Rec."Serial No."));
            'local':
                Mode.Add(Format(LocalXSerial()));
            'extproc':
                Mode.Add(Format(Rec.NxExtXSerial()));
            'baseproc':
                Mode.Add(Format(Rec.NxBaseXSerial()));
            'varcaller':
                Mode.Add(Format(VarCaller.ViaVarRec(Rec)));
            'byvalue':
                Mode.Add(Format(SerialOf(xRec)));
            'holder':
                Holder.Keep(xRec);
            'variant':
                Holder.KeepVariant(xRec);
            'copy':
                begin
                    Other.Copy(xRec);
                    Mode.Add(Format(Other."Serial No."));
                end;
            'transfer':
                begin
                    Other.TransferFields(xRec);
                    Mode.Add(Format(Other."Serial No."));
                end;
            'recref':
                begin
                    RecRef.GetTable(xRec);
                    Mode.Add(Format(RecRef.Field(2).Value));
                end;
            'format':
                Mode.Add(Format(xRec));
            'getfilters':
                Mode.Add('[' + xRec.GetFilters() + ']');
            'get':
                begin
                    xRec.Get(Rec."Entry No.");
                    Mode.Add(Format(xRec."Serial No."));
                end;
            'write':
                begin
                    xRec."Serial No." := 7;
                    Mode.Add('w');
                end;
            'assign':
                begin
                    xRec := Rec;
                    Mode.Add(Format(xRec."Serial No."));
                end;
        end;
    end;
}

page 50350 "NX Line Card"
{
    PageType = Card;
    SourceTable = "NX Line";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field(EntryNo; Rec."Entry No.") { ApplicationArea = All; }
            field(SerialNo; Rec."Serial No.") { ApplicationArea = All; }
        }
    }
}
