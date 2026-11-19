//! Record paths the pinned corpus does not cover refuse instead of answering (al-runner issue 7632). Each refusal has a
//! neighbor that keeps main's answer.

use al_builtins::native::Signal;
use al_builtins::value::Value;

fn text(r: Result<Value, Signal>) -> String {
    match r {
        Ok(Value::Text(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

fn refused(r: Result<Value, Signal>, what: &str) {
    match r {
        Err(Signal::Unsupported(m)) => assert!(m.contains(what), "{m}"),
        other => panic!("expected a refusal naming {what:?}, got {other:?}"),
    }
}

const DELETE: &str = r#"
table 60990 "UP Row"
{
    fields
    {
        field(1; Id; Integer) { }
    }
    keys { key(PK; Id) { } }
    trigger OnDelete()
    var L: Codeunit "UP Log";
    begin
        L.Add(Format(Id));
    end;
}
table 60989 "UP Evt Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
}
codeunit 60988 "UP Evt Sub"
{
    [EventSubscriber(ObjectType::Table, Database::"UP Evt Row", 'OnAfterDeleteEvent', '', false, false)]
    local procedure OnAfterDelete(var Rec: Record "UP Evt Row")
    var L: Codeunit "UP Log";
    begin
        L.Add('e' + Format(Rec.Id));
    end;
}
codeunit 60991 "UP Log"
{
    SingleInstance = true;
    var Log: Text;
    procedure Add(T: Text) begin Log += T + ';'; end;
    procedure Take(): Text var T: Text; begin T := Log; Log := ''; exit(T); end;
}
codeunit 50100 C
{
    local procedure Seed()
    var R: Record "UP Row";
    begin
        R.Id := 1; R.Insert();
        R.Id := 2; R.Insert();
        R.Id := 3; R.Insert();
    end;
    procedure DescendingRun(): Text
    var R: Record "UP Row"; L: Codeunit "UP Log";
    begin
        Seed();
        R.Ascending(false);
        R.SetRange(Id, 1, 2);
        R.DeleteAll(true);
        exit(L.Take());
    end;
    procedure AscendingRun(): Text
    var R: Record "UP Row"; L: Codeunit "UP Log";
    begin
        Seed();
        R.SetRange(Id, 1, 2);
        R.DeleteAll(true);
        R.Reset();
        exit(L.Take() + Format(R.Count()));
    end;
    procedure DescendingOneRow(): Text
    var R: Record "UP Row"; L: Codeunit "UP Log";
    begin
        Seed();
        R.Ascending(false);
        R.SetRange(Id, 2);
        R.DeleteAll(true);
        R.Reset();
        exit(L.Take() + Format(R.Count()));
    end;
    procedure DescendingOneEvent(): Text
    var E: Record "UP Evt Row"; L: Codeunit "UP Log";
    begin
        E.Id := 1; E.Insert();
        E.Id := 2; E.Insert();
        E.Id := 3; E.Insert();
        E.Ascending(false);
        E.SetRange(Id, 3);
        E.DeleteAll();
        E.Reset();
        exit(L.Take() + Format(E.Count()));
    end;
    procedure DescendingFieldRun(): Text
    var R: Record "UP Row"; L: Codeunit "UP Log";
    begin
        Seed();
        R.SetAscending(Id, false);
        R.SetRange(Id, 1, 2);
        R.DeleteAll(true);
        exit(L.Take());
    end;
    procedure DescendingFieldOneRow(): Text
    var R: Record "UP Row"; L: Codeunit "UP Log";
    begin
        Seed();
        R.SetAscending(Id, false);
        R.SetRange(Id, 2);
        R.DeleteAll(true);
        R.Reset();
        exit(L.Take() + Format(R.Count()));
    end;
    procedure DescendingBulk(): Text
    var R: Record "UP Row"; L: Codeunit "UP Log";
    begin
        Seed();
        R.Ascending(false);
        R.DeleteAll();
        R.Reset();
        exit(L.Take() + Format(R.Count()));
    end;
}
"#;

// DeleteAll(true) of two or more rows on a descending record with stored filters only: the order of the OnDelete calls
// is not pinned (the FlowField-filter form is, record/crud/R6961DeleteAllFlowFilter.al:258-266, and runs in
// delete_all_flow.rs). Ascending(false) and SetAscending(Field, false) are the same question.
#[test]
fn delete_all_true_of_two_rows_on_a_descending_record_refuses() {
    refused(al_testkit::run_in(DELETE, "C", "DescendingRun"), "descending");
    refused(al_testkit::run_in(DELETE, "C", "DescendingFieldRun"), "descending");
}

// Neighbor: with one matching row there is no order, and main's per-row delete stands (OnDelete and the delete event
// once, record/TestTriggerContracts.al:DeleteAll_WithTrigger_FiresForEachFilteredRecord).
#[test]
fn delete_all_of_one_row_on_a_descending_record_keeps_mains_answer() {
    assert_eq!(text(al_testkit::run_in(DELETE, "C", "DescendingOneRow")), "2;2");
    assert_eq!(text(al_testkit::run_in(DELETE, "C", "DescendingFieldOneRow")), "2;2");
    assert_eq!(text(al_testkit::run_in(DELETE, "C", "DescendingOneEvent")), "e3;2");
}

// Neighbors: ascending DeleteAll(true) runs OnDelete per row in key order
// (record/TestTriggerContracts.al:DeleteAll_WithTrigger_FiresForEachFilteredRecord); a descending DeleteAll() without
// triggers or events runs no row code and keeps main's bulk delete.
#[test]
fn delete_all_true_ascending_and_descending_bulk_keep_mains_answer() {
    assert_eq!(text(al_testkit::run_in(DELETE, "C", "AscendingRun")), "1;2;1");
    assert_eq!(text(al_testkit::run_in(DELETE, "C", "DescendingBulk")), "0");
}

const DATE_FILTER: &str = r#"
table 60992 "UP Dated"
{
    fields { field(1; Id; Integer) { } field(2; D; Date) { } }
    keys { key(PK; Id) { } }
}
codeunit 60993 "UP Date Sub"
{
    SingleInstance = true;
    var Log: Text;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Filter Helper Triggers", 'MakeDateFilter', '', false, false)]
    local procedure MakeDateFilter(var DateFilterText: Text)
    begin
        Log += 'MakeDateFilter(' + DateFilterText + ')';
        if DateFilterText = 'abc' then
            DateFilterText := '2020-12-31';
    end;
    procedure Take(): Text var T: Text; begin T := Log; Log := ''; exit(T); end;
}
codeunit 50100 C
{
    procedure English(): Text
    var R: Record "UP Dated"; S: Codeunit "UP Date Sub";
    begin
        R.SetFilter(D, 'abc');
        exit(S.Take());
    end;
    procedure UnboundManualPlaceholders(): Text
    var R: Record "UP Dated";
    begin
        asserterror R.SetFilter(D, 'abc|%1', 20201231D);
        exit(CopyStr(GetLastErrorText(), 1, 19));
    end;
    procedure Placeholders(): Text
    var R: Record "UP Dated";
    begin
        R.SetFilter(D, 'abc|%1', 20201231D);
        exit('');
    end;
}
"#;

// OnAfterMakeDateFilter is pinned for an English session without placeholders (events/platform-triggers/PtObserve.al:227-231);
// a filter with placeholders is not.
#[test]
fn make_date_filter_outside_the_pinned_shape_refuses() {
    refused(al_testkit::run_in(DATE_FILTER, "C", "Placeholders"), "MakeDateFilter");
}

#[test]
fn make_date_filter_in_the_pinned_shape_keeps_mains_answer() {
    assert_eq!(
        text(al_testkit::run_in(DATE_FILTER, "C", "English")),
        "MakeDateFilter(abc)"
    );
}

const FILTER_PAGE: &str = r#"
table 60994 "UP Flag Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Flag; Boolean) { }
        field(3; Kind; Option) { OptionMembers = A,B; }
    }
    keys { key(PK; Id) { } }
}
codeunit 50100 C
{
    procedure Flag(): Text
    var B: FilterPageBuilder;
    begin
        B.AddTable('Rows', Database::"UP Flag Row");
        B.SetView('Rows', 'WHERE(Flag=FILTER(Yes))');
        exit(B.GetView('Rows', false));
    end;
    procedure FlagNames(): Text
    var B: FilterPageBuilder;
    begin
        B.AddTable('Rows', Database::"UP Flag Row");
        B.SetView('Rows', 'WHERE(Flag=FILTER(Yes))');
        exit(B.GetView('Rows', true));
    end;
}
"#;

// A Boolean filter in FilterPageBuilder.GetView(Name, false) is not pinned; GetView(Name, true) keeps main's answer.
#[test]
fn filter_page_get_view_without_names_of_a_boolean_filter_refuses() {
    refused(al_testkit::run_in(FILTER_PAGE, "C", "Flag"), "Boolean");
    assert_eq!(
        text(al_testkit::run_in(FILTER_PAGE, "C", "FlagNames")),
        "VERSION(1) SORTING(Id) WHERE(Flag=FILTER(true))"
    );
}

const LOADED: &str = r#"
table 60995 "UP Loaded"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } field(3; Qty; Integer) { } }
    keys { key(PK; Id) { } }
}
codeunit 50100 C
{
    procedure ClearAssign(): Text
    var R: Record "UP Loaded";
    begin
        R.Id := 1; R.Name := 'one'; R.Insert();
        R.Get(1);
        Clear(R);
        R.Name := 'x';
        exit(Format(R.AreFieldsLoaded(Qty)));
    end;
    procedure ClearValidate(): Text
    var R: Record "UP Loaded";
    begin
        R.Id := 1; R.Name := 'one'; R.Insert();
        R.Get(1);
        Clear(R);
        R.Validate(Name, 'x');
        exit(Format(R.AreFieldsLoaded(Qty)));
    end;
    procedure PartialClearAssign(): Text
    var R: Record "UP Loaded";
    begin
        R.Id := 1; R.Name := 'one'; R.Insert();
        R.SetLoadFields(Name);
        R.Get(1);
        Clear(R);
        R.Name := 'x';
        exit(Format(R.AreFieldsLoaded(Qty)));
    end;
    procedure PartialClearOnly(): Text
    var R: Record "UP Loaded";
    begin
        R.Id := 1; R.Name := 'one'; R.Insert();
        R.SetLoadFields(Name);
        R.Get(1);
        Clear(R);
        exit(Format(R.AreFieldsLoaded(Name)));
    end;
    procedure ClearOnly(): Text
    var R: Record "UP Loaded";
    begin
        R.Id := 1; R.Name := 'one'; R.Insert();
        R.Get(1);
        Clear(R);
        exit(Format(R.AreFieldsLoaded(Qty)));
    end;
}
"#;

