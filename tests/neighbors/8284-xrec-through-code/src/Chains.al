// The shapes next to the chains record/validate/XRecTableProcThroughSubscriber.al pins and record/validate/
// XRecTableProcCallers.al measures (al-runner issue 8284): the same AL as the al-test-cli suite xrec_through_subscriber.rs.
// Tests named Refused_ refuse a read of xRec on purpose; Assumed_ run through xrec-table-proc-callers-subscribers.
codeunit 50200 "XS Log"
{
    SingleInstance = true;
    var
        CfgMode: Text;
        Entries: Text;
    procedure Set(NewMode: Text) begin CfgMode := NewMode; Entries := ''; end;
    procedure Mode(): Text begin exit(CfgMode); end;
    procedure Add(Entry: Text) begin Entries += Entry + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
    procedure Fmt(Line: Record "XS Line"): Text begin exit(Format(Line.Mode, 0, 2) + '/' + Line.Series + '/' + Line."Item No."); end;
}

table 50201 "XS Other" { fields { field(1; "Code"; Code[20]) { } } keys { key(PK; "Code") { Clustered = true; } } }

table 50202 "XS Line"
{
    fields
    {
        field(1; "Line No."; Integer) { }
        field(2; Mode; Option)
        {
            OptionMembers = Purchase,Production;
            trigger OnValidate()
            var
                Log: Codeunit "XS Log";
            begin
                if Log.Mode() = '' then
                    exit;
                Log.Add('T x=' + Log.Fmt(xRec));
                if Log.Mode() = 'nested-inner-assign' then
                    xRec.Series := 'INN';
                if Log.Mode() = 'sub-validate-first' then
                    Validate(Series, Series);
                case Log.Mode() of
                    'sub-manual':
                        OnModeManual(Rec);
                    'sub-single':
                        OnModeSingle(Rec);
                    'sub-two-subs':
                        OnModeTwoSubs(Rec);
                    'sub-two-single':
                        OnModeTwoSingle(Rec);
                    'sub-wide':
                        OnModeWide(Rec, OtherRow);
                    'sub-two-noparams':
                        OnModeTwoNoParams(Rec);
                    'sub', 'nested', 'nested0', 'nested1', 'nested3', 'nested4', 'sub-assign', 'nested-assign', 'nested-assign-err', 'nested-assign-restore', 'nested-cu', 'nested-peer', 'nested-ext',
                    'sub-modify', 'sub-get', 'sub-validate-first', 'sub-oninsert', 'sub-rec-proc', 'sub-own-proc',
                    'nested-outer-assign', 'nested-inner-assign', 'nested-filter', 'nested-marked', 'nested-recproc', 'nested-onearg', 'sub-proc-val':
                        OnModeElse(Rec);
                    'sub-two':
                        OnModeTwo(Rec, OtherRow);
                    'sub-sender':
                        OnModeSender(Rec);
                    'sub-byval2':
                        OnModeByValueSecond(Rec, OtherRow);
                    'sub-byvalue':
                        OnModeByValue(Rec);
                    'sub-fromproc':
                        RaiseFromProc();
                end;
                if Log.Mode() in ['sub-assign', 'nested-assign'] then
                    Log.Add('T after x=' + Log.Fmt(xRec));
                if Log.Mode() = 'nested-filter' then
                    Log.Add('T filters=' + xRec.GetFilters());
            end;
        }
        field(3; Series; Code[20]) { }
        field(4; "Item No."; Code[20])
        {
            trigger OnValidate()
            var
                Log: Codeunit "XS Log";
                Helper: Codeunit "XS Helper";
                Peer: Record "XS Line";
            begin
                if not (Log.Mode() in ['nested', 'nested0', 'nested1', 'nested3', 'nested4', 'nested-assign', 'nested-assign-err', 'nested-assign-restore', 'nested-cu', 'nested-peer',
                    'nested-outer-assign', 'nested-inner-assign', 'nested-filter', 'nested-marked', 'nested-recproc', 'nested-onearg'])
                then
                    exit;
                Log.Add('I x=' + Log.Fmt(xRec));
                Series := 'MID';
                if Log.Mode() = 'nested-outer-assign' then
                    xRec.Series := 'OUT';
                case Log.Mode() of
                    'nested', 'nested-assign', 'nested-assign-err', 'nested-assign-restore', 'nested-outer-assign', 'nested-inner-assign', 'nested-filter', 'nested-marked':
                        CopyFromItem();
                    'nested0':
                        Validate(Mode, Mode::Production);
                    'nested1':
                        UpdateMode();
                    'nested3':
                        PrepareFromItem();
                    'nested4':
                        StartFromItem();
                    'nested-cu':
                        Helper.ValidateMode(Rec);
                    'nested-peer':
                        Peer.ValidateModeOn(Rec);
                    'nested-recproc':
                        Rec.UpdateMode();
                    'nested-onearg':
                        begin
                            Mode := Mode::Production;
                            Validate(Mode);
                        end;
                end;
                if Log.Mode() in ['nested-assign', 'nested-outer-assign', 'nested-inner-assign'] then
                    Log.Add('I after x=' + Log.Fmt(xRec));
                if Log.Mode() = 'nested-filter' then
                    Log.Add('I filters=' + xRec.GetFilters());
            end;
        }
        field(5; ViaCodeunit; Boolean)
        {
            trigger OnValidate()
            var
                Log: Codeunit "XS Log";
                Helper: Codeunit "XS Helper";
                IfaceMover: Codeunit "XS Iface Mover";
                SingleHelper: Codeunit "XS Single Helper";
                Mover: Interface "XS IMover";
            begin
                Log.Add('V x=' + Log.Fmt(xRec));
                case Log.Mode() of
                    'cu-ext':
                        Helper.CallExt(Rec);
                    'cu-base':
                        Helper.CallBase(Rec);
                    'cu-assign':
                        begin
                            Helper.CallExtAssign(Rec);
                            Log.Add('V after x=' + Log.Fmt(xRec));
                        end;
                    'cu-iface':
                        begin
                            Mover := IfaceMover;
                            Mover.Handle(Rec);
                        end;
                    'modify-event':
                        Rec.Modify(true);
                    'cu-try':
                        if Helper.TryCallExt(Rec) then
                            Log.Add('try ok');
                    'cu-single':
                        SingleHelper.CallExt(Rec);
                end;
            end;
        }
        field(6; Other; Integer)
        {
            trigger OnValidate()
            var
                Log: Codeunit "XS Log";
            begin
                if Log.Mode() <> '' then
                    Log.Add('O x=' + Log.Fmt(xRec));
            end;
        }
    }
    keys { key(PK; "Line No.") { Clustered = true; } }
    var
        OtherRow: Record "XS Other";

    trigger OnInsert()
    var
        Log: Codeunit "XS Log";
    begin
        if Log.Mode() = 'sub-oninsert' then
            Validate(Mode, Mode::Production);
    end;

    local procedure StartFromItem() begin PrepareFromItem(); end;
    local procedure PrepareFromItem() begin CopyFromItem(); end;
    local procedure CopyFromItem() begin UpdateMode(); end;
    procedure UpdateMode() begin Validate(Mode, Mode::Production); end;
    local procedure RaiseFromProc() begin OnModeElse(Rec); end;

    procedure ValidateModeOn(var Target: Record "XS Line")
    begin
        Target.Validate(Mode, Target.Mode::Production);
    end;

    procedure TableRead(Tag: Text) var Log: Codeunit "XS Log"; begin Log.Add(Tag + ' x=' + Log.Fmt(xRec)); end;

    [IntegrationEvent(false, false)]
    local procedure OnModeElse(var Line: Record "XS Line") begin end;
    [IntegrationEvent(false, false)]
    local procedure OnModeManual(var Line: Record "XS Line") begin end;
    [IntegrationEvent(false, false)]
    local procedure OnModeSingle(var Line: Record "XS Line") begin end;
    [IntegrationEvent(false, false)]
    local procedure OnModeTwoSubs(var Line: Record "XS Line") begin end;
    [IntegrationEvent(false, false)]
    local procedure OnModeTwoSingle(var Line: Record "XS Line") begin end;
    [IntegrationEvent(false, false)]
    local procedure OnModeWide(var Line: Record "XS Line"; var Other: Record "XS Other") begin end;
    [IntegrationEvent(false, false)]
    local procedure OnModeTwoNoParams(var Line: Record "XS Line") begin end;
    [IntegrationEvent(false, false)]
    local procedure OnModeTwo(var Line: Record "XS Line"; var Other: Record "XS Other") begin end;
    [IntegrationEvent(true, false)]
    local procedure OnModeSender(var Line: Record "XS Line") begin end;
    [IntegrationEvent(false, false)]
    local procedure OnModeByValueSecond(var Line: Record "XS Line"; Other: Record "XS Other") begin end;
    [IntegrationEvent(false, false)]
    local procedure OnModeByValue(Line: Record "XS Line") begin end;
}

