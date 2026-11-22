// The shapes next to the OnBeforeValidateEvent and OnAfterValidateEvent cells record/validate/
// XRecTableProcThroughSubscriber.al pins (al-runner issue 8306): the same AL as the al-test-cli suite xrec_validate_event.rs.
// Tests named Refused_ refuse a read of xRec on purpose; KeepsMain_ keeps main's answer.
codeunit 50400 "VE Log"
{
    SingleInstance = true;
    var
        CfgMode: Text;
        Entries: Text;
    procedure Set(NewMode: Text) begin CfgMode := NewMode; Entries := ''; end;
    procedure Mode(): Text begin exit(CfgMode); end;
    procedure Add(Entry: Text) begin Entries += Entry + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}

table 50401 "VE Line"
{
    fields
    {
        field(1; "Line No."; Integer) { }
        field(2; Mode; Option)
        {
            OptionMembers = Purchase,Production;
            trigger OnValidate()
            var
                Log: Codeunit "VE Log";
            begin
                if Log.Mode() = 'before-assign' then
                    Log.Add('T=' + xRec.Series);
            end;
        }
        field(3; Series; Code[20]) { }
        field(4; Outer; Integer)
        {
            trigger OnValidate()
            begin
                Validate(Mode, Mode::Production);
            end;
        }
    }
    keys { key(PK; "Line No.") { Clustered = true; } }

    trigger OnInsert()
    var
        Log: Codeunit "VE Log";
    begin
        if Log.Mode() = 'own-insert' then
            Validate(Mode, Mode::Production);
    end;

    trigger OnModify()
    var
        Log: Codeunit "VE Log";
    begin
        if Log.Mode() = 'own-modify' then
            Validate(Mode, Mode::Production);
    end;

    procedure TableRead(Tag: Text) var Log: Codeunit "VE Log"; begin Log.Add(Tag + '=' + xRec.Series); end;
}

tableextension 50402 "VE Line Ext" extends "VE Line"
{
    fields
    {
        field(50400; ExtField; Integer) { trigger OnValidate() begin end; }
    }
    procedure ExtRead(Tag: Text) var Log: Codeunit "VE Log"; begin Log.Add(Tag + '=' + xRec.Series); end;
    procedure ExtCallsRecProc() begin Rec.TableRead('R'); end;
    procedure ExtCallsOwnProc() begin ExtRead('U'); end;
    procedure ExtAssign() begin xRec.Series := 'SET'; end;
    procedure ExtThroughHelper() var Helper: Codeunit "VE Helper"; begin Helper.CallExt(Rec); end;
}

codeunit 50403 "VE Helper"
{
    procedure CallExt(var Line: Record "VE Line") begin Line.ExtRead('C'); end;
}