// After Get and Clear, AreFieldsLoaded is No, for a whole row (record/load-fields/Y87RecordRefLoadFields.al codeunit
// 94656 Rec_AfterClearOfFetched) and a partial one (errors/PnBErrorsCollections.al:658-659 `afterClear.name`). Any write
// after that Clear is not pinned: an assignment, a Validate, on a whole or a partial buffer.
#[test]
fn are_fields_loaded_after_get_clear_and_a_write_refuses() {
    refused(al_testkit::run_in(LOADED, "C", "ClearAssign"), "AreFieldsLoaded");
    refused(al_testkit::run_in(LOADED, "C", "ClearValidate"), "AreFieldsLoaded");
    refused(al_testkit::run_in(LOADED, "C", "PartialClearAssign"), "AreFieldsLoaded");
}

#[test]
fn are_fields_loaded_after_get_and_clear_keeps_mains_answer() {
    assert_eq!(text(al_testkit::run_in(LOADED, "C", "ClearOnly")), "No");
    assert_eq!(text(al_testkit::run_in(LOADED, "C", "PartialClearOnly")), "No");
}

const CRM: &str = r#"
table 60996 "UP CRM Entity"
{
    TableType = CRM;
    fields { field(1; EntityId; Guid) { } field(2; Amount; Decimal) { } field(3; Owner; Code[20]) { } }
    keys { key(PK; EntityId) { } }
}
table 60997 "UP Local Entry"
{
    fields { field(1; Id; Integer) { } field(2; Amount; Decimal) { } field(3; Owner; Code[20]) { } }
    keys { key(PK; Id) { } }
}
table 60998 "UP Owner"
{
    fields
    {
        field(1; Code; Code[20]) { }
        field(2; CrmTotal; Decimal) { FieldClass = FlowField; CalcFormula = sum("UP CRM Entity".Amount where(Owner = field(Code))); }
        field(3; LocalTotal; Decimal) { FieldClass = FlowField; CalcFormula = sum("UP Local Entry".Amount where(Owner = field(Code))); }
        field(4; Entity; Guid) { TableRelation = "UP CRM Entity"; }
    }
    keys { key(PK; Code) { } }
}
codeunit 50100 C
{
    local procedure Reg()
    begin
        RegisterTableConnection(TableConnectionType::CRM, 'TEST', '@@test@@');
        SetDefaultTableConnection(TableConnectionType::CRM, 'TEST');
    end;
    procedure CrmFlowField(): Text
    var O: Record "UP Owner";
    begin
        Reg();
        O.Code := 'A'; O.Insert();
        O.CalcFields(CrmTotal);
        exit(Format(O.CrmTotal));
    end;
    procedure LocalFlowField(): Text
    var O: Record "UP Owner"; E: Record "UP Local Entry";
    begin
        E.Id := 1; E.Amount := 5; E.Owner := 'A'; E.Insert();
        O.Code := 'A'; O.Insert();
        O.CalcFields(LocalTotal);
        exit(Format(O.LocalTotal));
    end;
    procedure CrmRelation(): Text
    var O: Record "UP Owner";
    begin
        Reg();
        O.Code := 'A';
        O.Validate(Entity, CreateGuid());
        exit('');
    end;
}
"#;