tableextension 50203 "XS Line Ext" extends "XS Line"
{
    fields
    {
        field(50200; ExtTrig; Integer)
        {
            trigger OnValidate()
            var
                Log: Codeunit "XS Log";
            begin
                Log.Add('E x=' + Log.Fmt(xRec));
                Series := 'MID';
                Validate(Mode, Mode::Production);
            end;
        }
    }
    procedure ExtRead(Tag: Text) var Log: Codeunit "XS Log"; begin Log.Add(Tag + ' x=' + Log.Fmt(xRec)); end;
    procedure ExtAssign() var Log: Codeunit "XS Log"; begin Log.Add('XS x=' + Log.Fmt(xRec)); xRec.Series := 'SET'; end;
    procedure ExtAssignThenError() begin xRec.Series := 'SET'; Error('stop'); end;
    procedure ExtAssignAndRestore() var Old: Code[20]; begin Old := xRec.Series; xRec.Series := 'SET'; xRec.Series := Old; end;
    procedure ExtFilter() var Log: Codeunit "XS Log"; begin xRec.SetRange(Series, 'F'); Log.Add('XF filters=' + xRec.GetFilters()); end;
    procedure ExtValidateOther() begin Rec.Validate(Other, 5); end;
    procedure ExtMarkedOnly() begin xRec.MarkedOnly(true); end;
    procedure ExtCallsRecProc() begin Rec.TableRead('R'); end;
    procedure ExtCallsOwnProc() begin ExtRead('U'); end;
}

