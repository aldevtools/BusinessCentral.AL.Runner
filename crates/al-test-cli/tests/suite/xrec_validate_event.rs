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
    assert!(hits.contains(&Assumption::XrecTableProcFromValidateEvent), "{name}");
}

/// The validate-event cells of record/validate/XRecTableProcThroughSubscriber.al (corpus PR 1438, measured on a local
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
        field(2; Mode; Option) { OptionMembers = Purchase,Production; }
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

    procedure TableRead(Tag: Text) var Log: Codeunit "VE Log"; begin Log.Add(Tag + '=' + xRec.Series); end;
}

tableextension 50402 "VE Line Ext" extends "VE Line"
{
    procedure ExtRead(Tag: Text) var Log: Codeunit "VE Log"; begin Log.Add(Tag + '=' + xRec.Series); end;
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
        end;
    end;

    [EventSubscriber(ObjectType::Table, Database::"VE Line", 'OnBeforeValidateEvent', 'Mode', false, false)]
    local procedure OnBeforeValidateMode(var Rec: Record "VE Line"; var xRec: Record "VE Line"; CurrFieldNo: Integer)
    var
        Log: Codeunit "VE Log";
    begin
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
        if Log.Mode() <> 'rec-only' then
            exit;
        Rec.ExtRead('X');
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
    // The shared procedure hands Rec to a codeunit that calls a procedure on it: main's plain call (blank xRec), not a
    // refusal; the test runs through the assumption all the same.
    [Test] procedure After_DescendantThroughCodeunit_KeepsMainBlank() var L: Record "VE Line"; begin AfterInsert(L); ValidateMode(L, 'after-descendant', 'C=;'); end;

    [Test] procedure KeepsMain_ThroughCodeunit() var L: Record "VE Line"; begin AfterInsert(L); ValidateMode(L, 'after-helper', 'C=;'); end;
    [Test] procedure KeepsMain_OtherRecord() var L: Record "VE Line"; begin AfterInsert(L); ValidateMode(L, 'after-other', 'O=;'); end;
    [Test] procedure KeepsMain_XRecByValue() var L: Record "VE Line"; begin AfterInsert(L); ValidateMode(L, 'xrec-by-value', 'X=;'); end;
    [Test] procedure KeepsMain_ManualSubscriber() var L: Record "VE Line"; M: Codeunit "VE Manual Subscriber"; begin AfterInsert(L); BindSubscription(M); ValidateMode(L, 'manual', 'X=;'); end;
    [Test] procedure KeepsMain_FieldRefValidate() var L: Record "VE Line"; R: RecordRef; begin AfterInsert(L); R.GetTable(L); Log.Set('rec-only'); R.Field(2).Validate(1); Check('X=;'); end;
    [Test]
    procedure KeepsMain_TestPageSetValue()
    var
        L: Record "VE Line";
        P: TestPage "VE Card";
    begin
        AfterInsert(L);
        P.OpenEdit();
        P.GoToRecord(L);
        Log.Set('rec-only');
        P.Mode.SetValue(L.Mode::Production);
        Check('X=;');
        P.Close();
    end;
    [Test] procedure Refused_NestedInTrigger() var L: Record "VE Line"; begin AfterInsert(L); Log.Set('rec-only'); L.Validate(Outer, 1); end;
}
"##;

#[test]
fn validate_event_subscribers_share_the_validate_xrec_and_their_neighbors_keep_main() {
    use Assumption::XrecTableProcFromValidateEvent as A;
    let o = run(EVENTS, "VE Tests");
    let mut wrong = Vec::new();
    for (name, outcome, hits) in &o {
        let ok = if name.starts_with("KeepsMain_") {
            outcome == "Pass" && !hits.contains(&A)
        } else if name.starts_with("Refused_") {
            outcome.starts_with("NotRun") && outcome.contains("xRec in a table procedure called through other code")
        } else {
            outcome == "Pass" && hits.contains(&A)
        };
        if !ok {
            wrong.push(format!("{name}: {outcome} {hits:?}"));
        }
    }
    assert_eq!(o.len(), 16);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
