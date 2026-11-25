tableextension 50410 "PX Line Ext" extends "PX Line"
{
    fields
    {
        modify(Mode)
        {
            trigger OnBeforeValidate()
            var
                Cfg: Codeunit "PX Cfg";
            begin
                if Cfg.Get() = 'ext-before' then
                    Rec.Series := 'EXT';
            end;
        }
    }
}

codeunit 50411 "PX Sub A"
{
    [EventSubscriber(ObjectType::Table, Database::"PX Line", 'OnAfterValidateEvent', 'Kind', false, false)]
    local procedure OnAfterValidateKind(var Rec: Record "PX Line"; var xRec: Record "PX Line"; CurrFieldNo: Integer)
    var
        Cfg: Codeunit "PX Cfg";
    begin
        if Cfg.Get() in ['two', 'two-trg', 'fieldref', 'direct'] then
            Rec.TableProcRead();
    end;
}

codeunit 50412 "PX Sub B"
{
    [EventSubscriber(ObjectType::Table, Database::"PX Line", 'OnAfterValidateEvent', 'Kind', false, false)]
    local procedure OnAfterValidateKind(var Rec: Record "PX Line"; var xRec: Record "PX Line"; CurrFieldNo: Integer)
    var
        Cfg: Codeunit "PX Cfg";
    begin
        if Cfg.Get() in ['two', 'two-trg'] then
            Rec.Note := 'B';
    end;
}

codeunit 50413 "PX Tests"
{
    Subtype = Test;

    var
        Cfg: Codeunit "PX Cfg";

    local procedure Prepare(var Line: Record "PX Line")
    begin
        Line.DeleteAll();
        Line.Init();
        Line."Entry No." := 1;
        Line.Series := 'OLD';
        Line.Insert();
    end;

    local procedure Check(Line: Record "PX Line")
    begin
        Cfg.Set('');
        if Line.Seen <> 'OLD' then
            Error('xRec.Series in the table procedure: ''%1''', Line.Seen);
    end;

    local procedure RunMode(CfgMode: Text)
    var
        Line: Record "PX Line";
    begin
        Prepare(Line);
        Cfg.Set(CfgMode);
        Line.Validate(Mode, 1);
        Check(Line);
    end;

    local procedure RunKind(CfgMode: Text)
    var
        Line: Record "PX Line";
    begin
        Prepare(Line);
        Cfg.Set(CfgMode);
        Line.Validate(Kind, 1);
        Check(Line);
    end;

    [Test]
    procedure Chain_Control_TriggerAssigns()
    begin
        RunMode('cu-direct');
    end;

    [Test]
    procedure Chain_ExtOnBeforeValidateAssigns()
    begin
        RunMode('ext-before');
    end;

    [Test]
    procedure Chain_RecordAssignment()
    begin
        RunMode('assign-rec');
    end;

    [Test]
    procedure Chain_CalcFields()
    begin
        RunMode('calc');
    end;

    [Test]
    procedure After_Control_TriggerAssigns()
    begin
        RunKind('direct');
    end;

    [Test]
    procedure After_TwoSubscribers_OtherAssigns()
    begin
        RunKind('two');
    end;

    [Test]
    procedure After_TwoSubscribers_TriggerAssigns()
    begin
        RunKind('two-trg');
    end;

    [Test]
    procedure After_FieldRefValidate_TriggerAssigns()
    var
        Line: Record "PX Line";
        RecRef: RecordRef;
        FldRef: FieldRef;
    begin
        Prepare(Line);
        Cfg.Set('fieldref');
        RecRef.GetTable(Line);
        FldRef := RecRef.Field(Line.FieldNo(Kind));
        FldRef.Validate(1);
        RecRef.SetTable(Line);
        Check(Line);
    end;
}