interface "XS IMover"
{
    procedure Handle(var Line: Record "XS Line");
}

codeunit 50208 "XS Iface Mover" implements "XS IMover"
{
    procedure Handle(var Line: Record "XS Line") begin Line.ExtRead('CI'); end;
}

codeunit 50204 "XS Subscriber"
{
    [EventSubscriber(ObjectType::Table, Database::"XS Line", 'OnModeElse', '', false, false)]
    local procedure OnModeElse(var Line: Record "XS Line")
    var
        Log: Codeunit "XS Log";
    begin
        case Log.Mode() of
            'sub-modify':
                Line.Modify();
            'sub-get':
                Line.Get(Line."Line No.");
            'sub-rec-proc':
                begin
                    Line.ExtCallsRecProc();
                    exit;
                end;
            'sub-own-proc':
                begin
                    Line.ExtCallsOwnProc();
                    exit;
                end;
        end;
        if Log.Mode() in ['sub-assign', 'nested-assign'] then begin
            Line.ExtAssign();
            exit;
        end;
        if Log.Mode() = 'nested-assign-err' then begin
            asserterror Line.ExtAssignThenError();
            exit;
        end;
        if Log.Mode() = 'nested-assign-restore' then begin
            Line.ExtAssignAndRestore();
            exit;
        end;
        if Log.Mode() = 'nested-filter' then begin
            Line.ExtFilter();
            exit;
        end;
        if Log.Mode() = 'nested-marked' then begin
            Line.ExtMarkedOnly();
            exit;
        end;
        if Log.Mode() = 'sub-proc-val' then begin
            Line.ExtValidateOther();
            exit;
        end;
        Line.ExtRead('X');
        Line.TableRead('P');
    end;

    [EventSubscriber(ObjectType::Table, Database::"XS Line", 'OnModeTwoSubs', '', false, false)]
    local procedure OnModeTwoSubs(var Line: Record "XS Line")
    begin
        Line.ExtRead('X');
    end;

    [EventSubscriber(ObjectType::Table, Database::"XS Line", 'OnModeTwoSingle', '', false, false)]
    local procedure OnModeTwoSingle(var Line: Record "XS Line")
    begin
        Line.ExtRead('X');
    end;

    [EventSubscriber(ObjectType::Table, Database::"XS Line", 'OnModeWide', '', false, false)]
    local procedure OnModeWide(var Line: Record "XS Line"; var Other: Record "XS Other")
    begin
        Line.ExtRead('X');
    end;

    [EventSubscriber(ObjectType::Table, Database::"XS Line", 'OnModeTwoNoParams', '', false, false)]
    local procedure OnModeTwoNoParams(var Line: Record "XS Line")
    begin
        Line.ExtRead('X');
    end;

    [EventSubscriber(ObjectType::Table, Database::"XS Line", 'OnModeTwo', '', false, false)]
    local procedure OnModeTwo(var Line: Record "XS Line"; var Other: Record "XS Other")
    begin
        Line.ExtRead('X2');
        Line.TableRead('P2');
    end;

    [EventSubscriber(ObjectType::Table, Database::"XS Line", 'OnModeSender', '', false, false)]
    local procedure OnModeSender(var Sender: Record "XS Line"; var Line: Record "XS Line")
    begin
        Line.ExtRead('XE');
    end;

    [EventSubscriber(ObjectType::Table, Database::"XS Line", 'OnModeByValueSecond', '', false, false)]
    local procedure OnModeByValueSecond(var Line: Record "XS Line"; Other: Record "XS Other")
    begin
        Line.ExtRead('XB');
    end;

    [EventSubscriber(ObjectType::Table, Database::"XS Line", 'OnModeByValue', '', false, false)]
    local procedure OnModeByValue(Line: Record "XS Line")
    begin
        Line.ExtRead('XV');
        Line.TableRead('PV');
    end;

    [EventSubscriber(ObjectType::Table, Database::"XS Line", 'OnBeforeModifyEvent', '', false, false)]
    local procedure OnBeforeModify(var Rec: Record "XS Line"; var xRec: Record "XS Line"; RunTrigger: Boolean)
    var
        Log: Codeunit "XS Log";
    begin
        if Log.Mode() = 'modify-event' then
            Rec.ExtRead('XT');
    end;
}