codeunit 50404 "VE Subscriber"
{
    [EventSubscriber(ObjectType::Table, Database::"VE Line", 'OnAfterValidateEvent', 'Mode', false, false)]
    local procedure OnAfterValidateMode(var Rec: Record "VE Line"; var xRec: Record "VE Line"; CurrFieldNo: Integer)
    var
        Log: Codeunit "VE Log";
        Helper: Codeunit "VE Helper";
        Other: Record "VE Line";
    begin
        case Log.Mode() of
            'after':
                begin
                    Log.Add('A=' + xRec.Series);
                    Rec.ExtRead('X');
                    Rec.TableRead('P');
                end;
            'after-assign':
                begin
                    Rec.ExtAssign();
                    Log.Add('A after=' + xRec.Series);
                end;
            'after-helper':
                Helper.CallExt(Rec);
            'after-descendant':
                Rec.ExtThroughHelper();
            'after-other':
                begin
                    Other := Rec;
                    Other.ExtRead('O');
                end;
            'after-modify':
                begin
                    Rec.Modify();
                    Rec.ExtRead('X');
                end;
            'after-get':
                begin
                    Rec.Get(Rec."Line No.");
                    Rec.ExtRead('X');
                end;
            'after-write':
                begin
                    Rec.Series := 'SUB';
                    Rec.ExtRead('X');
                end;
            'after-rec-proc':
                Rec.ExtCallsRecProc();
            'after-own-proc':
                Rec.ExtCallsOwnProc();
        end;
    end;

    [EventSubscriber(ObjectType::Table, Database::"VE Line", 'OnBeforeValidateEvent', 'Mode', false, false)]
    local procedure OnBeforeValidateMode(var Rec: Record "VE Line"; var xRec: Record "VE Line"; CurrFieldNo: Integer)
    var
        Log: Codeunit "VE Log";
    begin
        if Log.Mode() = 'before-assign' then
            Rec.ExtAssign();
        if Log.Mode() <> 'before' then
            exit;
        Log.Add('B=' + xRec.Series);
        Rec.ExtRead('X');
        Rec.TableRead('P');
    end;

    [EventSubscriber(ObjectType::Table, Database::"VE Line", 'OnAfterValidateEvent', 'Mode', false, false)]
    local procedure OnAfterValidateModeRecOnly(var Rec: Record "VE Line")
    var
        Log: Codeunit "VE Log";
    begin
        if not (Log.Mode() in ['rec-only', 'own-modify', 'own-insert', 'modify-sub']) then
            exit;
        Rec.ExtRead('X');
    end;

    [EventSubscriber(ObjectType::Table, Database::"VE Line", 'OnBeforeValidateEvent', 'Mode', false, false)]
    local procedure OnBeforeValidateModeRecOnly(var Rec: Record "VE Line")
    var
        Log: Codeunit "VE Log";
    begin
        if Log.Mode() = 'before-rec-only' then
            Rec.ExtRead('X');
    end;

    [EventSubscriber(ObjectType::Table, Database::"VE Line", 'OnAfterModifyEvent', '', false, false)]
    local procedure OnAfterModifyLine(var Rec: Record "VE Line"; var xRec: Record "VE Line"; RunTrigger: Boolean)
    var
        Log: Codeunit "VE Log";
    begin
        if Log.Mode() = 'modify-sub' then
            Rec.Validate(Mode, Rec.Mode::Production);
    end;

    [EventSubscriber(ObjectType::Table, Database::"VE Line", 'OnAfterValidateEvent', 'Mode', false, false)]
    local procedure OnAfterValidateModeXRecByValue(var Rec: Record "VE Line"; xRec: Record "VE Line"; CurrFieldNo: Integer)
    var
        Log: Codeunit "VE Log";
    begin
        if Log.Mode() <> 'xrec-by-value' then
            exit;
        Rec.ExtRead('X');
    end;

    // Parameters bind by name: CurrFieldNo by var in the second place, xRec third.
    [EventSubscriber(ObjectType::Table, Database::"VE Line", 'OnAfterValidateEvent', 'Mode', false, false)]
    local procedure OnAfterValidateModeFieldNoSecond(var Rec: Record "VE Line"; var CurrFieldNo: Integer; xRec: Record "VE Line")
    var
        Log: Codeunit "VE Log";
    begin
        if Log.Mode() <> 'fieldno-second' then
            exit;
        Rec.ExtRead('X');
    end;
}

codeunit 50408 "VE Single Subscriber"
{
    SingleInstance = true;

    [EventSubscriber(ObjectType::Table, Database::"VE Line", 'OnAfterValidateEvent', 'Mode', false, false)]
    local procedure OnAfterValidateMode(var Rec: Record "VE Line")
    var
        Log: Codeunit "VE Log";
    begin
        if Log.Mode() = 'single' then
            Rec.ExtRead('X');
    end;

    [EventSubscriber(ObjectType::Table, Database::"VE Line", 'OnAfterValidateEvent', 'Series', false, false)]
    local procedure OnAfterValidateSeries(var Rec: Record "VE Line")
    var
        Log: Codeunit "VE Log";
    begin
        if Log.Mode() = 'no-trigger' then
            Rec.ExtRead('X');
    end;

    [EventSubscriber(ObjectType::Table, Database::"VE Line", 'OnAfterValidateEvent', 'ExtField', false, false)]
    local procedure OnAfterValidateExtField(var Rec: Record "VE Line")
    var
        Log: Codeunit "VE Log";
    begin
        if Log.Mode() = 'ext-field' then
            Rec.ExtRead('X');
    end;
}

codeunit 50409 "VE Runner"
{
    TableNo = "VE Line";
    trigger OnRun()
    begin
        Rec.Validate(Mode, Rec.Mode::Production);
    end;
}

codeunit 50405 "VE Manual Subscriber"
{
    EventSubscriberInstance = Manual;

    [EventSubscriber(ObjectType::Table, Database::"VE Line", 'OnAfterValidateEvent', 'Mode', false, false)]
    local procedure OnAfterValidateMode(var Rec: Record "VE Line"; var xRec: Record "VE Line"; CurrFieldNo: Integer)
    begin
        Rec.ExtRead('X');
    end;
}

