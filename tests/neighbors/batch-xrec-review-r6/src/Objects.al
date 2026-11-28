// Review round 5 of runner PR 9328, findings 2 and 3: a guarded Insert that answers false (the key exists), in the outer
// trigger or before the outer Validate, and Page.SetSelectionFilter into the buffer before the outer Validate. No cell of corpus
// record/validate/XRecNestedValidateShapes.al measures them. The trigger of "Item No." sets Series := 'MID', assigns
// xRec.Series := 'OUT', may try an Insert, then nests Validate(Mode) through two procedures of the table. Mode's trigger
// logs its xRec. Each test expects main's answer: the nested Validate gets a copy of the outer before-image.
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
                if Cfg.Get() = 'failinsert' then
                    if not Insert(false) then;
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
    PageType = List;
    SourceTable = "NBR Line";

    layout
    {
        area(Content)
        {
            field(Series; Rec.Series) { }
        }
    }

    procedure Pick(var Line: Record "NBR Line")
    begin
        CurrPage.SetSelectionFilter(Line);
    end;
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