codeunit 50205 "XS Manual Subscriber"
{
    EventSubscriberInstance = Manual;

    [EventSubscriber(ObjectType::Table, Database::"XS Line", 'OnModeManual', '', false, false)]
    local procedure OnModeManual(var Line: Record "XS Line")
    begin
        Line.ExtRead('XM');
    end;
}

codeunit 50209 "XS Single Subscriber"
{
    SingleInstance = true;

    [EventSubscriber(ObjectType::Table, Database::"XS Line", 'OnModeSingle', '', false, false)]
    local procedure OnModeSingle(var Line: Record "XS Line")
    begin
        Line.ExtRead('X');
    end;

    [EventSubscriber(ObjectType::Table, Database::"XS Line", 'OnModeTwoSingle', '', false, false)]
    local procedure OnModeTwoSingle(var Line: Record "XS Line")
    begin
    end;
}

codeunit 50210 "XS Second Subscriber"
{
    [EventSubscriber(ObjectType::Table, Database::"XS Line", 'OnModeTwoSubs', '', false, false)]
    local procedure OnModeTwoSubs(var Line: Record "XS Line")
    begin
    end;

    // Base Application's Asm. Requisition Line: only the record of a two-parameter event.
    [EventSubscriber(ObjectType::Table, Database::"XS Line", 'OnModeWide', '', false, false)]
    local procedure OnModeWide(var Line: Record "XS Line")
    begin
    end;

    [EventSubscriber(ObjectType::Table, Database::"XS Line", 'OnModeTwoNoParams', '', false, false)]
    local procedure OnModeTwoNoParams()
    begin
    end;
}

codeunit 50211 "XS Single Helper"
{
    SingleInstance = true;
    procedure CallExt(var Line: Record "XS Line") begin Line.ExtRead('C'); end;
    procedure ValidateMode(var Line: Record "XS Line") begin Line.Validate(Mode, Line.Mode::Production); end;
}

// The callers of the Validate record/validate/XRecTableProcCallers.al measured (corpus PR 1482): Codeunit.Run and a report.
codeunit 50212 "XS Runner"
{
    TableNo = "XS Line";

    trigger OnRun()
    var
        Log: Codeunit "XS Log";
    begin
        case Log.Mode() of
            'nested':
                Rec.Validate("Item No.", 'I2');
            'cu-ext':
                Rec.Validate(ViaCodeunit, true);
            else
                Rec.Validate(Mode, Rec.Mode::Production);
        end;
    end;
}

report 50213 "XS Report"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset
    {
        dataitem(Line; "XS Line")
        {
            trigger OnAfterGetRecord()
            begin
                Line.Series := 'NEW';
                Codeunit.Run(Codeunit::"XS Runner", Line);
            end;
        }
    }
}

report 50214 "XS Direct Report"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset
    {
        dataitem(Line; "XS Line")
        {
            trigger OnAfterGetRecord()
            begin
                Line.Series := 'NEW';
                Line.Validate(Mode, Line.Mode::Production);
            end;
        }
    }
}

// Base Application's Calculate Plan - Plan. Wksh.: a Commit, then the codeunit run guarded.
report 50215 "XS Guarded Report"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset
    {
        dataitem(Line; "XS Line")
        {
            trigger OnAfterGetRecord()
            var
                Log: Codeunit "XS Log";
            begin
                Line.Series := 'NEW';
                Commit();
                if Codeunit.Run(Codeunit::"XS Runner", Line) then
                    Log.Add('run ok');
            end;
        }
    }
}

interface "XS IValidator"
{
    procedure ValidateMode(var Line: Record "XS Line");
}

codeunit 50216 "XS Iface Validator" implements "XS IValidator"
{
    procedure ValidateMode(var Line: Record "XS Line") begin Line.Validate(Mode, Line.Mode::Production); end;
}

codeunit 50206 "XS Helper"
{
    procedure CallExt(var Line: Record "XS Line") begin Line.ExtRead('C'); end;
    procedure CallBase(var Line: Record "XS Line") begin Line.TableRead('CP'); end;
    procedure CallExtAssign(var Line: Record "XS Line") begin Line.ExtAssign(); end;
    procedure ValidateMode(var Line: Record "XS Line") begin Line.Validate(Mode, Line.Mode::Production); end;
    procedure ValidateViaCodeunit(var Line: Record "XS Line") begin Line.Validate(ViaCodeunit, true); end;
    [TryFunction] procedure TryValidateMode(var Line: Record "XS Line") begin Line.Validate(Mode, Line.Mode::Production); end;
    [TryFunction] procedure TryCallExt(var Line: Record "XS Line") begin Line.ExtRead('CT'); end;
}