page 50406 "VE Card"
{
    PageType = Card;
    SourceTable = "VE Line";
    ApplicationArea = All;
    UsageCategory = None;
    layout { area(Content) { field(Mode; Rec.Mode) { ApplicationArea = All; } } }
    actions { area(Processing) { action(SetProduction) { trigger OnAction() begin Rec.Validate(Mode, Rec.Mode::Production); end; } } }
}

codeunit 50407 "VE Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "VE Log";

    local procedure AfterInsert(var Line: Record "VE Line")
    begin
        Log.Set('');
        Line.DeleteAll();
        Line."Line No." := 1;
        Line.Series := 'OLD';
        Line.Insert();
        Line.Series := 'NEW';
    end;

    local procedure ValidateMode(var Line: Record "VE Line"; CfgMode: Text; Expected: Text)
    begin
        Log.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
        Check(Expected);
    end;

    local procedure Check(Expected: Text)
    var
        Got: Text;
    begin
        Got := Log.Take();
        Log.Set('');
        if Got <> Expected then
            Error('Logged %1', Got);
    end;

    [Test] procedure After_AfterInsert() var L: Record "VE Line"; begin AfterInsert(L); ValidateMode(L, 'after', 'A=NEW;X=NEW;P=NEW;'); end;
    [Test] procedure After_AfterGet() var L: Record "VE Line"; begin AfterInsert(L); Clear(L); L.Get(1); L.Series := 'NEW'; ValidateMode(L, 'after', 'A=NEW;X=NEW;P=NEW;'); end;
    [Test] procedure After_FreshRecord() var L: Record "VE Line"; begin Log.Set(''); L.DeleteAll(); L."Line No." := 1; L.Series := 'NEW'; ValidateMode(L, 'after', 'A=NEW;X=NEW;P=NEW;'); end;
    [Test] procedure After_ProcAssigns_SubscriberReadsAfter() var L: Record "VE Line"; begin AfterInsert(L); ValidateMode(L, 'after-assign', 'A after=SET;'); end;
    [Test] procedure Before_AfterInsert() var L: Record "VE Line"; begin AfterInsert(L); ValidateMode(L, 'before', 'B=NEW;X=NEW;P=NEW;'); end;
    [Test] procedure RecOnlySubscriber() var L: Record "VE Line"; begin AfterInsert(L); ValidateMode(L, 'rec-only', 'X=NEW;'); end;
    [Test] procedure After_TemporaryRecord() var L: Record "VE Line" temporary; begin AfterInsert(L); ValidateMode(L, 'after', 'A=NEW;X=NEW;P=NEW;'); end;
    [Test] procedure After_ReadByFindFirst() var L: Record "VE Line"; R: Record "VE Line"; begin AfterInsert(L); R.FindFirst(); R.Series := 'NEW'; ValidateMode(R, 'after', 'A=NEW;X=NEW;P=NEW;'); end;
    [Test] procedure After_OneArgumentValidate() var L: Record "VE Line"; begin AfterInsert(L); L.Mode := L.Mode::Production; Log.Set('after'); L.Validate(Mode); Check('A=NEW;X=NEW;P=NEW;'); end;
    [Test] procedure After_FromAfterModifySubscriber() var L: Record "VE Line"; begin AfterInsert(L); Log.Set('modify-sub'); L.Modify(true); Check('X=NEW;'); end;
    [Test] procedure Before_RecOnlySubscriber() var L: Record "VE Line"; begin AfterInsert(L); ValidateMode(L, 'before-rec-only', 'X=NEW;'); end;
    [Test] procedure Before_ProcAssigns_TriggerReadsAfter() var L: Record "VE Line"; begin AfterInsert(L); ValidateMode(L, 'before-assign', 'T=SET;'); end;
    [Test] procedure After_FieldRefValidate() var L: Record "VE Line"; R: RecordRef; begin AfterInsert(L); R.GetTable(L); Log.Set('rec-only'); R.Field(2).Validate(1); Check('X=NEW;'); end;
    // The page's record is the stored row: Series OLD.
    [Test]
    procedure After_TestPageSetValue()
    var
        L: Record "VE Line";
        P: TestPage "VE Card";
    begin
        AfterInsert(L);
        P.OpenEdit();
        P.GoToRecord(L);
        Log.Set('rec-only');
        P.Mode.SetValue(L.Mode::Production);
        Check('X=OLD;');
        P.Close();
    end;
    [Test]
    procedure After_PageAction()
    var
        L: Record "VE Line";
        P: TestPage "VE Card";
    begin
        AfterInsert(L);
        P.OpenEdit();
        P.GoToRecord(L);
        Log.Set('rec-only');
        P.SetProduction.Invoke();
        Check('X=OLD;');
        P.Close();
    end;

    // Another record variable copied from Rec: main's plain call (a blank xRec), not measured for this shape; real BC's
    // answer may differ.
    [Test] procedure KeepsMain_OtherRecord() var L: Record "VE Line"; begin AfterInsert(L); ValidateMode(L, 'after-other', 'O=;'); end;

    // Real BC hands the Validate's xRec (C=NEW, record/validate/XRecTableProcThroughSubscriber.al:1280); the runner refuses
    // what it has not measured.
    [Test] procedure Refused_DescendantThroughCodeunit() var L: Record "VE Line"; begin AfterInsert(L); ValidateMode(L, 'after-descendant', 'C=NEW;'); end;
    [Test] procedure Refused_ThroughCodeunit() var L: Record "VE Line"; begin AfterInsert(L); ValidateMode(L, 'after-helper', 'C=NEW;'); end;
    [Test] procedure Refused_XRecByValue() var L: Record "VE Line"; begin AfterInsert(L); ValidateMode(L, 'xrec-by-value', 'X=NEW;'); end;
    [Test] procedure Refused_CurrFieldNoSecondXRecThird() var L: Record "VE Line"; begin AfterInsert(L); ValidateMode(L, 'fieldno-second', 'X=NEW;'); end;
    [Test] procedure Refused_ManualSubscriber() var L: Record "VE Line"; M: Codeunit "VE Manual Subscriber"; begin AfterInsert(L); BindSubscription(M); ValidateMode(L, 'manual', 'X=NEW;'); end;
    [Test] procedure Refused_ValidateFromOwnOnModify() var L: Record "VE Line"; begin AfterInsert(L); Log.Set('own-modify'); L.Modify(true); end;
    [Test] procedure Refused_ValidateFromOwnOnInsert() var L: Record "VE Line"; begin Log.Set(''); L.DeleteAll(); L."Line No." := 1; L.Series := 'NEW'; Log.Set('own-insert'); L.Insert(true); end;
    [Test] procedure Refused_NestedInTrigger() var L: Record "VE Line"; begin AfterInsert(L); Log.Set('rec-only'); L.Validate(Outer, 1); end;
    // Shapes no cell measured (review of PR 8307, 2026-11-20): each refuses a read of xRec.
    [Test] procedure Refused_SingleInstanceSubscriber() var L: Record "VE Line"; begin AfterInsert(L); ValidateMode(L, 'single', ''); end;
    [Test] procedure Refused_ModifyBeforeCall() var L: Record "VE Line"; begin AfterInsert(L); ValidateMode(L, 'after-modify', ''); end;
    [Test] procedure Refused_GetBeforeCall() var L: Record "VE Line"; begin AfterInsert(L); ValidateMode(L, 'after-get', ''); end;
    [Test] procedure Refused_FieldWrittenBeforeCall() var L: Record "VE Line"; begin AfterInsert(L); ValidateMode(L, 'after-write', ''); end;
    [Test] procedure Refused_FieldWithoutOnValidate() var L: Record "VE Line"; begin AfterInsert(L); Log.Set('no-trigger'); L.Validate(Series, 'NEW'); end;
    [Test] procedure Refused_TableExtensionField() var L: Record "VE Line"; begin AfterInsert(L); Log.Set('ext-field'); L.Validate(ExtField, 1); end;
    [Test] procedure Refused_ValidateFromCodeunitRun() var L: Record "VE Line"; begin AfterInsert(L); Log.Set('rec-only'); Codeunit.Run(Codeunit::"VE Runner", L); end;
    [Test] procedure Refused_ValidateFromTryFunction() var L: Record "VE Line"; begin AfterInsert(L); Log.Set('rec-only'); if TryValidate(L) then; end;
    [TryFunction] local procedure TryValidate(var L: Record "VE Line") begin L.Validate(Mode, L.Mode::Production); end;
    // Rec.OtherProc() inside the procedure that shares the Validate's xRec: not measured; the unqualified call shares it.
    [Test] procedure Refused_RecDotProcInSharedProc() var L: Record "VE Line"; begin AfterInsert(L); ValidateMode(L, 'after-rec-proc', ''); end;
    [Test] procedure After_UnqualifiedProcInSharedProc() var L: Record "VE Line"; begin AfterInsert(L); ValidateMode(L, 'after-own-proc', 'U=NEW;'); end;
}
