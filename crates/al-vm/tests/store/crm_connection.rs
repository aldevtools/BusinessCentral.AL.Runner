//! A `TableType = CRM` table through the platform's CRM test connection (`@@test@@`): one in-memory store per registered
//! connection name, used through the default connection (corpus tests/al-language-onprem/session/Y16CrmTableConnectionOps.al
//! :143-318, codeunit 98620, BC 27.5/28.5/29.0; session/TestDatabaseTableConnectionCrm.al:128-170).

use al_builtins::native::Signal;
use al_builtins::value::Value;

const CONN: &str = "table 60291 \"ALT CRM Entity\" { TableType = CRM; Caption = 'ALT CRM Entity';
    fields { field(1; EntityId; Guid) { Caption = 'Entity Id'; } field(2; Name; Text[100]) { } field(3; Amount; Decimal) { } }
    keys { key(PK; EntityId) { } } }
table 60292 \"ALT CRM Trig\" { TableType = CRM;
    fields { field(1; Id; Guid) { } field(2; Name; Text[100]) { } }
    keys { key(PK; Id) { } }
    trigger OnInsert() begin Name := 'trig'; end; }
codeunit 50100 C {
    local procedure Cleanup()
    begin
        if HasTableConnection(TableConnectionType::CRM, 'TEST') then UnregisterTableConnection(TableConnectionType::CRM, 'TEST');
        if HasTableConnection(TableConnectionType::CRM, 'OTHER') then UnregisterTableConnection(TableConnectionType::CRM, 'OTHER');
    end;
    local procedure Reg()
    begin
        RegisterTableConnection(TableConnectionType::CRM, 'TEST', '@@test@@');
        SetDefaultTableConnection(TableConnectionType::CRM, 'TEST');
    end;
    local procedure Id(N: Integer): Guid
    var G: Guid;
    begin
        Evaluate(G, StrSubstNo('{0000000%1-0000-0000-0000-000000000000}', N));
        exit(G);
    end;
    local procedure Ins(N: Integer; Name: Text[100]; Amount: Decimal)
    var E: Record \"ALT CRM Entity\";
    begin
        E.Init(); E.EntityId := Id(N); E.Name := Name; E.Amount := Amount; E.Insert();
    end;
    local procedure Names(var E: Record \"ALT CRM Entity\"): Text
    var R: Text;
    begin
        if E.FindSet() then repeat R += E.Name + ';'; until E.Next() = 0;
        exit(R);
    end;
    [TryFunction] local procedure TryIns(N: Integer) begin Ins(N, 'dup', 0); end;
    [TryFunction] local procedure TryGet(N: Integer) var E: Record \"ALT CRM Entity\"; begin E.Get(Id(N)); end;
    [TryFunction] local procedure TryModify(N: Integer) var E: Record \"ALT CRM Entity\"; begin E.EntityId := Id(N); E.Name := 'x'; E.Modify(); end;
    [TryFunction] local procedure TryDelete(N: Integer) var E: Record \"ALT CRM Entity\"; begin E.EntityId := Id(N); E.Delete(); end;
    [TryFunction] local procedure TryCount(var N: Integer) var E: Record \"ALT CRM Entity\"; begin N := E.Count(); end;
    [TryFunction] local procedure TryRename(N: Integer; NewN: Integer) var E: Record \"ALT CRM Entity\"; begin E.Get(Id(N)); E.Rename(Id(NewN)); end;
    local procedure Err(Ok: Boolean): Text
    begin
        if Ok then exit('ok');
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure FindSetAndFilters(): Text
    var E: Record \"ALT CRM Entity\"; R: Text;
    begin
        Cleanup(); Reg();
        Ins(3, 'c', 3); Ins(1, 'a', 1); Ins(2, 'b', 2);
        R := Names(E);
        E.SetRange(Name, 'b'); R += '/' + Names(E) + Format(E.Count());
        E.Reset(); E.SetFilter(Amount, '>%1', 1); R += '/' + Names(E);
        E.Reset(); E.Ascending(false); R += '/' + Names(E);
        E.Reset(); R += '/' + Format(E.FindFirst()) + E.Name; R += '/' + Format(E.FindLast()) + E.Name;
        R += '/' + Format(E.IsEmpty());
        Cleanup();
        exit(R);
    end;

    procedure ModifyDeleteRename(): Text
    var E: Record \"ALT CRM Entity\"; R: Text; N: Integer;
    begin
        Cleanup(); Reg();
        Ins(1, 'a', 1); Ins(2, 'b', 2);
        E.Get(Id(1)); E.Name := 'a2'; R := Format(E.Modify());
        Clear(E); E.Get(Id(1)); R += '/' + E.Name + ' ' + Format(E.Amount);
        R += '/' + Format(E.Delete()) + '/' + Format(E.Get(Id(1))) + '/' + Format(E.Count());
        R += '/' + Err(TryRename(2, 4));
        R += '/' + Format(E.Get(Id(2))) + '/' + Format(E.Get(Id(4))) + E.Name;
        if TryCount(N) then; R += '/' + Format(N);
        Cleanup();
        exit(R);
    end;

    procedure Errors(): Text
    var R: Text;
    begin
        Cleanup(); Reg();
        Ins(1, 'a', 1);
        R := Err(TryGet(9)) + '#' + Err(TryIns(1)) + '#' + Err(TryModify(9)) + '#' + Err(TryDelete(9));
        Cleanup();
        exit(R);
    end;

    procedure NotDefault(): Text
    var R: Text; N: Integer;
    begin
        Cleanup();
        RegisterTableConnection(TableConnectionType::CRM, 'TEST', '@@test@@');
        R := Err(TryIns(1)) + '#' + Err(TryCount(N)) + Format(N);
        Cleanup();
        exit(R);
    end;

    procedure UnregisterAndSecond(): Text
    var E: Record \"ALT CRM Entity\"; R: Text; N: Integer;
    begin
        Cleanup(); Reg();
        Ins(1, 'a', 1);
        UnregisterTableConnection(TableConnectionType::CRM, 'TEST');
        Reg();
        if TryCount(N) then R := Format(N) else R := 'err';
        Ins(2, 'b', 2);
        RegisterTableConnection(TableConnectionType::CRM, 'OTHER', '@@test@@');
        SetDefaultTableConnection(TableConnectionType::CRM, 'OTHER');
        if TryCount(N) then R += '/' + Format(N) else R += '/err';
        Ins(3, 'c', 3);
        SetDefaultTableConnection(TableConnectionType::CRM, 'TEST');
        R += '/' + Names(E);
        Cleanup();
        exit(R);
    end;

    procedure AssignsGuid(): Text
    var E: Record \"ALT CRM Entity\"; F: Record \"ALT CRM Entity\"; First: Guid;
    begin
        Cleanup(); Reg();
        E.Init(); E.Name := 'first'; E.Insert(); First := E.EntityId;
        E.Init(); Clear(E.EntityId); E.Name := 'second'; E.Insert();
        F.Get(First);
        exit(Format(IsNullGuid(First)) + '/' + Format(First = E.EntityId) + '/' + F.Name + '/' + Format(F.Count()));
    end;

    procedure TriggerInsert(): Text
    var T: Record \"ALT CRM Trig\";
    begin
        Cleanup(); Reg();
        T.Init(); T.Insert(true);
        exit(T.Name);
    end;

    procedure UnpinnedMethod(): Text
    var E: Record \"ALT CRM Entity\";
    begin
        Cleanup(); Reg();
        E.ModifyAll(Name, 'x');
        exit('ran');
    end;

    procedure RowsAfterRollback(): Text
    var E: Record \"ALT CRM Entity\";
    begin
        Cleanup(); Reg();
        asserterror begin Ins(1, 'a', 1); Error('boom'); end;
        exit(Format(E.Count()));
    end;
}";

fn text(routine: &str) -> String {
    match al_testkit::run_in(CONN, "C", routine) {
        Ok(Value::Text(t)) => t.to_string(),
        other => panic!("{routine}: {other:?}"),
    }
}

fn refused(routine: &str, fragment: &str) {
    match al_testkit::run_in(CONN, "C", routine) {
        Err(Signal::Unsupported(why)) => assert!(why.contains(fragment), "{routine}: {why}"),
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    }
}

const MUST_REGISTER: &str = "Table connection  for table type CRM must be registered using RegisterTableConnection or cmdlet New-NAVTableConnection before it can be used.|Argument";

// Y16CrmTableConnectionOps.al:154-182 (Y16_Crm_FindSetOrderAndFilters).
#[test]
fn find_set_runs_in_key_order_and_filters_apply() {
    assert_eq!(text("FindSetAndFilters"), "a;b;c;/b;1/b;c;/c;b;a;/Yesa/Yesc/No");
}

// Y16CrmTableConnectionOps.al:184-222 (Y16_Crm_ModifyDeleteRename).
#[test]
fn modify_delete_and_rename_change_the_connection_rows() {
    assert_eq!(text("ModifyDeleteRename"), "Yes/a2 1/Yes/No/1/ok/No/Yesb/1");
}

// Y16CrmTableConnectionOps.al:224-251 (Y16_Crm_ErrorsWithConnection).
#[test]
fn a_missing_key_and_a_duplicate_raise_the_database_errors() {
    assert_eq!(
        text("Errors"),
        "The ALT CRM Entity does not exist. Identification fields and values: Entity Id='{00000009-0000-0000-0000-000000000000}'|DB:RecordNotFound#\
         The record in table ALT CRM Entity already exists. Identification fields and values: Entity Id='{00000001-0000-0000-0000-000000000000}'|DB:RecordExists#\
         The ALT CRM Entity does not exist. Identification fields and values: Entity Id='{00000009-0000-0000-0000-000000000000}'|DB:RecordNotFound#\
         The ALT CRM Entity does not exist. Identification fields and values: Entity Id='{00000009-0000-0000-0000-000000000000}'|DB:RecordNotFound"
    );
}

// Y16CrmTableConnectionOps.al:253-279 (Y16_Crm_RegisteredButNotDefault).
#[test]
fn a_registered_connection_that_is_not_the_default_is_not_used() {
    assert_eq!(text("NotDefault"), format!("{MUST_REGISTER}#{MUST_REGISTER}0"));
}

// Y16CrmTableConnectionOps.al:281-310 (Y16_Crm_RowsAfterUnregisterAndSecondConnection).
#[test]
fn each_connection_has_its_own_rows_and_unregister_drops_them() {
    assert_eq!(text("UnregisterAndSecond"), "0/0/b;");
}

// session/TestDatabaseTableConnectionCrm.al:128-170: Insert gives a null Guid primary key a fresh Guid.
#[test]
fn insert_assigns_a_guid_to_a_null_primary_key() {
    assert_eq!(text("AssignsGuid"), "No/No/first/2");
}

// Whether a CRM table's own triggers run through the test connection is not pinned.
#[test]
fn a_crm_insert_trigger_is_refused() {
    refused("TriggerInsert", "TableType = CRM");
}

#[test]
fn an_unpinned_method_on_a_crm_table_is_refused() {
    refused("UnpinnedMethod", "TableType = CRM");
}

// Whether a rollback takes back rows of the test connection is not pinned: the rows are refused after one.
#[test]
fn rows_after_a_rollback_are_refused() {
    refused("RowsAfterRollback", "rollback");
}
