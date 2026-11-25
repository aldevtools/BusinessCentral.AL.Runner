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
        if Cfg.Get() in ['two', 'two-trg'] then
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

codeunit 50414 "PX Sub C"
{
    [EventSubscriber(ObjectType::Table, Database::"PX Line", 'OnAfterValidateEvent', 'Single', false, false)]
    local procedure OnAfterValidateSingle(var Rec: Record "PX Line"; var xRec: Record "PX Line"; CurrFieldNo: Integer)
    var
        Cfg: Codeunit "PX Cfg";
    begin
        if Cfg.Get() in ['s-ctrl', 's-fieldref', 's-testpage', 's-action'] then
            Rec.TableProcRead();
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

    local procedure RunPlain(CfgMode: Text)
    var
        Line: Record "PX Line";
    begin
        Prepare(Line);
        Cfg.Set(CfgMode);
        Line.Validate(Plain, 1);
        Check(Line);
    end;

    local procedure RunSingle(CfgMode: Text)
    var
        Line: Record "PX Line";
    begin
        Prepare(Line);
        Cfg.Set(CfgMode);
        Line.Validate(Single, 1);
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
    procedure Chain_ExtOnBeforeValidateAssigns()
    begin
        RunMode('ext-before');
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

    // Field Plain: no table extension trigger, no subscriber. One writer per test.
    [Test]
    procedure Plain_Control_TriggerAssigns()
    begin
        RunPlain('p-ctrl');
    end;

    [Test]
    procedure Plain_Assign()
    begin
        RunPlain('p-assign');
    end;

    [Test]
    procedure Plain_VariantAssign()
    begin
        RunPlain('p-variant');
    end;

    [Test]
    procedure Plain_Calc()
    begin
        RunPlain('p-calc');
    end;

    [Test]
    procedure Plain_RecRefSetTable()
    begin
        RunPlain('p-settable');
    end;

    [Test]
    procedure Plain_CalcSums()
    begin
        RunPlain('p-calcsums');
    end;

    [Test]
    procedure Plain_BlobStream()
    begin
        RunPlain('p-blob');
    end;

    [Test]
    procedure Plain_PageGetRecord()
    begin
        RunPlain('p-getrecord');
    end;

    [Test]
    procedure Plain_PageSetSelectionFilter()
    begin
        RunPlain('p-selfilter');
    end;

    // Field Single: one subscriber, no table extension trigger. One caller per test.
    [Test]
    procedure Single_Control_TriggerAssigns()
    begin
        RunSingle('s-ctrl');
    end;

    [Test]
    procedure Single_FieldRefValidate_TriggerAssigns()
    var
        Line: Record "PX Line";
        RecRef: RecordRef;
        FldRef: FieldRef;
    begin
        Prepare(Line);
        Cfg.Set('s-fieldref');
        RecRef.GetTable(Line);
        FldRef := RecRef.Field(Line.FieldNo(Single));
        FldRef.Validate(1);
        RecRef.SetTable(Line);
        Check(Line);
    end;

    [Test]
    procedure Single_TestPageSetValue_TriggerAssigns()
    var
        Line: Record "PX Line";
        Card: TestPage "PX Card";
    begin
        Prepare(Line);
        Cfg.Set('s-testpage');
        Card.OpenEdit();
        Card.GoToRecord(Line);
        Card.Single.SetValue(1);
        Card.Close();
        Line.Get(1);
        Check(Line);
    end;

    [Test]
    procedure Single_PageAction_TriggerAssigns()
    var
        Line: Record "PX Line";
        Card: TestPage "PX Card";
    begin
        Prepare(Line);
        Cfg.Set('s-action');
        Card.OpenEdit();
        Card.GoToRecord(Line);
        Card.ValidateSingle.Invoke();
        Card.Close();
        Line.Get(1);
        Check(Line);
    end;
}
