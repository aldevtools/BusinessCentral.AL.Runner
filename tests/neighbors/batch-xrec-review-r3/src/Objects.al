// Copilot round of runner PR 9328 (train 9390) and runner issue 9380: shapes of one xRec across a nested Validate whose buffer
// provenance changed in a way no cell of corpus record/validate/XRecNestedValidateShapes.al measures. The trigger of "Item No."
// sets Series := 'MID', may assign xRec.Series := 'OUT', may write the whole record or insert it, then nests Validate(Mode)
// through two procedures of the table. Mode's trigger logs its xRec. Each test expects main's answer: the nested Validate gets
// a copy of the outer before-image, so its xRec does not see 'OUT'.
table 50300 "NBR Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Line No."; Integer) { }
        field(2; Mode; Option)
        {
            OptionMembers = Purchase,Production;

            trigger OnValidate()
            var
                Cfg: Codeunit "NBR Cfg";
            begin
                if Cfg.Get() <> '' then
                    Cfg.Add('T x=' + Cfg.Fmt(xRec));
            end;
        }
        field(3; Series; Code[20]) { }
        field(4; "Item No."; Code[20])
        {
            trigger OnValidate()
            var
                Cfg: Codeunit "NBR Cfg";
                Other: Record "NBR Line";
                RecRef: RecordRef;
            begin
                if Cfg.Get() = '' then
                    exit;
                Cfg.Add('I x=' + Cfg.Fmt(xRec));
                Series := 'MID';
                xRec.Series := 'OUT';
                if Cfg.Get() = 'settable' then begin
                    Other := Rec;
                    RecRef.GetTable(Other);
                    RecRef.SetTable(Rec);
                end;
                if Cfg.Get() = 'reinsert' then begin
                    "Line No." := 2;
                    Insert(false);
                end;
                CopyFromItem();
                Cfg.Add('I after x=' + Cfg.Fmt(xRec));
            end;
        }
    }

    keys
    {
        key(PK; "Line No.") { Clustered = true; }
    }

    local procedure CopyFromItem()
    begin
        UpdateMode();
    end;

    local procedure UpdateMode()
    begin
        Validate(Mode, Mode::Production);
    end;
}

page 50301 "NBR Card"
{
    PageType = Card;
    SourceTable = "NBR Line";

    layout
    {
        area(Content)
        {
            field(Series; Rec.Series) { }
        }
    }
}

codeunit 50302 "NBR Cfg"
{
    SingleInstance = true;

    var
        CurrentMode: Text;
        Log: Text;

    procedure Set(NewMode: Text)
    begin
        CurrentMode := NewMode;
    end;

    procedure Get(): Text
    begin
        exit(CurrentMode);
    end;

    procedure Add(Value: Text)
    begin
        Log += Value + ';';
    end;

    procedure Take(): Text
    var
        Result: Text;
    begin
        Result := Log;
        Log := '';
        CurrentMode := '';
        exit(Result);
    end;

    procedure Fmt(Line: Record "NBR Line"): Text
    begin
        exit(Format(Line.Mode, 0, 2) + '/' + Line.Series + '/' + Line."Item No.");
    end;

    // The buffer of the probe: inserted by the record itself, Series changed after.
    procedure AfterInsert(var Line: Record "NBR Line")
    begin
        Line.DeleteAll();
        Line.Init();
        Line."Line No." := 1;
        Line.Mode := Line.Mode::Purchase;
        Line.Series := 'OLD';
        Line."Item No." := 'I1';
        Line.Insert(false);
        Line.Series := 'NEW';
    end;

    procedure Check(Expected: Text)
    var
        Got: Text;
    begin
        Got := Take();
        if Got <> Expected then
            Error('log %1', Got);
    end;
}
