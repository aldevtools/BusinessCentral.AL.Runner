//! A table's global variables live as long as the record variable (ADR-021). Each test mirrors
//! a probe test of the corpus, `tests/al-language/tableglobals/` (TestTableGlobalsRecordVariable.al,
//! TestTableGlobalsHolders.al; the corpus submodule holds them since eb46da5), measured on real
//! BC 27.5. The comment above a test names the probe test and gives file:line of the first one.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;
use al_testkit::text;
use al_vm::{Engine, Session};

const OBJECTS: &str = r#"
table 50500 "TG Row"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Payload; Integer)
        {
            trigger OnValidate()
            begin
                Counter += 1;
                Tag := 'V' + Format(Payload);
                XCounterInValidate := xRec.GetCounter();
                XTagInValidate := xRec.GetTag();
            end;
        }
        field(3; Seen; Text[100]) { }
        field(4; XSeen; Text[100]) { }
        field(5; Probe; Integer) { trigger OnValidate() begin Counter += 10; end; }
        field(6; RecSeen; Text[100]) { }
        field(7; ModSeq; Integer) { }
        field(8; Probe2; Integer)
        {
            trigger OnValidate()
            var
                LogRec: Record "TG Log";
            begin
                LogRec.Add(StrSubstNo('P2:%1|%2', Counter, Tag));
                Counter += 1;
            end;
        }
    }
    keys { key(PK; "No.") { Clustered = true; } }
    var
        Counter: Integer;
        Tag: Text[50];
        XCounterInValidate: Integer;
        XTagInValidate: Text[50];
        Child: Record "TG Row";
        ModRuns: Integer;

    trigger OnInsert()
    begin
        Seen := StrSubstNo('%1|%2', Counter, Tag);
        RecSeen := StrSubstNo('%1|%2', Rec.GetCounter(), Rec.GetTag());
    end;

    trigger OnModify()
    begin
        ModRuns += 1;
        ModSeq := ModRuns;
        Seen := StrSubstNo('%1|%2', Counter, Tag);
        XSeen := StrSubstNo('%1|%2', xRec.GetCounter(), xRec.GetTag());
        RecSeen := StrSubstNo('%1|%2', Rec.GetCounter(), Rec.GetTag());
    end;

    trigger OnDelete()
    var
        LogRec: Record "TG Log";
    begin
        LogRec.Add(StrSubstNo('D:%1|%2', Counter, Tag));
        Counter += 1;
    end;

    procedure Bump(): Integer begin Counter += 1; exit(Counter); end;
    procedure GetCounter(): Integer begin exit(Counter); end;
    procedure SetTag(NewTag: Text[50]) begin Tag := NewTag; end;
    procedure GetTag(): Text[50] begin exit(Tag); end;
    procedure GetXCounterInValidate(): Integer begin exit(XCounterInValidate); end;
    procedure GetXTagInValidate(): Text[50] begin exit(XTagInValidate); end;
    procedure ResetSelf() begin Rec.Reset(); end;
    procedure ChildBump(): Integer begin exit(Child.Bump()); end;
    procedure ChildCounter(): Integer begin exit(Child.GetCounter()); end;
    procedure TakeFrom(var Other: Record "TG Row") begin Rec := Other; end;
    procedure CopyFromRow(var Other: Record "TG Row") begin Rec.Copy(Other); end;
    procedure TransferFromRow(var Other: Record "TG Row") begin Rec.TransferFields(Other); end;
    procedure ByVar(var R: Record "TG Row"): Integer begin exit(R.Bump()); end;
    procedure ByValue(R: Record "TG Row"): Integer begin exit(R.Bump()); end;
    procedure SelfByValue(): Integer begin exit(ByValue(Rec)); end;
    procedure Make() Result: Record "TG Row" begin Result.Bump(); Result.Bump(); end;
    procedure Echo(R: Record "TG Row") Result: Record "TG Row" begin Result := R; end;
}

table 50501 "TG Log"
{
    fields { field(1; "Entry No."; Integer) { } field(2; Note; Text[100]) { } }
    keys { key(PK; "Entry No.") { Clustered = true; } }
    procedure Add(NewNote: Text[100])
    var
        LogRec: Record "TG Log";
    begin
        if LogRec.FindLast() then
            LogRec."Entry No." += 1
        else
            LogRec."Entry No." := 1;
        LogRec.Note := NewNote;
        LogRec.Insert();
    end;
}

tableextension 50502 "TG Row Ext" extends "TG Row"
{
    fields
    {
        field(50502; ExtPayload; Integer)
        {
            trigger OnValidate()
            begin
                ExtCounter += 1;
                ExtTag := 'E' + Format(ExtPayload);
            end;
        }
    }
    var
        ExtCounter: Integer;
        ExtTag: Text[50];
    procedure ExtBump(): Integer begin ExtCounter += 1; exit(ExtCounter); end;
    procedure ExtGetCounter(): Integer begin exit(ExtCounter); end;
    procedure ExtGetTag(): Text[50] begin exit(ExtTag); end;
}

