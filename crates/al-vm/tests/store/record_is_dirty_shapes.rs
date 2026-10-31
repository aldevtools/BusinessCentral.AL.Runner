//! `Record.IsDirty` in the shapes lt-ac/Y8RecordIsDirtyShapes.al pins (codeunit 91415, BC 29): a temporary record, Find forms,
//! writes with triggers and inside them, Copy, Clear, CalcFields, GetBySystemId, SetPosition, reads and writes that answer
//! false, by-value copies, RecordRef.SetTable, SystemId, InitValue fields, a partial record, a Blob, ModifyAll and DeleteAll.

use al_testkit::{run, text};

const TABLE: &str = r#"
codeunit 50110 "Dirty Log"
{
    SingleInstance = true;
    var
        Entries: Text;
    procedure Add(Entry: Text)
    begin
        Entries += Entry + ';';
    end;
    procedure Take() Result: Text
    begin
        Result := Entries;
        Entries := '';
    end;
}

table 50100 "Dirty Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; Amount; Decimal) { InitValue = 7; }
        field(4; Stamp; Text[30]) { }
        field(5; Total; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("Dirty Row" where(Id = field(Id)));
        }
        field(6; Data; Blob) { }
        field(7; Flag; Boolean) { InitValue = true; }
    }
    keys { key(PK; Id) { Clustered = true; } }

    trigger OnInsert()
    var
        Log: Codeunit "Dirty Log";
    begin
        Log.Add('ins:' + Format(Rec.IsDirty()));
        Stamp := 'by OnInsert';
    end;

    trigger OnModify()
    var
        Log: Codeunit "Dirty Log";
    begin
        Log.Add('mod:' + Format(Rec.IsDirty()) + '/x:' + Format(xRec.IsDirty()));
        Stamp := 'by OnModify';
    end;

    trigger OnDelete()
    var
        Log: Codeunit "Dirty Log";
    begin
        Log.Add('del:' + Format(Rec.IsDirty()));
        Stamp := 'by OnDelete';
    end;
}
"#;

fn with(vars: &str, body: &str) -> String {
    format!(
        "{TABLE}\ncodeunit 50101 \"Dirty Probe\"\n{{\n    local procedure Seed()\n    var\n        Row: Record \"Dirty Row\";\n        Index: Integer;\n        Log: Codeunit \"Dirty Log\";\n    begin\n        Row.DeleteAll();\n        for Index := 1 to 3 do begin\n            Row.Init();\n            Row.Id := Index;\n            Row.Name := 'n' + Format(Index);\n            Row.Insert();\n        end;\n        Log.Take();\n    end;\n\n    local procedure ByValue(R: Record \"Dirty Row\"): Text\n    begin\n        exit(Format(R.IsDirty()));\n    end;\n\n    procedure Probe(): Text\n    var\n        Row: Record \"Dirty Row\";\n        Log: Codeunit \"Dirty Log\";\n        S: Text;\n{vars}    begin\n        Seed();\n{body}\n        exit(S);\n    end;\n}}\n"
    )
}

fn got(vars: &str, body: &str) -> String {
    text(run(&with(vars, body), "Probe"))
}