codeunit 50207 "XS Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "XS Log";

    local procedure AfterInsert(var Line: Record "XS Line")
    begin
        Log.Set('');
        Line.DeleteAll();
        Line.Init();
        Line."Line No." := 1;
        Line.Series := 'OLD';
        Line."Item No." := 'I1';
        Line.Insert(false);
        Line.Series := 'NEW';
    end;

    local procedure AfterGet(var Line: Record "XS Line")
    begin
        AfterInsert(Line);
        Clear(Line);
        Line.Get(1);
        Line.Series := 'NEW';
    end;

    local procedure ValidateMode(var Line: Record "XS Line"; CfgMode: Text; Expected: Text)
    begin
        Log.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
        Check(Expected);
    end;

    local procedure ValidateItem(var Line: Record "XS Line"; CfgMode: Text; Expected: Text)
    begin
        Log.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
        Check(Expected);
    end;

    local procedure ValidateViaCodeunit(var Line: Record "XS Line"; CfgMode: Text; Expected: Text)
    begin
        Log.Set(CfgMode);
        Line.Validate(ViaCodeunit, true);
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

    [Test] procedure Sub_AfterInsert() var L: Record "XS Line"; begin AfterInsert(L); ValidateMode(L, 'sub', 'T x=0/NEW/I1;X x=0/NEW/I1;P x=0/NEW/I1;'); end;
    [Test] procedure Sub_AfterGet() var L: Record "XS Line"; begin AfterGet(L); ValidateMode(L, 'sub', 'T x=0/NEW/I1;X x=0/NEW/I1;P x=0/NEW/I1;'); end;
    [Test] procedure Sub_FreshRecord() var L: Record "XS Line"; begin L.DeleteAll(); L."Line No." := 1; L.Series := 'NEW'; L."Item No." := 'I1'; ValidateMode(L, 'sub', 'T x=0/NEW/I1;X x=0/NEW/I1;P x=0/NEW/I1;'); end;
    [Test] procedure Sub_Temporary() var L: Record "XS Line" temporary; begin AfterInsert(L); ValidateMode(L, 'sub', 'T x=0/NEW/I1;X x=0/NEW/I1;P x=0/NEW/I1;'); end;
    [Test] procedure Sub_TwoVarParams() var L: Record "XS Line"; begin AfterInsert(L); ValidateMode(L, 'sub-two', 'T x=0/NEW/I1;X2 x=0/NEW/I1;P2 x=0/NEW/I1;'); end;
    [Test] procedure Sub_Assign() var L: Record "XS Line"; begin AfterInsert(L); ValidateMode(L, 'sub-assign', 'T x=0/NEW/I1;XS x=0/NEW/I1;T after x=0/SET/I1;'); end;
    [Test] procedure Nested_NoProcedure() var L: Record "XS Line"; begin AfterInsert(L); ValidateItem(L, 'nested0', 'I x=0/NEW/I1;T x=0/NEW/I1;X x=0/NEW/I1;P x=0/NEW/I1;'); end;
    [Test] procedure Nested_OneProcedure() var L: Record "XS Line"; begin AfterInsert(L); ValidateItem(L, 'nested1', 'I x=0/NEW/I1;T x=0/NEW/I1;X x=0/NEW/I1;P x=0/NEW/I1;'); end;
    [Test] procedure Nested_TwoProcedures() var L: Record "XS Line"; begin AfterInsert(L); ValidateItem(L, 'nested', 'I x=0/NEW/I1;T x=0/NEW/I1;X x=0/NEW/I1;P x=0/NEW/I1;'); end;
    [Test] procedure Nested_ThreeProcedures() var L: Record "XS Line"; begin AfterInsert(L); ValidateItem(L, 'nested3', 'I x=0/NEW/I1;T x=0/NEW/I1;X x=0/NEW/I1;P x=0/NEW/I1;'); end;
    [Test] procedure Nested_AfterGet() var L: Record "XS Line"; begin AfterGet(L); ValidateItem(L, 'nested', 'I x=0/NEW/I1;T x=0/NEW/I1;X x=0/NEW/I1;P x=0/NEW/I1;'); end;
    [Test] procedure RefusedAssign_Nested() var L: Record "XS Line"; begin AfterInsert(L); ValidateItem(L, 'nested-assign', 'I x=0/NEW/I1;T x=0/NEW/I1;XS x=0/NEW/I1;T after x=0/SET/I1;I after x=0/SET/I1;'); end;
    [Test] procedure RefusedAssign_Nested_ErrorCaughtAfterAssign() var L: Record "XS Line"; begin AfterInsert(L); ValidateItem(L, 'nested-assign-err', ''); end;
    // The value assigned back: neither trigger can tell (the record was assigned before, so `AreFieldsLoaded` reads the same).
    [Test] procedure Nested_AssignedAndRestored() var L: Record "XS Line"; begin AfterInsert(L); ValidateItem(L, 'nested-assign-restore', 'I x=0/NEW/I1;T x=0/NEW/I1;'); end;
    [Test] procedure Refused_Codeunit_ThroughInterfaceFromTrigger() var L: Record "XS Line"; begin AfterInsert(L); ValidateViaCodeunit(L, 'cu-iface', ''); end;
    [Test] procedure Codeunit_ExtProc() var L: Record "XS Line"; begin AfterInsert(L); ValidateViaCodeunit(L, 'cu-ext', 'V x=0/NEW/I1;C x=0/NEW/I1;'); end;
    [Test] procedure Codeunit_BaseProc() var L: Record "XS Line"; begin AfterInsert(L); ValidateViaCodeunit(L, 'cu-base', 'V x=0/NEW/I1;CP x=0/NEW/I1;'); end;
    [Test] procedure Codeunit_ExtProc_AfterGet() var L: Record "XS Line"; begin AfterGet(L); ValidateViaCodeunit(L, 'cu-ext', 'V x=0/NEW/I1;C x=0/NEW/I1;'); end;
    [Test] procedure Codeunit_Assign() var L: Record "XS Line"; begin AfterInsert(L); ValidateViaCodeunit(L, 'cu-assign', 'V x=0/NEW/I1;XS x=0/NEW/I1;V after x=0/SET/I1;'); end;
    [Test] procedure Sub_ByValueParam_KeepsMain() var L: Record "XS Line"; begin AfterInsert(L); ValidateMode(L, 'sub-byvalue', 'T x=0/NEW/I1;XV x=0//;PV x=0//;'); end;
    [Test] procedure Sub_ReadByFindFirst() var L: Record "XS Line"; R: Record "XS Line"; begin AfterInsert(L); R.FindFirst(); R.Series := 'NEW'; ValidateMode(R, 'sub', 'T x=0/NEW/I1;X x=0/NEW/I1;P x=0/NEW/I1;'); end;
    [Test] procedure Sub_TemporaryNeverInserted() var L: Record "XS Line" temporary; begin Log.Set(''); L."Line No." := 1; L.Series := 'NEW'; L."Item No." := 'I1'; ValidateMode(L, 'sub', 'T x=0/NEW/I1;X x=0/NEW/I1;P x=0/NEW/I1;'); end;
    [Test] procedure Nested_ReadByFindLast() var L: Record "XS Line"; R: Record "XS Line"; begin AfterInsert(L); R.FindLast(); R.Series := 'NEW'; ValidateItem(R, 'nested', 'I x=0/NEW/I1;T x=0/NEW/I1;X x=0/NEW/I1;P x=0/NEW/I1;'); end;
    [Test]
    procedure Nested_FoundThenInitNewLine()
    var
        L: Record "XS Line";
        R: Record "XS Line";
    begin
        // Base Application's planning line (Inventory Profile Offsetting.MaintainPlanningLine).
        AfterInsert(L);
        R.FindLast();
        R.Init();
        R."Line No." := 2;
        R.Series := 'NEW';
        R."Item No." := 'I2';
        Log.Set('nested');
        R.Validate(R."Item No.");
        Check('I x=0/NEW/I2;T x=0/NEW/I2;X x=0/NEW/I2;P x=0/NEW/I2;');
    end;

    [Test] procedure Refused_Sub_IncludeSender() var L: Record "XS Line"; begin AfterInsert(L); ValidateMode(L, 'sub-sender', ''); end;
    [Test] procedure Refused_Sub_SecondParamByValue() var L: Record "XS Line"; begin AfterInsert(L); ValidateMode(L, 'sub-byval2', ''); end;
    [Test] procedure Refused_Sub_ManualSubscriber() var L: Record "XS Line"; M: Codeunit "XS Manual Subscriber"; begin AfterInsert(L); BindSubscription(M); ValidateMode(L, 'sub-manual', ''); end;
    [Test] procedure Refused_Sub_RaisedFromProcedure() var L: Record "XS Line"; begin AfterInsert(L); ValidateMode(L, 'sub-fromproc', ''); end;
    [Test] procedure Refused_Nested_FourProcedures() var L: Record "XS Line"; begin AfterInsert(L); ValidateItem(L, 'nested4', ''); end;
    [Test] procedure Refused_Nested_ThroughCodeunit() var L: Record "XS Line"; begin AfterInsert(L); ValidateItem(L, 'nested-cu', ''); end;
    [Test] procedure Refused_Nested_ThroughOtherRecord() var L: Record "XS Line"; begin AfterInsert(L); ValidateItem(L, 'nested-peer', ''); end;
    [Test] procedure Refused_Nested_FromExtensionTrigger() var L: Record "XS Line"; begin AfterInsert(L); Log.Set('nested-ext'); L.Validate(ExtTrig, 1); end;
    [Test] procedure Refused_Codeunit_ReadByFindFirst() var L: Record "XS Line"; R: Record "XS Line"; begin AfterInsert(L); R.FindFirst(); ValidateViaCodeunit(R, 'cu-ext', ''); end;
    [Test] procedure Refused_TableEventSubscriberAboveTrigger() var L: Record "XS Line"; begin AfterInsert(L); ValidateViaCodeunit(L, 'modify-event', ''); end;
    [Test] procedure Nested_RecDotProcedure() var L: Record "XS Line"; begin AfterInsert(L); ValidateItem(L, 'nested-recproc', 'I x=0/NEW/I1;T x=0/NEW/I1;X x=0/NEW/I1;P x=0/NEW/I1;'); end;
    [Test] procedure Nested_OneArgumentValidate() var L: Record "XS Line"; begin AfterInsert(L); ValidateItem(L, 'nested-onearg', 'I x=0/NEW/I1;T x=0/NEW/I1;X x=0/NEW/I1;P x=0/NEW/I1;'); end;
    [Test] procedure Nested_Temporary() var L: Record "XS Line" temporary; begin AfterInsert(L); ValidateItem(L, 'nested', 'I x=0/NEW/I1;T x=0/NEW/I1;X x=0/NEW/I1;P x=0/NEW/I1;'); end;
    [Test] procedure Nested_FreshRecord() var L: Record "XS Line"; begin Log.Set(''); L.DeleteAll(); L."Line No." := 1; L.Series := 'NEW'; L."Item No." := 'I1'; ValidateItem(L, 'nested', 'I x=0/NEW/I1;T x=0/NEW/I1;X x=0/NEW/I1;P x=0/NEW/I1;'); end;
    [Test] procedure Sub_ProcedureValidatesOtherField() var L: Record "XS Line"; begin AfterInsert(L); ValidateMode(L, 'sub-proc-val', 'T x=0/NEW/I1;O x=0/NEW/I1;'); end;
    [Test] procedure Codeunit_TryFunction() var L: Record "XS Line"; begin AfterInsert(L); ValidateViaCodeunit(L, 'cu-try', 'V x=0/NEW/I1;CT x=0/NEW/I1;try ok;'); end;
    // BC keeps one xRec for both Validates (I x=0/NEW/I1;T x=0/OUT/I1;X x=0/OUT/I1;...); the runner keeps two.
    [Test] procedure Refused_Nested_OuterAssignsXRecFirst() var L: Record "XS Line"; begin AfterInsert(L); ValidateItem(L, 'nested-outer-assign', ''); end;
    // BC: the procedures and the outer trigger read the inner trigger's assignment (X x=0/INN/I1;...;I after x=0/INN/I1).
    [Test] procedure Refused_Nested_InnerAssignsXRecFirst() var L: Record "XS Line"; begin AfterInsert(L); ValidateItem(L, 'nested-inner-assign', ''); end;
    // BC: the filter shows in both triggers (T filters=Series: F;I filters=Series: F).
    // Not measured: any other change of the shared xRec's view refuses the same way.
    [Test] procedure RefusedAssign_Nested_ProcedureMarksXRec() var L: Record "XS Line"; begin AfterInsert(L); ValidateItem(L, 'nested-marked', ''); end;
    // Shapes no cell measured (review of PR 8304, 2026-11-20): each refuses a read of xRec.
    [Test] procedure Refused_Sub_SingleInstanceSubscriber() var L: Record "XS Line"; begin AfterInsert(L); ValidateMode(L, 'sub-single', ''); end;
    [Test] procedure Refused_Codeunit_SingleInstanceHelper() var L: Record "XS Line"; begin AfterInsert(L); ValidateViaCodeunit(L, 'cu-single', ''); end;
    [Test] procedure Refused_Sub_ModifyBeforeRead() var L: Record "XS Line"; begin AfterInsert(L); ValidateMode(L, 'sub-modify', ''); end;
    [Test] procedure Refused_Sub_GetBeforeRead() var L: Record "XS Line"; begin AfterInsert(L); ValidateMode(L, 'sub-get', ''); end;
    [Test] procedure Refused_Sub_OtherFieldValidatedFirst() var L: Record "XS Line"; begin AfterInsert(L); ValidateMode(L, 'sub-validate-first', ''); end;
    [Test] procedure Refused_Sub_ValidateFromOnInsert() var L: Record "XS Line"; begin Log.Set(''); L.DeleteAll(); L."Line No." := 1; L.Series := 'NEW'; L."Item No." := 'I1'; Log.Set('sub-oninsert'); L.Insert(true); end;
    // Rec.OtherProc() inside the procedure that shares the trigger's xRec: not measured; the unqualified call shares it.
    [Test] procedure Refused_Sub_RecDotProcInSharedProc() var L: Record "XS Line"; begin AfterInsert(L); ValidateMode(L, 'sub-rec-proc', ''); end;
    [Test] procedure Sub_UnqualifiedProcInSharedProc() var L: Record "XS Line"; begin AfterInsert(L); ValidateMode(L, 'sub-own-proc', 'T x=0/NEW/I1;U x=0/NEW/I1;'); end;
    [Test] procedure RefusedAssign_Nested_ProcedureFiltersXRec() var L: Record "XS Line"; begin AfterInsert(L); ValidateItem(L, 'nested-filter', ''); end;
    // Assumption xrec-table-proc-callers-subscribers (record/validate/XRecTableProcCallers.al, corpus PR 1482): the Validate
    // called by other code than the test procedure, and an event with two subscribers.
    [Test] procedure Assumed_Sub_TwoSubscribers() var L: Record "XS Line"; begin AfterInsert(L); ValidateMode(L, 'sub-two-subs', 'T x=0/NEW/I1;X x=0/NEW/I1;'); end;
    [Test] procedure Assumed_Sub_FromCodeunitProcedure() var L: Record "XS Line"; H: Codeunit "XS Helper"; begin AfterInsert(L); Log.Set('sub'); H.ValidateMode(L); Check('T x=0/NEW/I1;X x=0/NEW/I1;P x=0/NEW/I1;'); end;
    [Test] procedure Assumed_Sub_FromCodeunitRun() var L: Record "XS Line"; begin AfterInsert(L); Log.Set('sub'); Codeunit.Run(Codeunit::"XS Runner", L); Check('T x=0/NEW/I1;X x=0/NEW/I1;P x=0/NEW/I1;'); end;
    [Test] procedure Assumed_Sub_FromTryFunction() var L: Record "XS Line"; H: Codeunit "XS Helper"; begin AfterInsert(L); Log.Set('sub'); if H.TryValidateMode(L) then Log.Add('try ok'); Check('T x=0/NEW/I1;X x=0/NEW/I1;P x=0/NEW/I1;try ok;'); end;
    [Test] procedure Assumed_Nested_FromReportThroughCodeunitRun() var L: Record "XS Line"; begin AfterInsert(L); Log.Set('nested'); L.SetRange("Line No.", 1); Report.Run(Report::"XS Report", false, false, L); Check('I x=0/NEW/I1;T x=0/NEW/I1;X x=0/NEW/I1;P x=0/NEW/I1;'); end;
    [Test] procedure Assumed_Codeunit_FromCodeunitProcedure() var L: Record "XS Line"; H: Codeunit "XS Helper"; begin AfterInsert(L); Log.Set('cu-ext'); H.ValidateViaCodeunit(L); Check('V x=0/NEW/I1;C x=0/NEW/I1;'); end;
    // Callers and subscribers no cell measured: each refuses a read of xRec.
    [Test] procedure Refused_Codeunit_FromCodeunitRun() var L: Record "XS Line"; begin AfterInsert(L); Log.Set('cu-ext'); Codeunit.Run(Codeunit::"XS Runner", L); end;
    [Test] procedure Refused_Sub_FromReportDirectly() var L: Record "XS Line"; begin AfterInsert(L); Log.Set('sub'); L.SetRange("Line No.", 1); Report.Run(Report::"XS Direct Report", false, false, L); end;
    [Test] procedure Refused_Sub_FromSingleInstanceCodeunit() var L: Record "XS Line"; H: Codeunit "XS Single Helper"; begin AfterInsert(L); Log.Set('sub'); H.ValidateMode(L); end;
    [Test] procedure Refused_Sub_TwoSubscribersOneSingleInstance() var L: Record "XS Line"; begin AfterInsert(L); ValidateMode(L, 'sub-two-single', ''); end;
    [Test] procedure Refused_Sub_TwoSubscribersOneWithoutParams() var L: Record "XS Line"; begin AfterInsert(L); ValidateMode(L, 'sub-two-noparams', ''); end;
    [Test] procedure Assumed_Sub_FromTryFunctionStatement() var L: Record "XS Line"; H: Codeunit "XS Helper"; begin AfterInsert(L); Log.Set('sub'); H.TryValidateMode(L); Check('T x=0/NEW/I1;X x=0/NEW/I1;P x=0/NEW/I1;'); end;
    [Test] procedure Refused_Sub_FromInterface() var L: Record "XS Line"; V: Codeunit "XS Iface Validator"; I: Interface "XS IValidator"; begin AfterInsert(L); Log.Set('sub'); I := V; I.ValidateMode(L); end;
    [Test] procedure Refused_Sub_FromGuardedCodeunitRunInTest() var L: Record "XS Line"; begin AfterInsert(L); Log.Set('sub'); Commit(); if Codeunit.Run(Codeunit::"XS Runner", L) then; end;
    // Base Application's planning shapes (record/validate/XRecTableProcCallers.al Wide_*, *ThroughGuardedCodeunitRun).
    [Test] procedure Assumed_Sub_TwoSubscribersOneDeclaringOnlyTheRecord() var L: Record "XS Line"; begin AfterInsert(L); ValidateMode(L, 'sub-wide', 'T x=0/NEW/I1;X x=0/NEW/I1;'); end;
    [Test] procedure Assumed_Nested_FromReportThroughGuardedCodeunitRun() var L: Record "XS Line"; begin AfterInsert(L); Log.Set('nested'); L.SetRange("Line No.", 1); Report.Run(Report::"XS Guarded Report", false, false, L); Check('I x=0/NEW/I1;T x=0/NEW/I1;X x=0/NEW/I1;P x=0/NEW/I1;run ok;'); end;
}