codeunit 50504 "TG Holder"
{
    var
        Row: Record "TG Row";
    procedure BumpRow(): Integer begin exit(Row.Bump()); end;
    procedure RowCounter(): Integer begin exit(Row.GetCounter()); end;
    procedure ValidateRow(NewPayload: Integer) begin Row.Validate(Payload, NewPayload); end;
    procedure RowTag(): Text[50] begin exit(Row.GetTag()); end;
    procedure ResetRow() begin Row.Reset(); end;
    procedure LocalBump(): Integer
    var
        LocalRow: Record "TG Row";
    begin
        exit(LocalRow.Bump());
    end;
    procedure SetRowFrom(var Other: Record "TG Row") begin Row := Other; end;
}

codeunit 50505 "TG Subscribers"
{
    [EventSubscriber(ObjectType::Table, Database::"TG Row", 'OnBeforeInsertEvent', '', false, false)]
    local procedure BeforeInsert(var Rec: Record "TG Row"; RunTrigger: Boolean)
    var
        LogRec: Record "TG Log";
    begin
        LogRec.Add(StrSubstNo('BI:%1|%2', Rec.GetCounter(), Rec.GetTag()));
    end;

    [EventSubscriber(ObjectType::Table, Database::"TG Row", 'OnAfterInsertEvent', '', false, false)]
    local procedure AfterInsert(var Rec: Record "TG Row"; RunTrigger: Boolean)
    var
        LogRec: Record "TG Log";
    begin
        LogRec.Add(StrSubstNo('AI:%1|%2', Rec.GetCounter(), Rec.GetTag()));
    end;

    [EventSubscriber(ObjectType::Table, Database::"TG Row", 'OnBeforeModifyEvent', '', false, false)]
    local procedure BeforeModify(var Rec: Record "TG Row"; var xRec: Record "TG Row"; RunTrigger: Boolean)
    var
        LogRec: Record "TG Log";
    begin
        LogRec.Add(StrSubstNo('BM:%1|%2/%3|%4', Rec.GetCounter(), Rec.GetTag(), xRec.GetCounter(), xRec.GetTag()));
    end;

    [EventSubscriber(ObjectType::Table, Database::"TG Row", 'OnAfterModifyEvent', '', false, false)]
    local procedure AfterModify(var Rec: Record "TG Row"; var xRec: Record "TG Row"; RunTrigger: Boolean)
    var
        LogRec: Record "TG Log";
    begin
        LogRec.Add(StrSubstNo('AM:%1|%2/%3|%4', Rec.GetCounter(), Rec.GetTag(), xRec.GetCounter(), xRec.GetTag()));
    end;

    [EventSubscriber(ObjectType::Table, Database::"TG Row", 'OnBeforeValidateEvent', 'Probe', false, false)]
    local procedure BeforeValidateProbe(var Rec: Record "TG Row"; var xRec: Record "TG Row"; CurrFieldNo: Integer)
    var
        LogRec: Record "TG Log";
    begin
        Rec.Bump();
        LogRec.Add(StrSubstNo('BV:%1/%2', Rec.GetCounter(), xRec.GetCounter()));
        xRec.Bump();
    end;

    [EventSubscriber(ObjectType::Table, Database::"TG Row", 'OnAfterValidateEvent', 'Probe', false, false)]
    local procedure AfterValidateProbe(var Rec: Record "TG Row"; var xRec: Record "TG Row"; CurrFieldNo: Integer)
    var
        LogRec: Record "TG Log";
    begin
        LogRec.Add(StrSubstNo('AV:%1/%2', Rec.GetCounter(), xRec.GetCounter()));
    end;
}

codeunit 50506 "TG Runner"
{
    TableNo = "TG Row";
    trigger OnRun()
    begin
        Rec.Bump();
        Rec.SetTag('ran');
    end;
}

page 50503 "TG Card"
{
    PageType = Card;
    SourceTable = "TG Row";
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { }
            field(Payload; Rec.Payload) { }
            field(Probe2; Rec.Probe2) { }
            field(LastCount; LastCount) { }
            field(AfterGetCount; AfterGetCount) { }
            field(Seen; Rec.Seen) { }
        }
    }
    actions
    {
        area(Processing)
        {
            action(BumpRec) { trigger OnAction() begin LastCount := Rec.Bump(); end; }
            action(ReadRec) { trigger OnAction() begin LastCount := Rec.GetCounter(); end; }
        }
    }
    trigger OnAfterGetRecord() begin AfterGetCount := Rec.GetCounter(); end;
    var
        LastCount: Integer;
        AfterGetCount: Integer;
}

page 50507 "TG List"
{
    PageType = List;
    SourceTable = "TG Row";
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("No."; Rec."No.") { }
                field(Payload; Rec.Payload) { }
                field(LastCount; LastCount) { }
            }
        }
    }
    actions
    {
        area(Processing)
        {
            action(BumpRec) { trigger OnAction() begin LastCount := Rec.Bump(); end; }
        }
    }
    var
        LastCount: Integer;
}
"#;

