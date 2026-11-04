//! ModifyAll(Field, Value, false) over a row that has the value already
//! (record/crud/ModifyWithoutChange.al:286-365, writer QD; record/crud/RcBTriggers.al:682-722).

use al_testkit::text;

const SRC: &str = r#"
table 50390 MU { fields { field(1; Id; Integer) { } field(2; Qty; Integer) { } } keys { key(PK; Id) { } } }
table 50391 MN { fields { field(1; Id; Integer) { } field(2; Qty; Integer) { } } keys { key(PK; Id) { } } }
codeunit 50392 Subs
{
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Table, Database::MU, 'OnAfterModifyEvent', '', false, false)]
    local procedure AfterModify(var Rec: Record MU; var xRec: Record MU; RunTrigger: Boolean) begin end;
}
codeunit 50393 T
{
    procedure Unbound(): Text
    var R: Record MU; V1: BigInteger; V2: BigInteger;
    begin
        R.Id := 1; R.Qty := 5; R.Insert();
        R.Id := 2; R.Qty := 6; R.Insert();
        R.Get(1); V1 := R.SystemRowVersion;
        R.Get(2); V2 := R.SystemRowVersion;
        R.Reset();
        R.ModifyAll(Qty, 5, false);
        R.Get(1); exit(Format(R.SystemRowVersion <> V1) + Format(R.Get(2) and (R.SystemRowVersion <> V2)));
    end;

    procedure NoSubscriber(): Text
    var R: Record MN; V1: BigInteger;
    begin
        R.Id := 1; R.Qty := 5; R.Insert();
        R.Get(1); V1 := R.SystemRowVersion;
        R.Reset();
        R.ModifyAll(Qty, 5, false);
        R.Get(1); exit(Format(R.SystemRowVersion <> V1));
    end;
}
"#;

/// ModifyWithoutChange.al:318 QD_ModifyAll_SameValue_NoSubscriber: a table without triggers whose modify events a manual
/// subscriber (not bound) listens to: the row that has the value keeps its row version (`none_row1_version_changed = No`), the
/// other row gets a new one. Pinned, so no assumption is marked.
#[test]
fn modify_all_keeps_an_unchanged_row_when_a_manual_subscriber_is_declared_but_not_bound() {
    al_builtins::assumptions::clear_hits();
    assert_eq!(text(al_testkit::run_in(SRC, "T", "Unbound")), "NoYes");
    assert_eq!(al_builtins::assumptions::take_hits(), vec![]);
}

/// Without any declared modify-event subscriber ModifyAll writes in bulk: the row that has the value already gets a new
/// SystemModifiedAt (RcBTriggers.al:720 `row1StampChangedFalse = Yes`) and a new SystemRowVersion, with or without an OnModify
/// trigger, and no trigger runs (ModifyWithoutChange.al:375-448 QF_ModifyAll_SameValue_NoSubscriberDeclared_NoTrigger and
/// _OnModifyTrigger, `row1_version_changed = Yes`, corpus bump 37). Pinned, so no assumption is marked.
#[test]
fn bulk_modify_all_gives_an_unchanged_row_a_new_row_version() {
    al_builtins::assumptions::clear_hits();
    assert_eq!(text(al_testkit::run_in(SRC, "T", "NoSubscriber")), "Yes");
    assert_eq!(al_builtins::assumptions::take_hits(), vec![]);
}