fn refused(vars: &str, body: &str) -> String {
    match run(&with(vars, body), "Probe") {
        Err(al_builtins::native::Signal::Unsupported(why)) => why,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// Y8RecordIsDirtyShapes.al:119-151 Y8_IsDirty_Temporary: a temporary record behaves like a database record.
#[test]
fn a_temporary_record_is_answered() {
    let s = got(
        "        T: Record \"Dirty Row\" temporary;\n",
        r#"
        S += Format(T.IsDirty()) + ',';
        T.Init();
        T.Id := 1;
        S += Format(T.IsDirty()) + ',';
        T.Insert();
        S += Format(T.IsDirty()) + ',';
        T.Name := 'x';
        S += Format(T.IsDirty()) + ',';
        T.Modify();
        S += Format(T.IsDirty()) + ',';
        T.Name := 'y';
        T.Get(1);
        S += Format(T.IsDirty()) + ',';
        T.Name := 'z';
        T.FindFirst();
        S += Format(T.IsDirty()) + ',';
        T.Delete();
        S += Format(T.IsDirty());"#,
    );
    assert_eq!(s, "No,Yes,No,Yes,No,No,No,No");
}

// Y8RecordIsDirtyShapes.al:154-187 Y8_IsDirty_FindForms.
#[test]
fn find_forms_clean_and_next_at_the_end_keeps_dirty() {
    let s = got(
        "",
        r#"
        Row.Name := 'dirty';
        Row.Find('-');
        S += Format(Row.IsDirty()) + ',';
        Row.Name := 'dirty';
        Row.FindSet();
        S += Format(Row.IsDirty()) + ',';
        Row.Name := 'dirty';
        Row.Next();
        S += Format(Row.IsDirty()) + ',';
        Row.Name := 'dirty';
        Row.Find('=');
        S += Format(Row.IsDirty()) + ',';
        Row.Name := 'dirty';
        Row.Find('>');
        S += Format(Row.IsDirty()) + ',';
        Row.Name := 'dirty';
        S += Format(Row.Next()) + ',';
        S += Format(Row.IsDirty());"#,
    );
    assert_eq!(s, "No,No,No,No,No,0,Yes");
}

// Y8RecordIsDirtyShapes.al:189-226 Y8_IsDirty_WritesWithTriggers.
#[test]
fn writes_with_triggers_and_inside_them() {
    let s = got(
        "",
        r#"
        Row.Init();
        Row.Id := 10;
        Row.Insert(true);
        S += Format(Row.IsDirty()) + ',' + Row.Stamp + ',';
        Row.Name := 'changed';
        Row.Modify(true);
        S += Format(Row.IsDirty()) + ',';
        Row.Modify(true);
        S += Format(Row.IsDirty()) + ',';
        Row.Delete(true);
        S += Format(Row.IsDirty()) + ',' + Row.Stamp + ',';
        S += Log.Take() + ',';
        Row.Init();
        Row.Id := 11;
        Row.Insert(false);
        S += Row.Stamp + ',' + Format(Row.IsDirty());"#,
    );
    assert_eq!(
        s,
        "No,by OnInsert,No,No,Yes,by OnDelete,ins:Yes;mod:Yes/x:Yes;mod:No/x:No;del:No;,,No"
    );
}

// Y8RecordIsDirtyShapes.al:228-271 Y8_IsDirty_CopyClearCalcFields.
#[test]
fn copy_clear_calcfields_getbysystemid_setposition() {
    let s = got(
        "        Other: Record \"Dirty Row\";\n        Id: Guid;\n",
        r#"
        Row.Get(1);
        Row.Name := 'dirty';
        Other.Copy(Row);
        S += Format(Other.IsDirty()) + ',';
        Row.Get(2);
        Other.Copy(Row);
        S += Format(Other.IsDirty()) + ',';
        Row.Name := 'dirty';
        Clear(Row);
        S += Format(Row.IsDirty()) + ',';
        Row.Get(1);
        Row.CalcFields(Total);
        S += Format(Row.IsDirty()) + ',';
        Row.Total := 99;
        S += Format(Row.IsDirty()) + ',';
        Row.Get(1);
        Id := Row.SystemId;
        Row.Name := 'dirty';
        Row.GetBySystemId(Id);
        S += Format(Row.IsDirty()) + ',';
        Row.Get(2);
        Row.Name := 'dirty';
        Row.SetPosition('Id=CONST(3)');
        S += Format(Row.IsDirty()) + ',' + Row.Name;"#,
    );
    assert_eq!(s, "Yes,No,No,No,No,No,Yes,dirty");
}

// Y8RecordIsDirtyShapes.al:273-308 Y8_IsDirty_FailedReadsAndWrites.
#[test]
fn reads_and_writes_that_answer_false_keep_a_dirty_record_dirty() {
    let s = got(
        "",
        r#"
        Row.Get(1);
        Row.Name := 'dirty';
        S += Format(Row.Get(99)) + Format(Row.IsDirty()) + ',';
        Row.Get(1);
        Row.Name := 'dirty';
        Row.SetRange(Id, 50);
        S += Format(Row.FindFirst()) + Format(Row.IsDirty()) + ',';
        Row.Reset();
        Row.Get(1);
        Row.Name := 'dirty';
        S += Format(Row.Insert()) + Format(Row.IsDirty()) + ',';
        Row.Get(1);
        Row.Id := 98;
        Row.Name := 'dirty';
        S += Format(Row.Modify()) + Format(Row.IsDirty());"#,
    );
    assert_eq!(s, "NoYes,NoYes,NoYes,NoYes");
}

// Only a record that differs from its snapshot is pinned after a read that answers false.
#[test]
fn a_false_read_of_a_clean_record_is_refused() {
    let why = refused(
        "",
        "        Row.Get(1);\n        if Row.Get(99) then;\n        S := Format(Row.IsDirty());",
    );
    assert!(why.starts_with("Record.IsDirty"), "{why}");
}

// Y8RecordIsDirtyShapes.al:310-344 Y8_IsDirty_Copies.
#[test]
fn by_value_copies_and_settable_keep_the_state() {
    let s = got(
        "        Filled: Record \"Dirty Row\";\n        RR: RecordRef;\n",
        r#"
        Row.Get(1);
        Row.Name := 'dirty';
        S += ByValue(Row) + ',';
        Row.Get(1);
        S += ByValue(Row) + ',';
        RR.Open(Database::"Dirty Row");
        RR.FindFirst();
        RR.SetTable(Filled);
        S += Format(Filled.IsDirty()) + ',';
        RR.Field(2).Value := 'changed by fieldref';
        RR.SetTable(Filled);
        S += Format(Filled.IsDirty()) + ',';
        Row.Get(2);
        Row.Name := 'x';
        Row.Modify(true);
        S += Log.Take();"#,
    );
    assert_eq!(s, "Yes,No,No,Yes,mod:Yes/x:Yes;");
}

// Y8RecordIsDirtyShapes.al:346-375 Y8_IsDirty_SystemAndInitValueFields.
#[test]
fn system_id_and_init_value_fields() {
    let s = got(
        "        Fresh: Record \"Dirty Row\";\n",
        r#"
        S += Format(Fresh.IsDirty()) + ',' + Format(Fresh.Amount) + ',';
        Fresh.Init();
        S += Format(Fresh.IsDirty()) + ',';
        Fresh.Amount := 0;
        S += Format(Fresh.IsDirty()) + ',';
        Row.Get(1);
        Row.SystemId := CreateGuid();
        S += Format(Row.IsDirty()) + ',';
        Row.Get(1);
        Row.Init();
        S += Format(Row.IsDirty()) + ',' + Format(IsNullGuid(Row.SystemId));"#,
    );
    assert_eq!(s, "No,7,No,Yes,No,Yes,Yes");
}

// Y8RecordIsDirtyShapes.al:377-403 Y8_IsDirty_PartialAndBlob.
#[test]
fn partial_record_and_blob() {
    let s = got(
        "        OutS: OutStream;\n",
        r#"
        Row.SetLoadFields(Name);
        Row.Get(1);
        S += Format(Row.IsDirty()) + ',';
        Row.Amount := 3;
        S += Format(Row.IsDirty()) + ',';
        Row.SetLoadFields();
        Row.Get(1);
        Row.Data.CreateOutStream(OutS);
        OutS.WriteText('blob');
        S += Format(Row.IsDirty()) + ',';
        Row.Get(1);
        Row.CalcFields(Data);
        S += Format(Row.IsDirty());"#,
    );
    assert_eq!(s, "No,Yes,Yes,No");
}

// An unloaded field assigned the value it holds: not pinned.
#[test]
fn an_unloaded_field_assigned_its_own_value_is_refused() {
    let why = refused(
        "",
        "        Row.SetLoadFields(Name);\n        Row.Get(1);\n        Row.Amount := 7;\n        S := Format(Row.IsDirty());",
    );
    assert!(why.starts_with("Record.IsDirty"), "{why}");
}

// Y8RecordIsDirtyShapes.al:405-428 Y8_IsDirty_ModifyAllDeleteAll.
#[test]
fn modifyall_and_deleteall_leave_the_variable_alone() {
    let s = got(
        "",
        r#"
        Row.Get(1);
        Row.ModifyAll(Name, 'all');
        S += Format(Row.IsDirty()) + ',' + Row.Name + ',';
        Row.Get(1);
        Row.Name := 'dirty';
        Row.ModifyAll(Amount, 5);
        S += Format(Row.IsDirty()) + ',';
        Row.Get(1);
        Row.DeleteAll();
        S += Format(Row.IsDirty());"#,
    );
    assert_eq!(s, "No,n1,Yes,No");
}

// IsDirty inside a trigger after the trigger changed the record: not pinned.
#[test]
fn is_dirty_inside_a_trigger_after_a_change_is_refused() {
    let src = r#"
table 50120 "Dirty Trig"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { Clustered = true; } }
    trigger OnModify()
    begin
        Name := 'trigger';
        if Rec.IsDirty() then;
    end;
}
codeunit 50121 "Dirty Probe"
{
    procedure Probe(): Text
    var
        Row: Record "Dirty Trig";
    begin
        Row.Id := 1;
        Row.Insert();
        Row.Name := 'x';
        Row.Modify(true);
        exit('ran');
    end;
}
"#;
    match run(src, "Probe") {
        Err(al_builtins::native::Signal::Unsupported(why)) => assert!(why.contains("trigger changed"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
