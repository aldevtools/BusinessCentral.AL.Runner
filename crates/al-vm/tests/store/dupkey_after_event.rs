//! Gap C3-17: a statement-form duplicate-key Insert into a table with an OnAfterInsertEvent subscriber or a table-extension
//! OnAfterInsert trigger (record/lt-y38-dupkey-after-event/Y38DupKeyAfterEvent.al, corpus writer Y38, measured on BC 27.5, 28.5,
//! 29.0): the After event or trigger runs with the duplicate's values, and the error is deferred to the next access as without
//! them; a guarded Insert of the same key drops it.

const SRC: &str = r#"
table 92580 "Dk T"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { } }
    trigger OnInsert()
    var Log: Codeunit "Dk Log";
    begin
        Log.Add('TI' + Format(Id) + '(' + Name + ')');
    end;
}
table 92582 "Dk V" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { } } }
tableextension 92583 "Dk V Ext" extends "Dk V"
{
    trigger OnAfterInsert()
    var Log: Codeunit "Dk Log";
    begin
        Log.Add('XAI' + Format(Rec.Id) + '(' + Rec.Name + ')');
    end;
}
codeunit 92584 "Dk Log"
{
    SingleInstance = true;
    var Entries: Text; ReadT: Boolean;
    procedure Add(Entry: Text) begin if Entries <> '' then Entries += ','; Entries += Entry; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
    procedure SetReadT(B: Boolean) begin ReadT := B; end;
    procedure ReadsT(): Boolean begin exit(ReadT); end;
}
codeunit 92585 "Dk Subs"
{
    [EventSubscriber(ObjectType::Table, Database::"Dk T", 'OnBeforeInsertEvent', '', false, false)]
    local procedure OnBeforeInsertT(var Rec: Record "Dk T"; RunTrigger: Boolean)
    var Log: Codeunit "Dk Log";
    begin
        Log.Add('BI' + Format(Rec.Id) + '(' + Rec.Name + ')/' + Format(RunTrigger));
    end;

    [EventSubscriber(ObjectType::Table, Database::"Dk T", 'OnAfterInsertEvent', '', false, false)]
    local procedure OnAfterInsertT(var Rec: Record "Dk T"; RunTrigger: Boolean)
    var Other: Record "Dk T"; Log: Codeunit "Dk Log";
    begin
        Log.Add('AI' + Format(Rec.Id) + '(' + Rec.Name + ')/' + Format(RunTrigger) + '/sid-null=' + Format(IsNullGuid(Rec.SystemId)));
        if Log.ReadsT() then
            Log.Add('cnt=' + Format(Other.Count()));
    end;
}
codeunit 92586 C
{
    local procedure Setup()
    var T: Record "Dk T"; V: Record "Dk V"; Log: Codeunit "Dk Log";
    begin
        T.DeleteAll(); V.DeleteAll();
        T.Id := 1; T.Name := 'orig'; T.Insert(false);
        V.Id := 1; V.Name := 'orig'; V.Insert(false);
        Commit();
        Log.Take();
    end;

    local procedure InsertDup(Name: Text[30]; RunTrigger: Boolean)
    var T: Record "Dk T";
    begin
        T.Id := 1; T.Name := Name;
        T.Insert(RunTrigger);
    end;

    local procedure InsertDupV()
    var V: Record "Dk V";
    begin
        V.Id := 1; V.Name := 'dup';
        V.Insert(true);
    end;

    local procedure LastError(): Text
    begin
        exit('error:' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    local procedure Stored(): Text
    var T: Record "Dk T";
    begin
        T.Get(1);
        exit(T.Name);
    end;

    // D1 (Y38DupKeyAfterEvent.al DK_AfterEvent_Statement_ThenCount) and D6 (DK_AfterEvent_InsertFalse).
    procedure ThenCount(RunTrigger: Boolean): Text
    var T: Record "Dk T"; Log: Codeunit "Dk Log"; N: Integer; R: Text;
    begin
        Setup();
        InsertDup('dup', RunTrigger);
        R := 'ok ' + Log.Take();
        asserterror N := T.Count();
        R += ' ' + LastError();
        exit(R + ' ' + Stored());
    end;

    // D3 (DK_AfterEvent_Statement_ThenGuardedInsert).
    procedure ThenGuarded(): Text
    var T: Record "Dk T"; Log: Codeunit "Dk Log"; R: Text;
    begin
        Setup();
        InsertDup('dup', true);
        R := 'ok ' + Log.Take();
        T.Id := 1; T.Name := 'dup2';
        R += ' ' + Format(T.Insert(true)) + ' ' + GetLastErrorText() + ' ' + Log.Take();
        Commit();
        exit(R + ' ' + Format(T.Count()) + ' ' + Stored());
    end;

    // D5 (DK_AfterEvent_ReadInsideSubscriber).
    procedure ReadInside(): Text
    var Log: Codeunit "Dk Log"; R: Text;
    begin
        Setup();
        Log.SetReadT(true);
        asserterror InsertDup('dup', true);
        R := LastError() + ' ' + Log.Take();
        Log.SetReadT(false);
        exit(R + ' ' + Stored());
    end;

    // D7 (DK_ExtAfterTrigger_Statement).
    procedure ExtTrigger(): Text
    var V: Record "Dk V"; Log: Codeunit "Dk Log"; N: Integer; R: Text;
    begin
        Setup();
        InsertDupV();
        R := 'ok ' + Log.Take();
        asserterror N := V.Count();
        exit(R + ' ' + LastError());
    end;
}
"#;

const DUP_T: &str = "The record in table Dk T already exists. Identification fields and values: Id='1'";

fn call(b: &mut al_testkit::Built, routine: &str, args: Vec<al_builtins::value::Value>) -> String {
    let (oi, ri) = b.routine_in("C", routine);
    let (v, _) = b.call(oi, ri, args).unwrap_or_else(|e| panic!("{routine}: {e:?}"));
    al_testkit::text(Ok(v))
}

// Y38DupKeyAfterEvent.al D1 and D6: the After subscriber runs (RunTrigger as given), the next Count raises the duplicate.
#[test]
fn the_after_event_runs_and_the_next_read_raises_the_duplicate() {
    use al_builtins::value::Value;
    let mut b = al_testkit::Built::from_source(SRC);
    assert_eq!(
        call(&mut b, "ThenCount", vec![Value::Boolean(true)]),
        format!("ok BI1(dup)/Yes,TI1(dup),AI1(dup)/Yes/sid-null=No error:{DUP_T}|DB:RecordExists orig")
    );
    assert_eq!(
        call(&mut b, "ThenCount", vec![Value::Boolean(false)]),
        format!("ok BI1(dup)/No,AI1(dup)/No/sid-null=No error:{DUP_T}|DB:RecordExists orig")
    );
}

// Y38DupKeyAfterEvent.al D3: a guarded Insert of the same key answers false, runs no After event, and drops the pending error.
#[test]
fn a_guarded_insert_of_the_same_key_drops_the_pending_error() {
    let mut b = al_testkit::Built::from_source(SRC);
    assert_eq!(
        call(&mut b, "ThenGuarded", vec![]),
        format!("ok BI1(dup)/Yes,TI1(dup),AI1(dup)/Yes/sid-null=No No {DUP_T} BI1(dup2)/Yes,TI1(dup2) 1 orig")
    );
}

// Y38DupKeyAfterEvent.al D5: a read of the table inside the After subscriber raises the duplicate, so the Insert fails.
#[test]
fn a_read_inside_the_after_subscriber_raises_the_duplicate() {
    let mut b = al_testkit::Built::from_source(SRC);
    assert_eq!(
        call(&mut b, "ReadInside", vec![]),
        format!("error:{DUP_T}|DB:RecordExists BI1(dup)/Yes,TI1(dup),AI1(dup)/Yes/sid-null=No orig")
    );
}

// Y38DupKeyAfterEvent.al D7: a table-extension OnAfterInsert trigger runs, the next Count raises the duplicate.
#[test]
fn a_table_extension_after_trigger_runs() {
    let mut b = al_testkit::Built::from_source(SRC);
    assert_eq!(
        call(&mut b, "ExtTrigger", vec![]),
        "ok XAI1(dup) error:The record in table Dk V already exists. Identification fields and values: Id='1'|DB:RecordExists"
    );
}
