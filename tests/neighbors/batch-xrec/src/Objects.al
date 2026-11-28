// Neighbors of al-runner issue 8312 (one xRec across a Validate the table's own code nests) and of the [Scope('OnPrem')]
// test caller of assumption xrec-table-proc-callers-subscribers (issue 8974). The trigger of "Item No." nests Validate(Mode)
// in the routes below; Mode's trigger may nest Validate(Other) once more. Every cell logs each trigger's xRec ('I', 'T',
// 'O', each also 'after' the nested Validate) as Mode ordinal/Series/"Item No.", and 'P' what a table procedure called by
// the subscriber of an integration event Mode's trigger raises reads of xRec. Each test is a test codeunit of its own, so a
// refusal does not reach the next one. Each test expects what one xRec for both Validates gives (the measured cells of
// corpus record/validate/XRecTableProcThroughSubscriber.al:1042-1083).
table 50000 "NBX Line"
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
                Cfg: Codeunit "NBX Cfg";
            begin
                if Cfg.Get() = '' then
                    exit;
                Cfg.Add('T x=' + Cfg.Fmt(xRec));
                case Cfg.Get() of
                    'inner-assign':
                        xRec.Series := 'INN';
                    'inner-assign-error':
                        begin
                            xRec.Series := 'INN';
                            Error('inner stop');
                        end;
                    'three-level':
                        begin
                            CopyToOther();
                            Cfg.Add('T after x=' + Cfg.Fmt(xRec));
                        end;
                    'outer-filter':
                        Cfg.Add('T filters=' + xRec.GetFilters());
                    'event', 'event-nested':
                        OnModeElse(Rec);
                    'blank-assign':
                        begin
                            OnModeElse(Rec);
                            Cfg.Add('T after x=' + Cfg.Fmt(xRec));
                        end;
                end;
            end;
        }
        field(3; Series; Code[20]) { }
        field(4; "Item No."; Code[20])
        {
            trigger OnValidate()
            var
                Cfg: Codeunit "NBX Cfg";
            begin
                if Cfg.Get() in ['', 'event', 'blank-assign'] then
                    exit;
                if Cfg.Get() = 'same-field-inner' then begin
                    Cfg.Add('J x=' + Cfg.Fmt(xRec));
                    exit;
                end;
                Cfg.Add('I x=' + Cfg.Fmt(xRec));
                Series := 'MID';
                case Cfg.Get() of
                    'outer-assign':
                        begin
                            xRec.Series := 'OUT';
                            CopyFromItem();
                        end;
                    'inner-assign', 'three-level', 'event-nested':
                        CopyFromItem();
                    'inner-assign-error':
                        if not TryCopyFromItem() then;
                    'direct':
                        begin
                            xRec.Series := 'OUT';
                            Validate(Mode, Mode::Production);
                        end;
                    'one-proc':
                        begin
                            xRec.Series := 'OUT';
                            UpdateMode();
                        end;
                    'three-procs':
                        begin
                            xRec.Series := 'OUT';
                            PrepareFromItem();
                        end;
                    'one-arg':
                        begin
                            xRec.Series := 'OUT';
                            CopyFromItemOneArg();
                        end;
                    'rec-dot':
                        begin
                            xRec.Series := 'OUT';
                            Rec.CopyFromItem();
                        end;
                    'same-field':
                        begin
                            Cfg.Set('same-field-inner');
                            xRec.Series := 'OUT';
                            RevalidateItem();
                        end;
                    'outer-filter':
                        begin
                            xRec.SetRange(Series, 'F');
                            CopyFromItem();
                        end;
                end;
                Cfg.Add('I after x=' + Cfg.Fmt(xRec));
            end;
        }
        field(5; Other; Integer)
        {
            trigger OnValidate()
            var
                Cfg: Codeunit "NBX Cfg";
            begin
                if Cfg.Get() <> 'three-level' then
                    exit;
                Cfg.Add('O x=' + Cfg.Fmt(xRec));
                xRec.Series := 'DEEP';
            end;
        }
    }

    keys
    {
        key(PK; "Line No.") { Clustered = true; }
    }

    local procedure PrepareFromItem()
    begin
        CopyFromItem();
    end;

    procedure CopyFromItem()
    begin
        UpdateMode();
    end;

    [TryFunction]
    local procedure TryCopyFromItem()
    begin
        UpdateMode();
    end;

    procedure UpdateMode()
    begin
        Validate(Mode, Mode::Production);
    end;

    local procedure CopyFromItemOneArg()
    begin
        UpdateModeOneArg();
    end;

    local procedure UpdateModeOneArg()
    begin
        Mode := Mode::Production;
        Validate(Mode);
    end;

    local procedure RevalidateItem()
    begin
        RevalidateItemNow();
    end;

    local procedure RevalidateItemNow()
    begin
        Validate("Item No.", 'I3');
    end;

    local procedure CopyToOther()
    begin
        UpdateOther();
    end;

    local procedure UpdateOther()
    begin
        Validate(Other, 1);
    end;

    procedure TableProcRead(Tag: Text)
    var
        Cfg: Codeunit "NBX Cfg";
    begin
        Cfg.Add(Tag + ' x=' + Cfg.Fmt(xRec));
    end;

    procedure TableProcAssign()
    begin
        xRec.Series := 'SET';
    end;

    [IntegrationEvent(false, false)]
    local procedure OnModeElse(var Line: Record "NBX Line")
    begin
    end;
}

codeunit 50001 "NBX Cfg"
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

    procedure Fmt(Line: Record "NBX Line"): Text
    begin
        exit(Format(Line.Mode, 0, 2) + '/' + Line.Series + '/' + Line."Item No.");
    end;
}

codeunit 50002 "NBX Subscriber"
{
    [EventSubscriber(ObjectType::Table, Database::"NBX Line", 'OnModeElse', '', false, false)]
    local procedure OnModeElse(var Line: Record "NBX Line")
    var
        Cfg: Codeunit "NBX Cfg";
    begin
        if Cfg.Get() = 'blank-assign' then
            Line.TableProcAssign()
        else
            Line.TableProcRead('P');
    end;
}

// A codeunit that is no test codeunit, as the caller of the outer Validate.
codeunit 50003 "NBX Caller"
{
    procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    var
        Cfg: Codeunit "NBX Cfg";
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;
}

codeunit 50004 "NBX Setup"
{
    // The buffer of corpus XRecTableProcThroughSubscriber.al `AfterInsert`: inserted by the record itself, Series changed after.
    procedure AfterInsert(var Line: Record "NBX Line")
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
        Cfg: Codeunit "NBX Cfg";
        Got: Text;
    begin
        Got := Cfg.Take();
        if Got <> Expected then
            Error('log %1', Got);
    end;
}
