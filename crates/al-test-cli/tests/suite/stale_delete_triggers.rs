//! A Delete through a stale own-Insert buffer, seven Commits later, after the new transaction deleted another row of the
//! table: the row is deleted and the table's OnDelete trigger and the delete events run as for a fresh buffer
//! (al-language-testlibraries/lt-wb-6286/WbStaleDeleteTests.al:13-53, al-runner issue 6286). With other writes in the new
//! transaction (a Modify of another row) the triggers are not pinned and the Delete stays refused.

use al_testkit::outcomes_in;

const SRC: &str = r#"
table 50100 "P5 Row"
{
    fields { field(1; "No."; Code[20]) { } field(2; Note; Text[30]) { } field(3; Qty; Integer) { } }
    keys { key(PK; "No.") { } }
    trigger OnDelete() var Trace: Codeunit "P5 Trace"; begin Trace.Add('OnDelete', "No.", Note); end;
}
codeunit 50101 "P5 Trace"
{
    SingleInstance = true;
    var Lines: Text;
    procedure Add(What: Text; No: Code[20]; Note: Text) begin if Lines <> '' then Lines += ' '; Lines += What + '(' + No + ':' + Note + ')'; end;
    procedure Get(): Text begin exit(Lines); end;
    procedure Reset() begin Lines := ''; end;
}
codeunit 50102 "P5 Events"
{
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Table, Database::"P5 Row", 'OnBeforeDeleteEvent', '', false, false)]
    local procedure B(var Rec: Record "P5 Row") var Trace: Codeunit "P5 Trace"; begin Trace.Add('OnBeforeDeleteEvent', Rec."No.", Rec.Note); end;
    [EventSubscriber(ObjectType::Table, Database::"P5 Row", 'OnAfterDeleteEvent', '', false, false)]
    local procedure A(var Rec: Record "P5 Row") var Trace: Codeunit "P5 Trace"; begin Trace.Add('OnAfterDeleteEvent', Rec."No.", Rec.Note); end;
}
codeunit 50103 T
{
    Subtype = Test;
    [Test] procedure OnDeleteTrue()
    var Row: Record "P5 Row";
    begin
        Prepare(Row);
        DeleteAnother();
        Row.Delete(true);
        Verify('OnDelete(P5:start)');
    end;
    [Test] procedure EventsFalse()
    var Row: Record "P5 Row"; Ev: Codeunit "P5 Events";
    begin
        Prepare(Row);
        BindSubscription(Ev);
        DeleteAnother();
        Row.Delete(false);
        UnbindSubscription(Ev);
        Verify('OnBeforeDeleteEvent(P5-OTHER:) OnAfterDeleteEvent(P5-OTHER:) OnBeforeDeleteEvent(P5:start) OnAfterDeleteEvent(P5:start)');
    end;
    [Test] procedure ModifyOtherIsRefused()
    var Row: Record "P5 Row"; Another: Record "P5 Row";
    begin
        Prepare(Row);
        Another.Get('P5-OTHER'); Another.Qty := 9; Another.Modify(false);
        Row.Delete(true);
    end;
    local procedure Prepare(var Row: Record "P5 Row")
    var Another: Record "P5 Row"; Changer: Record "P5 Row"; Reader: Record "P5 Row"; Trace: Codeunit "P5 Trace"; i: Integer;
    begin
        Row.DeleteAll(); Commit();
        Row."No." := 'P5'; Row.Note := 'start'; Row.Insert(true);
        Another."No." := 'P5-OTHER'; Another.Insert(true);
        Changer.Get('P5'); Changer.Note := 'other'; Changer.Modify(false);
        Commit(); Commit(); Commit();
        Reader.LockTable(); Reader.FindSet();
        Another.Get('P5-OTHER'); Another.Qty := 3; Another.Modify(false);
        for i := 4 to 7 do Commit();
        Trace.Reset();
    end;
    local procedure DeleteAnother()
    var Another: Record "P5 Row";
    begin
        Another.Get('P5-OTHER'); Another.Delete(false);
    end;
    local procedure Verify(Expected: Text)
    var Stored: Record "P5 Row"; Trace: Codeunit "P5 Trace";
    begin
        if Stored.Get('P5') then Error('still there');
        if Trace.Get() <> Expected then Error('trace %1', Trace.Get());
    end;
}
"#;

#[test]
fn a_stale_delete_after_deletes_of_other_rows_runs_the_delete_triggers() {
    let o = outcomes_in(SRC, "T");
    let get = |n: &str| format!("{:?}", o.iter().find(|(name, _)| name == n).map(|(_, x)| x));
    assert_eq!(get("OnDeleteTrue"), "Some(Pass)");
    assert_eq!(get("EventsFalse"), "Some(Pass)");
    let g = get("ModifyOtherIsRefused");
    assert!(g.contains("NotRun") && g.contains("with triggers or events"), "{g}");
}
