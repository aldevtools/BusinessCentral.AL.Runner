//! The xRec a procedure of a table or of one of its table extensions reads when a subscriber of the OnBeforeValidateEvent or
//! OnAfterValidateEvent of a Validate calls it on Rec (al-runner issue 8306). The first test is the reporter's repro, unchanged.

use al_builtins::assumptions::Assumption;
use al_testkit::{blocked, compile};
use al_testrun::isolation::TestIsolation;

/// Each test of codeunit `codeunit` in `src`, each rolled back on its own: its outcome (debug form) and the assumptions it ran
/// through.
fn run(src: &str, codeunit: &str) -> Vec<(String, String, Vec<Assumption>)> {
    let l = compile(src);
    let engine = al_vm::Engine::new(l.program.clone(), blocked(&l));
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == codeunit)
        .expect("test codeunit");
    let mut hits = Vec::new();
    let o = al_testrun::runtime::run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        TestIsolation::Function,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
        al_testrun::host::RunClock::begin(),
    );
    o.into_iter()
        .map(|(name, outcome)| {
            let h = hits
                .iter()
                .filter(|(n, _): &&(String, Vec<Assumption>)| *n == name)
                .flat_map(|(_, h)| h.iter().copied())
                .collect();
            (name, format!("{outcome:?}"), h)
        })
        .collect()
}

/// al-runner issue 8306, the reporter's app and test as written.
const ISSUE_8306: &str = r##"
table 50100 "Repro Val Line"
{
    fields
    {
        field(1; "Line No."; Integer) { }
        field(2; Mode; Option)
        {
            OptionMembers = Purchase,Production;
            trigger OnValidate()
            var
                XLog: Codeunit "Repro XLog";
            begin
                XLog.Add('trigger', xRec);
            end;
        }
        field(3; Series; Code[20]) { }
    }
    keys
    {
        key(PK; "Line No.") { Clustered = true; }
    }

    procedure TableProcReadsXRec()
    var
        XLog: Codeunit "Repro XLog";
    begin
        XLog.Add('table proc', xRec);
    end;
}

tableextension 50101 "Repro Val Line Ext" extends "Repro Val Line"
{
    procedure ExtProcReadsXRec()
    var
        XLog: Codeunit "Repro XLog";
    begin
        XLog.Add('ext proc', xRec);
    end;
}

codeunit 50102 "Repro XLog"
{
    SingleInstance = true;

    var
        Entries: Text;

    procedure Clear()
    begin
        Entries := '';
    end;

    procedure Add(Who: Text; Line: Record "Repro Val Line")
    begin
        Entries += StrSubstNo('%1 xRec Mode=%2 Series=%3;', Who, Format(Line.Mode, 0, 2), Line.Series);
    end;

    procedure Joined(): Text
    begin
        exit(Entries);
    end;
}

codeunit 50103 "Repro After Validate Sub"
{
    [EventSubscriber(ObjectType::Table, Database::"Repro Val Line", 'OnAfterValidateEvent', 'Mode', false, false)]
    local procedure OnAfterValidateMode(var Rec: Record "Repro Val Line"; var xRec: Record "Repro Val Line"; CurrFieldNo: Integer)
    var
        XLog: Codeunit "Repro XLog";
    begin
        XLog.Add('event', xRec);
        Rec.TableProcReadsXRec();
        Rec.ExtProcReadsXRec();
    end;
}

codeunit 50110 "Repro After Validate Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure AfterValidateSubscriberCallsProceduresOnRec()
    var
        Line: Record "Repro Val Line";
        XLog: Codeunit "Repro XLog";
    begin
        Line.DeleteAll();
        Line.Init();
        Line."Line No." := 1;
        Line.Mode := Line.Mode::Purchase;
        Line.Series := 'OLD';
        Line.Insert();
        Line.Series := 'NEW';
        XLog.Clear();
        Line.Validate(Mode, Line.Mode::Production);
        if XLog.Joined() <> 'trigger xRec Mode=0 Series=NEW;event xRec Mode=0 Series=NEW;table proc xRec Mode=0 Series=NEW;ext proc xRec Mode=0 Series=NEW;' then
            Error('Logged %1', XLog.Joined());
    end;
}
"##;

#[test]
fn issue_8306_reporter_repro_hands_the_validate_xrec_to_procedures_an_after_validate_subscriber_calls() {
    let o = run(ISSUE_8306, "Repro After Validate Tests");
    assert_eq!(o.len(), 1);
    let (name, outcome, hits) = &o[0];
    assert_eq!(outcome, "Pass", "{name}");
    assert!(hits.is_empty(), "{name}: {hits:?}");
}

/// The validate-event cells of record/validate/XRecTableProcThroughSubscriber.al (pinned at corpus 8fcde9a6; measured on a local
/// Business Central container) and their neighbors. Each test validates Mode of a record whose stored row holds Series OLD
/// and whose variable holds Series NEW; the log holds what each subscriber and procedure read of xRec.Series.
const EVENTS: &str = r##"
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
"##;

#[test]
fn validate_event_subscribers_share_the_validate_xrec_clean_and_their_neighbors_refuse() {
    let o = run(EVENTS, "VE Tests");
    let mut wrong = Vec::new();
    for (name, outcome, hits) in &o {
        let ok = if name.starts_with("KeepsMain_") {
            outcome == "Pass" && hits.is_empty()
        } else if name.starts_with("Refused_") {
            // The test's own refusal, not the distrust a refusal earlier in the codeunit puts on a later failure.
            outcome.starts_with("NotRun(\"xRec in a table procedure called through other code")
        } else {
            outcome == "Pass" && hits.is_empty()
        };
        if !ok {
            wrong.push(format!("{name}: {outcome} {hits:?}"));
        }
    }
    assert_eq!(o.len(), 34);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
