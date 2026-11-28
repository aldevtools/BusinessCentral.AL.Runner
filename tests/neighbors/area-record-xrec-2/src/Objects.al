// The table of record/validate/XRecTableProcThroughCodeunit.al in small: Value's OnValidate and the write triggers dispatch on
// Mode; the procedures write what they read of xRec into Seen, the triggers what they read after the call into After.
table 50400 "XR2 Item"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Value; Integer)
        {
            trigger OnValidate()
            var
                Mover: Codeunit "XR2 Mover";
            begin
                case Mode of
                    1:
                        CallMover();
                    2:
                        Mover.ReadXRec(Rec);
                end;
            end;
        }
        field(3; Mode; Integer) { }
        field(4; Seen; Integer) { }
        field(5; After; Integer) { }
        field(6; Extra; Integer) { }
        field(7; Other; Integer)
        {
            trigger OnValidate()
            begin
                Rec.Seen := xRec.Value + 1000;
            end;
        }
    }
    keys { key(PK; "No.") { Clustered = true; } }

    trigger OnInsert()
    begin
        RunWrite();
    end;

    trigger OnModify()
    begin
        if Mode = 17 then
            Rec.ReadXRec();
        RunWrite();
    end;

    trigger OnDelete()
    begin
        RunWrite();
    end;

    trigger OnRename()
    begin
        RunWrite();
    end;

    local procedure RunWrite()
    var
        Mover: Codeunit "XR2 Mover";
        SingleMover: Codeunit "XR2 Single Mover";
    begin
        case Mode of
            11:
                CallMover();
            12:
                Rec.ReadXRec();
            13:
                begin
                    Rec.AssignXRec();
                    Rec.After := xRec.Value;
                end;
            14:
                begin
                    Rec.AssignThenReadXRec();
                    Rec.After := xRec.Value;
                end;
            15:
                Mover.ReadXRec(Rec);
            16:
                begin
                    CallMoverAssign();
                    Rec.After := xRec.Value;
                end;
            18:
                SingleMover.ReadXRec(Rec);
        end;
    end;

    procedure ReadXRec()
    begin
        Rec.Seen := xRec.Value;
    end;

    procedure AssignXRec()
    begin
        xRec.Value := 99;
    end;

    procedure AssignThenReadXRec()
    begin
        xRec.Value := 99;
        Rec.Seen := xRec.Value;
    end;

    procedure AssignWholeXRec()
    begin
        xRec := Rec;
    end;

    local procedure CallMover()
    var
        Mover: Codeunit "XR2 Mover";
    begin
        Mover.ReadXRec(Rec);
    end;

    local procedure CallMoverAssign()
    var
        Mover: Codeunit "XR2 Mover";
    begin
        Mover.AssignWholeXRec(Rec);
    end;
}

tableextension 50401 "XR2 Item Ext" extends "XR2 Item"
{
    trigger OnBeforeModify()
    var
        Mover: Codeunit "XR2 Mover";
    begin
        case Mode of
            21:
                Rec.ReadXRec();
            22:
                Mover.ReadXRec(Rec);
            23:
                ExtCallsReadXRec();
        end;
    end;

    procedure ExtCallsReadXRec()
    begin
        Rec.ReadXRec();
    end;
}

codeunit 50402 "XR2 Mover"
{
    procedure ReadXRec(var Item: Record "XR2 Item")
    begin
        Item.ReadXRec();
    end;

    procedure AssignWholeXRec(var Item: Record "XR2 Item")
    begin
        Item.AssignWholeXRec();
    end;
}

codeunit 50403 "XR2 Single Mover"
{
    SingleInstance = true;

    procedure ReadXRec(var Item: Record "XR2 Item")
    begin
        Item.ReadXRec();
    end;
}

// Code that is no test codeunit writing the record.
codeunit 50404 "XR2 Helper"
{
    procedure ModifyWithTrigger(var Item: Record "XR2 Item")
    begin
        Item.Modify(true);
    end;

    procedure FieldRefValidate(var Item: Record "XR2 Item")
    var
        RecRef: RecordRef;
        FldRef: FieldRef;
    begin
        RecRef.GetTable(Item);
        FldRef := RecRef.Field(2);
        FldRef.Validate(15);
        RecRef.SetTable(Item);
    end;
}

page 50405 "XR2 Item Card"
{
    PageType = Card;
    SourceTable = "XR2 Item";

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { }
            field(Value; Rec.Value) { }
            field(Seen; Rec.Seen) { }
        }
    }
}