fn run(body: &str, routine: &str) -> Result<Value, Signal> {
    let src = format!("{OBJECTS}\ncodeunit 50100 C {{ {HELPERS} {body} }}");
    let l = al_testkit::compile(&src);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == routine)
        .unwrap();
    let mut seen = std::collections::BTreeSet::new();
    let mut stack = vec![(oi, ri)];
    while let Some((o, r)) = stack.pop() {
        if !seen.insert((o, r)) {
            continue;
        }
        if let Some(why) = l.unsupported[o][r].first() {
            return Err(Signal::Unsupported(why.clone()));
        }
    }
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

const HELPERS: &str = r#"
    local procedure LogText(): Text
    var
        LogRec: Record "TG Log";
        Result: Text;
    begin
        if LogRec.FindSet() then
            repeat
                if Result <> '' then
                    Result += ' ; ';
                Result += LogRec.Note;
            until LogRec.Next() = 0;
        exit(Result);
    end;

    local procedure LogOf(Prefix: Text): Text
    var
        LogRec: Record "TG Log";
        Result: Text;
    begin
        LogRec.SetFilter(Note, Prefix + '*');
        if LogRec.FindSet() then
            repeat
                if Result <> '' then
                    Result += ' ; ';
                Result += LogRec.Note;
            until LogRec.Next() = 0;
        exit(Result);
    end;

    local procedure ClearLog()
    var
        LogRec: Record "TG Log";
    begin
        LogRec.DeleteAll(false);
    end;

    local procedure Kept(var Row: Record "TG Row"; MethodName: Text): Text
    var
        Result: Text;
    begin
        if Row.GetTag() = 'x' then
            Result := MethodName + ':kept '
        else
            Result := MethodName + ':reset ';
        Row.SetTag('x');
        exit(Result);
    end;

    local procedure MakeRows()
    var
        Row: Record "TG Row";
    begin
        Row."No." := 'A'; Row.Insert();
        Row."No." := 'B'; Row.Insert();
        Row."No." := 'C'; Row.Insert();
    end;
"#;

fn check(body: &str, expected: &str) {
    assert_eq!(text(run(&format!("procedure F(): Text {body}"), "F")), expected);
}

// TableGlobals_PersistAcrossProcedureCalls_OnTheSameVariable (tableglobals/TestTableGlobalsRecordVariable.al:72)
#[test]
fn globals_persist_across_procedure_calls() {
    check(
        "var Row: Record \"TG Row\"; begin Row.Bump(); Row.Bump(); Row.SetTag('abc');
            exit(StrSubstNo('%1|%2', Row.GetCounter(), Row.GetTag())); end;",
        "2|abc",
    );
}

// TableGlobals_FieldTriggerAndProcedure_ShareTheGlobals (tableglobals/TestTableGlobalsRecordVariable.al:86)
#[test]
fn field_trigger_and_procedure_share_the_globals() {
    check(
        "var Row: Record \"TG Row\"; S: Text; begin
            Row.Validate(Payload, 5); S := StrSubstNo('%1|%2', Row.GetCounter(), Row.GetTag());
            Row.Validate(Payload, 7); S += StrSubstNo('|%1|%2', Row.GetCounter(), Row.GetTag());
            exit(S); end;",
        "1|V5|2|V7",
    );
}

// TableGlobals_SetByValidate_AreSeenByOnInsert (tableglobals/TestTableGlobalsRecordVariable.al:101)
#[test]
fn validate_globals_are_seen_by_on_insert() {
    check(
        "var Row: Record \"TG Row\"; begin Row.\"No.\" := 'A'; Row.Validate(Payload, 5); Row.Insert(true);
            exit(Row.Seen + '/' + Row.RecSeen); end;",
        "1|V5/1|V5",
    );
}

// TableGlobals_SecondVariable_HasItsOwnGlobals (tableglobals/TestTableGlobalsRecordVariable.al:115)
#[test]
fn a_second_variable_has_its_own_globals() {
    check(
        "var First: Record \"TG Row\"; Second: Record \"TG Row\"; S: Text; begin
            First.Bump(); First.SetTag('first');
            S := StrSubstNo('%1|%2', Second.GetCounter(), Second.GetTag());
            Second.Bump(); Second.Bump();
            exit(S + '|' + Format(First.GetCounter())); end;",
        "0||1",
    );
}

