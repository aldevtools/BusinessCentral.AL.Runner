//! A SystemId that Init() or Clear(SystemId) in an insert trigger or OnBeforeInsertEvent makes null: the row gets a new id and
//! the variable holds it (corpus record/lt-qr-systemid-insert-events/QrSystemIdTests.al:393-590, AL Runner issue 5803).

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;

const SRC: &str = r#"
table 69990 "SI Row"
{
    fields { field(1; Id; Integer) { } field(2; Note; Text[20]) { } }
    keys { key(PK; Id) { } }
    trigger OnInsert()
    var State: Codeunit "SI State";
    begin
        State.SetBefore(Rec.SystemId);
        case State.Mode() of
            1: Init();
            2: Clear(Rec.SystemId);
        end;
    end;
}
codeunit 69991 "SI State"
{
    SingleInstance = true;
    var M: Integer; Before: Guid;
    procedure Arm(Value: Integer) begin M := Value; end;
    procedure Mode(): Integer begin exit(M); end;
    procedure SetBefore(Value: Guid) begin Before := Value; end;
    procedure GetBefore(): Guid begin exit(Before); end;
}
codeunit 69992 "SI Ext"
{
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Table, Database::"SI Row", 'OnBeforeInsertEvent', '', false, false)]
    local procedure OnBefore(var Rec: Record "SI Row"; RunTrigger: Boolean)
    var State: Codeunit "SI State";
    begin
        if State.Mode() = 3 then begin State.SetBefore(Rec.SystemId); Rec.Init(); end;
    end;
}
codeunit 69993 T
{
    procedure Run(Mode: Integer; RunTrigger: Boolean; WithId: Boolean): Text
    var Row: Record "SI Row"; Stored: Record "SI Row"; State: Codeunit "SI State"; Sub: Codeunit "SI Ext"; Preset: Guid;
    begin
        State.Arm(Mode);
        BindSubscription(Sub);
        Row.Init(); Row.Id := 1; Row.Note := 'caller';
        if WithId then begin
            Preset := CreateGuid(); Row.SystemId := Preset; Row.Insert(RunTrigger, true);
        end else
            Row.Insert(RunTrigger);
        UnbindSubscription(Sub);
        State.Arm(0);
        Stored.Get(1);
        exit(Format(IsNullGuid(Row.SystemId)) + '|' + Format(Stored.SystemId = Row.SystemId) + '|' +
            Format(Stored.SystemId = State.GetBefore()) + '|' + Format(Stored.SystemId = Preset) + '|' + Stored.Note);
    end;
}
"#;

#[test]
fn a_systemid_cleared_during_insert_gets_a_new_one() {
    let run = |m: i32, t: bool, w: bool| {
        let mut b = al_testkit::Built::from_source(SRC);
        let (oi, ri) = b.routine_in("T", "Run");
        text(
            b.call(oi, ri, vec![Value::Integer(m), Value::Boolean(t), Value::Boolean(w)])
                .map(|(v, _)| v),
        )
    };
    // QrSystemIdTests.al:393 (Init in OnInsert), :415 (with InsertWithSystemId), :440 and :462 (Clear(SystemId)).
    assert_eq!(run(1, true, false), "No|Yes|No|No|");
    assert_eq!(run(1, true, true), "No|Yes|No|No|");
    assert_eq!(run(2, true, false), "No|Yes|No|No|caller");
    assert_eq!(run(2, true, true), "No|Yes|No|No|caller");
    // :487, :509: Init in OnBeforeInsertEvent, Insert(true) and Insert(false).
    assert_eq!(run(3, true, false), "No|Yes|No|No|");
    assert_eq!(run(3, false, false), "No|Yes|No|No|");
}

const EXT: &str = r#"
table 69994 "SX Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
    trigger OnInsert() begin Init(); end;
}
tableextension 69995 "SX Ext" extends "SX Row"
{
    trigger OnAfterInsert() begin end;
    trigger OnInsert() begin end;
}
codeunit 69996 T
{
    procedure Run()
    var Row: Record "SX Row";
    begin
        Row.Id := 1; Row.Insert(true);
    end;
}
"#;

// Not pinned: a table extension's insert triggers run when the id was cleared.
#[test]
fn a_cleared_systemid_with_table_extension_insert_triggers_is_refused() {
    let r = al_testkit::run_in(EXT, "T", "Run");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}