// A FlowField or a TableRelation over a TableType = CRM table: BC reads it through the table connection; not pinned.
#[test]
fn a_flow_field_or_relation_over_a_crm_table_refuses() {
    refused(al_testkit::run_in(CRM, "C", "CrmFlowField"), "CRM");
    refused(al_testkit::run_in(CRM, "C", "CrmRelation"), "CRM");
}

#[test]
fn a_flow_field_over_a_normal_table_keeps_mains_answer() {
    assert_eq!(text(al_testkit::run_in(CRM, "C", "LocalFlowField")), "5");
}

// A Manual subscriber codeunit that was never bound receives no MakeDateFilter call, so a filter with placeholders keeps
// main's invalid-date error.
#[test]
fn make_date_filter_with_placeholders_and_only_an_unbound_manual_subscriber_keeps_mains_answer() {
    let manual = DATE_FILTER.replace("SingleInstance = true;", "EventSubscriberInstance = Manual;");
    assert_eq!(
        text(al_testkit::run_in(&manual, "C", "UnboundManualPlaceholders")),
        "The filter \"abc|%1\""
    );
}

// A query column that averages, takes the minimum or the maximum of a FlowField first asks whether the source holds a
// row (flowfields.rs calc_has_source_rows): over a TableType = CRM table that read refuses too.
#[test]
fn a_query_max_of_a_flow_field_over_a_crm_table_refuses() {
    let src = format!(
        "{CRM}{}",
        r#"
query 60999 "UP Max Crm"
{
    elements { dataitem(O; "UP Owner") { column(MaxCrm; CrmTotal) { Method = Max; } } }
}
codeunit 50101 Q
{
    procedure MaxCrm(): Text
    var O: Record "UP Owner"; Q: Query "UP Max Crm";
    begin
        RegisterTableConnection(TableConnectionType::CRM, 'TEST', '@@test@@');
        SetDefaultTableConnection(TableConnectionType::CRM, 'TEST');
        O.Code := 'A'; O.Insert();
        Q.Open();
        if Q.Read() then exit(Format(Q.MaxCrm));
        exit('norow');
    end;
}
"#
    );
    refused(al_testkit::run_in(&src, "Q", "MaxCrm"), "CRM");
}