// TableGlobals_Assignment_/Copy_/TransferFields_WhatHappensToBothSides: fields move, globals do not (tableglobals/TestTableGlobalsRecordVariable.al:132, :153, :174)
#[test]
fn assignment_copy_and_transfer_fields_leave_both_sides_globals_alone() {
    for op in [
        "Target := Source",
        "Target.Copy(Source)",
        "Target.TransferFields(Source)",
    ] {
        check(
            &format!(
                "var Source: Record \"TG Row\"; Target: Record \"TG Row\"; begin
                    Source.\"No.\" := 'S'; Source.Bump(); Source.Bump(); Source.Bump(); Source.SetTag('src');
                    Target.Bump(); Target.SetTag('tgt');
                    {op};
                    exit(StrSubstNo('%1|%2|%3|%4|%5', Target.\"No.\", Target.GetCounter(), Target.GetTag(), Source.GetCounter(), Source.GetTag())); end;"
            ),
            "S|1|tgt|3|src",
        );
    }
}

// TableGlobals_PassedByVar_IsTheSameInstance (tableglobals/TestTableGlobalsRecordVariable.al:195)
#[test]
fn by_var_is_the_same_instance() {
    check(
        "var Row: Record \"TG Row\"; Helper: Record \"TG Row\"; begin Row.Bump();
            exit(StrSubstNo('%1|%2', Helper.ByVar(Row), Row.GetCounter())); end;",
        "2|2",
    );
}

// TableGlobals_PassedByValue_WhatTheCalleeSeesAndLeaves, _RecPassedByValueInsideTable_ (tableglobals/TestTableGlobalsRecordVariable.al:209)
#[test]
fn by_value_is_a_copy_with_fresh_globals() {
    check(
        "var Row: Record \"TG Row\"; Helper: Record \"TG Row\"; begin Row.Bump(); Row.Bump(); Row.SetTag('caller');
            exit(StrSubstNo('%1|%2|%3', Helper.ByValue(Row), Row.GetCounter(), Row.SelfByValue())); end;",
        "1|2|1",
    );
}

// TableGlobals_ReturnedRecord_WhatTheReceiverSees, _EchoedRecord_ (tableglobals/TestTableGlobalsRecordVariable.al:239)
#[test]
fn a_returned_record_leaves_the_receivers_globals_alone() {
    check(
        "var Row: Record \"TG Row\"; Maker: Record \"TG Row\"; Source: Record \"TG Row\"; Receiver: Record \"TG Row\"; S: Text; begin
            Row.Bump(); Row := Maker.Make(); S := Format(Row.GetCounter());
            Source.Bump(); Source.Bump(); Source.Bump();
            Receiver := Maker.Echo(Source);
            exit(S + '|' + StrSubstNo('%1|%2', Receiver.GetCounter(), Source.GetCounter())); end;",
        "1|0|3",
    );
}

// TableGlobals_Clear_ / _Init_ / _Reset_ / _ClearAll_ / _ResetInsideTable_ / _ResetOnTemporaryRecord_ (tableglobals/TestTableGlobalsRecordVariable.al:272-412)
#[test]
fn only_clear_and_reset_reset_the_globals() {
    check(
        "var Row: Record \"TG Row\"; Temp: Record \"TG Row\" temporary; S: Text; begin
            Row.Bump(); Row.SetTag('x'); Clear(Row); S := StrSubstNo('clear:%1|%2', Row.GetCounter(), Row.GetTag());
            Row.Bump(); Row.SetTag('x'); Row.Init(); S += StrSubstNo(' init:%1|%2', Row.GetCounter(), Row.GetTag());
            Row.Reset(); S += StrSubstNo(' reset:%1|%2', Row.GetCounter(), Row.GetTag());
            Row.Bump(); ClearAll(); S += StrSubstNo(' clearall:%1', Row.GetCounter());
            Row.SetTag('x'); Row.ResetSelf(); S += StrSubstNo(' self:%1|%2', Row.GetCounter(), Row.GetTag());
            Temp.Bump(); Temp.SetTag('x'); Temp.Reset(); S += StrSubstNo(' temp:%1|%2', Temp.GetCounter(), Temp.GetTag());
            exit(S); end;",
        "clear:0| init:1|x reset:0| clearall:1 self:0| temp:0|",
    );
}

// TableGlobals_OtherOperations_WhichOnesResetTheGlobals (tableglobals/TestTableGlobalsRecordVariable.al:331)
#[test]
fn other_operations_keep_the_globals() {
    check(
        "var Row: Record \"TG Row\"; Writer: Record \"TG Row\"; S: Text; begin
            Writer.\"No.\" := 'A'; Writer.Insert();
            Row.SetTag('x');
            Row.SetRange(\"No.\", 'A'); S := Kept(Row, 'SetRange');
            Row.SetFilter(\"No.\", 'A'); S += Kept(Row, 'SetFilter');
            Row.SetCurrentKey(\"No.\"); S += Kept(Row, 'SetCurrentKey');
            Row.Mark(true); S += Kept(Row, 'Mark');
            Row.ClearMarks(); S += Kept(Row, 'ClearMarks');
            Row.SetLoadFields(\"No.\"); S += Kept(Row, 'SetLoadFields');
            Row.FindFirst(); S += Kept(Row, 'FindFirst');
            Row.SetRecFilter(); S += Kept(Row, 'SetRecFilter');
            Row.CopyFilters(Writer); S += Kept(Row, 'CopyFilters');
            Row.Validate(\"No.\", 'A'); S += Kept(Row, 'Validate');
            Row.ChangeCompany(CompanyName()); S += Kept(Row, 'ChangeCompany');
            Row.Reset(); S += Kept(Row, 'Reset');
            exit(S); end;",
        "SetRange:kept SetFilter:kept SetCurrentKey:kept Mark:kept ClearMarks:kept SetLoadFields:kept FindFirst:kept SetRecFilter:kept CopyFilters:kept Validate:kept ChangeCompany:kept Reset:reset ",
    );
}

// TableGlobals_GetFindNext_WhatHappensToTheGlobals (tableglobals/TestTableGlobalsRecordVariable.al:441)
#[test]
fn reads_keep_the_globals() {
    check(
        "var Row: Record \"TG Row\"; S: Text; begin
            MakeRows();
            Row.Bump(); Row.Get('A'); S := StrSubstNo('get:%1', Row.GetCounter());
            Row.Bump(); Row.\"No.\" := 'B'; Row.Find('='); S += StrSubstNo(' find:%1', Row.GetCounter());
            Row.Bump(); Row.FindSet(); S += StrSubstNo(' findset:%1', Row.GetCounter());
            Row.Bump(); Row.Next(); S += StrSubstNo(' next:%1', Row.GetCounter());
            Row.Reset(); Row.FindLast(); S += StrSubstNo(' findlast:%1', Row.GetCounter());
            exit(S); end;",
        "get:1 find:2 findset:3 next:4 findlast:0",
    );
}

// TableGlobals_InsertModifyDelete_WhatHappensToTheGlobals (tableglobals/TestTableGlobalsRecordVariable.al:475)
#[test]
fn writes_with_triggers_run_on_the_variable() {
    check(
        "var Row: Record \"TG Row\"; S: Text; begin
            Row.\"No.\" := 'A'; Row.Bump(); Row.Insert(true); S := StrSubstNo('ins:%1', Row.GetCounter());
            Row.Bump(); Row.Modify(true); S += StrSubstNo(' mod:%1', Row.GetCounter());
            Row.Bump(); Row.Delete(true); S += StrSubstNo(' del:%1', Row.GetCounter());
            exit(S + ' / ' + LogText()); end;",
        "ins:1 mod:2 del:4 / BI:1| ; AI:1| ; BM:2|/0| ; AM:2|/0| ; D:3|",
    );
}

// TableGlobals_ModifyTwice_OnModifyKeepsItsOwnGlobals, _Modify_OnAnotherVariableOfTheSameRow_ (tableglobals/TestTableGlobalsRecordVariable.al:525)
#[test]
fn on_modify_counts_on_the_variable() {
    check(
        "var Row: Record \"TG Row\"; Other: Record \"TG Row\"; S: Text; begin
            Row.\"No.\" := 'A'; Row.Insert(); Row.Modify(true); S := Format(Row.ModSeq);
            Row.Modify(true); S += '|' + Format(Row.ModSeq);
            Other.Get('A'); Other.Modify(true); S += '|' + Format(Other.ModSeq);
            exit(S); end;",
        "1|2|1",
    );
}

// TableGlobals_ModifyAll_WhatTheTriggersAndSubscribersSee, _DeleteAll_TriggersRunOnOneInstanceForAllRows (tableglobals/TestTableGlobalsRecordVariable.al:497, :561)
#[test]
fn bulk_operations_run_triggers_on_one_fresh_instance() {
    check(
        "var Row: Record \"TG Row\"; Writer: Record \"TG Row\"; Check: Record \"TG Row\"; S: Text; begin
            MakeRows(); ClearLog();
            Row.Bump(); Row.Bump(); Row.SetTag('t');
            Row.ModifyAll(Probe, 1, true);
            S := StrSubstNo('mod:%1|%2', Row.GetCounter(), Row.GetTag());
            Check.Get('A'); S += StrSubstNo(' A:%1/%2/%3', Check.Seen, Check.XSeen, Check.ModSeq);
            Check.Get('B'); S += StrSubstNo(' B:%1/%2/%3', Check.Seen, Check.XSeen, Check.ModSeq);
            S += ' / ' + LogText();
            ClearLog();
            Row.DeleteAll(true);
            exit(S + ' / ' + StrSubstNo('%1|%2', Row.GetCounter(), Row.GetTag()) + ' / ' + LogText()); end;",
        "mod:2|t A:0|/0|/1 B:0|/0|/2 / BM:0|/0| ; AM:0|/0| ; BM:0|/0| ; AM:0|/0| ; BM:0|/0| ; AM:0|/0| / 2|t / D:0| ; D:1| ; D:2|",
    );
}

// TableGlobals_XRecInOnModify_WhichGlobalsItHas, _XRecInOnValidate_, _XRecAcrossTwoModifies_ (tableglobals/TestTableGlobalsRecordVariable.al:582)
#[test]
fn xrec_has_its_own_globals() {
    check(
        "var Row: Record \"TG Row\"; Second: Record \"TG Row\"; S: Text; begin
            Row.\"No.\" := 'A'; Row.Insert();
            Row.Bump(); Row.Bump(); Row.Bump(); Row.SetTag('t'); Row.Modify(true);
            S := Row.Seen + '/' + Row.RecSeen + '/' + Row.XSeen;
            Row.Modify(true); S += '/' + Row.XSeen;
            Second.Bump(); Second.Bump(); Second.SetTag('t'); Second.Validate(Payload, 4);
            exit(S + '/' + StrSubstNo('%1|%2', Second.GetXCounterInValidate(), Second.GetXTagInValidate())); end;",
        "3|t/3|t/0|/0|/0|",
    );
}

// TableGlobals_XRecInOnValidate_KeepsItsGlobalsBetweenValidations, _XRecIsOneInstanceForValidateAndModify, (tableglobals/TestTableGlobalsRecordVariable.al:634-662)
// _XRecGlobals_AfterResetAndClear
#[test]
fn xrec_globals_persist_per_variable_and_survive_reset_and_clear() {
    check(
        "var Row: Record \"TG Row\"; Other: Record \"TG Row\"; S: Text; begin
            Row.Validate(Probe, 1); ClearLog();
            Row.Validate(Probe, 2); S := LogText();
            Other.\"No.\" := 'A'; Other.Insert(); Other.Validate(Probe, 1); ClearLog();
            Other.Modify(true); S += ' // ' + LogText();
            ClearLog(); Row.Reset(); Row.Validate(Probe, 3); S += ' // ' + LogText();
            ClearLog(); Clear(Row); Row.Validate(Probe, 4); S += ' // ' + LogText();
            exit(S); end;",
        "BV:12/1 ; AV:22/2 // BM:11|/1| ; AM:11|/1| // BV:1/2 ; AV:11/3 // BV:1/3 ; AV:11/4",
    );
}

// TableGlobals_Events_RecIsTheCallersInstance (tableglobals/TestTableGlobalsRecordVariable.al:697)
#[test]
fn event_subscribers_work_on_the_callers_record() {
    check(
        "var Row: Record \"TG Row\"; S: Text; begin
            Row.\"No.\" := 'A'; Row.Validate(Payload, 3); Row.Insert(true); Row.Bump(); Row.Modify(true);
            S := LogText(); ClearLog();
            Row.Validate(Probe, 1);
            exit(S + ' // ' + LogText() + ' // ' + Format(Row.GetCounter())); end;",
        "BI:1|V3 ; AI:1|V3 ; BM:2|V3/0| ; AM:2|V3/0| // BV:3/0 ; AV:13/1 // 13",
    );
}

// TableGlobals_TemporaryRecord_KeepsGlobalsBetweenCalls, _AssignedFromPersistent (tableglobals/TestTableGlobalsRecordVariable.al:725)
#[test]
fn temporary_records_behave_alike() {
    check(
        "var Temp: Record \"TG Row\" temporary; Source: Record \"TG Row\"; S: Text; begin
            Temp.\"No.\" := 'T'; Temp.Validate(Payload, 6); Temp.Insert(true); Temp.Bump();
            S := StrSubstNo('%1|%2|%3', Temp.GetCounter(), Temp.GetTag(), Temp.Seen);
            Source.\"No.\" := 'S'; Source.Bump(); Temp.Bump(); Temp := Source;
            exit(S + '/' + StrSubstNo('%1|%2', Temp.GetCounter(), Temp.\"No.\")); end;",
        "2|V6|1|V6/3|S",
    );
}

// TableGlobals_RecAssignedInsideTable_, _RecCopyInsideTable_, _RecTransferFieldsInsideTable_ (tableglobals/TestTableGlobalsRecordVariable.al:759-797)
#[test]
fn rec_assignment_inside_the_table_keeps_the_globals() {
    for call in ["TakeFrom", "CopyFromRow", "TransferFromRow"] {
        check(
            &format!(
                "var Target: Record \"TG Row\"; Other: Record \"TG Row\"; begin
                    Other.\"No.\" := 'O'; Other.Bump(); Other.Bump(); Other.Bump(); Target.Bump();
                    Target.{call}(Other);
                    exit(StrSubstNo('%1|%2|%3', Target.\"No.\", Target.GetCounter(), Other.GetCounter())); end;"
            ),
            "O|1|3",
        );
    }
}

// TableGlobals_RecordGlobalOfOwnTable_IsPerInstanceAndLazy (tableglobals/TestTableGlobalsRecordVariable.al:816)
#[test]
fn a_record_global_of_the_own_table_is_per_instance() {
    check(
        "var First: Record \"TG Row\"; Second: Record \"TG Row\"; begin
            First.ChildBump(); First.ChildBump(); Second.ChildBump();
            exit(StrSubstNo('%1|%2|%3|%4', First.ChildCounter(), Second.ChildCounter(), First.GetCounter(), Second.GetCounter())); end;",
        "2|1|0|0",
    );
}

// TableGlobals_RecordRef_GetTableSetTable_, _RecordRef_ValidateAndInsert_, _Variant_ (tableglobals/TestTableGlobalsRecordVariable.al:832-870)
#[test]
fn record_ref_and_variant_copy_fields_only() {
    check(
        "var Row: Record \"TG Row\"; Other: Record \"TG Row\"; Third: Record \"TG Row\"; RecRef: RecordRef; V: Variant; Reader: Record \"TG Row\"; S: Text; begin
            Row.\"No.\" := 'R'; Row.Bump(); Row.Bump(); Other.Bump();
            RecRef.GetTable(Row); RecRef.SetTable(Other);
            S := StrSubstNo('%1|%2|%3', Other.\"No.\", Other.GetCounter(), Row.GetCounter());
            RecRef.Close();
            RecRef.Open(Database::\"TG Row\"); RecRef.Field(1).Value := 'RR'; RecRef.Field(2).Validate(5); RecRef.Insert(true);
            Reader.Get('RR'); S += '/' + Reader.Seen;
            Third.Bump(); V := Row; Third := V;
            exit(S + '/' + StrSubstNo('%1|%2|%3', Third.\"No.\", Third.GetCounter(), Row.GetCounter())); end;",
        "R|1|2/1|V5/R|1|2",
    );
}

// TableExtensionGlobals_PersistAcrossCallsOnTheSameVariable, _CopyAndClear_ (tableglobals/TestTableGlobalsRecordVariable.al:890)
#[test]
fn table_extension_globals_follow_the_same_rules() {
    check(
        "var Row: Record \"TG Row\"; Second: Record \"TG Row\"; Source: Record \"TG Row\"; Target: Record \"TG Row\"; S: Text; begin
            Row.ExtBump(); Row.ExtBump(); Row.Validate(ExtPayload, 3); Row.Bump();
            S := StrSubstNo('%1|%2|%3|%4|%5', Row.ExtGetCounter(), Row.ExtGetTag(), Row.GetCounter(), Second.ExtGetCounter(), Second.GetCounter());
            Source.ExtBump(); Source.ExtBump(); Target.ExtBump(); Target := Source;
            S += StrSubstNo('/%1|%2', Target.ExtGetCounter(), Source.ExtGetCounter());
            Clear(Target); S += StrSubstNo(' clear:%1', Target.ExtGetCounter());
            exit(S); end;",
        "3|E3|1|0|0/1|2 clear:0",
    );
}

// TableGlobals_CodeunitGlobalRecord_PersistsAcrossCalls, _TwoCodeunitInstances_HaveTheirOwn, (tableglobals/TestTableGlobalsHolders.al:28-71)
// _LocalRecordVariable_IsFreshPerCall, _AssignmentIntoCodeunitGlobal_, _ResetOnCodeunitGlobalRecord_
#[test]
fn records_held_by_codeunits() {
    check(
        "var Holder: Codeunit \"TG Holder\"; Second: Codeunit \"TG Holder\"; Source: Record \"TG Row\"; S: Text; begin
            Holder.BumpRow(); Holder.BumpRow(); Holder.ValidateRow(8);
            S := StrSubstNo('%1|%2', Holder.RowCounter(), Holder.RowTag());
            Second.BumpRow(); S += StrSubstNo('/%1|%2', Holder.RowCounter(), Second.RowCounter());
            S += StrSubstNo('/%1|%2|%3', Holder.LocalBump(), Holder.LocalBump(), Holder.LocalBump());
            Source.\"No.\" := 'S'; Source.Bump(); Source.Bump(); Second.SetRowFrom(Source); S += '/' + Format(Second.RowCounter());
            Holder.ResetRow(); S += '/' + Format(Holder.RowCounter());
            exit(S); end;",
        "3|V8/3|1/1|1|1/1/0",
    );
}

// TableGlobals_CodeunitRunWithRecord_RunsOnTheCallersInstance (tableglobals/TestTableGlobalsRecordVariable.al:682)
#[test]
fn codeunit_run_with_a_record_runs_on_the_callers_record() {
    check(
        "var Row: Record \"TG Row\"; begin Row.Bump(); Row.Bump(); Codeunit.Run(Codeunit::\"TG Runner\", Row);
            exit(StrSubstNo('%1|%2', Row.GetCounter(), Row.GetTag())); end;",
        "3|ran",
    );
}

// TableGlobals_PageRec_PersistsAcrossActions, _ListPageRec_AcrossRows_, _PageRec_AfterGetRecordSeesTheGlobals, (tableglobals/TestTableGlobalsHolders.al:119-183)
// _NewPageInstance_StartsFresh
#[test]
fn the_page_record_keeps_its_globals_across_actions_and_rows() {
    check(
        "var Card: TestPage \"TG Card\"; List: TestPage \"TG List\"; S: Text; begin
            MakeRows();
            Card.OpenEdit(); Card.GoToKey('A');
            Card.BumpRec.Invoke(); S := Card.LastCount.Value();
            Card.BumpRec.Invoke(); S += '|' + Card.LastCount.Value();
            Card.ReadRec.Invoke(); S += '|' + Card.LastCount.Value();
            Card.GoToKey('B'); S += '/agr:' + Card.AfterGetCount.Value();
            Card.Close();
            Card.OpenEdit(); Card.GoToKey('A'); Card.BumpRec.Invoke(); S += '/fresh:' + Card.LastCount.Value();
            Card.Close();
            List.OpenEdit(); List.GoToKey('A');
            List.BumpRec.Invoke(); S += '/list:' + List.LastCount.Value();
            List.Next(); List.BumpRec.Invoke(); S += '|' + List.LastCount.Value();
            List.Next(); List.BumpRec.Invoke(); S += '|' + List.LastCount.Value();
            List.Close();
            exit(S); end;",
        "1|2|2/agr:2/fresh:1/list:1|2|3",
    );
}

// TableGlobals_PageFieldValidate_ThenAction_SeesTheTriggerGlobals, (tableglobals/TestTableGlobalsHolders.al:140)
// _PageRec_ActionGlobalsAreLostAfterPageFieldValidation, _PageFieldValidate_WhatTheTriggerSeesOfThePageRecGlobals
#[test]
fn a_page_validation_runs_on_fresh_globals_and_leaves_the_page_record_fresh() {
    check(
        "var Card: TestPage \"TG Card\"; S: Text; begin
            MakeRows(); ClearLog();
            Card.OpenEdit(); Card.GoToKey('A');
            Card.Payload.SetValue(5); Card.ReadRec.Invoke(); S := Card.LastCount.Value();
            Card.BumpRec.Invoke(); Card.BumpRec.Invoke(); Card.Probe2.SetValue(1);
            Card.ReadRec.Invoke(); S += '|' + Card.LastCount.Value();
            Card.BumpRec.Invoke(); S += '|' + Card.LastCount.Value();
            Card.Close();
            exit(S + ' // ' + LogOf('P2')); end;",
        "0|0|1 // P2:0|",
    );
}

// TableGlobals_PageInsert_OnInsertSeesTheValidateGlobals, _PageModify_OnModifySeesTheValidateGlobals (tableglobals/TestTableGlobalsHolders.al:202, :219)
#[test]
fn on_insert_and_on_modify_after_a_page_validation_see_fresh_globals() {
    check(
        "var Card: TestPage \"TG Card\"; Row: Record \"TG Row\"; S: Text; begin
            Card.OpenNew(); Card.\"No.\".SetValue('Z'); Card.Payload.SetValue(5); Card.Close();
            Row.Get('Z'); S := Row.Seen + '/' + Row.RecSeen;
            MakeRows();
            Card.OpenEdit(); Card.GoToKey('A'); Card.Payload.SetValue(5); Card.Payload.SetValue(6); Card.Close();
            Row.Get('A'); S += '/' + Row.Seen + '/' + Row.XSeen;
            exit(S); end;",
        "0|/0|/0|/0|",
    );
}

// ADR-021 decision 3: no Rc cycle. After calls (also a failing one) the record variable is
// held once, and its table instance keeps no pointer back to it.
#[test]
fn a_table_instance_holds_no_pointer_to_its_record_at_rest() {
    let src = format!(
        "{OBJECTS}\ncodeunit 50100 C {{ var Row: Record \"TG Row\";
            procedure F(): Integer begin Row.Bump(); Row.Bump(); Row.Validate(Payload, 1); exit(Row.GetCounter()); end;
            procedure G() begin Row.\"No.\" := 'A'; Row.Insert(); Row.\"No.\" := 'A'; Row.Insert(true); Row.FindFirst(); end; }}"
    );
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src)],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = l.unsupported.iter().map(|o| o.iter().map(|_| None).collect()).collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ti = l.program.objects.iter().position(|o| o.name == "TG Row").unwrap();
    let row_slot = l.program.objects[oi]
        .globals
        .iter()
        .position(|g| g.name == "Row")
        .unwrap();
    let (rec_slot, xrec_slot) = (
        l.program.objects[ti].rec_slot.unwrap(),
        l.program.objects[ti].xrec_slot.unwrap(),
    );
    let ri = |name: &str| {
        l.program.objects[oi]
            .routines
            .iter()
            .position(|r| r.name == name)
            .unwrap()
    };
    let (f, g) = (ri("F"), ri("G"));
    let mut s = Session::new(Engine::new(l.program, blocked));
    let inst = s.instance(oi);
    assert!(matches!(
        s.call(oi, f, inst.clone(), vec![]).unwrap().0,
        Value::Integer(3)
    ));
    // The second Insert fails (duplicate key, raised by the read after it) after the first call set up the instance.
    assert!(s.call(oi, g, inst.clone(), vec![]).is_err());
    let Value::Record(row) = inst.borrow().globals[row_slot].clone() else {
        panic!("Row is not a record")
    };
    // `row` here, and the codeunit's slot: two handles, no more.
    assert_eq!(std::rc::Rc::strong_count(&row), 2);
    let instances = row.borrow().globals.instances();
    assert!(!instances.is_empty());
    for (_, i) in instances {
        let i = i.borrow();
        assert!(matches!(i.globals[rec_slot], Value::Null), "Rec slot at rest");
        assert!(matches!(i.globals[xrec_slot], Value::Null), "xRec slot at rest");
    }
}
