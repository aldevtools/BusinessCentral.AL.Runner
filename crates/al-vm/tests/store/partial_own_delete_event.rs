//! A read of an unloaded field of a partial record in the context of its own Delete(true): the OnAfterDeleteEvent subscriber reads a
//! field the SetLoadFields did not load (al-runner issue 6924, waldo's 'Quantity to Ship' case). Delete(true) loads the whole row first,
//! so the subscriber and the caller's buffer afterwards read the stored value (record/load-fields/P170JitLoadGone.al WA_Delete_*,
//! WA_Delete_Control_PlainTable, WA_Delete_Control_TriggerTable). A row deleted through ANOTHER variable keeps the pinned JIT error.

const SRC: &str = r#"
table 50871 "OD Line"
{
    fields { field(1; "Line No."; Integer) { } field(2; Track; Integer) { } field(3; "Qty To Ship"; Decimal) { } }
    keys { key(PK; "Line No.") { Clustered = true; } }
}
codeunit 50872 Sub
{
    SingleInstance = true;
    [EventSubscriber(ObjectType::Table, Database::"OD Line", 'OnAfterDeleteEvent', '', false, false)]
    local procedure OnAfter(var Rec: Record "OD Line")
    begin
        Seen := Rec."Qty To Ship";
    end;
    var Seen: Decimal;
    procedure GetSeen(): Decimal begin exit(Seen); end;
}
codeunit 50873 C
{
    local procedure Seed()
    var L: Record "OD Line";
    begin
        L.DeleteAll();
        L."Line No." := 1; L.Track := 5; L."Qty To Ship" := 3; L.Insert();
    end;
    procedure EventAfterOwnDelete(): Text
    var L: Record "OD Line"; S: Codeunit Sub;
    begin
        Seed();
        L.SetLoadFields(Track);
        L.FindSet(true);
        repeat L.Delete(true); until L.Next() = 0;
        exit(Format(S.GetSeen()));
    end;
    procedure ReadAfterOwnDelete(): Text
    var L: Record "OD Line"; Q: Decimal;
    begin
        Seed();
        L.SetLoadFields(Track);
        L.FindFirst();
        L.Delete(true);
        Q := L."Qty To Ship";
        exit(Format(Q));
    end;
    procedure OtherVariableDeletes(): Text
    var L: Record "OD Line"; G: Record "OD Line"; Q: Decimal;
    begin
        Seed();
        L.SetLoadFields(Track);
        L.FindFirst();
        G.Get(1);
        G.Delete();
        Q := L."Qty To Ship";
        exit('done');
    end;
}
"#;

#[test]
fn an_event_read_of_an_unloaded_field_during_own_delete_true_gets_the_stored_value() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "EventAfterOwnDelete")),
        "3"
    );
}

#[test]
fn a_read_of_an_unloaded_field_after_own_delete_true_gets_the_stored_value() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "ReadAfterOwnDelete")),
        "3"
    );
}

#[test]
fn a_row_deleted_through_another_variable_keeps_the_pinned_jit_error() {
    let m = al_testkit::error_message(al_testkit::run_in(SRC, "C", "OtherVariableDeletes"));
    assert!(m.contains("JIT loading of field(s): 'Qty To Ship' failed"), "{m}");
}
